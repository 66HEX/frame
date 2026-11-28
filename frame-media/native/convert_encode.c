#include "convert.h"

/* Codec names in saved presets may use the descriptor name (mp3, vp9)
 * while the library exposes a specific implementation (libmp3lame). */
const AVCodec *frame_find_encoder(const char *name) {
    const AVCodec *codec = avcodec_find_encoder_by_name(name);
    const AVCodecDescriptor *desc = avcodec_descriptor_get_by_name(name);
    if (desc && (!codec || codec->capabilities & AV_CODEC_CAP_EXPERIMENTAL)) {
        const AVCodec *preferred = avcodec_find_encoder(desc->id);
        if (preferred)
            return preferred;
    }
    return codec;
}

static int audio_output_rate(const char *requested, int source_rate, const AVCodec *codec,
                             const int *rates, const char *muxer, int channels) {
    if (requested) {
        char *end;
        errno = 0;
        long value = strtol(requested, &end, 10);
        if (errno || end == requested || *end || value <= 0 || value > INT_MAX) {
            fprintf(stderr, "Invalid audio sample rate '%s': expected positive Hz\n", requested);
            return AVERROR(EINVAL);
        }
        int supported = !rates;
        for (const int *rate = rates; rate && *rate; ++rate)
            supported |= *rate == value;
        // Match each encoded output rate before opening its resampler. RIFF
        // and ALAC write byte/bit-rate fields using signed integer products.
        if (value > INT_MAX / 2 ||
            (codec->id == AV_CODEC_ID_FLAC && value >= (1 << 20)) ||
            (codec->id == AV_CODEC_ID_OPUS && value != 48000) ||
            (codec->id == AV_CODEC_ID_ALAC && value > INT_MAX / 24 / channels) ||
            (codec->id == AV_CODEC_ID_PCM_S16LE && !strcmp(muxer, "wav") &&
             value > INT_MAX / 2 / channels))
            supported = 0;
        if (!supported) {
            fprintf(stderr, "Encoder %s cannot export audio at %ld Hz\n", codec->name, value);
            return AVERROR(EINVAL);
        }
        return (int)value;
    }
    if (!rates || !*rates)
        return source_rate;
    int closest = *rates;
    for (const int *rate = rates + 1; *rate; ++rate)
        if (llabs((int64_t)*rate - source_rate) < llabs((int64_t)closest - source_rate))
            closest = *rate;
    return closest;
}

static int append_constraints(const ConvertPlan *p, const ConvertTrack *t, const AVCodec *codec,
                              const char *muxer, AVBPrint *tail) {
    const AVCodecParameters *src = t->src->codecpar;
    int err;
    if (t->kind == 'v') {
        const char *fixed = frame_plan_opt(p, "pix_fmt");
        if (t->route.device) {
            av_bprintf(tail, "format=pix_fmts=%s", av_get_pix_fmt_name(t->route.sample_format));
            return av_bprint_is_complete(tail) ? 0 : AVERROR(ENOMEM);
        }
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
        const char *channels = frame_track_opt(p, t, "ac");
        int channel_count = channels ? atoi(channels) : src->ch_layout.nb_channels;
        int sample_rate = audio_output_rate(ar, src->sample_rate, codec, rates, muxer,
                                            FFMAX(channel_count, 1));
        if (sample_rate <= 0)
            return sample_rate < 0 ? sample_rate : AVERROR_INVALIDDATA;
        av_bprintf(tail, "aformat=sample_rates=%d", sample_rate);
        if (fmts) {
            av_bprintf(tail, ":sample_fmts=");
            for (int i = 0; fmts[i] != AV_SAMPLE_FMT_NONE; i++)
                av_bprintf(tail, "%s%s", i ? "|" : "", av_get_sample_fmt_name(fmts[i]));
        }
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

static int share_local_container(const ConvertTrack *t) {
    const char *protocol = avio_find_protocol_name(t->source->path);
    if (!protocol || strcmp(protocol, "file") || !t->input->pb || !t->input->pb->seekable)
        return 0;
    // The AVIO cursor represents one seekable file. Manifest demuxers keep
    // ownership of their nested inputs and the protocols that resolve them.
    if (t->input->iformat->flags & AVFMT_NOFILE)
        return 0;
    static const char *const manifests[] = {"hls", "dash", "concat", "sdp", NULL};
    for (const char *const *name = manifests; *name; ++name)
        if (av_match_name(*name, t->input->iformat->name))
            return 0;
    return 1;
}

/* The first decoded frame proves that the input profile works on this adapter.
 * Retry only decoder initialization/first-frame failures; graph and overlay
 * errors must still identify the offending operation. */
static int negotiate_decoder(const ConvertPlan *p, ConvertTrack *t, FrameRenderPlan *request) {
    const char *pin = t->kind == 'v' ? frame_plan_opt(p, "frame_decoder") : NULL;
    if (pin && !strcmp(pin, "cpu"))
        request->hardware_device = NULL;
    if (pin && strcmp(pin, "cpu") &&
        (!request->hardware_device || strcmp(pin, request->hardware_device)))
        return AVERROR(EINVAL);
    int decoder_failed = 0;
    request->hardware_failed = &decoder_failed;
    int err = frame_render_open(&t->render, request);
    if (err < 0 && err != AVERROR_EOF && err != AVERROR(ENOMEM) && decoder_failed && !pin) {
        request->hardware_device = NULL;
        err = frame_render_open(&t->render, request);
        t->route.fallback = "Hardware decoding could not initialize for this source";
    }
    // A seek past EOF still has a valid stream description. Open at the start
    // to configure the encoder and drain an empty interval.
    if (err == AVERROR_EOF && request->start_us > 0) {
        request->start_us = 0;
        request->duration_us = 0;
        return negotiate_decoder(p, t, request);
    }
    if (err < 0 || t->kind != 'v')
        return err;
    if (frame_render_downloaded(t->render))
        t->route.decoder = request->hardware_device;
    else if (request->hardware_device)
        t->route.fallback = "Hardware decoding is unavailable for this source";
    if (pin && strcmp(t->route.decoder, pin)) {
        fprintf(stderr, "The decoder changed after hardware preflight. Check the export again.\n");
        return AVERROR(ENODEV);
    }
    return 0;
}

static int open_render(const ConvertPlan *p, ConvertTrack *t, const AVCodec *codec, const char *muxer) {
    AVBPrint tail;
    av_bprint_init(&tail, 128, AV_BPRINT_SIZE_UNLIMITED);
    int err = append_constraints(p, t, codec, muxer, &tail);
    if (err >= 0) {
        const char *graph =
            t->kind == 'a' ? frame_plan_opt(p, "af") : frame_plan_opt(p, "filter_complex");
        if (!graph && t->kind == 'v')
            graph = frame_plan_opt(p, "vf");
        if (!t->source->shared_file && t->source->mapped_count > 1 && share_local_container(t)) {
            err = frame_media_file_open(&t->source->shared_file, t->source->path);
        }
        FrameRenderPlan request = {
            .device_context = t->route.device,
            .path = t->source->path,
            .shared_file = t->source->shared_file,
            .threads = t->kind == 'a' && t->source->mapped_count > 1 ? 1 : 0,
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
        if (err >= 0)
            err = negotiate_decoder(p, t, &request);
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
        t->route.sample_format = ctx->pix_fmt;
        const char *pin = frame_plan_opt(p, "frame_format");
        if (pin && strcmp(pin, av_get_pix_fmt_name(ctx->pix_fmt))) {
            fprintf(stderr, "The pixel format changed after hardware preflight. Check the export again.\n");
            return AVERROR(EINVAL);
        }
        if ((err = frame_prepare_surface(ctx, &t->route)) < 0)
            return err;
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

static int set_pass_options(const ConvertPlan *p, AVCodecContext *ctx, AVDictionary **options) {
    ctx->flags |= p->video_pass == 1 ? AV_CODEC_FLAG_PASS1 : AV_CODEC_FLAG_PASS2;
    const char *codec = ctx->codec->name;
    if (!strcmp(codec, "libvpx-vp9")) {
        // libvpx exports its entire base64 statistics block when drained.
        // The caller owns stats_in until the second encoder is closed.
        ctx->stats_in = p->vpx_stats;
        return 0;
    }
    if (!strcmp(codec, "libx264"))
        return av_dict_set(options, "stats", p->stats_path, 0);

    // Set the x265 dictionary directly. Windows drive letters and colons in
    // filenames must remain part of the filename. Older libavcodec releases
    // also require pass and stats through this dictionary.
    AVDictionary *params = NULL;
    const AVDictionaryEntry *existing = av_dict_get(*options, "x265-params", NULL, 0);
    int err = existing ? av_dict_parse_string(&params, existing->value, "=", ":", 0) : 0;
    if (err >= 0)
        err = av_dict_set(&params, "pass", p->video_pass == 1 ? "1" : "2", 0);
    if (err >= 0)
        err = av_dict_set(&params, "stats", p->stats_path, 0);
    if (err >= 0)
        err = av_opt_set_dict_val(ctx->priv_data, "x265-params", params, 0);
    av_dict_free(&params);
    if (err >= 0)
        err = av_dict_set(options, "x265-params", NULL, 0);
    return err;
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
    if (p->video_pass && t->kind == 'v' && (err = set_pass_options(p, t->encoder, &options)) < 0)
        goto end;
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
    const AVCodec *codec = frame_find_encoder(name);
    if (!codec || codec->type != t->src->codecpar->codec_type)
        return AVERROR_ENCODER_NOT_FOUND;
    t->encoder = avcodec_alloc_context3(codec);
    t->frame = av_frame_alloc();
    if (!t->encoder || !t->frame)
        return AVERROR(ENOMEM);
    if (out->oformat->flags & AVFMT_GLOBALHEADER)
        t->encoder->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    if (t->kind == 'v') {
        enum AVPixelFormat samples = frame_surface_samples(codec, frame_plan_opt(p, "pix_fmt"),
                                                           t->src->codecpar->format);
        int rc = frame_hardware_route(&t->route, codec, samples, frame_plan_opt(p, "frame_device"));
        if (rc < 0) {
            fprintf(stderr, "Cannot open %s on a compatible adapter: %s\n", name, av_err2str(rc));
            return rc;
        }
    }
    int err = t->kind == 's' ? open_subtitle_decoder(t) : open_render(p, t, codec, out->oformat->name);
    if (err < 0 || (err = open_codec(p, t, codec)) < 0)
        return err;
    if (t->kind == 'a' && t->encoder->frame_size) {
        t->audio_input = av_frame_alloc();
        if (!t->audio_input)
            return AVERROR(ENOMEM);
    }
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

int frame_encode_subtitle(ConvertTrack *t, AVPacket *output) {
    int err;
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
            if (sub.end_display_time > sub.start_display_time && sub.end_display_time < UINT32_MAX)
                duration = av_rescale_q(sub.end_display_time - sub.start_display_time,
                                        (AVRational){1, 1000}, t->src->time_base);
            if (pts != AV_NOPTS_VALUE)
                pts +=
                    av_rescale_q(sub.start_display_time, (AVRational){1, 1000}, t->src->time_base);
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
    return err < 0 ? err : got ? 0 : AVERROR(EAGAIN);
}

/* Keep the unconsumed input outside libavfilter. A format change rebuilds the
 * graph, but the encoder still needs every sample in its partially filled
 * block. */
static int read_audio_block(ConvertTrack *t) {
    AVCodecContext *ctx = t->encoder;
    AVFrame *block = t->frame, *in = t->audio_input;
    int filled = 0;
    if (t->audio_eof)
        return AVERROR_EOF;
    while (filled < ctx->frame_size) {
        if (t->consumed_samples == in->nb_samples) {
            if (t->audio_eof)
                break;
            av_frame_unref(in);
            int rc = frame_render_next(t->render, in, &t->audio_input_tb);
            if (rc == AVERROR_EOF) {
                t->audio_eof = 1;
                break;
            }
            if (rc < 0)
                return rc;
            if (in->format != ctx->sample_fmt || in->sample_rate != ctx->sample_rate ||
                av_channel_layout_compare(&in->ch_layout, &ctx->ch_layout))
                return AVERROR_INPUT_CHANGED;
            t->consumed_samples = 0;
        }
        if (!filled) {
            int rc = av_frame_copy_props(block, in);
            if (rc < 0 || (rc = av_channel_layout_copy(&block->ch_layout, &ctx->ch_layout)) < 0)
                return rc;
            block->format = ctx->sample_fmt;
            block->sample_rate = ctx->sample_rate;
            block->nb_samples = ctx->frame_size;
            block->time_base = ctx->time_base;
            block->pts =
                in->pts == AV_NOPTS_VALUE
                    ? AV_NOPTS_VALUE
                    : av_sat_add64(av_rescale_q(in->pts, t->audio_input_tb, ctx->time_base),
                                   t->consumed_samples);
            if ((rc = av_frame_get_buffer(block, 0)) < 0)
                return rc;
        }
        int take = FFMIN(ctx->frame_size - filled, in->nb_samples - t->consumed_samples);
        int rc =
            av_samples_copy(block->extended_data, in->extended_data, filled, t->consumed_samples,
                            take, ctx->ch_layout.nb_channels, ctx->sample_fmt);
        if (rc < 0)
            return rc;
        filled += take;
        t->consumed_samples += take;
    }
    if (!filled)
        return AVERROR_EOF;
    block->nb_samples = filled;
    block->duration = filled;
    return 0;
}

int frame_read_encoded(ConvertTrack *t, AVPacket *packet) {
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
        } else if (t->audio_input) {
            err = read_audio_block(t);
            tb = t->encoder->time_base;
        } else
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
            err = t->kind == 'v' ? frame_send_video_frame(t->encoder, t->frame)
                                 : avcodec_send_frame(t->encoder, t->frame);
            if (t->analysis_io && err >= 0 && t->frame->pts != AV_NOPTS_VALUE) {
                int64_t end = av_rescale_q(av_sat_add64(t->frame->pts, t->frame->duration),
                                           t->encoder->time_base, AV_TIME_BASE_Q);
                t->analysis_end = FFMAX(t->analysis_end, end);
                frame_output_stats(t->analysis_io, t->frames + 1, t->analysis_end);
            }
            av_frame_unref(t->frame);
            t->frames++;
        }
        if (err < 0)
            return err;
    }
}
