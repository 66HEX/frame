//! CSS zoom-in/out-95 without reflowing the contents or changing typography.
use gpui::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, Window, point,
};

#[derive(Clone, Copy)]
pub(in crate::app) enum SurfaceOrigin {
    Top,
    Center,
}

pub(in crate::app) struct SurfaceMotion {
    content: AnyElement,
    progress: f32,
    origin: SurfaceOrigin,
}

pub(in crate::app) fn frame_surface_motion(
    content: impl IntoElement,
    progress: f32,
    origin: SurfaceOrigin,
) -> SurfaceMotion {
    SurfaceMotion {
        content: content.into_any_element(),
        progress,
        origin,
    }
}

pub(in crate::app) const fn surface_scale(progress: f32) -> f32 {
    0.05_f32.mul_add(progress.clamp(0.0, 1.0), 0.95)
}

impl IntoElement for SurfaceMotion {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SurfaceMotion {
    type RequestLayoutState = ();
    type PrepaintState = Point<Pixels>;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.content.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Point<Pixels> {
        let origin = match self.origin {
            SurfaceOrigin::Top => point(bounds.center().x, bounds.top()),
            SurfaceOrigin::Center => bounds.center(),
        };
        let factor = surface_scale(self.progress);
        window.with_prepaint_scale(origin, factor, |window| self.content.prepaint(window, cx));
        origin
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        origin: &mut Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_paint_scale(*origin, surface_scale(self.progress), |window| {
            self.content.paint(window, cx);
        });
    }
}
