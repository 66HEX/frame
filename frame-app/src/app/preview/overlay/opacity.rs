use crate::app::components::{
    frame_slider, frame_slider_content_bounds, frame_slider_fraction_from_bounds,
    frame_slider_handle,
};
use crate::app::preview::PreviewTimelineDragPreview;
use crate::app::root::FrameRoot;
use crate::numeric::f64_to_f32;
use crate::theme;
use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, DragMoveEvent, Element, ElementId, Entity, GlobalElementId, IntoElement,
    LayoutId, MouseButton, MouseDownEvent, Pixels, Position, Style, Window, relative, size,
};

pub(super) const OVERLAY_OPACITY_SLIDER_WIDTH: f32 = 96.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PreviewOverlayOpacityDrag;

pub(super) struct PreviewOverlayOpacityBoundsProbe {
    pub(super) owner: Entity<FrameRoot>,
}

impl IntoElement for PreviewOverlayOpacityBoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PreviewOverlayOpacityBoundsProbe {
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
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.owner.update(cx, |root, _cx| {
            root.set_preview_overlay_opacity_slider_bounds(frame_slider_content_bounds(
                bounds,
                window.rem_size(),
            ));
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

pub(super) fn preview_overlay_opacity_slider(
    value: f64,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let value = f64_to_f32(value.clamp(0.0, 1.0));
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    frame_slider(
        "preview-overlay-opacity-slider",
        "Overlay opacity",
        value,
        !enabled,
        palette,
        window,
        cx,
    )
    .w(theme::ui_rem(OVERLAY_OPACITY_SLIDER_WIDTH))
    .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
        if !enabled {
            return;
        }
        owner.update(cx, move |root, cx| {
            if let Some(next_opacity) = overlay_opacity_for_key(f64::from(value), "right")
                && root.set_selected_overlay_opacity(next_opacity)
            {
                cx.notify();
            }
        });
    })
    .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
        if !enabled {
            return;
        }
        decrement_owner.update(cx, move |root, cx| {
            if let Some(next_opacity) = overlay_opacity_for_key(f64::from(value), "left")
                && root.set_selected_overlay_opacity(next_opacity)
            {
                cx.notify();
            }
        });
    })
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(|root, event: &MouseDownEvent, _window, cx| {
            cx.stop_propagation();
            if root.commit_preview_overlay_opacity_at_position(event.position) {
                cx.notify();
            }
        }),
    )
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
            let Some(next_opacity) =
                overlay_opacity_for_key(f64::from(value), event.keystroke.key.as_str())
            else {
                return;
            };
            if root.set_selected_overlay_opacity(next_opacity) {
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
    .when(enabled, |this| {
        this.cursor_default().on_drag(
            PreviewOverlayOpacityDrag,
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        )
    })
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<PreviewOverlayOpacityDrag>, window, cx| {
            let opacity = frame_slider_fraction_from_bounds(
                event.event.position,
                event.bounds,
                window.rem_size(),
            );
            if root.set_selected_overlay_opacity(opacity) {
                cx.notify();
            }
        },
    ))
    .child(frame_slider_handle(
        "preview-overlay-opacity-handle",
        value,
        enabled,
    ))
    .child(PreviewOverlayOpacityBoundsProbe { owner: cx.entity() })
}

pub(in crate::app) fn overlay_opacity_for_key(value: f64, key: &str) -> Option<f64> {
    let next = match key {
        "left" | "down" => value - 0.01,
        "right" | "up" => value + 0.01,
        "pageup" => value + 0.10,
        "pagedown" => value - 0.10,
        "home" => 0.0,
        "end" => 1.0,
        _ => return None,
    };

    Some(next.clamp(0.0, 1.0))
}
