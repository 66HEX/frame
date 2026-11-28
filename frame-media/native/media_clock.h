#ifndef FRAME_MEDIA_CLOCK_H
#define FRAME_MEDIA_CLOCK_H
#include <libavformat/avformat.h>

typedef struct {
    int64_t last_us, next_us;
    int enabled, observed;
} FrameStreamClock;
typedef struct {
    FrameStreamClock *streams;
    unsigned count;
    int64_t offset_us, last_us;
    int observed;
} FramePacketClock;

/* For discontinuous formats the first selected track can begin after the
 * demuxer-wide origin. Input seeking still uses the demuxer's own timeline. */
int64_t frame_clock_origin(const AVFormatContext *input, const int *indices, int count,
                           int64_t seek_us);
int frame_clock_init(FramePacketClock *clock, const AVFormatContext *input, const int *indices,
                     int count);
void frame_clock_packet(FramePacketClock *clock, const AVFormatContext *input, AVPacket *packet);
void frame_clock_free(FramePacketClock *clock);
#endif
