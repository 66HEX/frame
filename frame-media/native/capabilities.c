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
