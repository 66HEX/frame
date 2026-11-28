#include "convert.h"
#include <math.h>

/* The output-rate setting acts after filtering. An fps filter chooses source
 * pictures at sample points, whereas the export clock covers the duration of
 * each picture and fills a gap with the preceding picture. Keep only these
 * two references, even when a low-rate input needs many repeated frames. */
static int plan_picture(ConvertTrack *t, AVRational tb) {
    FrameCadence *c = &t->cadence;
    AVRational encoder_tb = t->encoder->time_base;
    AVFrame *f = c->current;
    const double units = av_q2d(tb) / av_q2d(encoder_tb);
    double tick = f->pts == AV_NOPTS_VALUE ? (double)c->next_tick : (double)f->pts * units;
    if (!isfinite(tick) || tick <= (double)INT64_MIN || tick >= (double)INT64_MAX)
        return AVERROR_INVALIDDATA;
    double duration = f->duration * units;
    double gap = tick - c->next_tick;
    double span = gap + duration;
    c->pending = 1;
    c->repeat_previous = 0;
    if (c->fixed) {
        if (c->preserve_initial_offset && !t->frames && gap >= 0.5) {
            c->next_tick = llrint(tick);
            gap = 0;
            span = duration;
        }
        if (gap < 0 && span > 0)
            gap = 0;
        if (span < -1.1)
            c->pending = 0;
        if (span > 1.1) {
            /* The half-tick boundary belongs to the later picture for
             * nonintegral positive timestamps. */
            double bias = tick == nearbyint(tick) ? 0 : copysign(1.0 / 131072.0, tick);
            c->pending = llrint(span + bias);
            if (gap > 1.1)
                c->repeat_previous = llrint(gap - 0.6 + bias);
        }
        f->duration = 1;
    } else {
        c->next_tick = llrint(tick);
        f->duration = av_rescale_q(f->duration, tb, encoder_tb);
    }
    c->history[2] = c->history[1];
    c->history[1] = c->history[0];
    c->history[0] = c->repeat_previous;
    return 0;
}

int frame_cadence_next(ConvertTrack *t) {
    FrameCadence *c = &t->cadence;
    AVRational tb;
    while (!c->pending) {
        if (c->eof)
            return AVERROR_EOF;
        if (c->current->buf[0]) {
            av_frame_unref(c->previous);
            av_frame_move_ref(c->previous, c->current);
        }
        int err = frame_render_next(t->render, c->current, &tb);
        if (err == AVERROR_EOF) {
            c->eof = 1;
            /* A stable pattern of held pictures at the tail supplies the
             * final hold when no next timestamp exists. */
            int64_t small = FFMIN(c->history[0], c->history[1]);
            int64_t large = FFMAX(c->history[0], c->history[1]);
            c->pending = c->repeat_previous = av_clip64(c->history[2], small, large);
        } else if (err < 0 || (err = plan_picture(t, tb)) < 0)
            return err;
    }
    AVFrame *source = c->repeat_previous && c->previous->buf[0] ? c->previous : c->current;
    int err = av_frame_ref(t->frame, source);
    if (err < 0)
        return err;
    t->frame->pts = c->next_tick++;
    if (c->fixed)
        t->frame->duration = 1;
    if (c->repeat_previous)
        c->repeat_previous--;
    c->pending--;
    return 0;
}
