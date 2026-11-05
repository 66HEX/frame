//! Tooltip positioning after layout, so collision handling uses measured text bounds.
use super::tooltip::TooltipPlacement;
use gpui::{
    AnyElement, App, Bounds, Display, Element, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Point, Position, Size, Style, Window, point, px,
};
use std::{cell::Cell, rc::Rc};

pub(super) struct PositionedTooltip {
    pub content: AnyElement,
    pub arrow: AnyElement,
    pub anchor: Rc<Cell<Bounds<Pixels>>>,
    pub content_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub on_pointer_move: Rc<PointerMoveListener>,
    pub placement: TooltipPlacement,
    pub scale: f32,
    pub progress: f32,
    pub open: bool,
}

type PointerMoveListener = dyn Fn(&gpui::MouseMoveEvent, &mut Window, &mut App);

#[derive(Clone, Copy)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

struct Placement {
    content: Point<Pixels>,
    arrow: Option<Point<Pixels>>,
    transform_origin: Point<Pixels>,
}

fn place(
    anchor: Bounds<Pixels>,
    content: Size<Pixels>,
    viewport: Size<Pixels>,
    preferred: TooltipPlacement,
    scale: f32,
    progress: f32,
) -> Placement {
    // Radix Popper includes the 5 px arrow height in its side offset.
    let gap = px(5.0 * scale);
    let top = anchor.top();
    let bottom = viewport.height - anchor.bottom();
    let left = anchor.left();
    let right = viewport.width - anchor.right();
    let side = match preferred {
        TooltipPlacement::Above if top < content.height + gap && bottom > top => Side::Bottom,
        TooltipPlacement::Above => Side::Top,
        TooltipPlacement::Right if right < content.width + gap && left > right => Side::Left,
        TooltipPlacement::Right => Side::Right,
    };
    let motion = px((1.0 - progress) * 8.0 * scale);
    let mut origin = match side {
        Side::Top => point(
            anchor.center().x - content.width / 2.0,
            anchor.top() - gap - content.height + motion,
        ),
        Side::Bottom => point(
            anchor.center().x - content.width / 2.0,
            anchor.bottom() + gap - motion,
        ),
        Side::Right => point(
            anchor.right() + gap - motion,
            anchor.center().y - content.height / 2.0,
        ),
        Side::Left => point(
            anchor.left() - gap - content.width + motion,
            anchor.center().y - content.height / 2.0,
        ),
    };
    // Floating UI shifts the cross axis, retaining the selected side and its gap.
    match side {
        Side::Top | Side::Bottom => {
            origin.x = origin
                .x
                .clamp(px(0.0), (viewport.width - content.width).max(px(0.0)));
        }
        Side::Right | Side::Left => {
            origin.y = origin
                .y
                .clamp(px(0.0), (viewport.height - content.height).max(px(0.0)));
        }
    }
    let arrow_size = px(10.0 * std::f32::consts::SQRT_2 * scale);
    let half = arrow_size / 2.0;
    let arrow = match side {
        Side::Top | Side::Bottom => {
            let center = anchor.center().x;
            (center >= origin.x + half && center <= origin.x + content.width - half).then(|| {
                point(
                    center - half,
                    if matches!(side, Side::Top) {
                        origin.y + content.height + gap - arrow_size
                    } else {
                        origin.y - gap
                    },
                )
            })
        }
        Side::Right | Side::Left => {
            let center = anchor.center().y;
            (center >= origin.y + half && center <= origin.y + content.height - half).then(|| {
                point(
                    if matches!(side, Side::Right) {
                        origin.x - gap
                    } else {
                        origin.x + content.width + gap - arrow_size
                    },
                    center - half,
                )
            })
        }
    };
    let transform_origin = match side {
        Side::Top => point(anchor.center().x, origin.y + content.height + gap),
        Side::Bottom => point(anchor.center().x, origin.y - gap),
        Side::Right => point(origin.x - gap, anchor.center().y),
        Side::Left => point(origin.x + content.width + gap, anchor.center().y),
    };
    Placement {
        transform_origin,
        content: origin,
        arrow,
    }
}

impl IntoElement for PositionedTooltip {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PositionedTooltip {
    type RequestLayoutState = (LayoutId, LayoutId);
    type PrepaintState = (bool, Point<Pixels>);

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
    ) -> (LayoutId, Self::RequestLayoutState) {
        let content = self.content.request_layout(window, cx);
        let arrow = self.arrow.request_layout(window, cx);
        let layout = window.request_layout(
            Style {
                position: Position::Absolute,
                display: Display::Flex,
                ..Style::default()
            },
            [content, arrow],
            cx,
        );
        (layout, (content, arrow))
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        layouts: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let bounds = window.layout_bounds(layouts.0);
        let placement = place(
            self.anchor.get(),
            bounds.size,
            window.viewport_size(),
            self.placement,
            self.scale,
            if self.open { self.progress } else { 1.0 },
        );
        self.content_bounds
            .set(Bounds::new(placement.content, bounds.size));
        window.with_prepaint_scale(
            placement.transform_origin,
            super::surface_motion::surface_scale(self.progress),
            |window| {
                window.with_element_offset(placement.content - bounds.origin, |window| {
                    self.content.prepaint(window, cx);
                });
                if let Some(origin) = placement.arrow {
                    let arrow_bounds = window.layout_bounds(layouts.1);
                    window.with_element_offset(origin - arrow_bounds.origin, |window| {
                        self.arrow.prepaint(window, cx);
                    });
                }
            },
        );
        (placement.arrow.is_some(), placement.transform_origin)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_paint_scale(
            state.1,
            super::surface_motion::surface_scale(self.progress),
            |window| {
                self.content.paint(window, cx);
                if state.0 {
                    self.arrow.paint(window, cx);
                }
            },
        );
        let listener = self.on_pointer_move.clone();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
            if phase.bubble() {
                listener(event, window, cx);
            }
        });
    }
}
