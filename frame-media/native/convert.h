#ifndef FRAME_CONVERT_H
#define FRAME_CONVERT_H
#include "media_clock.h"
#include "render.h"
#include "shared_input.h"
#include <errno.h>
#include <inttypes.h>
#include <libavcodec/avcodec.h>
#include <libavcodec/bsf.h>
#include <libavfilter/buffersink.h>
#include <libavformat/avformat.h>
#include <libavutil/avstring.h>
#include <libavutil/bprint.h>
#include <libavutil/opt.h>
#include <libavutil/parseutils.h>
#include <libavutil/pixdesc.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char *name, *value;
} Option;
typedef struct {
    const char *path;
    int64_t seek;
    int *mapped;
    int mapped_count;
    AVFormatContext *input;
    AVPacket *pending;
    FramePacketClock clock;
    int *first_track, active;
    unsigned stream_count;
    int64_t origin_us;
    FrameMediaFile *shared_file;
} Source;
typedef struct {
    Option *options;
    Source *sources;
    int options_len, sources_len;
    const char *output;
    int video_pass;
    const char *stats_path;
    char *vpx_stats;
    int64_t duration;
} ConvertPlan;

typedef struct {
    AVFrame *previous, *current;
    int64_t next_tick, repeat_previous, pending, history[3];
    int eof, fixed, preserve_initial_offset;
} FrameCadence;

typedef struct {
    AVFormatContext *input;
    AVStream *src, *dst;
    Source *source;
    int next_track;
    char kind;
    int ordinal, input_num;
    int64_t origin, duration, frame_limit, frames;
    AVPacket *next, *raw, *filtered_input;
    AVRational packet_tb;
    FrameRenderer *render;
    AVCodecContext *encoder, *subdecoder;
    AVFrame *frame, *audio_input;
    AVRational audio_input_tb;
    int consumed_samples, audio_eof;
    void *analysis_io;
    int64_t analysis_end;
    FrameCadence cadence;
    AVBSFContext *bsf;
    int source_done, done, seen_key, strict;
} ConvertTrack;

int frame_convert_parse(ConvertPlan *plan, int argc, const char *const *argv);
const char *frame_plan_opt(const ConvertPlan *plan, const char *name);
const char *frame_track_opt(const ConvertPlan *plan, const ConvertTrack *track, const char *name);
int frame_option_matches(const char *option, const char *name, const ConvertTrack *track);
const AVCodec *frame_find_encoder(const char *name);
int frame_prepare_encoder(const ConvertPlan *plan, ConvertTrack *track, AVFormatContext *out,
                          const char *codec);
int frame_cadence_next(ConvertTrack *track);
int frame_read_encoded(ConvertTrack *track, AVPacket *packet);
int frame_encode_subtitle(ConvertTrack *track, AVPacket *output);
int frame_mux_packets(const ConvertPlan *plan, ConvertTrack *tracks, int count,
                      AVFormatContext *out, void *io);
extern int frame_output_open(void *io, const char *path, int overwrite, int progress);
extern int frame_output_write(void *io, const uint8_t *buffer, int size);
extern int64_t frame_output_seek(void *io, int64_t offset, int whence);
extern const char *frame_pass_path(void *io);
extern void frame_output_pass(void *io, int pass, int64_t duration);
extern void frame_output_quiet(void *io, int quiet);
extern void frame_output_stats(void *io, int64_t frames, int64_t time_us);
#endif
