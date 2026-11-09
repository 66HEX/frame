use crate::app::preview::panel::{PreviewCanvasRenderState, preview_media_render_state};
use crate::app::preview::state::{
    PREVIEW_CANVAS_INITIAL_CONTAIN_SCALE, PREVIEW_CANVAS_LERP_FACTOR, PREVIEW_CANVAS_MAX_PAN,
    PREVIEW_CANVAS_MAX_ZOOM, PREVIEW_CANVAS_MIN_ZOOM, PREVIEW_CANVAS_PAN_SNAP_EPSILON,
    PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON, PREVIEW_CANVAS_WHEEL_DEADZONE,
    PREVIEW_CANVAS_WHEEL_MAX_STEPS_PER_EVENT, PREVIEW_CANVAS_WHEEL_ZOOM_STEP,
    PREVIEW_CANVAS_ZOOM_SNAP_EPSILON, PREVIEW_CANVAS_ZOOM_STEP,
    PREVIEW_DIMENSION_DEBOUNCE_INTERVAL, PREVIEW_FRAME_TICK_INTERVAL, PreviewCanvasPanDragState,
    PreviewCanvasState, PreviewCanvasZoomDirection,
};
use crate::app::root::FrameRoot;
use crate::preview::Point as PreviewPoint;
use gpui::{Bounds, Context, Pixels, Point};
use std::time::{Duration, Instant};

impl FrameRoot {
    pub(in crate::app) fn sync_preview_canvas_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
    ) {
        if self.preview_ui.canvas_file_id.as_deref() == selected_file_id {
            return;
        }

        self.preview_ui.canvas_file_id = selected_file_id.map(str::to_string);
        self.preview_ui.canvas = PreviewCanvasState::default();
        self.preview_ui.canvas_pan_drag = None;
    }

    pub(in crate::app) fn sync_preview_canvas_auto_fit(&mut self) -> bool {
        self.apply_preview_canvas_auto_fit()
    }

    pub(in crate::app) fn preview_canvas_render_state(&self) -> PreviewCanvasRenderState {
        let (viewport_width, viewport_height) =
            self.preview_ui.canvas_bounds.map_or((0.0, 0.0), |bounds| {
                (
                    f64::from(bounds.size.width.as_f32()),
                    f64::from(bounds.size.height.as_f32()),
                )
            });
        PreviewCanvasRenderState {
            zoom: self.preview_ui.canvas.current_zoom,
            pan_x: self.preview_ui.canvas.current_pan_x,
            pan_y: self.preview_ui.canvas.current_pan_y,
            viewport_width,
            viewport_height,
        }
    }

    pub(in crate::app) fn zoom_preview_canvas(
        &mut self,
        direction: PreviewCanvasZoomDirection,
        cx: &Context<Self>,
    ) -> bool {
        let multiplier = match direction {
            PreviewCanvasZoomDirection::In => PREVIEW_CANVAS_ZOOM_STEP,
            PreviewCanvasZoomDirection::Out => 1.0 / PREVIEW_CANVAS_ZOOM_STEP,
        };
        let current_zoom = self.preview_ui.canvas.target_zoom;
        let next_zoom = clamp_preview_canvas_zoom(current_zoom * multiplier);
        if (next_zoom - current_zoom).abs() <= f64::EPSILON {
            return false;
        }

        let zoom_ratio = if current_zoom > f64::EPSILON {
            next_zoom / current_zoom
        } else {
            1.0
        };
        let target_pan_x = self.preview_ui.canvas.target_pan_x * zoom_ratio;
        let target_pan_y = self.preview_ui.canvas.target_pan_y * zoom_ratio;
        let (target_pan_x, target_pan_y) =
            self.clamp_preview_canvas_pan_for_state(target_pan_x, target_pan_y, next_zoom);

        self.preview_ui.canvas.target_zoom = next_zoom;
        self.preview_ui.canvas.target_pan_x = target_pan_x;
        self.preview_ui.canvas.target_pan_y = target_pan_y;
        self.preview_ui.canvas.auto_fit_pending = false;
        self.schedule_preview_frame_tick(cx);
        true
    }

    pub(in crate::app) fn zoom_preview_canvas_at_position(
        &mut self,
        position: Point<Pixels>,
        multiplier: f64,
        cx: &Context<Self>,
    ) -> bool {
        if !multiplier.is_finite() || multiplier <= 0.0 {
            return false;
        }
        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return false;
        };
        let viewport_width = f64::from(bounds.size.width.as_f32());
        let viewport_height = f64::from(bounds.size.height.as_f32());
        if viewport_width <= 0.0 || viewport_height <= 0.0 {
            return false;
        }
        if position.x < bounds.origin.x
            || position.x > bounds.origin.x + bounds.size.width
            || position.y < bounds.origin.y
            || position.y > bounds.origin.y + bounds.size.height
        {
            return false;
        }

        let current_zoom = self.preview_ui.canvas.target_zoom;
        let next_zoom = clamp_preview_canvas_zoom(current_zoom * multiplier);
        if (next_zoom - current_zoom).abs() <= f64::EPSILON {
            return false;
        }

        let ratio = if current_zoom > f64::EPSILON {
            next_zoom / current_zoom
        } else {
            1.0
        };
        let pointer_x = f64::from((position.x - bounds.origin.x).as_f32()) - (viewport_width / 2.0);
        let pointer_y =
            f64::from((position.y - bounds.origin.y).as_f32()) - (viewport_height / 2.0);
        let target_pan_x =
            (pointer_x - self.preview_ui.canvas.target_pan_x).mul_add(-ratio, pointer_x);
        let target_pan_y =
            (pointer_y - self.preview_ui.canvas.target_pan_y).mul_add(-ratio, pointer_y);
        let (target_pan_x, target_pan_y) =
            self.clamp_preview_canvas_pan_for_state(target_pan_x, target_pan_y, next_zoom);

        self.preview_ui.canvas.auto_fit_pending = false;
        self.preview_ui.canvas.target_zoom = next_zoom;
        self.preview_ui.canvas.target_pan_x = target_pan_x;
        self.preview_ui.canvas.target_pan_y = target_pan_y;
        self.schedule_preview_frame_tick(cx);
        true
    }

    pub(in crate::app) fn zoom_preview_canvas_from_wheel(
        &mut self,
        position: Point<Pixels>,
        delta_y: f64,
        cx: &Context<Self>,
    ) -> bool {
        let Some(multiplier) = preview_canvas_wheel_zoom_multiplier(delta_y) else {
            return false;
        };
        self.zoom_preview_canvas_at_position(position, multiplier, cx)
    }

    pub(in crate::app) fn apply_preview_canvas_pan_drag(
        &mut self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        cx: &Context<Self>,
    ) -> bool {
        if bounds.size.width.as_f32() <= 0.0 || bounds.size.height.as_f32() <= 0.0 {
            return false;
        }

        let drag_state = if let Some(state) = self.preview_ui.canvas_pan_drag {
            state
        } else {
            let state = PreviewCanvasPanDragState {
                start_position: position,
                start_pan_x: self.preview_ui.canvas.current_pan_x,
                start_pan_y: self.preview_ui.canvas.current_pan_y,
            };
            self.preview_ui.canvas_pan_drag = Some(state);
            state
        };

        let delta_x = f64::from((position.x - drag_state.start_position.x).as_f32());
        let delta_y = f64::from((position.y - drag_state.start_position.y).as_f32());
        let (next_pan_x, next_pan_y) = self.clamp_preview_canvas_pan_for_state(
            drag_state.start_pan_x + delta_x,
            drag_state.start_pan_y + delta_y,
            self.preview_ui.canvas.target_zoom,
        );
        let changed = (next_pan_x - self.preview_ui.canvas.target_pan_x).abs() > f64::EPSILON
            || (next_pan_y - self.preview_ui.canvas.target_pan_y).abs() > f64::EPSILON;

        self.preview_ui.canvas.target_pan_x = next_pan_x;
        self.preview_ui.canvas.target_pan_y = next_pan_y;
        if changed {
            self.preview_ui.canvas.auto_fit_pending = false;
            self.schedule_preview_frame_tick(cx);
        }
        changed
    }

    pub(in crate::app) const fn end_preview_canvas_pan_drag(&mut self) -> bool {
        let had_drag = self.preview_ui.canvas_pan_drag.is_some();
        self.preview_ui.canvas_pan_drag = None;
        had_drag
    }

    pub(in crate::app) fn adjust_preview_canvas_pan_from_keyboard(
        &mut self,
        key: &str,
        cx: &Context<Self>,
    ) -> bool {
        let Some(delta) = preview_canvas_keyboard_pan_delta(key) else {
            return false;
        };
        let (next_pan_x, next_pan_y) = self.clamp_preview_canvas_pan_for_state(
            self.preview_ui.canvas.target_pan_x + delta.x,
            self.preview_ui.canvas.target_pan_y + delta.y,
            self.preview_ui.canvas.target_zoom,
        );
        let changed = (next_pan_x - self.preview_ui.canvas.target_pan_x).abs() > f64::EPSILON
            || (next_pan_y - self.preview_ui.canvas.target_pan_y).abs() > f64::EPSILON;
        self.preview_ui.canvas.target_pan_x = next_pan_x;
        self.preview_ui.canvas.target_pan_y = next_pan_y;
        if changed {
            self.preview_ui.canvas.auto_fit_pending = false;
            self.schedule_preview_frame_tick(cx);
        }
        true
    }

    pub(in crate::app) fn set_preview_canvas_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
        cx: &Context<Self>,
    ) -> bool {
        let previous_bounds = self.preview_ui.canvas_bounds;
        let bounds_changed = previous_bounds != Some(bounds);
        self.preview_ui.canvas_bounds = Some(bounds);
        if bounds_changed {
            self.preview_ui.preview_dimensions_debounce_until =
                Some(Instant::now() + PREVIEW_DIMENSION_DEBOUNCE_INTERVAL);
            self.schedule_preview_frame_tick(cx);
        }
        let auto_fit_changed = self.apply_preview_canvas_auto_fit();
        bounds_changed || auto_fit_changed
    }

    pub(super) fn clamp_preview_canvas_pan_for_state(
        &self,
        pan_x: f64,
        pan_y: f64,
        zoom: f64,
    ) -> (f64, f64) {
        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return (0.0, 0.0);
        };
        let Some((media_width, media_height)) = self.preview_canvas_media_dimensions() else {
            return (0.0, 0.0);
        };
        let Some((max_x, max_y)) = preview_canvas_pan_limits(
            f64::from(bounds.size.width.as_f32()),
            f64::from(bounds.size.height.as_f32()),
            media_width,
            media_height,
            zoom,
        ) else {
            return (0.0, 0.0);
        };

        (pan_x.clamp(-max_x, max_x), pan_y.clamp(-max_y, max_y))
    }

    pub(super) fn preview_canvas_media_dimensions(&self) -> Option<(f64, f64)> {
        let media = preview_media_render_state(
            self.preview_ui.render_image.as_ref(),
            self.preview_ui.rendered_presentation,
        )?;
        Some(media.display_dimensions())
    }

    pub(super) fn apply_preview_canvas_auto_fit(&mut self) -> bool {
        if !self.preview_ui.canvas.auto_fit_pending {
            return false;
        }
        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return false;
        };
        let Some((media_width, media_height)) = self.preview_canvas_media_dimensions() else {
            return false;
        };
        let Some(zoom) = preview_canvas_initial_zoom(
            f64::from(bounds.size.width.as_f32()),
            f64::from(bounds.size.height.as_f32()),
            media_width,
            media_height,
        ) else {
            return false;
        };

        self.preview_ui.canvas.current_zoom = zoom;
        self.preview_ui.canvas.target_zoom = zoom;
        self.preview_ui.canvas.current_pan_x = 0.0;
        self.preview_ui.canvas.current_pan_y = 0.0;
        self.preview_ui.canvas.target_pan_x = 0.0;
        self.preview_ui.canvas.target_pan_y = 0.0;
        self.preview_ui.canvas.auto_fit_pending = false;
        true
    }

    pub(in crate::app) fn tick_preview_canvas_animation(&mut self) -> bool {
        if self.preview_canvas_transform_visually_settled() {
            return self.settle_preview_canvas_animation();
        }

        let now = Instant::now();
        let elapsed = self
            .preview_ui
            .canvas
            .last_animation_tick
            .map_or(PREVIEW_FRAME_TICK_INTERVAL, |last_tick| {
                now.saturating_duration_since(last_tick)
            });
        self.preview_ui.canvas.last_animation_tick = Some(now);

        let next_zoom = lerp_preview_canvas_value_for_elapsed(
            self.preview_ui.canvas.current_zoom,
            self.preview_ui.canvas.target_zoom,
            elapsed,
        );
        let next_pan_x = lerp_preview_canvas_value_for_elapsed(
            self.preview_ui.canvas.current_pan_x,
            self.preview_ui.canvas.target_pan_x,
            elapsed,
        );
        let next_pan_y = lerp_preview_canvas_value_for_elapsed(
            self.preview_ui.canvas.current_pan_y,
            self.preview_ui.canvas.target_pan_y,
            elapsed,
        );

        self.preview_ui.canvas.current_zoom = next_zoom;
        self.preview_ui.canvas.current_pan_x = next_pan_x;
        self.preview_ui.canvas.current_pan_y = next_pan_y;

        true
    }

    pub(super) fn preview_canvas_transform_visually_settled(&self) -> bool {
        let fallback = || {
            preview_canvas_transform_settled(
                self.preview_ui.canvas.current_zoom,
                self.preview_ui.canvas.target_zoom,
                self.preview_ui.canvas.current_pan_x,
                self.preview_ui.canvas.target_pan_x,
                self.preview_ui.canvas.current_pan_y,
                self.preview_ui.canvas.target_pan_y,
            )
        };

        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return fallback();
        };
        let Some((media_width, media_height)) = self.preview_canvas_media_dimensions() else {
            return fallback();
        };

        preview_canvas_transform_visual_delta(
            f64::from(bounds.size.width.as_f32()),
            f64::from(bounds.size.height.as_f32()),
            media_width,
            media_height,
            self.preview_ui.canvas.current_zoom,
            self.preview_ui.canvas.target_zoom,
            self.preview_ui.canvas.current_pan_x,
            self.preview_ui.canvas.target_pan_x,
            self.preview_ui.canvas.current_pan_y,
            self.preview_ui.canvas.target_pan_y,
        )
        .is_some_and(|delta| delta <= PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON)
    }

    pub(in crate::app) fn settle_preview_canvas_animation(&mut self) -> bool {
        let changed = (self.preview_ui.canvas.current_zoom - self.preview_ui.canvas.target_zoom)
            .abs()
            > f64::EPSILON
            || (self.preview_ui.canvas.current_pan_x - self.preview_ui.canvas.target_pan_x).abs()
                > f64::EPSILON
            || (self.preview_ui.canvas.current_pan_y - self.preview_ui.canvas.target_pan_y).abs()
                > f64::EPSILON;

        self.preview_ui.canvas.current_zoom = self.preview_ui.canvas.target_zoom;
        self.preview_ui.canvas.current_pan_x = self.preview_ui.canvas.target_pan_x;
        self.preview_ui.canvas.current_pan_y = self.preview_ui.canvas.target_pan_y;
        self.preview_ui.canvas.last_animation_tick = None;

        changed
    }
}

pub(in crate::app) const fn clamp_preview_canvas_zoom(value: f64) -> f64 {
    value.clamp(PREVIEW_CANVAS_MIN_ZOOM, PREVIEW_CANVAS_MAX_ZOOM)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewCanvasLayoutMetrics {
    pub(in crate::app) width: f64,
    pub(in crate::app) height: f64,
    pub(in crate::app) left: f64,
    pub(in crate::app) top: f64,
}

pub(in crate::app) fn preview_canvas_layout_metrics(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
) -> Option<PreviewCanvasLayoutMetrics> {
    if viewport_width <= 0.0 || viewport_height <= 0.0 || media_width <= 0.0 || media_height <= 0.0
    {
        return None;
    }

    let fit_scale = (viewport_width / media_width).min(viewport_height / media_height);
    if !fit_scale.is_finite() || fit_scale <= 0.0 {
        return None;
    }

    let width = media_width * fit_scale * zoom;
    let height = media_height * fit_scale * zoom;
    Some(PreviewCanvasLayoutMetrics {
        width,
        height,
        left: (viewport_width / 2.0) + pan_x - (width / 2.0),
        top: (viewport_height / 2.0) + pan_y - (height / 2.0),
    })
}

pub(in crate::app) fn preview_canvas_initial_zoom(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
) -> Option<f64> {
    if viewport_width <= 0.0 || viewport_height <= 0.0 || media_width <= 0.0 || media_height <= 0.0
    {
        return None;
    }

    let width_scale = viewport_width / media_width;
    let height_scale = viewport_height / media_height;
    let contain_scale = width_scale.min(height_scale);
    if !contain_scale.is_finite() || contain_scale <= 0.0 {
        return None;
    }

    Some(clamp_preview_canvas_zoom(
        PREVIEW_CANVAS_INITIAL_CONTAIN_SCALE,
    ))
}

pub(in crate::app) fn preview_canvas_pan_limits(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
    zoom: f64,
) -> Option<(f64, f64)> {
    let metrics = preview_canvas_layout_metrics(
        viewport_width,
        viewport_height,
        media_width,
        media_height,
        zoom,
        0.0,
        0.0,
    )?;
    Some((
        (viewport_width * PREVIEW_CANVAS_MAX_PAN).max(metrics.width) / 2.0,
        (viewport_height * PREVIEW_CANVAS_MAX_PAN).max(metrics.height) / 2.0,
    ))
}

pub(in crate::app) fn preview_canvas_keyboard_pan_delta(key: &str) -> Option<PreviewPoint> {
    const PAN_STEP: f64 = 24.0;
    match key {
        "left" => Some(PreviewPoint {
            x: -PAN_STEP,
            y: 0.0,
        }),
        "right" => Some(PreviewPoint {
            x: PAN_STEP,
            y: 0.0,
        }),
        "up" => Some(PreviewPoint {
            x: 0.0,
            y: -PAN_STEP,
        }),
        "down" => Some(PreviewPoint {
            x: 0.0,
            y: PAN_STEP,
        }),
        _ => None,
    }
}

#[cfg(test)]
pub(in crate::app) fn lerp_preview_canvas_value(current: f64, target: f64) -> f64 {
    lerp_preview_canvas_value_for_elapsed(current, target, PREVIEW_FRAME_TICK_INTERVAL)
}

pub(in crate::app) fn lerp_preview_canvas_value_for_elapsed(
    current: f64,
    target: f64,
    elapsed: Duration,
) -> f64 {
    let tick_seconds = PREVIEW_FRAME_TICK_INTERVAL.as_secs_f64();
    let elapsed_ticks = if tick_seconds > 0.0 {
        elapsed.as_secs_f64() / tick_seconds
    } else {
        1.0
    };
    let factor = if elapsed_ticks.is_finite() && elapsed_ticks > 0.0 {
        1.0 - (1.0 - PREVIEW_CANVAS_LERP_FACTOR).powf(elapsed_ticks)
    } else {
        PREVIEW_CANVAS_LERP_FACTOR
    };
    let factor = factor.clamp(PREVIEW_CANVAS_LERP_FACTOR, 1.0);
    (target - current).mul_add(factor, current)
}

pub(in crate::app) fn preview_canvas_wheel_zoom_multiplier(delta_y: f64) -> Option<f64> {
    if !delta_y.is_finite() {
        return None;
    }

    let magnitude = delta_y.abs();
    if magnitude <= PREVIEW_CANVAS_WHEEL_DEADZONE {
        return None;
    }

    let steps =
        (magnitude - PREVIEW_CANVAS_WHEEL_DEADZONE).min(PREVIEW_CANVAS_WHEEL_MAX_STEPS_PER_EVENT);
    let exponent = if delta_y < 0.0 { steps } else { -steps };
    let multiplier = PREVIEW_CANVAS_WHEEL_ZOOM_STEP.powf(exponent);
    (multiplier.is_finite() && multiplier > 0.0).then_some(multiplier)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Canvas transform metrics are scalar layout values kept explicit for focused tests."
)]
pub(in crate::app) fn preview_canvas_transform_visual_delta(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
    current_zoom: f64,
    target_zoom: f64,
    current_pan_x: f64,
    target_pan_x: f64,
    current_pan_y: f64,
    target_pan_y: f64,
) -> Option<f64> {
    let current = preview_canvas_layout_metrics(
        viewport_width,
        viewport_height,
        media_width,
        media_height,
        current_zoom,
        current_pan_x,
        current_pan_y,
    )?;
    let target = preview_canvas_layout_metrics(
        viewport_width,
        viewport_height,
        media_width,
        media_height,
        target_zoom,
        target_pan_x,
        target_pan_y,
    )?;

    Some(
        (target.left - current.left)
            .abs()
            .max((target.top - current.top).abs())
            .max((target.width - current.width).abs())
            .max((target.height - current.height).abs()),
    )
}

pub(in crate::app) fn preview_canvas_transform_settled(
    current_zoom: f64,
    target_zoom: f64,
    current_pan_x: f64,
    target_pan_x: f64,
    current_pan_y: f64,
    target_pan_y: f64,
) -> bool {
    (target_zoom - current_zoom).abs() <= PREVIEW_CANVAS_ZOOM_SNAP_EPSILON
        && (target_pan_x - current_pan_x).abs() <= PREVIEW_CANVAS_PAN_SNAP_EPSILON
        && (target_pan_y - current_pan_y).abs() <= PREVIEW_CANVAS_PAN_SNAP_EPSILON
}
