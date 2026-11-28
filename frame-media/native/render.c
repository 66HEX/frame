#include "render.h"
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
    AVFilterContext *source;
    int packet_pending, flushed, closed;
    int64_t origin, end_pts;
    enum AVPixelFormat hardware_format;
} RenderInput;

struct FrameRenderer {
    AVFilterGraph *graph;
    AVFilterContext *sink;
    RenderInput inputs[2];
    int count;
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
    if (!input->decoder || !input->packet || !input->frame) return AVERROR(ENOMEM);
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
    input->origin = av_rescale_q(origin, AV_TIME_BASE_Q, input->stream->time_base);
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
            } while (rc >= 0 && in->packet->stream_index != in->stream->index);
            if (rc == AVERROR_EOF) {
                in->flushed = 1;
                rc = avcodec_send_packet(in->decoder, NULL);
                if (rc < 0) return rc;
                continue;
            }
            if (rc < 0) return rc;
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
    AVFrame *frame = in->frame;
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
        if (p->duration_us >= 0) {
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

int frame_render_open(FrameRenderer **result, const FrameRenderPlan *plan)
{
    FrameRenderer *r = av_mallocz(sizeof(*r));
    *result = NULL;
    if (!r) return AVERROR(ENOMEM);
    int rc = AVERROR(ENOMEM);
    r->count = plan->overlay ? 2 : 1;
    r->graph = avfilter_graph_alloc();
    if (!r->graph) goto fail;
    r->graph->nb_threads = plan->threads;
    for (int i = 0; i < r->count; ++i) {
        rc = open_input(&r->inputs[i], i ? plan->overlay : plan->path,
                plan->audio ? AVMEDIA_TYPE_AUDIO : AVMEDIA_TYPE_VIDEO,
                i ? -1 : plan->stream, i ? 0 : plan->start_us, plan, i == 0);
        if (rc < 0 || (rc = next_decoded(&r->inputs[i])) < 0) goto fail;
    }
    if ((rc = configure_graph(r, plan)) < 0) goto fail;
    for (int i = 0; i < r->count; ++i)
        if ((rc = av_buffersrc_add_frame(r->inputs[i].source, r->inputs[i].frame)) < 0) goto fail;
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
            rc = av_buffersrc_add_frame(feed->source, feed->frame);
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
        av_packet_free(&r->inputs[i].packet);
        av_frame_free(&r->inputs[i].frame);
        avcodec_free_context(&r->inputs[i].decoder);
        avformat_close_input(&r->inputs[i].format);
    }
    av_freep(renderer);
}
