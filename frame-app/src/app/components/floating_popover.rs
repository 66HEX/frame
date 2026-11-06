//! Measured, viewport-aware positioning shared by Select and Popover content.
use super::surface_motion::surface_scale;
use crate::app::motion::popover_slide_offset;
use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Element, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Point, Position, Size, Style, Window, point, px, size,
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Default)]
pub(in crate::app) struct PopoverAnchor(Rc<Cell<Bounds<Pixels>>>);

impl PopoverAnchor {
    pub(in crate::app) fn measure(&self, bounds: Bounds<Pixels>) {
        self.0.set(bounds);
    }
}

pub(in crate::app) struct FloatingPopover {
    content: AnyElement,
    anchor: PopoverAnchor,
    progress: f32,
    scale: f32,
    collision_padding: f32,
}

pub(in crate::app) fn frame_floating_popover(
    content: impl IntoElement,
    anchor: PopoverAnchor,
    progress: f32,
    collision_padding: f32,
    window: &Window,
) -> FloatingPopover {
    FloatingPopover {
        content: content.into_any_element(),
        anchor,
        progress,
        scale: window.rem_size().as_f32() / crate::appearance::BASE_REM_PX,
        collision_padding,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
}

fn available_space(
    anchor: Bounds<Pixels>,
    content_height: Pixels,
    viewport: Size<Pixels>,
    gap: Pixels,
    padding: Pixels,
) -> (Side, Pixels) {
    let above = (anchor.top() - gap - padding).max(px(0.0));
    let below = (viewport.height - anchor.bottom() - gap - padding).max(px(0.0));
    if content_height > below && above > below {
        (Side::Top, above)
    } else {
        (Side::Bottom, below)
    }
}

fn placement(
    anchor: Bounds<Pixels>,
    content: Size<Pixels>,
    viewport: Size<Pixels>,
    side: Side,
    gap: Pixels,
    padding: Pixels,
    motion_offset: Pixels,
) -> (Point<Pixels>, Point<Pixels>) {
    let x = anchor.left().clamp(
        padding,
        (viewport.width - padding - content.width).max(padding),
    );
    let y = match side {
        Side::Top => anchor.top() - gap - content.height - motion_offset,
        Side::Bottom => anchor.bottom() + gap + motion_offset,
    };
    let origin = point(x, y);
    let transform_origin = point(
        anchor.center().x.clamp(x, x + content.width),
        match side {
            Side::Top => y + content.height,
            Side::Bottom => y,
        },
    );
    (origin, transform_origin)
}

impl IntoElement for FloatingPopover {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for FloatingPopover {
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
        // Deferred prepaint runs after the trigger has reported its actual bounds.
        let layout = window.request_layout(
            Style {
                position: Position::Absolute,
                ..Style::default()
            },
            [],
            cx,
        );
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Point<Pixels> {
        let anchor = self.anchor.0.get();
        let viewport = window.viewport_size();
        let padding = px(self.collision_padding * self.scale);
        let gap = px(4.0 * self.scale);
        let width = anchor
            .size
            .width
            .min((viewport.width - padding * 2.0).max(px(0.0)));
        let mut space = size(
            AvailableSpace::Definite(width),
            AvailableSpace::Definite((viewport.height - padding * 2.0).max(px(0.0))),
        );
        let measured = self.content.layout_as_root(space, window, cx);
        let (side, height) = available_space(anchor, measured.height, viewport, gap, padding);
        space.height = AvailableSpace::Definite(height);
        let content = self.content.layout_as_root(space, window, cx);
        let (origin, transform_origin) = placement(
            anchor,
            content,
            viewport,
            side,
            gap,
            padding,
            px(popover_slide_offset(self.progress) * self.scale),
        );
        window.with_prepaint_scale(transform_origin, surface_scale(self.progress), |window| {
            self.content.prepaint_at(origin, window, cx);
        });
        transform_origin
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

#[cfg(test)]
#[path = "floating_popover_tests.rs"]
mod tests;
