// Media inspection and packet observations for Frame's probe worker.
#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/avstring.h>
#include <libavutil/display.h>
#include <libavutil/intreadwrite.h>
#include <libavutil/opt.h>
#include <libavutil/pixdesc.h>
#include <libavutil/sha.h>

enum ProbeMode { METADATA, OPUS_TIMING, MANUAL_ALAC, NORMAL_ALAC, PHYSICAL_AUDIO };

// Packet captures are streamed, so the parent can enforce its capture budget
// without a second copy of a potentially long packet ledger in this process.
static void quoted(const char *value)
{
    const uint8_t *p = (const uint8_t *)value;
    const uint8_t *end = p + strlen(value);
    putchar('"');
    while (p < end) {
        if (*p >= 0x80) {
            const uint8_t *start = p;
            int32_t codepoint;
            if (av_utf8_decode(&codepoint, &p, end, 0) >= 0)
                fwrite(start, 1, p - start, stdout);
        } else if (*p < 0x20) {
            printf("\\u%04x", *p++);
        } else {
            if (*p == '"' || *p == '\\') putchar('\\');
            putchar(*p++);
        }
    }
    putchar('"');
}

static void string_field(const char *key, const char *value)
{
    if (!value) return;
    printf(",\"%s\":", key);
    quoted(value);
}

static void time_field(const char *key, int64_t value)
{
    if (value == AV_NOPTS_VALUE) return;
    uint64_t magnitude = value < 0 ? (uint64_t)-value : (uint64_t)value;
    printf(",\"%s\":\"%s%" PRIu64 ".%06" PRIu64 "\"", key, value < 0 ? "-" : "",
           magnitude / 1000000, magnitude % 1000000);
}

static void timestamp(const char *key, int64_t pts)
{
    if (pts != AV_NOPTS_VALUE) printf(",\"%s\":%" PRId64, key, pts);
}

static void tags(const AVDictionary *dict)
{
    if (!dict) return;
    const AVDictionaryEntry *entry = NULL;
    int count = 0;
    fputs(",\"tags\":{", stdout);
    while ((entry = av_dict_get(dict, "", entry, AV_DICT_IGNORE_SUFFIX))) {
        if (count++) putchar(',');
        quoted(entry->key);
        putchar(':');
        quoted(entry->value);
    }
    putchar('}');
}

static void layout(const AVChannelLayout *channels)
{
    printf(",\"channels\":%d", channels->nb_channels);
    if (channels->order != AV_CHANNEL_ORDER_UNSPEC) {
        char description[1024];
        av_channel_layout_describe(channels, description, sizeof(description));
        string_field("channel_layout", description);
    }
}

static void data_hash(const char *key, struct AVSHA *sha, const uint8_t *data, size_t size)
{
    uint8_t digest[32];
    av_sha_init(sha, 256);
    if (size) av_sha_update(sha, data, size);
    av_sha_final(sha, digest);
    printf(",\"%s\":\"SHA256:", key);
    for (size_t i = 0; i < sizeof(digest); ++i) printf("%02x", digest[i]);
    putchar('"');
}

static void video_fields(AVFormatContext *format, AVStream *stream)
{
    const AVCodecParameters *p = stream->codecpar;
    printf(",\"width\":%d,\"height\":%d", p->width, p->height);
    AVRational sar = av_guess_sample_aspect_ratio(format, stream, NULL);
    if (sar.num) printf(",\"sample_aspect_ratio\":\"%d:%d\"", sar.num, sar.den);
    string_field("pix_fmt", av_get_pix_fmt_name(p->format));
    if (p->color_range != AVCOL_RANGE_UNSPECIFIED)
        string_field("color_range", av_color_range_name(p->color_range));
    if (p->color_space != AVCOL_SPC_UNSPECIFIED)
        string_field("color_space", av_color_space_name(p->color_space));
    if (p->color_primaries != AVCOL_PRI_UNSPECIFIED)
        string_field("color_primaries", av_color_primaries_name(p->color_primaries));
#if LIBAVCODEC_VERSION_MAJOR >= 61
    const AVPacketSideData *side = av_packet_side_data_get(
        p->coded_side_data, p->nb_coded_side_data, AV_PKT_DATA_DISPLAYMATRIX);
    const uint8_t *matrix = side ? side->data : NULL;
    size_t size = side ? side->size : 0;
#else
    size_t size = 0;
    const uint8_t *matrix = av_stream_get_side_data(stream, AV_PKT_DATA_DISPLAYMATRIX, &size);
#endif
    if (matrix && size >= 9 * sizeof(int32_t)) {
        double rotation = av_display_rotation_get((const int32_t *)matrix);
        if (isfinite(rotation))
            printf(",\"side_data_list\":[{\"rotation\":%.0f}]", rotation);
    }
}

static void show_stream(AVFormatContext *format, AVStream *stream,
                        enum ProbeMode mode, const AVCodecContext *decoder, struct AVSHA *sha)
{
    const AVCodecParameters *p = stream->codecpar;
    printf("{\"index\":%d", stream->index);
    string_field("codec_name", avcodec_get_name(p->codec_id));
    string_field("codec_type", av_get_media_type_string(p->codec_type));
    printf(",\"time_base\":\"%d/%d\"", stream->time_base.num, stream->time_base.den);
    timestamp("start_pts", stream->start_time);
    if (mode == METADATA) {
        printf(",\"codec_tag\":\"0x%04" PRIx32 "\"", p->codec_tag);
        printf(",\"avg_frame_rate\":\"%d/%d\"", stream->avg_frame_rate.num, stream->avg_frame_rate.den);
        if (p->bit_rate > 0) printf(",\"bit_rate\":\"%" PRId64 "\"", p->bit_rate);
        const char *profile = avcodec_profile_name(p->codec_id, p->profile);
        if (profile) string_field("profile", profile);
        else if (p->profile != AV_PROFILE_UNKNOWN) printf(",\"profile\":\"%d\"", p->profile);
        if (p->codec_type == AVMEDIA_TYPE_VIDEO) video_fields(format, stream);
        tags(stream->metadata);
    }
    if (p->codec_type == AVMEDIA_TYPE_AUDIO && mode != OPUS_TIMING) {
        printf(",\"sample_rate\":\"%d\"", p->sample_rate);
        layout(&p->ch_layout);
        if (mode != METADATA) {
            string_field("sample_fmt", av_get_sample_fmt_name(p->format));
            int bits = decoder && decoder->bits_per_raw_sample > 0
                           ? decoder->bits_per_raw_sample : p->bits_per_raw_sample;
            printf(",\"initial_padding\":%d,\"bits_per_raw_sample\":%d", p->initial_padding, bits);
            if (p->extradata_size > 0) {
                printf(",\"extradata_size\":%d", p->extradata_size);
                data_hash("extradata_hash", sha, p->extradata, p->extradata_size);
            }
        }
    }
    putchar('}');
}

static void show_programs(AVFormatContext *input)
{
    fputs(",\"programs\":[", stdout);
    for (unsigned i = 0; i < input->nb_programs; i++) {
        const AVProgram *program = input->programs[i];
        if (i) putchar(',');
        printf("{\"program_id\":%d,\"streams\":[", program->id);
        for (unsigned j = 0; j < program->nb_stream_indexes; j++) {
            if (j) putchar(',');
            const AVStream *stream = input->streams[program->stream_index[j]];
            // Core uses the stream membership and the type from this nested list.
            printf("{\"index\":%d", stream->index);
            string_field("codec_type", av_get_media_type_string(stream->codecpar->codec_type));
            putchar('}');
        }
        putchar(']');
        tags(program->metadata);
        putchar('}');
    }
    putchar(']');
}

static void show_format(AVFormatContext *input)
{
    fputs(",\"format\":{\"format_name\":", stdout);
    quoted(input->iformat->name);
    time_field("start_time", input->start_time);
    time_field("duration", input->duration);
    if (input->bit_rate > 0) printf(",\"bit_rate\":\"%" PRId64 "\"", input->bit_rate);
    int64_t packet_size = 0;
    if (!strcmp(input->iformat->name, "mpegts") && input->priv_data &&
        av_opt_get_int(input->priv_data, "ts_packetsize", 0, &packet_size) == 0 && packet_size > 0)
        printf(",\"ts_packetsize\":\"%" PRId64 "\"", packet_size);
    tags(input->metadata);
    putchar('}');
}

static void show_packet(const AVPacket *pkt, enum ProbeMode mode, struct AVSHA *sha)
{
    printf("{\"stream_index\":%d", pkt->stream_index);
    timestamp("pts", pkt->pts);
    if (pkt->duration) printf(",\"duration\":%" PRId64, pkt->duration);
    if (mode != OPUS_TIMING) {
        string_field("type", "packet");
        string_field("codec_type", "audio");
        timestamp("dts", pkt->dts);
        printf(",\"pos\":%" PRId64 ",\"size\":%d", pkt->pos, pkt->size);
        printf(",\"flags\":\"%c%c%c\"", pkt->flags & AV_PKT_FLAG_KEY ? 'K' : '_',
               pkt->flags & AV_PKT_FLAG_DISCARD ? 'D' : '_',
               pkt->flags & AV_PKT_FLAG_CORRUPT ? 'C' : '_');
        data_hash("data_hash", sha, pkt->data, pkt->size);
        fputs(",\"side_data_list\":[", stdout);
        for (int i = 0; i < pkt->side_data_elems; ++i) {
            const AVPacketSideData *side = &pkt->side_data[i];
            if (i) putchar(',');
            fputs("{\"side_data_type\":", stdout);
            const char *name = av_packet_side_data_name(side->type);
            quoted(name ? name : "unknown");
            if (side->type == AV_PKT_DATA_SKIP_SAMPLES && side->size >= 10) {
                printf(",\"skip_samples\":%" PRIu32 ",\"discard_padding\":%" PRIu32,
                       AV_RL32(side->data), AV_RL32(side->data + 4));
            }
            putchar('}');
        }
        putchar(']');
    }
    putchar('}');
}

static void show_frame(AVFrame *frame, int stream)
{
    printf(",{\"type\":\"frame\",\"media_type\":\"audio\",\"stream_index\":%d", stream);
    timestamp("pts", frame->pts);
#if LIBAVCODEC_VERSION_MAJOR >= 60
    if (frame->opaque_ref && frame->opaque_ref->size == sizeof(int64_t)) {
        int64_t position;
        memcpy(&position, frame->opaque_ref->data, sizeof(position));
        printf(",\"pkt_pos\":%" PRId64, position);
    }
#else
    printf(",\"pkt_pos\":%" PRId64, frame->pkt_pos);
#endif
    printf(",\"nb_samples\":%d", frame->nb_samples);
    string_field("sample_fmt", av_get_sample_fmt_name(frame->format));
    layout(&frame->ch_layout);
    fputs(",\"side_data_list\":[", stdout);
    for (int i = 0; i < frame->nb_side_data; ++i) {
        if (i) putchar(',');
        fputs("{\"side_data_type\":", stdout);
        const char *name = av_frame_side_data_name(frame->side_data[i]->type);
        quoted(name ? name : "unknown");
        putchar('}');
    }
    fputs("]}", stdout);
}

static int decode_packet(AVCodecContext *decoder, AVFrame *frame, AVPacket *pkt, int stream)
{
#if LIBAVCODEC_VERSION_MAJOR >= 60
    if (pkt) {
        av_buffer_unref(&pkt->opaque_ref);
        pkt->opaque_ref = av_buffer_alloc(sizeof(pkt->pos));
        if (!pkt->opaque_ref) return AVERROR(ENOMEM);
        memcpy(pkt->opaque_ref->data, &pkt->pos, sizeof(pkt->pos));
    }
#endif
    int ret = avcodec_send_packet(decoder, pkt);
    if (ret < 0) return ret;
    while ((ret = avcodec_receive_frame(decoder, frame)) >= 0) {
        show_frame(frame, stream);
        av_frame_unref(frame);
    }
    return ret == AVERROR_EOF || ret == AVERROR(EAGAIN) ? 0 : ret;
}

static int selected(const AVStream *stream, enum ProbeMode mode, int index)
{
    return mode == METADATA || (mode == PHYSICAL_AUDIO
        ? stream->codecpar->codec_type == AVMEDIA_TYPE_AUDIO : stream->index == index);
}

static int scan_packets(AVFormatContext *input, AVCodecContext *decoder, int index, enum ProbeMode mode,
                        int64_t start, int64_t end, struct AVSHA *sha)
{
    if (start != AV_NOPTS_VALUE) {
        int ret = avformat_seek_file(input, -1, INT64_MIN, start, INT64_MAX, 0);
        if (ret < 0) return ret;
    }
    AVPacket *pkt = av_packet_alloc();
    AVFrame *frame = av_frame_alloc();
    int ret = AVERROR(ENOMEM);
    if (!pkt || !frame) goto cleanup;
    fputs(decoder ? ",\"packets_and_frames\":[" : ",\"packets\":[", stdout);
    int has_events = 0;
    while ((ret = av_read_frame(input, pkt)) >= 0) {
        AVStream *stream = input->streams[pkt->stream_index];
        if (selected(stream, mode, index)) {
            int64_t pts = pkt->pts != AV_NOPTS_VALUE ? pkt->pts : pkt->dts;
            if (end != AV_NOPTS_VALUE && pts != AV_NOPTS_VALUE &&
                av_rescale_q(pts, stream->time_base, AV_TIME_BASE_Q) >= end)
                break;
            if (has_events) putchar(',');
            has_events = 1;
            show_packet(pkt, mode, sha);
            if (decoder && (ret = decode_packet(decoder, frame, pkt, index)) < 0) goto cleanup;
        }
        av_packet_unref(pkt);
    }
    if (ret < 0 && ret != AVERROR_EOF) goto cleanup;
    // An underlying read error can also terminate an otherwise valid container.
    if (input->pb && input->pb->error < 0) { ret = input->pb->error; goto cleanup; }
    ret = decoder ? decode_packet(decoder, frame, NULL, index) : 0;
    putchar(']');
cleanup:
    av_packet_free(&pkt);
    av_frame_free(&frame);
    return ret;
}

int frame_probe_file(const char *path, int mode, int index, int64_t start, int64_t end)
{
    AVFormatContext *input = NULL;
    AVCodecContext *decoder = NULL;
    struct AVSHA *sha = NULL;
    AVDictionary *options = NULL;
    av_log_set_level(AV_LOG_ERROR);
    int ret = 0;
    if (mode == PHYSICAL_AUDIO && (ret = av_dict_set(&options, "ignore_editlist", "1", 0)) < 0)
        goto done;
    ret = avformat_open_input(&input, path, NULL, &options);
    if (ret >= 0 && av_dict_count(options)) ret = AVERROR_OPTION_NOT_FOUND;
    if (ret < 0 || (ret = avformat_find_stream_info(input, NULL)) < 0) goto done;
    if (index >= 0 && ((unsigned)index >= input->nb_streams ||
        input->streams[index]->codecpar->codec_type != AVMEDIA_TYPE_AUDIO)) {
        ret = AVERROR_STREAM_NOT_FOUND;
        goto done;
    }
    if (mode == MANUAL_ALAC || mode == NORMAL_ALAC) {
        const AVCodecParameters *p = input->streams[index]->codecpar;
        const AVCodec *codec = avcodec_find_decoder(p->codec_id);
        if (!codec || p->codec_id != AV_CODEC_ID_ALAC) { ret = AVERROR_DECODER_NOT_FOUND; goto done; }
        decoder = avcodec_alloc_context3(codec);
        if (!decoder) { ret = AVERROR(ENOMEM); goto done; }
        if ((ret = avcodec_parameters_to_context(decoder, p)) < 0) goto done;
        decoder->pkt_timebase = input->streams[index]->time_base;
        if (mode == MANUAL_ALAC) decoder->flags2 |= AV_CODEC_FLAG2_SKIP_MANUAL;
#if LIBAVCODEC_VERSION_MAJOR >= 60
        decoder->flags |= AV_CODEC_FLAG_COPY_OPAQUE;
#endif
        if ((ret = avcodec_open2(decoder, codec, NULL)) < 0) goto done;
    }
    if (mode == MANUAL_ALAC || mode == NORMAL_ALAC || mode == PHYSICAL_AUDIO) {
        sha = av_sha_alloc();
        if (!sha) { ret = AVERROR(ENOMEM); goto done; }
    }
    fputs("{\"streams\":[", stdout);
    int count = 0;
    for (unsigned i = 0; i < input->nb_streams; ++i) {
        if (selected(input->streams[i], mode, index)) {
            if (count++) putchar(',');
            show_stream(input, input->streams[i], mode, decoder, sha);
        }
    }
    putchar(']');
    if (mode == METADATA || mode == OPUS_TIMING) show_format(input);
    if (mode == METADATA) show_programs(input);
    else if ((ret = scan_packets(input, decoder, index, mode, start, end, sha)) < 0) goto done;
    puts("}");
    ret = fflush(stdout) || ferror(stdout) ? AVERROR(EIO) : 0;
done:
    if (ret < 0) {
        char message[AV_ERROR_MAX_STRING_SIZE];
        av_strerror(ret, message, sizeof(message));
        fprintf(stderr, "media probe: %s\n", message);
    }
    av_dict_free(&options);
    av_free(sha);
    avcodec_free_context(&decoder);
    avformat_close_input(&input);
    return ret;
}
