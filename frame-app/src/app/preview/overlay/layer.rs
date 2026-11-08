use crate::app::accessibility::apply_accessible_button;
use crate::app::components::frame_handle_hit_region;
use crate::app::preview::panel::{PreviewMediaRenderState, PreviewShellState};
use crate::app::preview::state::PreviewOverlayImageDimensions;
use crate::app::preview::{
    PreviewTimelineDragPreview, normalized_point_from_bounds, preview_resize_handle,
    preview_selection_line_color,
};
use crate::app::root::FrameRoot;
use crate::numeric::unit_f64_to_f32;
use crate::preview::{
    MAX_OVERLAY_WIDTH, MIN_OVERLAY_WIDTH, OverlayDragHandle, OverlayDragPoint,
    OverlaySizeDirection, PreviewOverlay,
};
use crate::theme;
use gpui::prelude::*;
use gpui::{Bounds, Context, ObjectFit, Pixels, div, img, relative};
use std::path::PathBuf;

pub(super) const OVERLAY_HANDLE_SIZE: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewOverlayDrag {
    pub(in crate::app) handle: OverlayDragHandle,
    pub(in crate::app) width: f64,
    pub(in crate::app) height: f64,
}

#[expect(
    clippy::too_many_lines,
    reason = "Overlay layer composes image, handles, drag behavior, and keyboard/a11y actions in one GPUI builder."
)]
pub(in crate::app) fn preview_overlay_layer(
    state: &PreviewShellState,
    cx: &Context<FrameRoot>,
) -> Option<gpui::Stateful<gpui::Div>> {
    let palette = state.palette;
    let overlay = state.overlay.overlay.as_ref()?;
    if !overlay.enabled || overlay.path.is_empty() {
        return None;
    }

    let (width, height) = preview_overlay_render_size(state, overlay);
    let left = (overlay.x - width / 2.0).clamp(0.0, 1.0);
    let top = (overlay.y - height / 2.0).clamp(0.0, 1.0);
    let drag = PreviewOverlayDrag {
        handle: OverlayDragHandle::Move,
        width,
        height,
    };

    let mut layer = div()
        .id("preview-overlay-layer")
        .debug_selector(|| "preview-overlay-layer".into())
        .absolute()
        .left(relative(unit_f64_to_f32(left)))
        .top(relative(unit_f64_to_f32(top)))
        .w(relative(unit_f64_to_f32(width)))
        .h(relative(unit_f64_to_f32(height)))
        .when(state.overlay.overlay_mode, |this| {
            this.cursor_grab()
                .on_drag(drag, |_drag, _position, _window, cx| {
                    cx.new(|_| PreviewTimelineDragPreview)
                })
        })
        .child(
            div()
                .absolute()
                .inset_0()
                .overflow_hidden()
                .opacity(unit_f64_to_f32(overlay.opacity))
                .child(
                    img(PathBuf::from(overlay.path.clone()))
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                ),
        )
        .when(state.overlay.overlay_mode, |this| {
            // Keep the selection outline below the resize handles. GPUI paints a
            // parent's border after its children, so the outline must be a child.
            this.child(
                div()
                    .absolute()
                    .inset_0()
                    .border_1()
                    .border_color(preview_selection_line_color()),
            )
        });

    if state.overlay.overlay_mode {
        let media = state.media;
        layer = apply_accessible_button(layer, "Move overlay image", true, palette)
            .aria_description(overlay_keyboard_description())
            .on_key_down(
                cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                    let key = event.keystroke.key.as_str();
                    let changed =
                        match key {
                            "enter" | "escape" => {
                                let changed = root.set_selected_overlay_mode(false);
                                root.focus_registered_control("preview-tool-overlay", window, cx);
                                changed
                            }
                            "delete" | "backspace" => {
                                let changed = root.remove_selected_overlay();
                                root.focus_registered_control("preview-tool-overlay", window, cx);
                                changed
                            }
                            "+" | "=" | "plus" => root
                                .nudge_selected_overlay_size(OverlaySizeDirection::Increase, media),
                            "-" | "minus" => root
                                .nudge_selected_overlay_size(OverlaySizeDirection::Decrease, media),
                            _ => root.adjust_preview_overlay_from_keyboard_with_step(
                                OverlayDragHandle::Move,
                                key,
                                media,
                                event.keystroke.modifiers.shift,
                            ),
                        };
                    if changed {
                        cx.notify();
                    }
                    if overlay_keyboard_key_is_handled(key) {
                        cx.stop_propagation();
                    }
                }),
            );
        layer = layer
            .child(preview_overlay_handle(
                OverlayDragHandle::NorthWest,
                0.0,
                0.0,
                width,
                height,
                state.media,
                palette,
                cx,
            ))
            .child(preview_overlay_handle(
                OverlayDragHandle::NorthEast,
                1.0,
                0.0,
                width,
                height,
                state.media,
                palette,
                cx,
            ))
            .child(preview_overlay_handle(
                OverlayDragHandle::SouthEast,
                1.0,
                1.0,
                width,
                height,
                state.media,
                palette,
                cx,
            ))
            .child(preview_overlay_handle(
                OverlayDragHandle::SouthWest,
                0.0,
                1.0,
                width,
                height,
                state.media,
                palette,
                cx,
            ));
    }

    Some(layer)
}

pub(in crate::app) fn overlay_drag_point_from_bounds(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
    drag: PreviewOverlayDrag,
) -> OverlayDragPoint {
    let point = normalized_point_from_bounds(position, bounds);
    OverlayDragPoint {
        x: point.x,
        y: point.y,
        width: Some(drag.width),
        height: Some(drag.height),
    }
}

pub(super) fn preview_overlay_render_size(
    state: &PreviewShellState,
    overlay: &PreviewOverlay,
) -> (f64, f64) {
    let width = overlay.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
    let height = width * preview_overlay_height_ratio(state);
    (width, height.clamp(MIN_OVERLAY_WIDTH, 1.0))
}

pub(super) fn preview_overlay_height_ratio(state: &PreviewShellState) -> f64 {
    let overlay_ratio = state
        .overlay
        .image_dimensions
        .map_or(1.0, PreviewOverlayImageDimensions::height_over_width);
    let media_ratio = state.media.map_or(1.0, |media| {
        if media.height == 0 {
            1.0
        } else {
            f64::from(media.width) / f64::from(media.height)
        }
    });
    overlay_ratio * media_ratio
}

#[expect(
    clippy::too_many_arguments,
    reason = "Overlay handles require explicit geometry, media state, palette, and root context."
)]
pub(super) fn preview_overlay_handle(
    handle: OverlayDragHandle,
    x: f32,
    y: f32,
    width: f64,
    height: f64,
    media: Option<PreviewMediaRenderState>,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let handle_element = overlay_handle_cursor(
        div()
            .id(format!(
                "preview-overlay-handle-{}",
                overlay_handle_id(handle)
            ))
            .debug_selector(move || format!("preview-overlay-handle-{}", overlay_handle_id(handle)))
            .flex()
            .items_center()
            .justify_center()
            .child(preview_resize_handle(OVERLAY_HANDLE_SIZE, palette)),
        handle,
    )
    .on_drag(
        PreviewOverlayDrag {
            handle,
            width,
            height,
        },
        |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
    );

    let handle_element =
        apply_accessible_button(handle_element, overlay_handle_label(handle), true, palette)
            .aria_description(overlay_keyboard_description())
            .on_key_down(
                cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                    let key = event.keystroke.key.as_str();
                    let changed =
                        match key {
                            "enter" | "escape" => {
                                let changed = root.set_selected_overlay_mode(false);
                                root.focus_registered_control("preview-tool-overlay", window, cx);
                                changed
                            }
                            "delete" | "backspace" => {
                                let changed = root.remove_selected_overlay();
                                root.focus_registered_control("preview-tool-overlay", window, cx);
                                changed
                            }
                            "+" | "=" | "plus" => root
                                .nudge_selected_overlay_size(OverlaySizeDirection::Increase, media),
                            "-" | "minus" => root
                                .nudge_selected_overlay_size(OverlaySizeDirection::Decrease, media),
                            _ => root.adjust_preview_overlay_from_keyboard_with_step(
                                handle,
                                key,
                                media,
                                event.keystroke.modifiers.shift,
                            ),
                        };
                    if changed {
                        cx.notify();
                    }
                    if overlay_keyboard_key_is_handled(key) {
                        cx.stop_propagation();
                    }
                }),
            );
    frame_handle_hit_region(x, y, 1.0, 1.0, handle_element)
}

pub(super) fn overlay_handle_cursor(
    handle: gpui::Stateful<gpui::Div>,
    drag_handle: OverlayDragHandle,
) -> gpui::Stateful<gpui::Div> {
    match drag_handle {
        OverlayDragHandle::NorthWest | OverlayDragHandle::SouthEast => handle.cursor_nwse_resize(),
        OverlayDragHandle::NorthEast | OverlayDragHandle::SouthWest => handle.cursor_nesw_resize(),
        OverlayDragHandle::Move => handle.cursor_grab(),
    }
}

pub(super) const fn overlay_handle_id(handle: OverlayDragHandle) -> &'static str {
    match handle {
        OverlayDragHandle::Move => "move",
        OverlayDragHandle::NorthWest => "nw",
        OverlayDragHandle::NorthEast => "ne",
        OverlayDragHandle::SouthEast => "se",
        OverlayDragHandle::SouthWest => "sw",
    }
}

pub(super) const fn overlay_handle_label(handle: OverlayDragHandle) -> &'static str {
    match handle {
        OverlayDragHandle::Move => "Move overlay image",
        OverlayDragHandle::NorthWest => "Resize overlay top left corner",
        OverlayDragHandle::NorthEast => "Resize overlay top right corner",
        OverlayDragHandle::SouthEast => "Resize overlay bottom right corner",
        OverlayDragHandle::SouthWest => "Resize overlay bottom left corner",
    }
}

pub(super) fn overlay_keyboard_key_is_handled(key: &str) -> bool {
    matches!(
        key,
        "left"
            | "right"
            | "up"
            | "down"
            | "enter"
            | "escape"
            | "delete"
            | "backspace"
            | "+"
            | "="
            | "plus"
            | "-"
            | "minus"
    )
}

pub(super) const fn overlay_keyboard_description() -> &'static str {
    "Use arrow keys to move or resize the overlay. Hold Shift for a larger step. Press plus or minus to resize, Enter or Escape to finish, or Delete to remove."
}
