use crate::app::preview::state::PREVIEW_CANVAS_WHEEL_PIXEL_DELTA_PER_STEP;
use crate::app::root::FrameRoot;
use crate::preview::Point as PreviewPoint;
use gpui::{
    App, Bounds, Element, ElementId, Entity, GlobalElementId, IntoElement, LayoutId, Pixels,
    Position, ScrollDelta, Style, Window, relative, size,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewCanvasPanDrag;

pub(super) struct PreviewCanvasBoundsProbe {
    pub(super) owner: Entity<FrameRoot>,
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

pub(in crate::app) fn preview_scroll_delta_y(delta: &ScrollDelta) -> f64 {
    match delta {
        ScrollDelta::Pixels(point) => {
            -f64::from(point.y.as_f32()) / PREVIEW_CANVAS_WHEEL_PIXEL_DELTA_PER_STEP
        }
        ScrollDelta::Lines(point) => -f64::from(point.y),
    }
}
