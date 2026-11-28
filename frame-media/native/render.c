#include "render.h"
#include "media_clock.h"
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

typedef struct {
    AVFormatContext *format;
    AVCodecContext *decoder;
    AVStream *stream;
    AVPacket *packet;
    AVFrame *frame;
    AVFrame *last;
    int strict;
    AVFilterContext *source;
    int packet_pending, flushed, closed;
    int64_t origin, end_pts;
    FramePacketClock clock;
    enum AVPixelFormat hardware_format;
} RenderInput;

struct FrameRenderer {
    AVFilterGraph *graph;
    AVFilterContext *sink;
    RenderInput inputs[2];
    int count;
    FrameRenderPlan filter_plan;
    char *graph_text, *tail_text;
    int initial_width, initial_height;
};

static enum AVPixelFormat select_device_format(AVCodecContext *codec, const enum AVPixelFormat *formats)
{
    const RenderInput *input = codec->opaque;
    for (; *formats != AV_PIX_FMT_NONE; ++formats)
        if (*formats == input->hardware_format) return *formats;
    return AV_PIX_FMT_NONE;
}

static int open_input(RenderInput *input, const char *path, enum AVMediaType type, int index,
                      int64_t start_us, const FrameRenderPlan *plan, int primary)
{
    input->strict = plan->strict;
    int rc = avformat_open_input(&input->format, path, NULL, NULL);
    if (rc < 0) return rc;
    if ((rc = avformat_find_stream_info(input->format, NULL)) < 0) return rc;
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
    if (input->stream->codecpar->codec_type != type) return AVERROR_STREAM_NOT_FOUND;
    const AVCodec *codec = avcodec_find_decoder(input->stream->codecpar->codec_id);
    if (!codec) return AVERROR_DECODER_NOT_FOUND;
    input->decoder = avcodec_alloc_context3(codec);
    input->packet = av_packet_alloc();
    input->frame = av_frame_alloc();
    input->last = av_frame_alloc();
    if (!input->last || !input->decoder || !input->packet || !input->frame) return AVERROR(ENOMEM);
    if ((rc = avcodec_parameters_to_context(input->decoder, input->stream->codecpar)) < 0) return rc;
    input->decoder->pkt_timebase = input->stream->time_base;
    input->decoder->thread_count = plan->threads;
    if (primary && type == AVMEDIA_TYPE_VIDEO && plan->hardware_device) {
        enum AVHWDeviceType hw = av_hwdevice_find_type_by_name(plan->hardware_device);
        if (hw == AV_HWDEVICE_TYPE_NONE) return AVERROR(ENOSYS);
        const AVCodecHWConfig *config;
        for (int i = 0; (config = avcodec_get_hw_config(codec, i)); ++i)
            if (config->device_type == hw && (config->methods & AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX))
                break;
        // A decoder with no hardware implementation (e.g. an image) uses its CPU path.
        if (config) {
            rc = av_hwdevice_ctx_create(&input->decoder->hw_device_ctx, hw, NULL, NULL, 0);
            if (rc < 0) return rc;
            input->hardware_format = config->pix_fmt;
            input->decoder->opaque = input;
            input->decoder->get_format = select_device_format;
        }
    }
    if ((rc = avcodec_open2(input->decoder, codec, NULL)) < 0) return rc;
    int64_t origin = input->format->start_time == AV_NOPTS_VALUE ? 0 : input->format->start_time;
    origin = av_sat_add64(origin, start_us);
    const int *indices = primary && plan->mapped_count ? plan->mapped_streams : &index;
    int count = primary && plan->mapped_count ? plan->mapped_count : 1;
    rc = frame_clock_init(&input->clock, input->format, indices, count);
    if (rc < 0) return rc;
    input->origin = av_rescale_q(frame_clock_origin(input->format, indices, count, start_us),
                                AV_TIME_BASE_Q, input->stream->time_base);
    if (start_us > 0) {
        rc = avformat_seek_file(input->format, -1, INT64_MIN, origin, origin, 0);
        if (rc < 0 && input->format->pb && input->format->pb->seekable) return rc;
    }
    return 0;
}

static int next_decoded(RenderInput *in)
{
    av_frame_unref(in->frame);
    for (;;) {
        int rc = avcodec_receive_frame(in->decoder, in->frame);
        if (rc == 0) {
            if (in->strict && (in->frame->decode_error_flags || (in->frame->flags & AV_FRAME_FLAG_CORRUPT)))
                return AVERROR_INVALIDDATA;
            if (in->frame->hw_frames_ctx) {
                AVFrame *software = av_frame_alloc();
                if (!software) return AVERROR(ENOMEM);
                rc = av_hwframe_transfer_data(software, in->frame, 0);
                if (rc >= 0) rc = av_frame_copy_props(software, in->frame);
                if (rc >= 0) {
                    av_frame_unref(in->frame);
                    av_frame_move_ref(in->frame, software);
                }
                av_frame_free(&software);
                if (rc < 0) return rc;
            }
            int64_t pts = in->frame->best_effort_timestamp;
            in->frame->pts = pts == AV_NOPTS_VALUE ? in->end_pts : av_sat_sub64(pts, in->origin);
            int64_t duration = in->frame->duration;
            if (duration <= 0 && in->frame->sample_rate > 0)
                duration = av_rescale_q(in->frame->nb_samples, (AVRational){1,in->frame->sample_rate}, in->stream->time_base);
            if (duration <= 0 && in->stream->avg_frame_rate.num > 0)
                duration = av_rescale_q(1, av_inv_q(in->stream->avg_frame_rate), in->stream->time_base);
            in->end_pts = av_sat_add64(in->frame->pts, FFMAX(duration, 1));
            return 0;
        }
        if (rc != AVERROR(EAGAIN)) return rc;
        if (in->flushed) return AVERROR_EOF;
        if (!in->packet_pending) {
            do {
                av_packet_unref(in->packet);
                rc = av_read_frame(in->format, in->packet);
                if (rc >= 0) frame_clock_packet(&in->clock, in->format, in->packet);
            } while (rc >= 0 && in->packet->stream_index != in->stream->index);
            if (rc == AVERROR_EOF) {
                in->flushed = 1;
                rc = avcodec_send_packet(in->decoder, NULL);
                if (rc < 0) return rc;
                continue;
            }
            if (rc < 0) return rc;
            if (in->strict && (in->packet->flags & AV_PKT_FLAG_CORRUPT)) return AVERROR_INVALIDDATA;
            in->packet_pending = 1;
        }
        rc = avcodec_send_packet(in->decoder, in->packet);
        if (rc == AVERROR(EAGAIN)) continue;
        av_packet_unref(in->packet);
        in->packet_pending = 0;
        if (rc < 0) return rc;
    }
}

static int add_filter(AVFilterGraph *graph, AVFilterContext **link, const char *name, const char *args)
{
    AVFilterContext *next = NULL;
    const AVFilter *filter = avfilter_get_by_name(name);
    if (!filter) return AVERROR_FILTER_NOT_FOUND;
    int rc = avfilter_graph_create_filter(&next, filter, NULL, args, NULL, graph);
    if (rc >= 0) rc = avfilter_link(*link, 0, next, 0);
    if (rc >= 0) *link = next;
    return rc;
}

static int configure_input(FrameRenderer *renderer, RenderInput *in, AVFilterContext **last,
                           int audio, int trim_start)
{
    AVFrame *frame = in->last;
    AVFilterContext *src = avfilter_graph_alloc_filter(renderer->graph,
            avfilter_get_by_name(audio ? "abuffer" : "buffer"), NULL);
    if (!src) return AVERROR(ENOMEM);
    AVBufferSrcParameters *params = av_buffersrc_parameters_alloc();
    if (!params) return AVERROR(ENOMEM);
    params->time_base = in->stream->time_base;
    params->format = frame->format;
    params->width = frame->width;
    params->height = frame->height;
    params->sample_aspect_ratio = av_guess_sample_aspect_ratio(in->format, in->stream, frame);
    params->frame_rate = av_guess_frame_rate(in->format, in->stream, frame);
    params->sample_rate = frame->sample_rate;
    params->color_space = frame->colorspace;
    params->color_range = frame->color_range;
    if (audio) {
        if (frame->ch_layout.order == AV_CHANNEL_ORDER_UNSPEC)
            av_channel_layout_default(&params->ch_layout, frame->ch_layout.nb_channels);
        else {
            int rc = av_channel_layout_copy(&params->ch_layout, &frame->ch_layout);
            if (rc < 0) { av_free(params); return rc; }
        }
    }
    int rc = av_buffersrc_parameters_set(src, params);
    av_channel_layout_uninit(&params->ch_layout);
    av_free(params);
    if (rc < 0 || (rc = avfilter_init_str(src, NULL)) < 0) return rc;
    in->source = *last = src;
    if (!audio) {
        const AVFrameSideData *side = av_frame_get_side_data(frame, AV_FRAME_DATA_DISPLAYMATRIX);
        const AVPacketSideData *packet_side = av_packet_side_data_get(in->stream->codecpar->coded_side_data,
            in->stream->codecpar->nb_coded_side_data, AV_PKT_DATA_DISPLAYMATRIX);
        const int32_t *matrix = side && side->size >= 36 ? (const int32_t *)side->data :
            packet_side && packet_side->size >= 36 ? (const int32_t *)packet_side->data : NULL;
        if (matrix) {
            double angle = -av_display_rotation_get(matrix);
            if (isfinite(angle)) {
                angle = fmod(angle + 360, 360);
                if (fabs(angle - 90) < 1) rc = add_filter(renderer->graph, last, "transpose", matrix[3] > 0 ? "cclock_flip" : "clock");
                else if (fabs(angle - 270) < 1) rc = add_filter(renderer->graph, last, "transpose", matrix[3] < 0 ? "clock_flip" : "cclock");
                else if (fabs(angle - 180) < 1 || angle < 1 || angle > 359) {
                    if (matrix[0] < 0) rc = add_filter(renderer->graph, last, "hflip", NULL);
                    if (rc >= 0 && matrix[4] < 0) rc = add_filter(renderer->graph, last, "vflip", NULL);
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

static int configure_graph(FrameRenderer *r, const FrameRenderPlan *p)
{
    AVFilterContext *sources[2];
    AVFilterInOut *inputs = NULL, *outputs = NULL;
    int rc;
    for (int i = 0; i < r->count; ++i) {
        rc = configure_input(r, &r->inputs[i], &sources[i], p->audio, i == 0 && p->start_us && !p->fast_seek);
        if (rc < 0) return rc;
    }
    rc = avfilter_graph_parse2(r->graph, p->graph, &inputs, &outputs);
    if (rc < 0) goto done;
    int connected = 0;
    for (AVFilterInOut *node = inputs; node; node = node->next) {
        int index = node->name && strcmp(node->name, "1:v:0") == 0 ? 1 : 0;
        if (index >= r->count || (connected & (1 << index))) { rc = AVERROR(EINVAL); goto done; }
        rc = avfilter_link(sources[index], 0, node->filter_ctx, node->pad_idx);
        if (rc < 0) goto done;
        connected |= 1 << index;
    }
    if (connected != (1 << r->count) - 1 || !outputs || outputs->next) { rc = AVERROR(EINVAL); goto done; }
    AVFilterInOut *tail_inputs = NULL, *tail_outputs = NULL;
    rc = avfilter_graph_parse2(r->graph, p->tail && *p->tail ? p->tail : (p->audio ? "anull" : "null"), &tail_inputs, &tail_outputs);
    if (rc >= 0 && (!tail_inputs || !tail_outputs || tail_inputs->next || tail_outputs->next)) rc = AVERROR(EINVAL);
    if (rc >= 0) rc = avfilter_link(outputs->filter_ctx, outputs->pad_idx, tail_inputs->filter_ctx, tail_inputs->pad_idx);
    if (rc >= 0) {
        AVFilterContext *last = tail_outputs->filter_ctx;
        if (!p->audio && r->initial_width > 0) {
            char size[80];
            snprintf(size, sizeof(size), "w=%d:h=%d", r->initial_width, r->initial_height);
            rc = add_filter(r->graph, &last, "scale", size);
        }
        if (rc >= 0 && p->duration_us >= 0) {
            char duration[80];
            snprintf(duration, sizeof(duration), "end=%" PRId64 "us", p->duration_us);
            rc = add_filter(r->graph, &last, p->audio ? "atrim" : "trim", duration);
        }
        if (rc >= 0) rc = add_filter(r->graph, &last, p->audio ? "abuffersink" : "buffersink", NULL);
        r->sink = last;
    }
    avfilter_inout_free(&tail_inputs);
    avfilter_inout_free(&tail_outputs);
    if (rc >= 0) rc = avfilter_graph_config(r->graph, NULL);
done:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);
    return rc;
}

// Filters can buffer pictures (fps, deinterlace, palette generation). Their
// old state belongs to the old format. Keep the last overlay so a one-frame
// logo remains available when the primary stream changes dimensions.
static int rebuild_graph(FrameRenderer *r)
{
    avfilter_graph_free(&r->graph);
    r->graph = avfilter_graph_alloc();
    if (!r->graph) return AVERROR(ENOMEM);
    r->graph->nb_threads = r->filter_plan.threads;
    int rc = configure_graph(r, &r->filter_plan);
    for (int i = 0; rc >= 0 && i < r->count; ++i) {
        RenderInput *in = &r->inputs[i];
        rc = av_buffersrc_write_frame(in->source, in->last);
        if (rc >= 0 && in->closed) rc = av_buffersrc_close(in->source, in->end_pts, 0);
    }
    return rc;
}

static int changed_format(const AVFrame *previous, const AVFrame *next)
{
    return previous->format != next->format || previous->width != next->width ||
        previous->height != next->height || previous->sample_rate != next->sample_rate ||
        previous->color_range != next->color_range || previous->colorspace != next->colorspace ||
        (previous->nb_samples && av_channel_layout_compare(&previous->ch_layout, &next->ch_layout));
}

int frame_render_open(FrameRenderer **result, const FrameRenderPlan *plan)
{
    FrameRenderer *r = av_mallocz(sizeof(*r));
    *result = NULL;
    if (!r) return AVERROR(ENOMEM);
    int rc = AVERROR(ENOMEM);
    r->count = plan->overlay && strstr(plan->graph, "[1:v:0]") ? 2 : 1;
    r->graph_text = av_strdup(plan->graph);
    r->tail_text = av_strdup(plan->tail ? plan->tail : "");
    if (!r->graph_text || !r->tail_text) goto fail;
    r->filter_plan = (FrameRenderPlan){.graph=r->graph_text, .tail=r->tail_text,
        .audio=plan->audio, .start_us=plan->start_us, .duration_us=plan->duration_us,
        .fast_seek=plan->fast_seek, .threads=plan->threads};
    for (int i = 0; i < r->count; ++i) {
        rc = open_input(&r->inputs[i], i ? plan->overlay : plan->path,
                plan->audio ? AVMEDIA_TYPE_AUDIO : AVMEDIA_TYPE_VIDEO,
                i ? -1 : plan->stream, i ? 0 : plan->start_us, plan, i == 0);
        if (rc < 0) goto fail;
        // Configure filters from the first selected video frame. A keyframe
        // before an accurate seek can have different dimensions or bit depth.
        do {
            rc = next_decoded(&r->inputs[i]);
            if (rc < 0) goto fail;
        } while (i == 0 && !plan->audio && plan->start_us > 0 && !plan->fast_seek &&
                 r->inputs[i].frame->pts < 0);
        av_frame_move_ref(r->inputs[i].last, r->inputs[i].frame);
    }
    if ((rc = rebuild_graph(r)) < 0) goto fail;
    if (!plan->audio) {
        r->initial_width = av_buffersink_get_w(r->sink);
        r->initial_height = av_buffersink_get_h(r->sink);
    }
    *result = r;
    return 0;
fail:
    frame_render_close(&r);
    return rc;
}

AVFilterContext *frame_render_sink(FrameRenderer *r) { return r->sink; }

int frame_render_samples(FrameRenderer *r, AVFrame *frame, AVRational *tb, int samples)
{
    *tb = av_buffersink_get_time_base(r->sink);
    for (;;) {
        int rc = samples ? av_buffersink_get_samples(r->sink, frame, samples) : av_buffersink_get_frame(r->sink, frame);
        if (rc != AVERROR(EAGAIN)) return rc;
        RenderInput *feed = NULL;
        for (int i = 0; i < r->count; ++i) {
            RenderInput *in = &r->inputs[i];
            if (!in->closed && (!feed || av_buffersrc_get_nb_failed_requests(in->source) >
                                        av_buffersrc_get_nb_failed_requests(feed->source))) feed = in;
        }
        if (!feed) return AVERROR_EOF;
        rc = next_decoded(feed);
        if (rc == AVERROR_EOF) {
            feed->closed = 1;
            rc = av_buffersrc_close(feed->source, feed->end_pts, 0);
        } else if (rc >= 0) {
            int reconfigure = changed_format(feed->last, feed->frame);
            av_frame_unref(feed->last);
            av_frame_move_ref(feed->last, feed->frame);
            rc = reconfigure ? rebuild_graph(r) : av_buffersrc_write_frame(feed->source, feed->last);
        }
        if (rc < 0) return rc;
    }
}

int frame_render_next(FrameRenderer *r, AVFrame *frame, AVRational *tb)
{
    return frame_render_samples(r, frame, tb, 0);
}

void frame_render_close(FrameRenderer **renderer)
{
    FrameRenderer *r = *renderer;
    if (!r) return;
    avfilter_graph_free(&r->graph);
    for (int i = 0; i < r->count; ++i) {
        frame_clock_free(&r->inputs[i].clock);
        av_packet_free(&r->inputs[i].packet);
        av_frame_free(&r->inputs[i].last);
        av_frame_free(&r->inputs[i].frame);
        avcodec_free_context(&r->inputs[i].decoder);
        avformat_close_input(&r->inputs[i].format);
    }
    av_free(r->graph_text);
    av_free(r->tail_text);
    av_freep(renderer);
}
