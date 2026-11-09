use crate::app::accessibility::{focus_ring_above_content, focus_ring_base, focus_visible_ring};
use crate::app::preview::actions::{
    preview_canvas_keyboard_pan_delta, preview_canvas_layout_metrics,
};
use crate::app::preview::crop_overlay::{
    PreviewCropDrag, preview_crop_aspect_bar, preview_crop_overlay,
};
use crate::app::preview::panel::{
    PreviewCanvasRenderState, PreviewMediaRenderState, PreviewShellState,
    PreviewTimecodeInputFocuses, PreviewViewportFocuses,
};
use crate::app::preview::state::PreviewTransportState;
use crate::app::preview::toolbar::{preview_toolbar, preview_zoom_toolbar};
use crate::app::preview::viewport::bounds::{
    PreviewCanvasBoundsProbe, PreviewCanvasPanDrag, normalized_point_from_bounds,
    preview_scroll_delta_y,
};
use crate::app::preview::viewport::frame::PreviewViewportFrame;
use crate::app::preview::viewport::media::preview_media_image;
use crate::app::preview::{
    PreviewOverlayDrag, PreviewTimelineDragPreview, overlay_drag_point_from_bounds,
    preview_overlay_controls, preview_overlay_layer,
};
use crate::app::primitives::{color, icon_svg};
use crate::app::root::FrameRoot;
use crate::numeric::f64_to_f32;
use crate::preview::{MetadataStatus as PreviewMetadataStatus, PreviewMediaKind};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{
    Bounds, Context, DragMoveEvent, MouseUpEvent, PinchEvent, Pixels, RenderImage,
    ScrollWheelEvent, Window, div, px, relative,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

pub(in crate::app) fn preview_viewport(
    state: &PreviewShellState,
    focuses: PreviewViewportFocuses<'_>,
    timecode_inputs: PreviewTimecodeInputFocuses<'_>,
    transport: PreviewTransportState,
    card_bounds: Rc<Cell<Bounds<Pixels>>>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let palette = state.palette;
    let pan_enabled = preview_canvas_pan_enabled(state);
    let transport_enabled = !state.availability.trim_disabled
        && state.duration_seconds > 0.0
        && !state.crop.crop_mode
        && !state.overlay.overlay_mode;
    let mut viewport = div()
        .id("preview-viewport")
        .debug_selector(|| "preview-viewport".to_string())
        .relative()
        .flex_1()
        .min_h_0()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.canvas))
        // The corner cutouts below must cover media, never the focus ring.
        .map(|element| focus_ring_above_content(focus_ring_base(element, palette)))
        .transitions(|t| {
            t.outline(crate::app::motion::interaction_motion())
                .border_color(crate::app::motion::interaction_motion())
        })
        .track_focus(focuses.viewport)
        .tab_stop(pan_enabled || transport_enabled)
        .role(gpui::Role::Pane)
        .aria_label("Preview canvas")
        .aria_description(if transport_enabled {
            "Space plays or pauses. Tab reveals playback and trim controls. Arrow keys pan the zoomed preview."
        } else {
            "Use arrow keys to pan the zoomed preview."
        })
        .focus_visible_style(move |style| {
            focus_ring_above_content(focus_visible_ring(style, palette))
        })
        .on_hover(cx.listener(|root, hovered: &bool, _window, cx| {
            if root.preview_ui.transport.hovered != *hovered {
                root.preview_ui.transport.hovered = *hovered;
                cx.notify();
            }
        }))
        .map(|viewport| {
            preview_viewport_keyboard(viewport, focuses.viewport, pan_enabled, transport_enabled, cx)
        })
        .child(preview_viewport_content(state, cx))
        // Fixed to the viewport, above media/crop layers and below floating controls.
        .child(PreviewViewportFrame {
            fill: theme::shadows::surface_fill(palette.surface, palette),
            card_bounds,
        });

    if state.crop.crop_mode && state.crop.draft_crop.is_some() {
        viewport = viewport.child(preview_crop_aspect_bar(
            state,
            focuses.crop_options,
            focuses.edit_toolbars.crop,
            window,
            cx,
        ));
    }

    if let Some(overlay_controls) =
        preview_overlay_controls(state, focuses.edit_toolbars.overlay, window, cx)
    {
        viewport = viewport.child(overlay_controls);
    }

    if preview_visual_controls_visible(state) {
        let ui_scale = f64::from(window.rem_size().as_f32() / crate::appearance::BASE_REM_PX);
        let compact_trim = transport.open
            && (state.canvas.viewport_width
                < f64::from(crate::app::layout::PREVIEW_TRIM_PANEL_WIDTH + 120.0) * ui_scale
                || state.canvas.viewport_height < 360.0 * ui_scale);
        if !compact_trim {
            viewport = viewport.child(preview_toolbar(state, focuses.tools, window, cx));
        }
        viewport = viewport.child(preview_zoom_toolbar(state, window, cx));
    }

    if let Some(transport) = crate::app::preview::preview_transport(
        state,
        timecode_inputs,
        focuses.viewport,
        transport,
        window,
        cx,
    ) {
        viewport = viewport.child(transport);
    }

    viewport
}

fn preview_viewport_keyboard(
    viewport: gpui::Stateful<gpui::Div>,
    focus: &gpui::FocusHandle,
    pan_enabled: bool,
    transport_enabled: bool,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let focus = focus.clone();
    viewport
        .when(pan_enabled, gpui::Styled::cursor_grab)
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                if key == "escape" {
                    let tool = if root.preview_ui.crop_mode {
                        root.toggle_selected_crop_mode();
                        Some("preview-tool-crop")
                    } else if root.preview_ui.overlay.overlay_mode() {
                        root.set_selected_overlay_mode(false);
                        Some("preview-tool-overlay")
                    } else {
                        None
                    };
                    if let Some(tool) = tool {
                        root.focus_registered_control(tool, window, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                } else if focus.is_focused(window) {
                    if transport_enabled && key == "space" {
                        if root.toggle_preview_playback_with_context(cx) {
                            cx.notify();
                        }
                        cx.stop_propagation();
                    } else if pan_enabled && preview_canvas_keyboard_pan_delta(key).is_some() {
                        if root.adjust_preview_canvas_pan_from_keyboard(key, cx) {
                            cx.notify();
                        }
                        cx.stop_propagation();
                    }
                }
            }),
        )
}

#[expect(
    clippy::too_many_lines,
    reason = "Preview viewport content composes the canvas, fallback, crop, and overlay layers in one render path."
)]
pub(in crate::app) fn preview_viewport_content(
    state: &PreviewShellState,
    cx: &Context<FrameRoot>,
) -> gpui::AnyElement {
    let palette = state.palette;
    if let (Some(render_image), Some(media)) = (&state.render_image, state.media) {
        let content = div()
            .id("preview-canvas-pan-layer")
            .absolute()
            .inset_0()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center();
        let content = if preview_canvas_pan_enabled(state) {
            content
                .cursor_grab()
                .on_drag(PreviewCanvasPanDrag, |_drag, _position, _window, cx| {
                    cx.new(|_| PreviewTimelineDragPreview)
                })
        } else {
            content
        };

        let content = content
            .on_pinch(cx.listener(|root, event: &PinchEvent, _window, cx| {
                let multiplier = 1.0 + f64::from(event.delta);
                if root.zoom_preview_canvas_at_position(event.position, multiplier, cx) {
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_scroll_wheel(cx.listener(|root, event: &ScrollWheelEvent, _window, cx| {
                if root.zoom_preview_canvas_from_wheel(
                    event.position,
                    preview_scroll_delta_y(&event.delta),
                    cx,
                ) {
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_drag_move(cx.listener(
                |root, event: &DragMoveEvent<PreviewCanvasPanDrag>, _window, cx| {
                    if root.apply_preview_canvas_pan_drag(event.event.position, event.bounds, cx) {
                        cx.notify();
                    }
                },
            ))
            .capture_any_mouse_up(cx.listener(|root, _event: &MouseUpEvent, _window, cx| {
                if root.end_preview_canvas_pan_drag() {
                    cx.notify();
                }
            }))
            .child(PreviewCanvasBoundsProbe { owner: cx.entity() });

        if preview_canvas_layout_ready(state.canvas) {
            return content
                .child(preview_media_stage(state, render_image.clone(), media, cx))
                .into_any_element();
        }

        return content.into_any_element();
    }

    let content = div()
        .id("preview-empty-state")
        .max_w(theme::ui_rem(360.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_center()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(palette.text_muted));

    if state.selected_file_name.is_none() {
        return content
            .child(theme::ui_text("Drop files or use Add Source"))
            .into_any_element();
    }

    let content = match state.metadata_status {
        PreviewMetadataStatus::Idle | PreviewMetadataStatus::Loading => {
            content.child(theme::ui_text("Analyzing source..."))
        }
        PreviewMetadataStatus::Error => {
            let mut error = content
                .role(gpui::Role::Alert)
                .aria_label("Preview unavailable")
                .text_color(color(palette.danger))
                .child(theme::ui_text("Preview unavailable"));
            if let Some(message) = state.metadata_error.as_deref() {
                error = error.child(
                    div()
                        .max_w(theme::ui_rem(320.0))
                        .truncate()
                        .text_color(color(palette.text_muted))
                        .child(message.to_string()),
                );
            }
            error
        }
        PreviewMetadataStatus::Ready => {
            if let Some(message) = state.runtime_error.as_deref() {
                return content
                    .role(gpui::Role::Alert)
                    .aria_label("Preview unavailable")
                    .text_color(color(palette.danger))
                    .child(theme::ui_text("Preview unavailable"))
                    .child(
                        div()
                            .max_w(theme::ui_rem(320.0))
                            .truncate()
                            .text_color(color(palette.text_muted))
                            .child(message.to_string()),
                    )
                    .into_any_element();
            }

            if preview_waiting_for_visual_render(state) {
                return div().into_any_element();
            }

            if state.availability.media_kind == PreviewMediaKind::Unknown {
                return content
                    .child(theme::ui_text("Preview unavailable"))
                    .into_any_element();
            }

            content.child(preview_media_placeholder(
                state.availability.media_kind,
                palette,
            ))
        }
    };

    content.into_any_element()
}

pub(in crate::app) fn preview_media_stage(
    state: &PreviewShellState,
    render_image: Arc<RenderImage>,
    media: PreviewMediaRenderState,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let canvas = state.canvas;
    let mut media_stage = div()
        .id("preview-media-stage")
        .debug_selector(|| "preview-media-stage".into())
        .on_drag_move(
            cx.listener(|root, event: &DragMoveEvent<PreviewCropDrag>, window, cx| {
                let drag = *event.drag(cx);
                if !root.accept_preview_drag_epoch(drag.request_epoch) {
                    cx.stop_active_drag(window);
                    cx.notify();
                    return;
                }
                let point = normalized_point_from_bounds(event.event.position, event.bounds);
                if root.apply_preview_crop_drag(drag.handle, point) {
                    cx.notify();
                }
            }),
        )
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewOverlayDrag>, window, cx| {
                let drag = *event.drag(cx);
                if !root.accept_preview_drag_epoch(drag.request_epoch) {
                    cx.stop_active_drag(window);
                    cx.notify();
                    return;
                }
                let point =
                    overlay_drag_point_from_bounds(event.event.position, event.bounds, drag);
                if root.apply_preview_overlay_drag(drag.handle, point) {
                    cx.notify();
                }
            },
        ))
        .capture_any_mouse_up(cx.listener(|root, _, _window, cx| {
            let crop_changed = root.end_preview_crop_drag();
            let overlay_changed = root.end_preview_overlay_drag();
            if crop_changed || overlay_changed {
                cx.notify();
            }
        }))
        .child(preview_media_image(render_image, state.presentation));

    if let Some(metrics) = preview_canvas_layout_metrics(
        canvas.viewport_width,
        canvas.viewport_height,
        media.display_dimensions().0,
        media.display_dimensions().1,
        canvas.zoom,
        canvas.pan_x,
        canvas.pan_y,
    ) {
        media_stage = media_stage
            .absolute()
            .left(px(f64_to_f32(metrics.left)))
            .top(px(f64_to_f32(metrics.top)))
            .w(px(f64_to_f32(metrics.width)))
            .h(px(f64_to_f32(metrics.height)));
    } else {
        media_stage = media_stage
            .relative()
            .h_full()
            .max_w(relative(1.0))
            .max_h(relative(1.0))
            .aspect_ratio(media.aspect_ratio());
    }

    if let Some(overlay) = preview_overlay_layer(state, cx) {
        media_stage = media_stage.child(overlay);
    }

    if state.crop.frame_ready && state.crop.crop_mode && state.crop.draft_crop.is_some() {
        media_stage = media_stage.child(preview_crop_overlay(state, cx));
    }

    media_stage
}

pub(in crate::app) fn preview_canvas_pan_enabled(state: &PreviewShellState) -> bool {
    preview_visual_controls_enabled(state) && !state.crop.crop_mode && !state.overlay.overlay_mode
}

pub(super) fn preview_canvas_layout_ready(canvas: PreviewCanvasRenderState) -> bool {
    canvas.viewport_width > 0.0 && canvas.viewport_height > 0.0
}

pub(super) const fn preview_waiting_for_visual_render(state: &PreviewShellState) -> bool {
    state.render_image.is_none()
        && matches!(
            state.availability.media_kind,
            PreviewMediaKind::Video | PreviewMediaKind::Image
        )
}

pub(in crate::app) fn preview_media_placeholder(
    media_kind: PreviewMediaKind,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div().flex().items_center().justify_center().child(icon_svg(
        preview_media_icon(media_kind),
        32.0,
        color(palette.text_muted),
    ))
}

pub(in crate::app) const fn preview_media_icon(media_kind: PreviewMediaKind) -> &'static str {
    match media_kind {
        PreviewMediaKind::Video | PreviewMediaKind::Unknown => assets::ICON_FILE_VIDEO,
        PreviewMediaKind::Audio => assets::ICON_MUSIC,
        PreviewMediaKind::Image => assets::ICON_FILE_IMAGE,
    }
}

pub(in crate::app) fn preview_visual_controls_visible(state: &PreviewShellState) -> bool {
    state.availability.media_kind != PreviewMediaKind::Unknown
        && !state.availability.hide_visual_controls
}

pub(in crate::app) fn preview_visual_controls_enabled(state: &PreviewShellState) -> bool {
    preview_visual_controls_visible(state) && !state.controls_disabled
}
