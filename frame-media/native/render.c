#include "render.h"
#include "media_clock.h"
#include "shared_input.h"
#include <errno.h>
#include <math.h>
#include <string.h>
#include <libavcodec/avcodec.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavformat/avformat.h>
#include <libavutil/avstring.h>
#include <libavutil/display.h>
#include <libavutil/hwcontext.h>
#include <libavutil/opt.h>
#include <libavutil/pixdesc.h>
#include <libavutil/samplefmt.h>

typedef struct {
    AVFormatContext *format;
    AVIOContext *cursor;
    AVCodecContext *decoder;
    AVStream *stream;
    AVPacket *packet;
    AVFrame *frame;
    AVFrame *last;
    int strict;
    int64_t decoded_frames, decode_errors;
    AVFilterContext *source;
    int packet_pending, flushed, closed;
    int64_t origin, end_pts;
    int64_t audio_delta;
    AVRational sample_clock;
    FramePacketClock clock;
    enum AVPixelFormat hardware_format;
    int32_t display_matrix[9];
    int has_display_matrix;
    int downloaded;
} RenderInput;

struct FrameRenderer {
    AVFilterGraph *graph;
    AVFilterContext *sink;
    RenderInput inputs[2];
    int count;
    FrameRenderPlan filter_plan;
    char *graph_text, *tail_text;
    int initial_width, initial_height;
    char *output_format;
};

static enum AVPixelFormat select_device_format(AVCodecContext *codec,
                                               const enum AVPixelFormat *formats) {
    const RenderInput *input = codec->opaque;
    for (; *formats != AV_PIX_FMT_NONE; ++formats)
        if (*formats == input->hardware_format)
            return *formats;
    return AV_PIX_FMT_NONE;
}

static int open_input(RenderInput *input, const char *path, enum AVMediaType type, int index,
                      int64_t start_us, const FrameRenderPlan *plan, int primary) {
    input->strict = plan->strict;
    input->audio_delta = AV_NOPTS_VALUE;
    int rc;
    if (primary && plan->shared_file) {
        rc = frame_media_cursor_open(plan->shared_file, &input->cursor);
        if (rc < 0)
            return rc;
        input->format = avformat_alloc_context();
        if (!input->format)
            return AVERROR(ENOMEM);
        input->format->pb = input->cursor;
        input->format->flags |= AVFMT_FLAG_CUSTOM_IO;
    }
    rc = avformat_open_input(&input->format, path, NULL, NULL);
    if (rc < 0)
        return rc;
    if ((rc = avformat_find_stream_info(input->format, NULL)) < 0)
        return rc;
    if (index < 0) {
        for (unsigned i = 0; i < input->format->nb_streams; ++i) {
            if (input->format->streams[i]->codecpar->codec_type == type) {
                index = i;
                break;
            }
        }
    }
    if (index < 0 || (unsigned)index >= input->format->nb_streams)
        return AVERROR_STREAM_NOT_FOUND;
    input->stream = input->format->streams[index];
    if (input->stream->codecpar->codec_type != type)
        return AVERROR_STREAM_NOT_FOUND;
    const AVCodec *codec = avcodec_find_decoder(input->stream->codecpar->codec_id);
    if (!codec)
        return AVERROR_DECODER_NOT_FOUND;
    input->decoder = avcodec_alloc_context3(codec);
    input->packet = av_packet_alloc();
    input->frame = av_frame_alloc();
    input->last = av_frame_alloc();
    if (!input->last || !input->decoder || !input->packet || !input->frame)
        return AVERROR(ENOMEM);
    if ((rc = avcodec_parameters_to_context(input->decoder, input->stream->codecpar)) < 0)
        return rc;
    input->decoder->pkt_timebase = input->stream->time_base;
    input->decoder->thread_count = plan->threads;
    if (primary && type == AVMEDIA_TYPE_VIDEO && plan->hardware_device) {
        enum AVHWDeviceType hw = av_hwdevice_find_type_by_name(plan->hardware_device);
        if (hw == AV_HWDEVICE_TYPE_NONE)
            return AVERROR(ENOSYS);
        const AVCodecHWConfig *config;
        for (int i = 0; (config = avcodec_get_hw_config(codec, i)); ++i)
            if (config->device_type == hw &&
                (config->methods & AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX))
                break;
        // A decoder with no hardware implementation (e.g. an image) uses its CPU path.
        if (config) {
            if (plan->device_context) {
                if (((AVHWDeviceContext *)plan->device_context->data)->type != hw)
                    return AVERROR(EINVAL);
                input->decoder->hw_device_ctx = av_buffer_ref(plan->device_context);
                if (!input->decoder->hw_device_ctx)
                    return AVERROR(ENOMEM);
            } else {
                rc = av_hwdevice_ctx_create(&input->decoder->hw_device_ctx, hw, NULL, NULL, 0);
            }
            if (rc < 0)
                return rc;
            input->hardware_format = config->pix_fmt;
            input->decoder->opaque = input;
            input->decoder->get_format = select_device_format;
        }
    }
    if ((rc = avcodec_open2(input->decoder, codec, NULL)) < 0)
        return rc;
    int64_t origin = input->format->start_time == AV_NOPTS_VALUE ? 0 : input->format->start_time;
    origin = av_sat_add64(origin, start_us);
    const int *indices = primary && plan->mapped_count ? plan->mapped_streams : &index;
    int count = primary && plan->mapped_count ? plan->mapped_count : 1;
    rc = frame_clock_init(&input->clock, input->format, indices, count);
    if (rc < 0)
        return rc;
    input->origin = av_rescale_q(frame_clock_origin(input->format, indices, count, start_us),
                                 AV_TIME_BASE_Q, input->stream->time_base);
    if (start_us > 0) {
        rc = avformat_seek_file(input->format, -1, INT64_MIN, origin, origin, 0);
        if (rc < 0 && input->format->pb && input->format->pb->seekable)
            return rc;
    }
    return 0;
}

// Keep audio in sample units when the container stores timestamps on a
// different grid. Packet rounding alone can introduce one-sample gaps.
static void audio_timing(RenderInput *in, int64_t pts) {
    AVFrame *frame = in->frame;
    AVRational samples = {1, frame->sample_rate};
    AVRational stream = in->stream->time_base;
    int64_t predicted = in->end_pts;
    if (in->sample_clock.den != samples.den) {
        if (in->sample_clock.den)
            predicted = av_rescale_q(predicted, in->sample_clock, samples);
        in->sample_clock = samples;
        in->audio_delta = AV_NOPTS_VALUE;
    }
    if (pts == AV_NOPTS_VALUE) {
        frame->pts = predicted;
    } else {
        pts = av_sat_sub64(pts, in->origin);
        if (pts > av_rescale_q_rnd(predicted, samples, stream, AV_ROUND_UP))
            in->audio_delta = AV_NOPTS_VALUE;
        frame->pts =
            av_rescale_delta(stream, pts, samples, frame->nb_samples, &in->audio_delta, samples);
    }
    frame->duration = frame->nb_samples;
    frame->time_base = samples;
    in->end_pts = av_sat_add64(frame->pts, frame->nb_samples);
}

static int next_decoded(RenderInput *in) {
    av_frame_unref(in->frame);
    for (;;) {
        int rc = avcodec_receive_frame(in->decoder, in->frame);
        if (rc == 0) {
            ++in->decoded_frames;
            if (in->strict &&
                (in->frame->decode_error_flags || (in->frame->flags & AV_FRAME_FLAG_CORRUPT)))
                return AVERROR_INVALIDDATA;
            if (in->frame->hw_frames_ctx) {
                AVFrame *software = av_frame_alloc();
                if (!software)
                    return AVERROR(ENOMEM);
                // Keep the surface's native transfer depth rather than forcing NV12.
                software->format = ((AVHWFramesContext *)in->frame->hw_frames_ctx->data)->sw_format;
                rc = av_hwframe_transfer_data(software, in->frame, 0);
                if (rc >= 0)
                    rc = av_frame_copy_props(software, in->frame);
                if (rc >= 0) {
                    av_frame_unref(in->frame);
                    av_frame_move_ref(in->frame, software);
                    in->downloaded = 1;
                }
                av_frame_free(&software);
                if (rc < 0)
                    return rc;
            }
            // abuffer guesses a standard layout when the demuxer supplies only
            // a channel count. Compare and feed the same canonical layout.
            if (in->frame->nb_samples && in->frame->ch_layout.order == AV_CHANNEL_ORDER_UNSPEC)
                av_channel_layout_default(&in->frame->ch_layout, in->frame->ch_layout.nb_channels);
            int64_t pts = in->frame->best_effort_timestamp;
            if (in->frame->nb_samples && in->frame->sample_rate > 0) {
                audio_timing(in, pts);
                return 0;
            }
            in->frame->pts = pts == AV_NOPTS_VALUE ? in->end_pts : av_sat_sub64(pts, in->origin);
            int64_t duration = in->frame->duration;
            if (duration <= 0 && in->stream->avg_frame_rate.num > 0)
                duration =
                    av_rescale_q(1, av_inv_q(in->stream->avg_frame_rate), in->stream->time_base);
            in->end_pts = av_sat_add64(in->frame->pts, FFMAX(duration, 1));
            return 0;
        }
        if (rc == AVERROR_EOF || (in->flushed && rc == AVERROR(EAGAIN))) {
            // The previous engine failed the job when more than two thirds
            // of the decoder's results were errors.
            return in->decode_errors > 2 * in->decoded_frames ? AVERROR_INVALIDDATA : AVERROR_EOF;
        }
        if (rc == AVERROR_INVALIDDATA && !in->strict) {
            ++in->decode_errors;
            av_frame_unref(in->frame);
            continue;
        }
        if (rc != AVERROR(EAGAIN))
            return rc;
        if (!in->packet_pending) {
            do {
                av_packet_unref(in->packet);
                rc = av_read_frame(in->format, in->packet);
                if (rc >= 0)
                    frame_clock_packet(&in->clock, in->format, in->packet);
            } while (rc >= 0 && in->packet->stream_index != in->stream->index);
            if (rc == AVERROR_EOF) {
                in->flushed = 1;
                rc = avcodec_send_packet(in->decoder, NULL);
                if (rc < 0)
                    return rc;
                continue;
            }
            if (rc < 0)
                return rc;
            if (in->strict && (in->packet->flags & AV_PKT_FLAG_CORRUPT))
                return AVERROR_INVALIDDATA;
            in->packet_pending = 1;
        }
        rc = avcodec_send_packet(in->decoder, in->packet);
        if (rc == AVERROR(EAGAIN))
            continue;
        av_packet_unref(in->packet);
        in->packet_pending = 0;
        if (rc == AVERROR_INVALIDDATA && !in->strict) {
            ++in->decode_errors;
            continue;
        }
        if (rc < 0)
            return rc;
    }
}

static int add_filter(AVFilterGraph *graph, AVFilterContext **link, const char *name,
                      const char *args) {
    AVFilterContext *next = NULL;
    const AVFilter *filter = avfilter_get_by_name(name);
    if (!filter)
        return AVERROR_FILTER_NOT_FOUND;
    int rc = avfilter_graph_create_filter(&next, filter, NULL, args, NULL, graph);
    if (rc >= 0)
        rc = avfilter_link(*link, 0, next, 0);
    if (rc >= 0)
        *link = next;
    return rc;
}

static int configure_input(FrameRenderer *renderer, RenderInput *in, AVFilterContext **last,
                           int audio, int trim_start) {
    AVFrame *frame = in->last;
    AVFilterContext *src = avfilter_graph_alloc_filter(
        renderer->graph, avfilter_get_by_name(audio ? "abuffer" : "buffer"), NULL);
    if (!src)
        return AVERROR(ENOMEM);
    AVBufferSrcParameters *params = av_buffersrc_parameters_alloc();
    if (!params)
        return AVERROR(ENOMEM);
    params->time_base = audio ? frame->time_base : in->stream->time_base;
    params->format = frame->format;
    params->width = frame->width;
    params->height = frame->height;
    params->sample_aspect_ratio = av_guess_sample_aspect_ratio(in->format, in->stream, frame);
    params->frame_rate = av_guess_frame_rate(in->format, in->stream, frame);
    params->sample_rate = frame->sample_rate;
    params->color_space = frame->colorspace;
    params->color_range = frame->color_range;
#if LIBAVFILTER_VERSION_INT >= AV_VERSION_INT(11, 8, 100)
    params->alpha_mode = frame->alpha_mode;
#endif
    if (audio) {
        if (frame->ch_layout.order == AV_CHANNEL_ORDER_UNSPEC)
            av_channel_layout_default(&params->ch_layout, frame->ch_layout.nb_channels);
        else {
            int rc = av_channel_layout_copy(&params->ch_layout, &frame->ch_layout);
            if (rc < 0) {
                av_free(params);
                return rc;
            }
        }
    }
    int rc = av_buffersrc_parameters_set(src, params);
    av_channel_layout_uninit(&params->ch_layout);
    av_free(params);
    if (rc < 0 || (rc = avfilter_init_str(src, NULL)) < 0)
        return rc;
    in->source = *last = src;
    if (!audio) {
        const AVFrameSideData *side = av_frame_get_side_data(frame, AV_FRAME_DATA_DISPLAYMATRIX);
        const AVPacketSideData *packet_side = av_packet_side_data_get(
            in->stream->codecpar->coded_side_data, in->stream->codecpar->nb_coded_side_data,
            AV_PKT_DATA_DISPLAYMATRIX);
        // A display-orientation SEI can advertise a persistent transform only
        // once. Keep it until the stream provides a replacement matrix.
        const size_t bytes = sizeof(in->display_matrix);
        if (side && side->size >= bytes) {
            memcpy(in->display_matrix, side->data, bytes);
            in->has_display_matrix = 1;
        } else if (!in->has_display_matrix && packet_side && packet_side->size >= bytes) {
            memcpy(in->display_matrix, packet_side->data, bytes);
            in->has_display_matrix = 1;
        }
        if (in->has_display_matrix) {
            const int32_t *matrix = in->display_matrix;
            double angle = -av_display_rotation_get(matrix);
            if (isfinite(angle)) {
                angle = fmod(angle + 360, 360);
                if (fabs(angle - 90) < 1)
                    rc = add_filter(renderer->graph, last, "transpose",
                                    matrix[3] > 0 ? "cclock_flip" : "clock");
                else if (fabs(angle - 270) < 1)
                    rc = add_filter(renderer->graph, last, "transpose",
                                    matrix[3] < 0 ? "clock_flip" : "cclock");
                else if (fabs(angle - 180) < 1 || angle < 1 || angle > 359) {
                    if (matrix[0] < 0)
                        rc = add_filter(renderer->graph, last, "hflip", NULL);
                    if (rc >= 0 && matrix[4] < 0)
                        rc = add_filter(renderer->graph, last, "vflip", NULL);
                } else {
                    char value[64];
                    snprintf(value, sizeof(value), "%.12g*PI/180", angle);
                    rc = add_filter(renderer->graph, last, "rotate", value);
                }
            }
        }
    }
    if (rc >= 0 && trim_start)
        rc = add_filter(renderer->graph, last, audio ? "atrim" : "trim", "start=0");
    return rc;
}

static int configure_graph(FrameRenderer *r, int resize) {
    const FrameRenderPlan *p = &r->filter_plan;
    AVFilterContext *sources[2];
    AVFilterInOut *inputs = NULL, *outputs = NULL;
    int rc;
    for (int i = 0; i < r->count; ++i) {
        rc = configure_input(r, &r->inputs[i], &sources[i], p->audio,
                             i == 0 && p->start_us && !p->fast_seek);
        if (rc < 0)
            return rc;
    }
    rc = avfilter_graph_parse2(r->graph, p->graph, &inputs, &outputs);
    if (rc < 0)
        goto done;
    int connected = 0;
    for (AVFilterInOut *node = inputs; node; node = node->next) {
        int index = node->name && strcmp(node->name, "1:v:0") == 0 ? 1 : 0;
        if (index >= r->count || (connected & (1 << index))) {
            rc = AVERROR(EINVAL);
            goto done;
        }
        rc = avfilter_link(sources[index], 0, node->filter_ctx, node->pad_idx);
        if (rc < 0)
            goto done;
        connected |= 1 << index;
    }
    if (connected != (1 << r->count) - 1 || !outputs || outputs->next) {
        rc = AVERROR(EINVAL);
        goto done;
    }
    AVFilterInOut *tail_inputs = NULL, *tail_outputs = NULL;
    rc = avfilter_graph_parse2(r->graph,
                               p->tail && *p->tail ? p->tail : (p->audio ? "anull" : "null"),
                               &tail_inputs, &tail_outputs);
    if (rc >= 0 && (!tail_inputs || !tail_outputs || tail_inputs->next || tail_outputs->next))
        rc = AVERROR(EINVAL);
    if (rc >= 0)
        rc = avfilter_link(outputs->filter_ctx, outputs->pad_idx, tail_inputs->filter_ctx,
                           tail_inputs->pad_idx);
    if (rc >= 0) {
        AVFilterContext *last = tail_outputs->filter_ctx;
        if (resize) {
            char size[80];
            snprintf(size, sizeof(size), "w=%d:h=%d", r->initial_width, r->initial_height);
            rc = add_filter(r->graph, &last, "scale", size);
        }
        if (rc >= 0 && r->output_format)
            rc = add_filter(r->graph, &last, p->audio ? "aformat" : "format", r->output_format);
        if (rc >= 0 && p->duration_us >= 0) {
            char duration[80];
            snprintf(duration, sizeof(duration), "end=%" PRId64 "us", p->duration_us);
            rc = add_filter(r->graph, &last, p->audio ? "atrim" : "trim", duration);
        }
        if (rc >= 0)
            rc = add_filter(r->graph, &last, p->audio ? "abuffersink" : "buffersink", NULL);
        r->sink = last;
    }
    avfilter_inout_free(&tail_inputs);
    avfilter_inout_free(&tail_outputs);
    if (rc >= 0)
        rc = avfilter_graph_config(r->graph, NULL);
done:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);
    return rc;
}

static int submit_picture(RenderInput *input) {
    const int flags = AV_BUFFERSRC_FLAG_KEEP_REF | AV_BUFFERSRC_FLAG_PUSH;
    if (!av_frame_get_side_data(input->last, AV_FRAME_DATA_DISPLAYMATRIX))
        return av_buffersrc_add_frame_flags(input->source, input->last, flags);
    // The graph has applied the display matrix to the pixels. Keep it only
    // on our decoder reference for detecting the next orientation change.
    AVFrame *picture = av_frame_clone(input->last);
    if (!picture)
        return AVERROR(ENOMEM);
    av_frame_remove_side_data(picture, AV_FRAME_DATA_DISPLAYMATRIX);
    int rc = av_buffersrc_add_frame_flags(input->source, picture, AV_BUFFERSRC_FLAG_PUSH);
    av_frame_free(&picture);
    return rc;
}

static int new_graph(FrameRenderer *r, int resize) {
    avfilter_graph_free(&r->graph);
    r->graph = avfilter_graph_alloc();
    if (!r->graph)
        return AVERROR(ENOMEM);
    r->graph->nb_threads = r->filter_plan.threads;
    return configure_graph(r, resize);
}

// Filters can buffer pictures (fps, deinterlace, palette generation). Their
// old state belongs to the old format. Reuse input format descriptors, but
// only submit pictures that have not already been consumed by the old graph.
static int rebuild_graph(FrameRenderer *r, RenderInput *changed) {
    int rc = new_graph(r, 0);
    if (rc >= 0 && r->initial_width > 0 &&
        (av_buffersink_get_w(r->sink) != r->initial_width ||
         av_buffersink_get_h(r->sink) != r->initial_height))
        rc = new_graph(r, 1);
    for (int i = 0; rc >= 0 && i < r->count; ++i) {
        RenderInput *in = &r->inputs[i];
        if (!changed || changed == in)
            rc = submit_picture(in);
        if (rc >= 0 && in->closed)
            rc = av_buffersrc_close(in->source, in->end_pts, 0);
    }
    return rc;
}

static int changed_side_data(const AVFrame *previous, const AVFrame *next,
                             enum AVFrameSideDataType type) {
    const AVFrameSideData *before = av_frame_get_side_data(previous, type);
    const AVFrameSideData *after = av_frame_get_side_data(next, type);
    if (!before || !after)
        return before != after;
    return before->size != after->size || memcmp(before->data, after->data, before->size);
}

static int changed_format(const AVFrame *previous, const AVFrame *next) {
    return previous->format != next->format || previous->width != next->width ||
           previous->height != next->height || previous->sample_rate != next->sample_rate ||
           previous->color_range != next->color_range || previous->colorspace != next->colorspace ||
#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(60, 11, 100)
           previous->alpha_mode != next->alpha_mode ||
#endif
           changed_side_data(previous, next, AV_FRAME_DATA_DISPLAYMATRIX) ||
           changed_side_data(previous, next, AV_FRAME_DATA_DOWNMIX_INFO) ||
           (previous->nb_samples &&
            av_channel_layout_compare(&previous->ch_layout, &next->ch_layout));
}

int frame_render_open(FrameRenderer **result, const FrameRenderPlan *plan) {
    FrameRenderer *r = av_mallocz(sizeof(*r));
    *result = NULL;
    if (!r)
        return AVERROR(ENOMEM);
    int rc = AVERROR(ENOMEM);
    r->count = plan->overlay && strstr(plan->graph, "[1:v:0]") ? 2 : 1;
    r->graph_text = av_strdup(plan->graph);
    r->tail_text = av_strdup(plan->tail ? plan->tail : "");
    if (!r->graph_text || !r->tail_text)
        goto fail;
    r->filter_plan = (FrameRenderPlan){.graph = r->graph_text,
                                       .tail = r->tail_text,
                                       .audio = plan->audio,
                                       .start_us = plan->start_us,
                                       .duration_us = plan->duration_us,
                                       .fast_seek = plan->fast_seek,
                                       .threads = plan->threads};
    for (int i = 0; i < r->count; ++i) {
        if (plan->hardware_failed)
            *plan->hardware_failed = i == 0 && plan->hardware_device;
        rc = open_input(&r->inputs[i], i ? plan->overlay : plan->path,
                        plan->audio ? AVMEDIA_TYPE_AUDIO : AVMEDIA_TYPE_VIDEO,
                        i ? -1 : plan->stream, i ? 0 : plan->start_us, plan, i == 0);
        if (rc < 0)
            goto fail;
        // Configure filters from the first selected video frame. A keyframe
        // before an accurate seek can have different dimensions or bit depth.
        do {
            rc = next_decoded(&r->inputs[i]);
            if (rc < 0)
                goto fail;
        } while (i == 0 && !plan->audio && plan->start_us > 0 && !plan->fast_seek &&
                 r->inputs[i].frame->pts < 0);
        if (plan->hardware_failed)
            *plan->hardware_failed = 0;
        av_frame_move_ref(r->inputs[i].last, r->inputs[i].frame);
    }
    if ((rc = rebuild_graph(r, NULL)) < 0)
        goto fail;
    if (!plan->audio) {
        r->initial_width = av_buffersink_get_w(r->sink);
        r->initial_height = av_buffersink_get_h(r->sink);
        r->output_format =
            av_asprintf("pix_fmts=%s", av_get_pix_fmt_name(av_buffersink_get_format(r->sink)));
    } else {
        AVChannelLayout layout = {0};
        char channels[512];
        rc = av_buffersink_get_ch_layout(r->sink, &layout);
        if (rc >= 0)
            rc = av_channel_layout_describe(&layout, channels, sizeof(channels));
        av_channel_layout_uninit(&layout);
        if (rc < 0)
            goto fail;
        if ((size_t)rc >= sizeof(channels)) {
            rc = AVERROR(ENOSPC);
            goto fail;
        }
        r->output_format = av_asprintf("sample_fmts=%s:sample_rates=%d:channel_layouts=%s",
                                       av_get_sample_fmt_name(av_buffersink_get_format(r->sink)),
                                       av_buffersink_get_sample_rate(r->sink), channels);
    }
    if (!r->output_format) {
        rc = AVERROR(ENOMEM);
        goto fail;
    }
    *result = r;
    return 0;
fail:
    frame_render_close(&r);
    return rc;
}

int frame_render_downloaded(const FrameRenderer *r) { return r->inputs[0].downloaded; }

AVFilterContext *frame_render_sink(FrameRenderer *r) { return r->sink; }

int frame_render_next(FrameRenderer *r, AVFrame *frame, AVRational *tb) {
    for (;;) {
        *tb = av_buffersink_get_time_base(r->sink);
        int rc = av_buffersink_get_frame_flags(r->sink, frame, AV_BUFFERSINK_FLAG_NO_REQUEST);
        if (rc != AVERROR(EAGAIN))
            return rc;
        rc = avfilter_graph_request_oldest(r->graph);
        if (rc >= 0)
            continue;
        if (rc != AVERROR(EAGAIN))
            return rc;
        RenderInput *feed = NULL;
        for (int i = 0; i < r->count; ++i) {
            RenderInput *in = &r->inputs[i];
            if (!in->closed && (!feed || av_buffersrc_get_nb_failed_requests(in->source) >
                                             av_buffersrc_get_nb_failed_requests(feed->source)))
                feed = in;
        }
        if (!feed)
            return AVERROR_EOF;
        rc = next_decoded(feed);
        if (rc == AVERROR_EOF) {
            feed->closed = 1;
            rc = av_buffersrc_close(feed->source, feed->end_pts, 0);
        } else if (rc >= 0) {
            int reconfigure = changed_format(feed->last, feed->frame);
            av_frame_unref(feed->last);
            av_frame_move_ref(feed->last, feed->frame);
            rc = reconfigure ? rebuild_graph(r, feed) : submit_picture(feed);
        }
        if (rc < 0)
            return rc;
    }
}

void frame_render_close(FrameRenderer **renderer) {
    FrameRenderer *r = *renderer;
    if (!r)
        return;
    avfilter_graph_free(&r->graph);
    for (int i = 0; i < r->count; ++i) {
        frame_clock_free(&r->inputs[i].clock);
        av_packet_free(&r->inputs[i].packet);
        av_frame_free(&r->inputs[i].last);
        av_frame_free(&r->inputs[i].frame);
        avcodec_free_context(&r->inputs[i].decoder);
        avformat_close_input(&r->inputs[i].format);
        frame_media_cursor_close(&r->inputs[i].cursor);
    }
    av_free(r->output_format);
    av_free(r->graph_text);
    av_free(r->tail_text);
    av_freep(renderer);
}
