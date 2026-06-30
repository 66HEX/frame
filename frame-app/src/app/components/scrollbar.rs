use super::*;

const FRAME_SCROLLBAR_WIDTH: f32 = 10.0;
const FRAME_SCROLLBAR_TRACK_WIDTH: f32 = 6.0;
const FRAME_SCROLLBAR_THUMB_WIDTH: f32 = 6.0;
const FRAME_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 28.0;

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

pub(in crate::app) fn frame_vertical_scrollbar(
    id: impl Into<String>,
    scroll_handle: ScrollHandle,
    content_height: f32,
) -> gpui::Stateful<gpui::Div> {
    let content_height = content_height.max(0.0);
    let drag = FrameScrollbarDrag {
        scroll_handle: scroll_handle.clone(),
        content_height,
    };
    let paint_handle = scroll_handle;

    div()
        .id(id.into())
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(px(FRAME_SCROLLBAR_WIDTH))
        .cursor_default()
        .hover(gpui::Styled::cursor_pointer)
        .on_drag(drag, |_drag, _offset, window, cx| {
            window.refresh();
            cx.new(|_| FrameScrollbarDragPreview)
        })
        .on_drag_move(
            move |event: &DragMoveEvent<FrameScrollbarDrag>, window, cx| {
                let drag = event.drag(cx);
                let y = (event.event.position.y - event.bounds.origin.y).as_f32();
                let viewport_height = event.bounds.size.height.as_f32();
                set_frame_vertical_scrollbar_offset(
                    &drag.scroll_handle,
                    drag.content_height,
                    viewport_height,
                    y,
                );
                window.refresh();
            },
        )
        .child(
            canvas(
                move |bounds, _window, _cx| {
                    frame_vertical_scrollbar_metrics(
                        bounds.size.height.as_f32(),
                        content_height,
                        paint_handle.offset().y.as_f32(),
                    )
                },
                |bounds, metrics, window, _cx| {
                    let Some(metrics) = metrics else {
                        return;
                    };

                    let track_bounds = Bounds::new(
                        point(
                            bounds.origin.x
                                + px((FRAME_SCROLLBAR_WIDTH - FRAME_SCROLLBAR_TRACK_WIDTH) / 2.0),
                            bounds.origin.y,
                        ),
                        size(px(FRAME_SCROLLBAR_TRACK_WIDTH), bounds.size.height),
                    );
                    window.paint_quad(
                        fill(track_bounds, color(theme::FRAME_GRAY_100))
                            .corner_radii(px(FRAME_SCROLLBAR_TRACK_WIDTH / 2.0)),
                    );

                    let thumb_bounds = Bounds::new(
                        point(
                            bounds.origin.x
                                + px((FRAME_SCROLLBAR_WIDTH - FRAME_SCROLLBAR_THUMB_WIDTH) / 2.0),
                            bounds.origin.y + px(metrics.thumb_top),
                        ),
                        size(px(FRAME_SCROLLBAR_THUMB_WIDTH), px(metrics.thumb_height)),
                    );
                    window.paint_quad(
                        fill(thumb_bounds, color(theme::FRAME_GRAY_600))
                            .corner_radii(px(FRAME_SCROLLBAR_THUMB_WIDTH / 2.0)),
                    );
                },
            )
            .size_full(),
        )
}

pub(in crate::app) fn frame_vertical_uniform_scrollbar(
    id: impl Into<String>,
    scroll_handle: &UniformListScrollHandle,
    content_height: f32,
) -> gpui::Stateful<gpui::Div> {
    let base_handle = scroll_handle.0.borrow().base_handle.clone();
    frame_vertical_scrollbar(id, base_handle, content_height)
}

pub(in crate::app) fn frame_vertical_scrollbar_metrics(
    viewport_height: f32,
    content_height: f32,
    offset_y: f32,
) -> Option<FrameScrollbarMetrics> {
    if viewport_height <= 0.0 || content_height <= viewport_height {
        return None;
    }

    let max_offset_y = content_height - viewport_height;

    let thumb_height = ((viewport_height / content_height) * viewport_height)
        .max(FRAME_SCROLLBAR_MIN_THUMB_HEIGHT);
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
