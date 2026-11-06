use crate::app::primitives::color;
use crate::theme;
use gpui::prelude::*;
use gpui::{
    Bounds, Context, DragMoveEvent, IntoElement, Render, ScrollHandle, UniformListScrollHandle,
    Window, canvas, div, point, px, size,
};

mod clip;

pub(in crate::app) const FRAME_SCROLLBAR_WIDTH: f32 = 10.0;
const FRAME_SCROLLBAR_VISUAL_WIDTH: f32 = 10.0;
const FRAME_SCROLLBAR_THUMB_WIDTH: f32 = 7.0;
const FRAME_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 18.0;

#[derive(Clone, Debug)]
pub(in crate::app) struct FrameScrollbarDrag {
    scroll_handle: ScrollHandle,
    content_height: f32,
}

struct FrameScrollbarDragPreview;

impl Render for FrameScrollbarDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct FrameScrollbarMetrics {
    pub(in crate::app) thumb_top: f32,
    pub(in crate::app) thumb_height: f32,
}

#[derive(Clone, Copy)]
struct FrameScrollbarPaintState {
    metrics: Option<FrameScrollbarMetrics>,
    ui_scale: f32,
    viewport: Bounds<gpui::Pixels>,
}

pub(in crate::app) fn frame_vertical_scrollbar(
    id: impl Into<String>,
    scroll_handle: ScrollHandle,
    content_height: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    frame_vertical_scrollbar_clipped(id, scroll_handle, content_height, 0.0, palette)
}

pub(super) fn frame_vertical_scrollbar_clipped(
    id: impl Into<String>,
    scroll_handle: ScrollHandle,
    content_height: f32,
    viewport_radius: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let debug_id = id.clone();
    let content_height = content_height.max(0.0);
    let drag = FrameScrollbarDrag {
        scroll_handle: scroll_handle.clone(),
        content_height,
    };
    let paint_handle = scroll_handle;

    div()
        .id(id)
        .debug_selector(move || debug_id)
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(theme::ui_rem(FRAME_SCROLLBAR_WIDTH))
        .cursor_default()
        .hover_style(gpui::Styled::cursor_default)
        .on_drag(drag, |_drag, _offset, window, cx| {
            window.refresh();
            cx.new(|_| FrameScrollbarDragPreview)
        })
        .on_drag_move(
            move |event: &DragMoveEvent<FrameScrollbarDrag>, window, cx| {
                let drag = event.drag(cx);
                let y = (event.event.position.y - event.bounds.origin.y).as_f32();
                let viewport_height = event.bounds.size.height.as_f32();
                let ui_scale = window.rem_size().as_f32() / crate::appearance::BASE_REM_PX;
                let content_height = frame_scrollbar_content_height(
                    &drag.scroll_handle,
                    drag.content_height * ui_scale,
                    viewport_height,
                );
                set_frame_vertical_scrollbar_offset(
                    &drag.scroll_handle,
                    content_height,
                    viewport_height,
                    y,
                );
                window.refresh();
            },
        )
        .child(
            canvas(
                move |bounds, window, _cx| {
                    let viewport_height = bounds.size.height.as_f32();
                    let ui_scale = window.rem_size().as_f32() / crate::appearance::BASE_REM_PX;
                    let content_height = frame_scrollbar_content_height(
                        &paint_handle,
                        content_height * ui_scale,
                        viewport_height,
                    );
                    FrameScrollbarPaintState {
                        metrics: frame_vertical_scrollbar_metrics_with_min_height(
                            viewport_height,
                            content_height,
                            paint_handle.offset().y.as_f32(),
                            FRAME_SCROLLBAR_MIN_THUMB_HEIGHT * ui_scale,
                        ),
                        ui_scale,
                        viewport: paint_handle.bounds(),
                    }
                },
                move |bounds, state, window, _cx| {
                    let Some(metrics) = state.metrics else {
                        return;
                    };
                    let thumb_width = FRAME_SCROLLBAR_THUMB_WIDTH * state.ui_scale;
                    let visual_width = FRAME_SCROLLBAR_VISUAL_WIDTH * state.ui_scale;

                    let thumb_bounds = Bounds::new(
                        point(
                            bounds.right() - px(visual_width.midpoint(thumb_width)),
                            bounds.origin.y + px(metrics.thumb_top),
                        ),
                        size(px(thumb_width), px(metrics.thumb_height)),
                    );
                    let radius = px(viewport_radius * state.ui_scale);
                    if radius > px(0.0)
                        && (thumb_bounds.top() < state.viewport.top() + radius
                            || thumb_bounds.bottom() > state.viewport.bottom() - radius)
                    {
                        // GPUI's overflow mask is rectangular. Intersect the thumb
                        // with the rounded viewport before painting, leaving the
                        // underlying content visible outside the popover's corners.
                        if let Some(path) = clip::thumb_path(thumb_bounds, state.viewport, radius) {
                            window.paint_path(path, color(palette.border_subtle));
                        }
                    } else {
                        paint_scrollbar_thumb(thumb_bounds, thumb_width, palette, window);
                    }
                },
            )
            .size_full(),
        )
}

fn paint_scrollbar_thumb(
    bounds: Bounds<gpui::Pixels>,
    width: f32,
    palette: &theme::ThemePalette,
    window: &mut Window,
) {
    let corners = gpui::Corners::all(px(width / 2.0));
    window.paint_quad(
        gpui::PaintQuad::new(bounds)
            .background(color(palette.border_subtle))
            .corner_radii(corners),
    );
}

fn frame_scrollbar_content_height(
    scroll_handle: &ScrollHandle,
    fallback_content_height: f32,
    viewport_height: f32,
) -> f32 {
    frame_scrollbar_measured_content_height(
        fallback_content_height,
        viewport_height,
        scroll_handle.max_offset().y.as_f32(),
    )
}

fn frame_scrollbar_measured_content_height(
    fallback_content_height: f32,
    viewport_height: f32,
    measured_scroll_max: f32,
) -> f32 {
    if measured_scroll_max > 0.0 {
        viewport_height + measured_scroll_max
    } else {
        fallback_content_height
    }
}

pub(in crate::app) fn frame_vertical_uniform_scrollbar(
    id: impl Into<String>,
    scroll_handle: &UniformListScrollHandle,
    content_height: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let base_handle = scroll_handle.0.borrow().base_handle.clone();
    frame_vertical_scrollbar(id, base_handle, content_height, palette)
}

pub(in crate::app) fn frame_vertical_scrollbar_metrics(
    viewport_height: f32,
    content_height: f32,
    offset_y: f32,
) -> Option<FrameScrollbarMetrics> {
    frame_vertical_scrollbar_metrics_with_min_height(
        viewport_height,
        content_height,
        offset_y,
        FRAME_SCROLLBAR_MIN_THUMB_HEIGHT,
    )
}

fn frame_vertical_scrollbar_metrics_with_min_height(
    viewport_height: f32,
    content_height: f32,
    offset_y: f32,
    minimum_thumb_height: f32,
) -> Option<FrameScrollbarMetrics> {
    if viewport_height <= 0.0 || content_height <= viewport_height {
        return None;
    }

    let max_offset_y = content_height - viewport_height;

    let thumb_height =
        ((viewport_height / content_height) * viewport_height).max(minimum_thumb_height);
    let max_thumb_top = (viewport_height - thumb_height).max(0.0);
    let progress = (-offset_y / max_offset_y).clamp(0.0, 1.0);

    Some(FrameScrollbarMetrics {
        thumb_top: max_thumb_top * progress,
        thumb_height: thumb_height.min(viewport_height),
    })
}

fn set_frame_vertical_scrollbar_offset(
    scroll_handle: &ScrollHandle,
    content_height: f32,
    viewport_height: f32,
    pointer_y: f32,
) {
    let Some(metrics) = frame_vertical_scrollbar_metrics(
        viewport_height,
        content_height,
        scroll_handle.offset().y.as_f32(),
    ) else {
        return;
    };

    let max_offset_y = (content_height - viewport_height).max(0.0);
    let max_thumb_top = (viewport_height - metrics.thumb_height).max(0.0);
    if max_thumb_top <= 0.0 || max_offset_y <= 0.0 {
        return;
    }

    let thumb_center = metrics.thumb_height / 2.0;
    let progress = ((pointer_y - thumb_center) / max_thumb_top).clamp(0.0, 1.0);
    let current_offset = scroll_handle.offset();
    scroll_handle.set_offset(point(current_offset.x, px(-(progress * max_offset_y))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_scrollbar_content_height_prefers_measured_scroll_extent() {
        let content_height = frame_scrollbar_measured_content_height(420.0, 360.0, 640.0);

        assert!((content_height - 1000.0).abs() <= f32::EPSILON);
    }

    #[test]
    fn frame_scrollbar_content_height_uses_fallback_before_measurement() {
        let content_height = frame_scrollbar_measured_content_height(420.0, 360.0, 0.0);

        assert!((content_height - 420.0).abs() <= f32::EPSILON);
    }
}
