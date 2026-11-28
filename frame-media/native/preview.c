#include "render.h"
#include <libavcodec/avcodec.h>
#include <libavfilter/buffersink.h>
#include <libavutil/imgutils.h>
#include <libavutil/parseutils.h>
#include <libavutil/time.h>
#include <stdio.h>
#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif

enum PreviewOutput { RAW_BGRA, RAW_AUDIO, SOURCE_PNG, ANALYSIS };

static int write_bytes(const uint8_t *data, size_t size)
{
    return fwrite(data, 1, size, stdout) == size ? 0 : AVERROR(EIO);
}

static int write_png(const AVFrame *frame, AVRational tb)
{
    const AVCodec *codec = avcodec_find_encoder(AV_CODEC_ID_PNG);
    if (!codec) return AVERROR_ENCODER_NOT_FOUND;
    AVCodecContext *ctx = avcodec_alloc_context3(codec);
    AVPacket *pkt = av_packet_alloc();
    int rc = AVERROR(ENOMEM);
    if (!ctx || !pkt) goto done;
    ctx->width = frame->width;
    ctx->height = frame->height;
    ctx->pix_fmt = frame->format;
    ctx->time_base = tb;
    ctx->sample_aspect_ratio = frame->sample_aspect_ratio;
    ctx->color_range = frame->color_range;
    ctx->colorspace = frame->colorspace;
    ctx->color_trc = frame->color_trc;
    ctx->color_primaries = frame->color_primaries;
    ctx->thread_count = 1;
    if ((rc = avcodec_open2(ctx, codec, NULL)) < 0 ||
        (rc = avcodec_send_frame(ctx, frame)) < 0) goto done;
    rc = avcodec_receive_packet(ctx, pkt);
    if (rc == AVERROR(EAGAIN)) {
        if ((rc = avcodec_send_frame(ctx, NULL)) >= 0) rc = avcodec_receive_packet(ctx, pkt);
    }
    if (rc >= 0) rc = write_bytes(pkt->data, pkt->size);
done:
    avcodec_free_context(&ctx);
    av_packet_free(&pkt);
    return rc;
}

int frame_preview(const char *path, const char *overlay, const char *graph, const char *tail,
                  int output, int stream, const char *start, const char *duration, int fast,
                  int realtime, int threads, int64_t limit, int progress, int verbose)
{
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) return AVERROR(EIO);
#endif
    av_log_set_level(verbose ? AV_LOG_INFO : AV_LOG_WARNING);
    FrameRenderPlan plan = {.path=path, .overlay=overlay, .graph=graph, .tail=tail,
        .audio=output == RAW_AUDIO, .stream=stream, .duration_us=-1, .fast_seek=fast, .threads=threads};
    int rc = 0;
    if (start && (rc = av_parse_time(&plan.start_us, start, 1)) < 0) return rc;
    if (duration && (rc = av_parse_time(&plan.duration_us, duration, 1)) < 0) return rc;
    FrameRenderer *renderer = NULL;
    AVFrame *frame = av_frame_alloc();
    if (!frame) return AVERROR(ENOMEM);
    int64_t count = 0;
    int64_t clock_start = av_gettime_relative();
    rc = frame_render_open(&renderer, &plan);
    while (rc >= 0 && count < limit) {
        AVRational tb;
        rc = frame_render_next(renderer, frame, &tb);
        if (rc < 0) break;
        if (realtime && frame->pts != AV_NOPTS_VALUE) {
            int64_t target = av_rescale_q(frame->pts, tb, AV_TIME_BASE_Q);
            int64_t wait;
            while ((wait = target - (av_gettime_relative() - clock_start)) > 0)
                av_usleep((unsigned)FFMIN(wait, 1000000));
        }
        if (output == RAW_BGRA) {
            if (frame->format != AV_PIX_FMT_BGRA) { rc = AVERROR_BUG; break; }
            for (int row = 0; rc >= 0 && row < frame->height; ++row)
                rc = write_bytes(frame->data[0] + (ptrdiff_t)row * frame->linesize[0], (size_t)frame->width * 4);
        } else if (output == RAW_AUDIO) {
            if (frame->format != AV_SAMPLE_FMT_FLT) { rc = AVERROR_BUG; break; }
            rc = write_bytes(frame->data[0], (size_t)frame->nb_samples * frame->ch_layout.nb_channels * sizeof(float));
        } else if (output == SOURCE_PNG) {
            rc = write_png(frame, tb);
        }
        av_frame_unref(frame);
        ++count;
    }
    if (rc == AVERROR_EOF) rc = 0;
    if (rc >= 0 && progress && printf("frame=%" PRId64 "\nprogress=end\n", count) < 0)
        rc = AVERROR(EIO);
    if (rc >= 0 && fflush(stdout) != 0) rc = AVERROR(EIO);
    av_frame_free(&frame);
    frame_render_close(&renderer);
    if (rc < 0) {
        char message[AV_ERROR_MAX_STRING_SIZE];
        av_strerror(rc, message, sizeof(message));
        fprintf(stderr, "Frame preview: %s\n", message);
    }
    return rc;
}
