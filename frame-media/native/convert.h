#ifndef FRAME_CONVERT_H
#define FRAME_CONVERT_H
#include "render.h"
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
} Source;
typedef struct {
    Option *options;
    Source *sources;
    int options_len, sources_len;
    const char *output;
    int64_t duration;
} ConvertPlan;

typedef struct {
    AVFormatContext *input;
    AVStream *src, *dst;
    const Source *source;
    char kind;
    int ordinal, input_num;
    int64_t origin, duration, frame_limit, frames;
    AVPacket *next, *raw, *filtered_input;
    AVRational packet_tb;
    FrameRenderer *render;
    AVCodecContext *encoder, *subdecoder;
    AVFrame *frame;
    AVBSFContext *bsf;
    int source_done, codec_done, done, seen_key;
} ConvertTrack;

int frame_convert_parse(ConvertPlan *plan, int argc, const char *const *argv);
const char *frame_plan_opt(const ConvertPlan *plan, const char *name);
const char *frame_track_opt(const ConvertPlan *plan, const ConvertTrack *track, const char *name);
int frame_option_matches(const char *option, const char *name, const ConvertTrack *track);
int frame_prepare_encoder(const ConvertPlan *plan, ConvertTrack *track, AVFormatContext *out,
                          const char *codec);
int frame_read_encoded(ConvertTrack *track, AVPacket *packet);
int frame_read_selected(ConvertTrack *track, AVPacket *packet);
extern int frame_output_open(void *io, const char *path, int overwrite, int progress);
extern int frame_output_write(void *io, const uint8_t *buffer, int size);
extern int64_t frame_output_seek(void *io, int64_t offset, int whence);
extern void frame_output_stats(void *io, int64_t frames, int64_t time_us);
#endif
