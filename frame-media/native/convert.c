#include "convert.h"
#include <libavutil/intreadwrite.h>

static int set_metadata(AVDictionary **dict, const char *pair) {
    const char *sep = strchr(pair, '=');
    if (!sep || sep == pair)
        return AVERROR(EINVAL);
    char *key = av_strndup(pair, (size_t)(sep - pair));
    if (!key)
        return AVERROR(ENOMEM);
    return av_dict_set(dict, key, sep[1] ? sep + 1 : NULL, AV_DICT_DONT_STRDUP_KEY);
}

static int input_for_map(ConvertTrack *t, const ConvertPlan *p, const char *map) {
    if (map[0] == '[') {
        if (!frame_plan_opt(p, "filter_complex"))
            return AVERROR(EINVAL);
        map = "0:v:0";
    }
    char *end;
    long source = strtol(map, &end, 10);
    if (end == map || *end != ':' || source < 0 || source >= p->sources_len)
        return AVERROR(EINVAL);
    t->input_num = (int)source;
    t->source = &p->sources[source];
    int err = avformat_open_input(&t->input, t->source->path, NULL, NULL);
    if (err < 0 || (err = avformat_find_stream_info(t->input, NULL)) < 0)
        return err;
    const char *selector = end + 1;
    for (unsigned i = 0; i < t->input->nb_streams; ++i) {
        AVStream *stream = t->input->streams[i];
        size_t length = strlen(selector);
        char *spec =
            av_strndup(selector, length && selector[length - 1] == '?' ? length - 1 : length);
        if (!spec)
            return AVERROR(ENOMEM);
        err = avformat_match_stream_specifier(t->input, stream, spec);
        av_free(spec);
        if (err < 0)
            return err;
        if (err > 0) {
            t->src = stream;
            break;
        }
    }
    if (!t->src)
        return AVERROR_STREAM_NOT_FOUND;
    switch (t->src->codecpar->codec_type) {
    case AVMEDIA_TYPE_VIDEO:
        t->kind = 'v';
        break;
    case AVMEDIA_TYPE_AUDIO:
        t->kind = 'a';
        break;
    case AVMEDIA_TYPE_SUBTITLE:
        t->kind = 's';
        break;
    default:
        return AVERROR(EINVAL);
    }
    t->origin = av_sat_add64(t->source->seek,
                             t->input->start_time == AV_NOPTS_VALUE ? 0 : t->input->start_time);
    if (t->source->seek > 0 && (t->input_num == 0 || t->kind != 's')) {
        err = avformat_seek_file(t->input, -1, INT64_MIN, t->origin, t->origin, 0);
        if (err < 0)
            return err;
    }
    t->origin = av_rescale_q(t->origin, AV_TIME_BASE_Q, t->src->time_base);
    t->duration =
        p->duration < 0 ? -1 : av_rescale_q(p->duration, AV_TIME_BASE_Q, t->src->time_base);
    return 0;
}

static int configure_track(const ConvertPlan *p, ConvertTrack *t, AVFormatContext *out) {
    t->next = av_packet_alloc();
    t->raw = av_packet_alloc();
    t->filtered_input = av_packet_alloc();
    t->dst = avformat_new_stream(out, NULL);
    if (!t->next || !t->raw || !t->filtered_input || !t->dst)
        return AVERROR(ENOMEM);
    const char *codec = frame_track_opt(p, t, "c");
    if (!codec)
        return AVERROR_ENCODER_NOT_FOUND;
    const char *limit = frame_track_opt(p, t, "frames");
    t->frame_limit = limit ? strtoll(limit, NULL, 10) : -1;
    int err;
    if (!strcmp(codec, "copy")) {
        if ((err = avcodec_parameters_copy(t->dst->codecpar, t->src->codecpar)) < 0)
            return err;
        const AVCodecParameters *par = t->src->codecpar;
        unsigned default_tag;
        if (out->oformat->codec_tag &&
            av_codec_get_id(out->oformat->codec_tag, par->codec_tag) != par->codec_id &&
            av_codec_get_tag2(out->oformat->codec_tag, par->codec_id, &default_tag))
            t->dst->codecpar->codec_tag = 0;
        t->dst->time_base = t->packet_tb = t->src->time_base;
        t->dst->avg_frame_rate = t->src->avg_frame_rate;
        t->dst->sample_aspect_ratio = t->src->sample_aspect_ratio;
        const char *size = frame_track_opt(p, t, "frame_size");
        if (size)
            t->dst->codecpar->frame_size = atoi(size);
        t->seen_key = frame_track_opt(p, t, "copyinkf") != NULL;
    } else if ((err = frame_prepare_encoder(p, t, out, codec)) < 0)
        return err;
    const char *tag = frame_track_opt(p, t, "tag");
    if (tag) {
        if (!strncmp(tag, "0x", 2))
            t->dst->codecpar->codec_tag = (uint32_t)strtoul(tag, NULL, 0);
        else if (strlen(tag) == 4)
            t->dst->codecpar->codec_tag = AV_RL32(tag);
        else
            return AVERROR(EINVAL);
    }
    const char *bsf = frame_track_opt(p, t, "bsf");
    if (bsf) {
        if ((err = av_bsf_list_parse_str(bsf, &t->bsf)) < 0 ||
            (err = avcodec_parameters_copy(t->bsf->par_in, t->dst->codecpar)) < 0)
            return err;
        t->bsf->time_base_in = t->packet_tb;
        if ((err = av_bsf_init(t->bsf)) < 0 ||
            (err = avcodec_parameters_copy(t->dst->codecpar, t->bsf->par_out)) < 0)
            return err;
        t->dst->time_base = t->bsf->time_base_out;
    }
    if (!frame_plan_opt(p, "map_metadata") &&
        (err = av_dict_copy(&t->dst->metadata, t->src->metadata, 0)) < 0)
        return err;
    t->dst->disposition = t->src->disposition;
    for (int i = 0; i < p->options_len; i++) {
        Option opt = p->options[i];
        if (frame_option_matches(opt.name, "metadata:s", t)) {
            if ((err = set_metadata(&t->dst->metadata, opt.value)) < 0)
                return err;
        } else if (frame_option_matches(opt.name, "disposition", t)) {
            if (!strcmp(opt.value, "default+forced")) t->dst->disposition = AV_DISPOSITION_DEFAULT | AV_DISPOSITION_FORCED;
            else if (!strcmp(opt.value, "0")) t->dst->disposition = 0;
            else if ((err = av_disposition_from_string(opt.value)) < 0) return err;
            else t->dst->disposition = err;
        }
    }
    return 0;
}

/* Return the next packet of a mapped stream, in its source time base relative
 * to the shared input origin. Negative decode timestamps are required for
 * B-frames and audio priming. The muxer decides how to represent them. */
int frame_read_selected(ConvertTrack *t, AVPacket *packet) {
    for (;;) {
        av_packet_unref(packet);
        int err = av_read_frame(t->input, packet);
        if (err < 0)
            return err;
        if (packet->stream_index != t->src->index)
            continue;
        if (!t->encoder && !t->seen_key) {
            if (!(packet->flags & AV_PKT_FLAG_KEY))
                continue;
            t->seen_key = 1;
        }
        if (packet->pts != AV_NOPTS_VALUE)
            packet->pts = av_sat_sub64(packet->pts, t->origin);
        if (packet->dts != AV_NOPTS_VALUE)
            packet->dts = av_sat_sub64(packet->dts, t->origin);
        int64_t clock = packet->dts == AV_NOPTS_VALUE ? packet->pts : packet->dts;
        if (t->duration >= 0 && clock != AV_NOPTS_VALUE && clock >= t->duration) {
            av_packet_unref(packet);
            return AVERROR_EOF;
        }
        packet->pos = -1;
        return 0;
    }
}

static int read_output_packet(ConvertTrack *t) {
    for (;;) {
        if (t->bsf) {
            int err = av_bsf_receive_packet(t->bsf, t->next);
            if (err != AVERROR(EAGAIN))
                return err;
            if (t->codec_done)
                return AVERROR_BUG;
        }
        AVPacket *dest = t->bsf ? t->filtered_input : t->next;
        int err = t->encoder ? frame_read_encoded(t, dest) : frame_read_selected(t, dest);
        if (!t->bsf || (err < 0 && err != AVERROR_EOF))
            return err;
        t->codec_done = err == AVERROR_EOF;
        err = av_bsf_send_packet(t->bsf, t->codec_done ? NULL : dest);
        if (err < 0)
            return err;
    }
}

static AVRational output_tb(const ConvertTrack *t) {
    return t->bsf ? t->bsf->time_base_out : t->packet_tb;
}

static int64_t clock_us(const ConvertTrack *t) {
    const int64_t stamp = t->next->dts != AV_NOPTS_VALUE ? t->next->dts : t->next->pts;
    return stamp == AV_NOPTS_VALUE ? INT64_MIN : av_rescale_q(stamp, output_tb(t), AV_TIME_BASE_Q);
}

static int fill_track(ConvertTrack *t) {
    int err = read_output_packet(t);
    if (err == AVERROR_EOF) {
        t->done = 1;
        return 0;
    }
    return err;
}

static int copy_chapters(const ConvertPlan *p, const AVFormatContext *input, AVFormatContext *out) {
    if (!input->nb_chapters)
        return 0;
    out->chapters = av_calloc(input->nb_chapters, sizeof(*out->chapters));
    if (!out->chapters)
        return AVERROR(ENOMEM);
    for (unsigned i = 0; i < input->nb_chapters; i++) {
        const AVChapter *src = input->chapters[i];
        int64_t seek = av_rescale_q(p->sources[0].seek, AV_TIME_BASE_Q, src->time_base);
        int64_t start = av_sat_sub64(src->start, seek), end = av_sat_sub64(src->end, seek);
        if (p->duration >= 0)
            end = FFMIN(end, av_rescale_q(p->duration, AV_TIME_BASE_Q, src->time_base));
        if (end <= FFMAX(0, start))
            continue;
        AVChapter *dst = av_mallocz(sizeof(*dst));
        if (!dst)
            return AVERROR(ENOMEM);
        out->chapters[out->nb_chapters++] = dst;
        dst->id = src->id;
        dst->time_base = src->time_base;
        dst->start = FFMAX(0, start);
        dst->end = end;
        int err = av_dict_copy(&dst->metadata, src->metadata, 0);
        if (err < 0)
            return err;
    }
    return 0;
}

static int setup_muxer(const ConvertPlan *p, AVFormatContext *out, ConvertTrack *tracks, int count,
                       void *io, AVIOContext **pb) {
    int err = 0;
    AVFormatContext *primary = NULL;
    for (int i = 0; i < count; ++i)
        if (tracks[i].input_num == 0) {
            primary = tracks[i].input;
            break;
        }
    if (primary) {
        if (!frame_plan_opt(p, "map_metadata") &&
            (err = av_dict_copy(&out->metadata, primary->metadata, 0)) < 0)
            return err;
        if ((err = copy_chapters(p, primary, out)) < 0)
            return err;
        if (primary->duration > 0) {
            int64_t secs = primary->duration / AV_TIME_BASE;
            fprintf(stderr, "Duration: %02" PRId64 ":%02" PRId64 ":%02" PRId64 ".%03" PRId64 "\n",
                    secs / 3600, (secs / 60) % 60, secs % 60,
                    (primary->duration % AV_TIME_BASE) / 1000);
        }
    }
    AVDictionary *options = NULL;
    static const char *const keys[] = {
        "mpegts_m2ts_mode", "allow_raw_vfw", "loop", "hash", "format_version", NULL};
    for (const char *const *k = keys; *k; k++) {
        const char *v = frame_plan_opt(p, *k);
        if (v && (err = av_dict_set(&options, *k, v, 0)) < 0)
            goto done;
    }
    for (int i = 0; i < p->options_len; i++) {
        if (!strcmp(p->options[i].name, "metadata") &&
            (err = set_metadata(&out->metadata, p->options[i].value)) < 0)
            goto done;
    }
    if (!(out->oformat->flags & AVFMT_NOFILE)) {
        int seekable = frame_output_open(io, p->output, frame_plan_opt(p, "y") != NULL, 0);
        if (seekable < 0) {
            err = AVERROR(EIO);
            goto done;
        }
        unsigned char *buf = av_malloc(32768);
        if (!buf) {
            err = AVERROR(ENOMEM);
            goto done;
        }
        *pb = avio_alloc_context(buf, 32768, 1, io, NULL, frame_output_write,
                                 seekable ? frame_output_seek : NULL);
        if (!*pb) {
            av_free(buf);
            err = AVERROR(ENOMEM);
            goto done;
        }
        out->pb = *pb;
        out->flags |= AVFMT_FLAG_CUSTOM_IO;
    }
    err = avformat_write_header(out, &options);
    if (err >= 0 && av_dict_count(options)) {
        fprintf(stderr, "Muxer %s rejected option %s\n", out->oformat->name,
                av_dict_get(options, "", NULL, AV_DICT_IGNORE_SUFFIX)->key);
        err = AVERROR_OPTION_NOT_FOUND;
    }
done:
    av_dict_free(&options);
    return err;
}

static int convert(const ConvertPlan *p, AVFormatContext **outp, ConvertTrack *tracks, int *count,
                   void *io, AVIOContext **pb) {
    const char *format = frame_plan_opt(p, "f");
    /* Still-image exports use the same exclusive file handle as containers.
     * image2pipe writes exactly the encoded image packet to that handle. */
    if (frame_plan_opt(p, "update"))
        format = "image2pipe";
    int err = avformat_alloc_output_context2(outp, NULL, format, p->output);
    if (err < 0)
        return err;
    AVFormatContext *out = *outp;
    for (int i = 0; i < p->options_len; i++) {
        if (strcmp(p->options[i].name, "map"))
            continue;
        ConvertTrack *track = &tracks[(*count)++];
        err = input_for_map(track, p, p->options[i].value);
        if (err == AVERROR_STREAM_NOT_FOUND && strchr(p->options[i].value, '?')) {
            avformat_close_input(&track->input);
            memset(track, 0, sizeof(*track));
            (*count)--;
            continue;
        }
        if (err < 0)
            return err;
        for (int k = 0; k < *count - 1; ++k)
            track->ordinal += tracks[k].kind == track->kind;
        if ((err = configure_track(p, track, out)) < 0)
            return err;
    }
    if (!*count)
        return AVERROR_STREAM_NOT_FOUND;
    const char *progress = frame_plan_opt(p, "progress");
    if (progress && frame_output_open(io, progress, 1, 1) < 0)
        return AVERROR(EIO);
    if ((err = setup_muxer(p, out, tracks, *count, io, pb)) < 0)
        return err;
    for (int i = 0; i < *count; ++i)
        if ((err = fill_track(&tracks[i])) < 0)
            return err;
    int64_t video_frames = 0;
    for (;;) {
        ConvertTrack *earliest = NULL;
        for (int i = 0; i < *count; ++i) {
            if (!tracks[i].done && (!earliest || clock_us(&tracks[i]) < clock_us(earliest)))
                earliest = &tracks[i];
        }
        if (!earliest)
            break;
        int64_t stamp = clock_us(earliest);
        AVPacket *pkt = earliest->next;
        av_packet_rescale_ts(pkt, output_tb(earliest), earliest->dst->time_base);
        pkt->stream_index = earliest->dst->index;
        pkt->pos = -1;
        if ((err = av_interleaved_write_frame(out, pkt)) < 0)
            return err;
        if (earliest->kind == 'v' && !earliest->ordinal)
            ++video_frames;
        frame_output_stats(io, video_frames, stamp);
        if ((err = fill_track(earliest)) < 0)
            return err;
    }
    err = av_write_trailer(out);
    if (*pb) {
        avio_flush(*pb);
        if (err >= 0 && (*pb)->error < 0)
            err = (*pb)->error;
    }
    return err;
}

int frame_convert(int argc, const char *const *argv, void *io) {
    ConvertPlan p = {0};
    AVIOContext *pb = NULL;
    AVFormatContext *out = NULL;
    ConvertTrack *tracks = NULL;
    int count = 0;
    int err = frame_convert_parse(&p, argc, argv);
    if (err >= 0) {
        tracks = av_calloc(p.options_len, sizeof(*tracks));
        if (!tracks)
            err = AVERROR(ENOMEM);
        else
            err = convert(&p, &out, tracks, &count, io, &pb);
    }
    for (int i = 0; i < count; ++i) {
        ConvertTrack *t = &tracks[i];
        av_packet_free(&t->next);
        av_packet_free(&t->raw);
        av_packet_free(&t->filtered_input);
        av_frame_free(&t->frame);
        av_bsf_free(&t->bsf);
        frame_render_close(&t->render);
        avcodec_free_context(&t->encoder);
        avcodec_free_context(&t->subdecoder);
        avformat_close_input(&t->input);
    }
    av_free(tracks);
    if (pb) {
        av_freep(&pb->buffer);
        avio_context_free(&pb);
    }
    avformat_free_context(out);
    av_free(p.sources);
    av_free(p.options);
    return err;
}
