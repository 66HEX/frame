#ifndef FRAME_RENDER_H
#define FRAME_RENDER_H
#include <libavfilter/avfilter.h>
#include <libavutil/frame.h>
#include <libavutil/avutil.h>

/* A single filtered stream. The graph may also read an overlay. All strings
 * are borrowed during open. Frame timestamps are relative to the requested
 * start, in the rational returned by frame_render_next. */
typedef struct FrameRenderPlan {
    const char *path;
    const char *overlay;
    const char *graph;         /* complex graph or simple filter chain */
    const char *tail;          /* final output format/encoder constraints */
    int audio;
    int stream;                /* absolute index, or -1 for first of type */
    int64_t start_us;
    int64_t duration_us;       /* -1 for unbounded */
    int fast_seek;
    int threads;               /* 0 for automatic */
    const char *hardware_device;
} FrameRenderPlan;

typedef struct FrameRenderer FrameRenderer;
int frame_render_open(FrameRenderer **result, const FrameRenderPlan *plan);
int frame_render_next(FrameRenderer *renderer, AVFrame *frame, AVRational *time_base);
AVFilterContext *frame_render_sink(FrameRenderer *renderer);
int frame_render_samples(FrameRenderer *renderer, AVFrame *frame, AVRational *time_base, int samples);
void frame_render_close(FrameRenderer **renderer);
#endif
