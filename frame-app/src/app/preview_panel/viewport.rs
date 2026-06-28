use super::*;

pub(in crate::app) fn normalized_point_from_bounds(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
) -> PreviewPoint {
    let width = bounds.size.width.as_f32();
    let height = bounds.size.height.as_f32();
    if width <= 0.0 || height <= 0.0 {
        return PreviewPoint { x: 0.0, y: 0.0 };
    }

    let x = ((position.x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0);
    let y = ((position.y - bounds.origin.y).as_f32() / height).clamp(0.0, 1.0);
    PreviewPoint {
        x: f64::from(x),
        y: f64::from(y),
    }
}

pub(in crate::app) fn preview_viewport(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let mut viewport = div()
        .id("preview-viewport")
        .relative()
        .flex_1()
        .min_h_0()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex("#000000"))
        .shadow(input_highlight_shadows())
        .child(preview_viewport_content(state, cx));

    if state.crop.crop_mode && state.crop.draft_crop.is_some() {
        viewport = viewport.child(preview_crop_aspect_bar(state, cx));
    }

    if let Some(overlay_controls) = preview_overlay_controls(state, cx) {
        viewport = viewport.child(overlay_controls);
    }

    if preview_visual_controls_visible(state) {
        viewport = viewport
            .child(preview_toolbar(state, cx))
            .child(preview_zoom_toolbar(state));
    }

    viewport
}

pub(in crate::app) fn preview_viewport_content(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    if let (Some(render_image), Some(media)) = (&state.render_image, state.media) {
        return div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(preview_media_stage(state, render_image.clone(), media, cx));
    }

    let content = div()
        .max_w(px(360.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_center()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600));

    let Some(file_name) = state.selected_file_name.as_deref() else {
        return content.child("Drop files or use Add Source");
    };

    match state.metadata_status {
        PreviewMetadataStatus::Idle | PreviewMetadataStatus::Loading => {
            content.child("Analyzing source...")
        }
        PreviewMetadataStatus::Error => {
            let mut error = content
                .text_color(color(theme::FRAME_RED))
                .child("Preview unavailable");
            if let Some(message) = state.metadata_error.as_deref() {
                error = error.child(
                    div()
                        .max_w(px(320.0))
                        .truncate()
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(message.to_string()),
                );
            }
            error
        }
        PreviewMetadataStatus::Ready => {
            if let Some(message) = state.runtime_error.as_deref() {
                return content
                    .text_color(color(theme::FRAME_RED))
                    .child("Preview unavailable")
                    .child(
                        div()
                            .max_w(px(320.0))
                            .truncate()
                            .text_color(color(theme::FRAME_GRAY_600))
                            .child(message.to_string()),
                    );
            }

            if state.availability.media_kind == PreviewMediaKind::Unknown {
                return content.child("Preview unavailable");
            }

            content
                .child(preview_media_placeholder(state.availability.media_kind))
                .child(
                    div()
                        .max_w(px(320.0))
                        .truncate()
                        .whitespace_nowrap()
                        .text_color(color(theme::FOREGROUND))
                        .child(file_name.to_string()),
                )
                .child(preview_media_kind_label(state.availability.media_kind))
        }
    }
}

pub(in crate::app) fn preview_media_stage(
    state: &PreviewShellState,
    render_image: Arc<RenderImage>,
    media: PreviewMediaRenderState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let mut stage = div()
        .id("preview-media-stage")
        .relative()
        .h_full()
        .max_w(relative(1.0))
        .max_h(relative(1.0))
        .aspect_ratio(media.aspect_ratio())
        .overflow_hidden()
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewCropDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let point = normalized_point_from_bounds(event.event.position, event.bounds);
                if root.apply_preview_crop_drag(drag.handle, point) {
                    cx.notify();
                }
            },
        ))
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewOverlayDrag>, _window, cx| {
                let drag = *event.drag(cx);
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
        .child(img(render_image).size_full().object_fit(ObjectFit::Fill));

    if let Some(overlay) = preview_overlay_layer(state) {
        stage = stage.child(overlay);
    }

    if state.crop.crop_mode && state.crop.draft_crop.is_some() {
        stage = stage.child(preview_crop_overlay(state));
    }

    stage
}

pub(in crate::app) fn preview_media_placeholder(media_kind: PreviewMediaKind) -> gpui::Div {
    div()
        .w(px(240.0))
        .h(px(136.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_MD))
        .border_1()
        .border_color(color(theme::FRAME_GRAY_200))
        .bg(color(theme::BACKGROUND))
        .shadow(input_highlight_shadows())
        .child(icon_svg(
            preview_media_icon(media_kind),
            32.0,
            color(theme::FRAME_GRAY_600),
        ))
}

pub(in crate::app) fn preview_media_icon(media_kind: PreviewMediaKind) -> &'static str {
    match media_kind {
        PreviewMediaKind::Video | PreviewMediaKind::Unknown => assets::ICON_FILE_VIDEO,
        PreviewMediaKind::Audio => assets::ICON_MUSIC,
        PreviewMediaKind::Image => assets::ICON_FILE_IMAGE,
    }
}

pub(in crate::app) fn preview_media_kind_label(media_kind: PreviewMediaKind) -> &'static str {
    match media_kind {
        PreviewMediaKind::Video => "VIDEO SOURCE",
        PreviewMediaKind::Audio => "AUDIO SOURCE",
        PreviewMediaKind::Image => "IMAGE SOURCE",
        PreviewMediaKind::Unknown => "UNKNOWN SOURCE",
    }
}

pub(in crate::app) fn preview_visual_controls_visible(state: &PreviewShellState) -> bool {
    state.availability.media_kind != PreviewMediaKind::Unknown
        && !state.availability.hide_visual_controls
}

pub(in crate::app) fn preview_visual_controls_enabled(state: &PreviewShellState) -> bool {
    preview_visual_controls_visible(state) && !state.controls_disabled
}
