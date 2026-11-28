#include "media_clock.h"
#include <errno.h>

int64_t frame_clock_origin(const AVFormatContext *input, const int *indices, int count,
                           int64_t seek_us) {
    int64_t format_start = input->start_time == AV_NOPTS_VALUE ? 0 : input->start_time;
    int64_t origin = av_sat_add64(format_start, seek_us);
    if (!(input->iformat->flags & AVFMT_TS_DISCONT) || input->start_time == AV_NOPTS_VALUE)
        return origin;
    int64_t earliest = INT64_MAX;
    for (int i = 0; i < count; ++i) {
        if (indices[i] < 0 || (unsigned)indices[i] >= input->nb_streams)
            continue;
        const AVStream *s = input->streams[indices[i]];
        if (s->start_time != AV_NOPTS_VALUE) {
            int64_t start = av_rescale_q(s->start_time, s->time_base, AV_TIME_BASE_Q);
            earliest = FFMIN(earliest, start);
        }
    }
    return earliest == INT64_MAX || earliest == format_start ? origin : FFMAX(earliest, origin);
}

int frame_clock_init(FramePacketClock *clock, const AVFormatContext *input, const int *indices,
                     int count) {
    if (!(input->iformat->flags & AVFMT_TS_DISCONT))
        return 0;
    clock->streams = av_calloc(input->nb_streams, sizeof(*clock->streams));
    if (!clock->streams)
        return AVERROR(ENOMEM);
    clock->count = input->nb_streams;
    for (int i = 0; i < count; ++i)
        if (indices[i] >= 0 && (unsigned)indices[i] < clock->count)
            clock->streams[indices[i]].enabled = 1;
    return 0;
}

void frame_clock_packet(FramePacketClock *clock, const AVFormatContext *input, AVPacket *packet) {
    if (!clock->streams || (unsigned)packet->stream_index >= clock->count)
        return;
    FrameStreamClock *stream = &clock->streams[packet->stream_index];
    const AVStream *s = input->streams[packet->stream_index];
    int av = s->codecpar->codec_type == AVMEDIA_TYPE_AUDIO ||
             s->codecpar->codec_type == AVMEDIA_TYPE_VIDEO;
    if (stream->enabled && av && packet->dts != AV_NOPTS_VALUE) {
        int64_t dts =
            av_sat_add64(av_rescale_q(packet->dts, s->time_base, AV_TIME_BASE_Q), clock->offset_us);
        int64_t expected = stream->observed ? stream->next_us : clock->last_us;
        int64_t delta = av_sat_sub64(dts, expected);
        /* Match the ten-second discontinuity tolerance used for MPEG-TS/HLS
         * imports. A backwards jump over 100ms also resets the clock. */
        if (clock->observed &&
            (delta > 10 * AV_TIME_BASE || delta < -10 * AV_TIME_BASE ||
             (stream->observed && dts < av_sat_sub64(stream->last_us, AV_TIME_BASE / 10)))) {
            clock->offset_us = av_sat_sub64(clock->offset_us, delta);
            dts = expected;
        }
        int64_t duration = av_rescale_q(packet->duration, s->time_base, AV_TIME_BASE_Q);
        if (duration <= 0 && s->avg_frame_rate.num > 0)
            duration = av_rescale_q(1, av_inv_q(s->avg_frame_rate), AV_TIME_BASE_Q);
        if (duration <= 0 && s->codecpar->codec_type == AVMEDIA_TYPE_AUDIO &&
            s->codecpar->sample_rate > 0)
            duration = av_rescale(s->codecpar->frame_size, AV_TIME_BASE, s->codecpar->sample_rate);
        stream->next_us = av_sat_add64(dts, FFMAX(0, duration));
        stream->last_us = clock->last_us = dts;
        stream->observed = clock->observed = 1;
    }
    int64_t offset = av_rescale_q(clock->offset_us, AV_TIME_BASE_Q, s->time_base);
    if (packet->pts != AV_NOPTS_VALUE)
        packet->pts = av_sat_add64(packet->pts, offset);
    if (packet->dts != AV_NOPTS_VALUE)
        packet->dts = av_sat_add64(packet->dts, offset);
}

void frame_clock_free(FramePacketClock *clock) { av_freep(&clock->streams); }
