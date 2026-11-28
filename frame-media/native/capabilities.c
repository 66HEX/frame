#include <stdio.h>
#include <string.h>
#include <libavcodec/avcodec.h>
#include <libavcodec/bsf.h>
#include <libavfilter/avfilter.h>
#include <libavutil/imgutils.h>

int frame_inventory(int kind) {
    void *iter = NULL;
    if (kind == 0) {
        puts("Encoders:");
        const AVCodec *codec;
        while ((codec = av_codec_iterate(&iter))) {
            if (av_codec_is_encoder(codec)) {
                char type = codec->type == AVMEDIA_TYPE_AUDIO      ? 'A'
                            : codec->type == AVMEDIA_TYPE_SUBTITLE ? 'S'
                                                                   : 'V';
                printf(" %c..... %s %s\n", type, codec->name,
                       codec->long_name ? codec->long_name : codec->name);
            }
        }
    } else if (kind == 1) {
        puts("Filters:");
        const AVFilter *filter;
        while ((filter = av_filter_iterate(&iter))) {
            printf(" ... %s ", filter->name);
            for (int out = 0; out <= 1; ++out) {
                unsigned count = avfilter_filter_pad_count(filter, out);
                if (!count)
                    putchar(filter->flags & (out ? AVFILTER_FLAG_DYNAMIC_OUTPUTS
                                                 : AVFILTER_FLAG_DYNAMIC_INPUTS)
                                ? 'N'
                                : '|');
                for (unsigned i = 0; i < count; ++i)
                    putchar(avfilter_pad_get_type(out ? filter->outputs : filter->inputs, i) ==
                                    AVMEDIA_TYPE_AUDIO
                                ? 'A'
                                : 'V');
                if (!out)
                    fputs("->", stdout);
            }
            putchar('\n');
        }
    } else {
        puts("Bitstream filters:");
        const AVBitStreamFilter *filter;
        while ((filter = av_bsf_iterate(&iter)))
            puts(filter->name);
    }
    return ferror(stdout) || fflush(stdout) ? AVERROR(EIO) : 0;
}

int frame_check_encoder(const char *name, int allow_sw, const uint8_t *input) {
    const AVCodec *codec = avcodec_find_encoder_by_name(name);
    if (!codec)
        return AVERROR_ENCODER_NOT_FOUND;
    AVCodecContext *ctx = avcodec_alloc_context3(codec);
    AVPacket *pkt = av_packet_alloc();
    AVFrame *frame = av_frame_alloc();
    AVDictionary *options = NULL;
    int rc = AVERROR(ENOMEM);
    if (!ctx || !pkt || !frame)
        goto done;
    ctx->width = ctx->height = 128;
    ctx->time_base = (AVRational){1, 1};
    ctx->framerate = (AVRational){1, 1};
    ctx->pix_fmt = AV_PIX_FMT_YUV420P;
    if (strstr(name, "_videotoolbox")) {
        av_dict_set(&options, "allow_sw", allow_sw ? "1" : "0", 0);
        av_dict_set(&options, "require_sw", "0", 0);
    }
    rc = avcodec_open2(ctx, codec, &options);
    if (rc < 0 || av_dict_count(options)) {
        if (rc >= 0)
            rc = AVERROR_OPTION_NOT_FOUND;
        goto done;
    }
    frame->width = frame->height = 128;
    frame->format = ctx->pix_fmt;
    frame->pts = 0;
    if ((rc = av_frame_get_buffer(frame, 32)) < 0)
        goto done;
    for (int plane = 0; plane < 3; ++plane) {
        int size = plane ? 64 : 128;
        for (int row = 0; row < size; ++row) {
            memcpy(frame->data[plane] + row * frame->linesize[plane], input, size);
            input += size;
        }
    }
    if ((rc = avcodec_send_frame(ctx, frame)) < 0)
        goto done;
    rc = avcodec_receive_packet(ctx, pkt);
    if (rc == AVERROR(EAGAIN) && (rc = avcodec_send_frame(ctx, NULL)) >= 0)
        rc = avcodec_receive_packet(ctx, pkt);
    if (rc >= 0 && pkt->size <= 0)
        rc = AVERROR_INVALIDDATA;
done:
    av_dict_free(&options);
    avcodec_free_context(&ctx);
    av_packet_free(&pkt);
    av_frame_free(&frame);
    return rc;
}
