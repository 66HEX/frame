#include "convert.h"

/* Codec names in saved presets may use the descriptor name (mp3, vp9)
 * while the library exposes a specific implementation (libmp3lame). */
static const AVCodec *find_encoder(const char *name) {
    const AVCodec *codec = avcodec_find_encoder_by_name(name);
    const AVCodecDescriptor *desc = avcodec_descriptor_get_by_name(name);
    if (desc && (!codec || codec->capabilities & AV_CODEC_CAP_EXPERIMENTAL)) {
        const AVCodec *preferred = avcodec_find_encoder(desc->id);
        if (preferred)
            return preferred;
    }
    return codec;
}

static int append_constraints(const ConvertPlan *p, const ConvertTrack *t, const AVCodec *codec,
                              AVBPrint *tail) {
    const AVCodecParameters *src = t->src->codecpar;
    int err;
    if (t->kind == 'v') {
        const char *fixed = frame_plan_opt(p, "pix_fmt");
        const enum AVPixelFormat *fmts = NULL;
#if LIBAVCODEC_VERSION_INT >= AV_VERSION_INT(61, 13, 100)
        if ((err = avcodec_get_supported_config(NULL, codec, AV_CODEC_CONFIG_PIX_FORMAT, 0,
                                                (const void **)&fmts, NULL)) < 0)
            return err;
#else
        fmts = codec->pix_fmts;
#endif
        if (fixed) {
            enum AVPixelFormat requested = av_get_pix_fmt(fixed);
            if (requested == AV_PIX_FMT_NONE)
                return AVERROR(EINVAL);
            enum AVPixelFormat best = requested;
            if (fmts) {
                best = AV_PIX_FMT_NONE;
                for (const enum AVPixelFormat *f = fmts; *f != AV_PIX_FMT_NONE; ++f) {
                    const AVPixFmtDescriptor *desc = av_pix_fmt_desc_get(*f);
                    if (desc && !(desc->flags & AV_PIX_FMT_FLAG_HWACCEL))
                        best = av_find_best_pix_fmt_of_2(best, *f, requested, 1, NULL);
                }
            }
            if (best == AV_PIX_FMT_NONE)
                return AVERROR(EINVAL);
            av_bprintf(tail, "format=pix_fmts=%s", av_get_pix_fmt_name(best));
        } else {
            if (!fmts)
                av_bprintf(tail, "null");
            else {
                av_bprintf(tail, "format=pix_fmts=");
                int count = 0;
                for (; *fmts != AV_PIX_FMT_NONE; ++fmts) {
                    const AVPixFmtDescriptor *desc = av_pix_fmt_desc_get(*fmts);
                    if (!desc || desc->flags & AV_PIX_FMT_FLAG_HWACCEL)
                        continue;
                    av_bprintf(tail, "%s%s", count++ ? "|" : "", desc->name);
                }
                if (!count)
                    return AVERROR(ENOSYS);
            }
        }
        if (codec->id == AV_CODEC_ID_MJPEG)
            av_bprintf(tail, ":color_ranges=pc");
    } else {
        const enum AVSampleFormat *fmts = NULL;
        const int *rates = NULL;
#if LIBAVCODEC_VERSION_INT >= AV_VERSION_INT(61, 13, 100)
        if ((err = avcodec_get_supported_config(NULL, codec, AV_CODEC_CONFIG_SAMPLE_FORMAT, 0,
                                                (const void **)&fmts, NULL)) < 0 ||
            (err = avcodec_get_supported_config(NULL, codec, AV_CODEC_CONFIG_SAMPLE_RATE, 0,
                                                (const void **)&rates, NULL)) < 0)
            return err;
#else
        fmts = codec->sample_fmts;
        rates = codec->supported_samplerates;
#endif
        const char *ar = frame_track_opt(p, t, "ar");
        int sample_rate = ar ? atoi(ar) : src->sample_rate;
        if (rates && !ar && *rates) {
            int closest = rates[0];
            for (const int *rate = rates; *rate; rate++)
                if (abs(*rate - sample_rate) < abs(closest - sample_rate))
                    closest = *rate;
            sample_rate = closest;
        }
        if (sample_rate <= 0)
            return AVERROR_INVALIDDATA;
        av_bprintf(tail, "aformat=sample_rates=%d", sample_rate);
        if (fmts) {
            av_bprintf(tail, ":sample_fmts=");
            for (int i = 0; fmts[i] != AV_SAMPLE_FMT_NONE; i++)
                av_bprintf(tail, "%s%s", i ? "|" : "", av_get_sample_fmt_name(fmts[i]));
        }
        const char *channels = frame_track_opt(p, t, "ac");
        if (channels) {
            if (strcmp(channels, "1") && strcmp(channels, "2"))
                return AVERROR(EINVAL);
            av_bprintf(tail, ":channel_layouts=%s", *channels == '1' ? "mono" : "stereo");
        } else {
            const AVChannelLayout *layouts = NULL;
#if LIBAVCODEC_VERSION_INT >= AV_VERSION_INT(61, 13, 100)
            if ((err = avcodec_get_supported_config(NULL, codec, AV_CODEC_CONFIG_CHANNEL_LAYOUT, 0,
                                                    (const void **)&layouts, NULL)) < 0)
                return err;
#else
            layouts = codec->ch_layouts;
#endif
            if (layouts && layouts->nb_channels) {
                av_bprintf(tail, ":channel_layouts=");
                for (int i = 0; layouts[i].nb_channels; ++i) {
                    char layout[256];
                    err = av_channel_layout_describe(&layouts[i], layout, sizeof(layout));
                    if (err < 0 || (size_t)err >= sizeof(layout))
                        return AVERROR(EINVAL);
                    av_bprintf(tail, "%s%s", i ? "|" : "", layout);
                }
            }
        }
    }
    return av_bprint_is_complete(tail) ? 0 : AVERROR(ENOMEM);
}

static int open_subtitle_decoder(ConvertTrack *t) {
    const AVCodec *decoder = avcodec_find_decoder(t->src->codecpar->codec_id);
    if (!decoder)
        return AVERROR_DECODER_NOT_FOUND;
    t->subdecoder = avcodec_alloc_context3(decoder);
    if (!t->subdecoder)
        return AVERROR(ENOMEM);
    int err = avcodec_parameters_to_context(t->subdecoder, t->src->codecpar);
    if (err < 0)
        return err;
    t->subdecoder->pkt_timebase = t->src->time_base;
    if ((err = avcodec_open2(t->subdecoder, decoder, NULL)) < 0)
        return err;
    AVCodecContext *enc = t->encoder;
    enc->time_base = (AVRational){1, 1000};
    enc->width = t->subdecoder->width;
    enc->height = t->subdecoder->height;
    if (t->subdecoder->subtitle_header_size) {
        enc->subtitle_header =
            av_mallocz(t->subdecoder->subtitle_header_size + AV_INPUT_BUFFER_PADDING_SIZE);
        if (!enc->subtitle_header)
            return AVERROR(ENOMEM);
        memcpy(enc->subtitle_header, t->subdecoder->subtitle_header,
               t->subdecoder->subtitle_header_size);
        enc->subtitle_header_size = t->subdecoder->subtitle_header_size;
    }
    return 0;
}

static int open_render(const ConvertPlan *p, ConvertTrack *t, const AVCodec *codec) {
    AVBPrint tail;
    av_bprint_init(&tail, 128, AV_BPRINT_SIZE_UNLIMITED);
    int err = append_constraints(p, t, codec, &tail);
    if (err >= 0) {
        const char *graph =
            t->kind == 'a' ? frame_plan_opt(p, "af") : frame_plan_opt(p, "filter_complex");
        if (!graph && t->kind == 'v')
            graph = frame_plan_opt(p, "vf");
        FrameRenderPlan request = {
            .path = t->source->path,
            .overlay = t->kind == 'v' && graph && strstr(graph, "[1:v") && p->sources_len > 1
                           ? p->sources[1].path
                           : NULL,
            .graph = graph ? graph : (t->kind == 'a' ? "anull" : "null"),
            .tail = tail.str,
            .audio = t->kind == 'a',
            .stream = t->src->index,
            .start_us = t->source->seek,
            .duration_us = p->duration,
            .mapped_streams = t->source->mapped,
            .mapped_count = t->source->mapped_count,
            .strict = t->strict,
            .hardware_device = t->kind == 'v' ? frame_plan_opt(p, "hwaccel") : NULL,
        };
        err = frame_render_open(&t->render, &request);
    }
    av_bprint_finalize(&tail, NULL);
    if (err < 0)
        return err;
    AVFilterContext *sink = frame_render_sink(t->render);
    AVCodecContext *ctx = t->encoder;
    if (t->kind == 'a') {
        ctx->sample_rate = av_buffersink_get_sample_rate(sink);
        ctx->sample_fmt = av_buffersink_get_format(sink);
        ctx->time_base = (AVRational){1, ctx->sample_rate};
        if ((err = av_buffersink_get_ch_layout(sink, &ctx->ch_layout)) < 0)
            return err;
    } else {
        ctx->width = av_buffersink_get_w(sink);
        ctx->height = av_buffersink_get_h(sink);
        ctx->pix_fmt = av_buffersink_get_format(sink);
        ctx->sample_aspect_ratio = av_buffersink_get_sample_aspect_ratio(sink);
        ctx->framerate = av_buffersink_get_frame_rate(sink);
        if (!ctx->framerate.num)
            ctx->framerate = av_guess_frame_rate(t->input, t->src, NULL);
        if (ctx->framerate.num <= 0 || ctx->framerate.den <= 0)
            ctx->framerate = (AVRational){25, 1};
        const char *rate = frame_plan_opt(p, "r");
        if (rate && (err = av_parse_video_rate(&ctx->framerate, rate)) < 0)
            return err;
        ctx->time_base = av_inv_q(ctx->framerate);
        ctx->flags |= AV_CODEC_FLAG_FRAME_DURATION;
        ctx->colorspace = t->src->codecpar->color_space;
        ctx->color_range = t->src->codecpar->color_range;
#if LIBAVFILTER_VERSION_MAJOR >= 10
        ctx->colorspace = av_buffersink_get_colorspace(sink);
        ctx->color_range = av_buffersink_get_color_range(sink);
#endif
        ctx->color_primaries = t->src->codecpar->color_primaries;
        ctx->color_trc = t->src->codecpar->color_trc;
        t->dst->avg_frame_rate = ctx->framerate;
    }
    return 0;
}

static int open_codec(const ConvertPlan *p, ConvertTrack *t, const AVCodec *codec) {
    static const char *const video[] = {"b",
                                        "g",
                                        "bf",
                                        "crf",
                                        "rc",
                                        "cq",
                                        "deadline",
                                        "cpu-used",
                                        "preset",
                                        "spatial-aq",
                                        "temporal-aq",
                                        "allow_sw",
                                        "huffman",
                                        "lossless",
                                        "quality",
                                        "compression_level",
                                        "compression_algo",
                                        "pred",
                                        "x265-params",
                                        NULL};
    static const char *const audio[] = {"b", "vbr", NULL};
    const char *const *names = t->kind == 'a' ? audio : video;
    AVDictionary *options = NULL;
    int err = 0;
    for (; t->kind != 's' && *names; names++) {
        const char *value = frame_track_opt(p, t, *names);
        if (value && (err = av_dict_set(&options, *names, value, 0)) < 0)
            goto end;
    }
    const char *quality = frame_track_opt(p, t, "q");
    if (quality) {
        t->encoder->flags |= AV_CODEC_FLAG_QSCALE;
        t->encoder->global_quality = FF_QP2LAMBDA * atoi(quality);
    }
    err = avcodec_open2(t->encoder, codec, &options);
    if (err >= 0 && av_dict_count(options)) {
        fprintf(stderr, "Encoder %s does not accept option %s\n", codec->name,
                av_dict_get(options, "", NULL, AV_DICT_IGNORE_SUFFIX)->key);
        err = AVERROR_OPTION_NOT_FOUND;
    }
end:
    av_dict_free(&options);
    return err;
}

int frame_prepare_encoder(const ConvertPlan *p, ConvertTrack *t, AVFormatContext *out,
                          const char *name) {
    const AVCodec *codec = find_encoder(name);
    if (!codec || codec->type != t->src->codecpar->codec_type)
        return AVERROR_ENCODER_NOT_FOUND;
    t->encoder = avcodec_alloc_context3(codec);
    t->frame = av_frame_alloc();
    if (!t->encoder || !t->frame)
        return AVERROR(ENOMEM);
    if (out->oformat->flags & AVFMT_GLOBALHEADER)
        t->encoder->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    int err = t->kind == 's' ? open_subtitle_decoder(t) : open_render(p, t, codec);
    if (err < 0 || (err = open_codec(p, t, codec)) < 0)
        return err;
    if (t->kind == 'v') {
        FrameCadence *c = &t->cadence;
        c->previous = av_frame_alloc();
        c->current = av_frame_alloc();
        if (!c->previous || !c->current)
            return AVERROR(ENOMEM);
        c->fixed = frame_plan_opt(p, "r") != NULL || (!(out->oformat->flags & AVFMT_VARIABLE_FPS) &&
                                                      strcmp(out->oformat->name, "avi"));
        if (frame_plan_opt(p, "fps_mode") || out->oformat->flags & AVFMT_NOTIMESTAMPS)
            c->fixed = 0;
        c->preserve_initial_offset =
            t->input->nb_streams == 1 && !frame_plan_opt(p, "filter_complex");
    }
    t->packet_tb = t->encoder->time_base;
    t->dst->time_base = t->packet_tb;
    return avcodec_parameters_from_context(t->dst->codecpar, t->encoder);
}

static int encode_subtitle(ConvertTrack *t, AVPacket *output) {
    int err;
    for (;;) {
        if ((err = frame_read_selected(t, t->raw)) < 0)
            return err;
        AVSubtitle sub = {0};
        int got = 0;
        err = avcodec_decode_subtitle2(t->subdecoder, &sub, &got, t->raw);
        if (err >= 0 && got) {
            /* DVB full-frame bitmaps can exceed a megabyte. The advertised
             * encoder interface takes a caller-owned buffer, not an AVPacket. */
            const int capacity = 4 * 1024 * 1024;
            err = av_new_packet(output, capacity);
            if (err >= 0) {
                int64_t pts = t->raw->pts;
                int64_t duration = t->raw->duration;
                if (sub.end_display_time > sub.start_display_time &&
                    sub.end_display_time < UINT32_MAX)
                    duration = av_rescale_q(sub.end_display_time - sub.start_display_time,
                                            (AVRational){1, 1000}, t->src->time_base);
                if (pts != AV_NOPTS_VALUE)
                    pts += av_rescale_q(sub.start_display_time, (AVRational){1, 1000},
                                        t->src->time_base);
                sub.start_display_time = 0;
                err = avcodec_encode_subtitle(t->encoder, output->data, capacity, &sub);
                if (err >= 0) {
                    av_shrink_packet(output, err);
                    output->pts = output->dts = pts;
                    output->duration = duration;
                    output->flags |= AV_PKT_FLAG_KEY;
                    av_packet_rescale_ts(output, t->src->time_base, t->packet_tb);
                }
            }
        }
        avsubtitle_free(&sub);
        av_packet_unref(t->raw);
        if (err < 0 || got)
            return err < 0 ? err : 0;
    }
}

int frame_read_encoded(ConvertTrack *t, AVPacket *packet) {
    if (t->kind == 's')
        return encode_subtitle(t, packet);
    for (;;) {
        int err = avcodec_receive_packet(t->encoder, packet);
        if (err != AVERROR(EAGAIN))
            return err;
        if (t->source_done)
            return AVERROR_BUG;
        AVRational tb;
        if (t->frame_limit >= 0 && t->frames >= t->frame_limit)
            err = AVERROR_EOF;
        else if (t->kind == 'v') {
            err = frame_cadence_next(t);
            tb = t->encoder->time_base;
        } else if (t->kind == 'a' && t->encoder->frame_size > 0)
            err = frame_render_samples(t->render, t->frame, &tb, t->encoder->frame_size);
        else
            err = frame_render_next(t->render, t->frame, &tb);
        if (err == AVERROR_EOF) {
            t->source_done = 1;
            err = avcodec_send_frame(t->encoder, NULL);
        } else if (err >= 0) {
            t->frame->pts = av_rescale_q_rnd(t->frame->pts, tb, t->encoder->time_base,
                                             AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX);
            t->frame->duration = av_rescale_q(t->frame->duration, tb, t->encoder->time_base);
            t->frame->pict_type = AV_PICTURE_TYPE_NONE;
            if (t->encoder->flags & AV_CODEC_FLAG_QSCALE)
                t->frame->quality = t->encoder->global_quality;
            err = avcodec_send_frame(t->encoder, t->frame);
            av_frame_unref(t->frame);
            t->frames++;
        }
        if (err < 0)
            return err;
    }
}
