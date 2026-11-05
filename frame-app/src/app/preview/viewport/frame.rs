use crate::numeric::usize_to_f32;
use crate::theme;
use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, IntoElement, LayoutId, Pixels, Position,
    Rgba, Style, Window, point, px, relative, size,
};

/// Paint-only foreground for rounded media cutouts.
/// No hitbox, so pan, zoom and crop interactions pass through to the content.
pub(super) struct PreviewViewportFrame {
    pub(super) color: Rgba,
}

impl IntoElement for PreviewViewportFrame {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PreviewViewportFrame {
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
        // GPUI clips overflow to a rectangular content mask. Cover media outside
        // the rounded corners; the viewport paints its focus ring above this mask.
        let radius = theme::ui_rem(theme::RADIUS_MD).to_pixels(window.rem_size());
        if let Some(path) = preview_viewport_rounded_clip_path(bounds, radius) {
            window.paint_path(path, self.color);
        }
    }
}

pub(super) fn preview_viewport_rounded_clip_path(
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

pub(super) fn preview_viewport_corner_cutout(
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
