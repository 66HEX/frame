use crate::app::preview::panel::preview_presented_frame;
use crate::numeric::u32_to_f32;
use crate::preview_engine::{PreviewRenderPresentation, PreviewTransform};
use gpui::prelude::*;
use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, IntoElement, LayoutId, Pixels, Point,
    RenderImage, Style, TransformationMatrix, Window, div, point, px, radians, relative, size,
};
use std::sync::Arc;

pub(super) struct PreviewMediaImage {
    pub(super) render_image: Arc<RenderImage>,
    pub(super) presentation: PreviewRenderPresentation,
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
            source_bounds,
            gpui::Corners::default(),
            0.0,
            Arc::clone(&self.render_image),
            0,
            false,
            transformation,
        );
    }
}

pub(super) fn preview_media_image(
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

pub(super) fn preview_image_transformation(
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
