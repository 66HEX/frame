use super::*;
use crate::app::preview_actions::preview_canvas_layout_metrics;
use crate::numeric::{f64_to_f32, u32_to_f32, usize_to_f32};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PreviewCanvasPanDrag;

struct PreviewCanvasBoundsProbe {
    owner: Entity<FrameRoot>,
}

struct PreviewViewportRoundedClip;

struct PreviewMediaImage {
    render_image: Arc<RenderImage>,
    presentation: PreviewRenderPresentation,
}

impl IntoElement for PreviewCanvasBoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PreviewCanvasBoundsProbe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            position: Position::Absolute,
            size: size(relative(1.0).into(), relative(1.0).into()),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            ..Style::default()
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.owner.update(cx, |root, cx| {
            if root.set_preview_canvas_bounds(bounds, cx) {
                cx.notify();
            }
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

impl IntoElement for PreviewViewportRoundedClip {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl IntoElement for PreviewMediaImage {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PreviewMediaImage {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: size(relative(1.0).into(), relative(1.0).into()),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            ..Style::default()
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let Some(frame) = preview_presented_frame(&self.render_image, self.presentation) else {
            return;
        };
        let image_size = self.render_image.size(0);
        if image_size.width.0 == 0 || image_size.height.0 == 0 {
            return;
        }

        let visible_width = u32_to_f32(frame.visible_width);
        let visible_height = u32_to_f32(frame.visible_height);
        if visible_width <= 0.0 || visible_height <= 0.0 {
            return;
        }

        let scale_x = bounds.size.width.as_f32() / visible_width;
        let scale_y = bounds.size.height.as_f32() / visible_height;
        let full_bounds = Bounds {
            origin: point(
                bounds.origin.x - px(u32_to_f32(frame.visible_x) * scale_x),
                bounds.origin.y - px(u32_to_f32(frame.visible_y) * scale_y),
            ),
            size: size(
                px(u32_to_f32(frame.full_width) * scale_x),
                px(u32_to_f32(frame.full_height) * scale_y),
            ),
        };
        let center = full_bounds.center();
        let source_size = if self.presentation.transform.has_side_rotation() {
            size(full_bounds.size.height, full_bounds.size.width)
        } else {
            full_bounds.size
        };
        let source_bounds = Bounds {
            origin: point(
                center.x - source_size.width / 2.0,
                center.y - source_size.height / 2.0,
            ),
            size: source_size,
        };
        let transformation = preview_image_transformation(
            self.presentation.transform,
            center,
            window.scale_factor(),
        );

        let _ = window.paint_image_transformed(
            source_bounds,
            gpui::Corners::default(),
            Arc::clone(&self.render_image),
            0,
            false,
            transformation,
        );
    }
}

impl Element for PreviewViewportRoundedClip {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            position: Position::Absolute,
            size: size(relative(1.0).into(), relative(1.0).into()),
            ..Style::default()
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        // GPUI clips overflow to a rectangular content mask, so the media layer needs
        // exact rounded-corner cutouts painted above it.
        if let Some(path) = preview_viewport_rounded_clip_path(bounds, px(theme::RADIUS_MD)) {
            window.paint_path(path, parse_hex("#1B1D21"));
        }
    }
}

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
    window: &mut Window,
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
        .bg(parse_hex("#14161A"))
        .shadow(input_highlight_shadows())
        .child(preview_viewport_content(state, cx));

    if state.render_image.is_some() && state.media.is_some() {
        viewport = viewport.child(PreviewViewportRoundedClip);
    }

    if state.crop.crop_mode && state.crop.draft_crop.is_some() {
        viewport = viewport.child(preview_crop_aspect_bar(state, window, cx));
    }

    if let Some(overlay_controls) = preview_overlay_controls(state, window, cx) {
        viewport = viewport.child(overlay_controls);
    }

    if preview_visual_controls_visible(state) {
        viewport = viewport
            .child(preview_toolbar(state, window, cx))
            .child(preview_zoom_toolbar(state, window, cx));
    }

    viewport
}

#[expect(
    clippy::too_many_lines,
    reason = "Preview viewport content composes the canvas, fallback, crop, and overlay layers in one render path."
)]
pub(in crate::app) fn preview_viewport_content(
    state: &PreviewShellState,
    cx: &Context<FrameRoot>,
) -> gpui::AnyElement {
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

        return content
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
            .child(PreviewCanvasBoundsProbe { owner: cx.entity() })
            .child(preview_media_stage(state, render_image.clone(), media, cx))
            .into_any_element();
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
                .text_color(color(theme::FRAME_RED))
                .child(theme::ui_text("Preview unavailable"));
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
                    .child(theme::ui_text("Preview unavailable"))
                    .child(
                        div()
                            .max_w(px(320.0))
                            .truncate()
                            .text_color(color(theme::FRAME_GRAY_600))
                            .child(message.to_string()),
                    )
                    .into_any_element();
            }

            if state.availability.media_kind == PreviewMediaKind::Unknown {
                return content
                    .child(theme::ui_text("Preview unavailable"))
                    .into_any_element();
            }

            content.child(preview_media_placeholder(state.availability.media_kind))
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
        .child(preview_media_image(render_image, state.presentation));

    if let Some(metrics) = preview_canvas_layout_metrics(
        canvas.viewport_width,
        canvas.viewport_height,
        f64::from(media.width),
        f64::from(media.height),
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

    if let Some(overlay) = preview_overlay_layer(state) {
        media_stage = media_stage.child(overlay);
    }

    if state.crop.crop_mode && state.crop.draft_crop.is_some() {
        media_stage = media_stage.child(preview_crop_overlay(state));
    }

    media_stage
}

pub(in crate::app) fn preview_canvas_pan_enabled(state: &PreviewShellState) -> bool {
    preview_visual_controls_enabled(state) && !state.crop.crop_mode && !state.overlay.overlay_mode
}

fn preview_media_image(
    render_image: Arc<RenderImage>,
    presentation: PreviewRenderPresentation,
) -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .child(PreviewMediaImage {
            render_image,
            presentation,
        })
}

fn preview_image_transformation(
    transform: PreviewTransform,
    center: Point<Pixels>,
    scale_factor: f32,
) -> TransformationMatrix {
    let scale_x = if transform.flip_horizontal { -1.0 } else { 1.0 };
    let scale_y = if transform.flip_vertical { -1.0 } else { 1.0 };

    TransformationMatrix::unit()
        .translate(center.scale(scale_factor))
        .rotate(radians(f32::from(transform.rotation_degrees).to_radians()))
        .scale(size(scale_x, scale_y))
        .translate(center.scale(-scale_factor))
}

fn preview_viewport_rounded_clip_path(
    bounds: Bounds<Pixels>,
    radius: Pixels,
) -> Option<gpui::Path<Pixels>> {
    let x0 = bounds.origin.x.as_f32();
    let y0 = bounds.origin.y.as_f32();
    let x1 = x0 + bounds.size.width.as_f32();
    let y1 = y0 + bounds.size.height.as_f32();
    let radius = radius
        .as_f32()
        .min((x1 - x0).max(0.0) / 2.0)
        .min((y1 - y0).max(0.0) / 2.0);

    if radius <= 0.0 {
        return None;
    }

    let mut builder = gpui::PathBuilder::fill();
    preview_viewport_corner_cutout(
        &mut builder,
        (x0, y0),
        (x0 + radius, y0 + radius),
        -std::f32::consts::FRAC_PI_2,
        -std::f32::consts::PI,
        radius,
    );
    preview_viewport_corner_cutout(
        &mut builder,
        (x1, y0),
        (x1 - radius, y0 + radius),
        -std::f32::consts::FRAC_PI_2,
        0.0,
        radius,
    );
    preview_viewport_corner_cutout(
        &mut builder,
        (x1, y1),
        (x1 - radius, y1 - radius),
        0.0,
        std::f32::consts::FRAC_PI_2,
        radius,
    );
    preview_viewport_corner_cutout(
        &mut builder,
        (x0, y1),
        (x0 + radius, y1 - radius),
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        radius,
    );

    builder.build().ok()
}

fn preview_viewport_corner_cutout(
    builder: &mut gpui::PathBuilder,
    outer: (f32, f32),
    center: (f32, f32),
    start_angle: f32,
    end_angle: f32,
    radius: f32,
) {
    const SEGMENTS: usize = 12;

    builder.move_to(point(px(outer.0), px(outer.1)));
    for index in 0..=SEGMENTS {
        let progress = usize_to_f32(index) / usize_to_f32(SEGMENTS);
        let angle = (end_angle - start_angle).mul_add(progress, start_angle);
        builder.line_to(point(
            px(angle.cos().mul_add(radius, center.0)),
            px(angle.sin().mul_add(radius, center.1)),
        ));
    }
    builder.close();
}

fn preview_scroll_delta_y(delta: &ScrollDelta) -> f64 {
    match delta {
        ScrollDelta::Pixels(point) => f64::from(point.y.as_f32()),
        ScrollDelta::Lines(point) => f64::from(point.y),
    }
}

pub(in crate::app) fn preview_media_placeholder(media_kind: PreviewMediaKind) -> gpui::Div {
    div().flex().items_center().justify_center().child(icon_svg(
        preview_media_icon(media_kind),
        32.0,
        color(theme::FRAME_GRAY_600),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_image_transformation_converts_degrees_to_radians() {
        let transformation = preview_image_transformation(
            PreviewTransform {
                rotation_degrees: 90,
                flip_horizontal: false,
                flip_vertical: false,
            },
            point(px(50.0), px(25.0)),
            1.0,
        );

        assert!((transformation.rotation_scale[0][0]).abs() < 0.000_001);
        assert!((transformation.rotation_scale[0][1] + 1.0).abs() < 0.000_001);
        assert!((transformation.rotation_scale[1][0] - 1.0).abs() < 0.000_001);
        assert!((transformation.rotation_scale[1][1]).abs() < 0.000_001);
    }
}
