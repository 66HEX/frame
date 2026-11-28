#include "convert.h"

typedef struct {
    AVFormatContext *format;
    void *io;
    int64_t video_frames;
} Writer;

static int64_t packet_time(const AVPacket *pkt, AVRational tb) {
    int64_t stamp = pkt->dts == AV_NOPTS_VALUE ? pkt->pts : pkt->dts;
    return stamp == AV_NOPTS_VALUE ? INT64_MIN : av_rescale_q(stamp, tb, AV_TIME_BASE_Q);
}

static int write_packet(Writer *w, ConvertTrack *t, AVPacket *pkt, AVRational tb) {
    int64_t pts = pkt->pts == AV_NOPTS_VALUE ? pkt->dts : pkt->pts;
    int64_t end = pts == AV_NOPTS_VALUE ? 0
                                        : av_rescale_q(av_sat_add64(pts, FFMAX(0, pkt->duration)),
                                                       tb, AV_TIME_BASE_Q);
    av_packet_rescale_ts(pkt, tb, t->dst->time_base);
    pkt->stream_index = t->dst->index;
    pkt->pos = -1;
    int err = av_interleaved_write_frame(w->format, pkt);
    if (err < 0)
        return err;
    if (t->kind == 'v' && !t->ordinal) {
        // Re-encoding counts submitted frames, including encoder lookahead.
        // Stream copy counts muxed video access units without decoding them.
        w->video_frames = t->encoder ? t->frames : w->video_frames + 1;
    }
    frame_output_stats(w->io, w->video_frames, end);
    return 0;
}

static int write_filtered(Writer *w, ConvertTrack *t, AVPacket *packet) {
    if (!t->bsf)
        return packet ? write_packet(w, t, packet, t->packet_tb) : 0;
    int err = av_bsf_send_packet(t->bsf, packet);
    if (err < 0 && err != AVERROR_EOF)
        return err;
    for (;;) {
        err = av_bsf_receive_packet(t->bsf, t->filtered_input);
        if (err == AVERROR(EAGAIN) || err == AVERROR_EOF)
            return 0;
        if (err < 0)
            return err;
        err = write_packet(w, t, t->filtered_input, t->bsf->time_base_out);
        if (err < 0)
            return err;
    }
}

static int finish(Writer *w, ConvertTrack *t) {
    if (t->done)
        return 0;
    t->done = 1;
    if (!t->render)
        t->source->active--;
    return write_filtered(w, t, NULL);
}

/* Each source owns one demuxer and at most one lookahead packet. Sparse
 * subtitle streams never make us queue the rest of the movie while waiting
 * for their first cue. Repeated maps share payloads through AVPacket refs. */
static int next_source(Source *source, ConvertTrack *tracks, Writer *w) {
    AVPacket *pkt = source->pending;
    while (source->active) {
        av_packet_unref(pkt);
        int err = av_read_frame(source->input, pkt);
        if (err == AVERROR_EOF) {
            for (unsigned s = 0; s < source->stream_count; ++s)
                for (int i = source->first_track[s]; i >= 0; i = tracks[i].next_track)
                    if ((err = finish(w, &tracks[i])) < 0)
                        return err;
            return 0;
        }
        if (err < 0)
            return err;
        if ((unsigned)pkt->stream_index >= source->stream_count)
            continue;
        frame_clock_packet(&source->clock, source->input, pkt);
        for (int i = source->first_track[pkt->stream_index]; i >= 0; i = tracks[i].next_track)
            if (!tracks[i].done)
                return 0;
    }
    return 0;
}

static int dispatch(Writer *w, ConvertTrack *t, const AVPacket *source) {
    if (t->done)
        return 0;
    if (t->strict && (source->flags & AV_PKT_FLAG_CORRUPT)) {
        av_log(t->input, AV_LOG_ERROR, "Corrupt packet in selected stream %d\n", t->src->index);
        return AVERROR_INVALIDDATA;
    }
    if (!t->encoder && !t->seen_key) {
        if (!(source->flags & AV_PKT_FLAG_KEY))
            return 0;
        t->seen_key = 1;
    }
    av_packet_unref(t->raw);
    int err = av_packet_ref(t->raw, source);
    if (err < 0)
        return err;
    AVPacket *pkt = t->raw;
    if (pkt->pts != AV_NOPTS_VALUE)
        pkt->pts = av_sat_sub64(pkt->pts, t->origin);
    if (pkt->dts != AV_NOPTS_VALUE)
        pkt->dts = av_sat_sub64(pkt->dts, t->origin);
    int64_t clock = pkt->dts == AV_NOPTS_VALUE ? pkt->pts : pkt->dts;
    if (t->duration >= 0 && clock != AV_NOPTS_VALUE && clock >= t->duration)
        return finish(w, t);
    if (t->encoder) {
        err = frame_encode_subtitle(t, t->next);
        if (err == AVERROR(EAGAIN))
            return 0;
        if (err < 0)
            return err;
        pkt = t->next;
    }
    return write_filtered(w, t, pkt);
}

static int subscribe(Source *source, ConvertTrack *t, int index) {
    if (!source->first_track) {
        source->stream_count = source->input->nb_streams;
        source->pending = av_packet_alloc();
        source->first_track = av_malloc_array(source->input->nb_streams, sizeof(int));
        if (!source->pending || !source->first_track)
            return AVERROR(ENOMEM);
        for (unsigned i = 0; i < source->input->nb_streams; ++i)
            source->first_track[i] = -1;
        int err =
            frame_clock_init(&source->clock, source->input, source->mapped, source->mapped_count);
        if (err < 0)
            return err;
        if (source->seek > 0 && (t->input_num == 0 || t->kind != 's')) {
            int64_t start =
                source->input->start_time == AV_NOPTS_VALUE ? 0 : source->input->start_time;
            int64_t seek_at = av_sat_add64(start, source->seek);
            err = avformat_seek_file(source->input, -1, INT64_MIN, seek_at, seek_at, 0);
            if (err < 0)
                return err;
        }
    }
    t->next_track = source->first_track[t->src->index];
    source->first_track[t->src->index] = index;
    source->active++;
    return 0;
}

static int next_encoder(Writer *w, ConvertTrack *t) {
    int rc = frame_read_encoded(t, t->next);
    return rc == AVERROR_EOF ? finish(w, t) : rc;
}

int frame_mux_packets(const ConvertPlan *plan, ConvertTrack *tracks, int count,
                      AVFormatContext *out, void *io) {
    Writer w = {.format = out, .io = io};
    int err;
    for (int i = count - 1; i >= 0; --i) {
        ConvertTrack *t = &tracks[i];
        if (t->render)
            err = next_encoder(&w, t);
        else
            err = subscribe(t->source, t, i);
        if (err < 0)
            return err;
    }
    for (int i = 0; i < plan->sources_len; ++i)
        if ((err = next_source(&plan->sources[i], tracks, &w)) < 0)
            return err;
    for (;;) {
        ConvertTrack *encoded = NULL;
        Source *raw = NULL;
        int64_t earliest = INT64_MAX;
        for (int i = 0; i < count; ++i) {
            ConvertTrack *t = &tracks[i];
            if (!t->render || t->done)
                continue;
            int64_t stamp = packet_time(t->next, t->packet_tb);
            if (!encoded || stamp < earliest) {
                encoded = t;
                earliest = stamp;
            }
        }
        for (int i = 0; i < plan->sources_len; ++i) {
            Source *s = &plan->sources[i];
            if (!s->active)
                continue;
            AVStream *st = s->input->streams[s->pending->stream_index];
            int64_t stamp = av_sat_sub64(packet_time(s->pending, st->time_base), s->origin_us);
            if ((!raw && !encoded) || stamp < earliest) {
                raw = s;
                earliest = stamp;
            }
        }
        if (raw) {
            for (int i = raw->first_track[raw->pending->stream_index]; i >= 0;
                 i = tracks[i].next_track)
                if ((err = dispatch(&w, &tracks[i], raw->pending)) < 0)
                    return err;
            if ((err = next_source(raw, tracks, &w)) < 0)
                return err;
        } else if (encoded) {
            if ((err = write_filtered(&w, encoded, encoded->next)) < 0 ||
                (err = next_encoder(&w, encoded)) < 0)
                return err;
        } else
            return 0;
    }
}
