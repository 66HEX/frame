use super::*;
use crate::numeric::{f64_to_f32, unit_f64_to_f32};

const OVERLAY_HANDLE_SIZE: f32 = 10.0;
const OVERLAY_OPACITY_SLIDER_WIDTH: f32 = 96.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PreviewOverlayDrag {
    pub(super) handle: OverlayDragHandle,
    pub(super) width: f64,
    pub(super) height: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewOverlayOpacityDrag;

struct PreviewOverlayOpacityBoundsProbe {
    owner: Entity<FrameRoot>,
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
        _window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.owner.update(cx, |root, _cx| {
            root.set_preview_overlay_opacity_slider_bounds(bounds);
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

pub(in crate::app) fn preview_overlay_layer(
    state: &PreviewShellState,
) -> Option<gpui::Stateful<gpui::Div>> {
    let overlay = state.overlay.overlay.as_ref()?;
    if !overlay.enabled || overlay.path.is_empty() {
        return None;
    }

    let (width, height) = preview_overlay_render_size(state, overlay);
    let left = (overlay.x - width / 2.0).clamp(0.0, 1.0);
    let top = (overlay.y - height / 2.0).clamp(0.0, 1.0);
    let drag = PreviewOverlayDrag {
        handle: OverlayDragHandle::Move,
        width,
        height,
    };

    let mut layer = div()
        .id("preview-overlay-layer")
        .absolute()
        .left(relative(unit_f64_to_f32(left)))
        .top(relative(unit_f64_to_f32(top)))
        .w(relative(unit_f64_to_f32(width)))
        .h(relative(unit_f64_to_f32(height)))
        .when(state.overlay.overlay_mode, |this| {
            this.cursor_grab()
                .border_1()
                .border_color(color(theme::FOREGROUND.with_alpha(0.90)))
                .on_drag(drag, |_drag, _position, _window, cx| {
                    cx.new(|_| PreviewTimelineDragPreview)
                })
        })
        .child(
            div()
                .absolute()
                .inset_0()
                .overflow_hidden()
                .opacity(unit_f64_to_f32(overlay.opacity))
                .child(
                    img(PathBuf::from(overlay.path.clone()))
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                ),
        );

    if state.overlay.overlay_mode {
        layer = layer
            .child(preview_overlay_handle(
                OverlayDragHandle::NorthWest,
                0.0,
                0.0,
                width,
                height,
            ))
            .child(preview_overlay_handle(
                OverlayDragHandle::NorthEast,
                1.0,
                0.0,
                width,
                height,
            ))
            .child(preview_overlay_handle(
                OverlayDragHandle::SouthEast,
                1.0,
                1.0,
                width,
                height,
            ))
            .child(preview_overlay_handle(
                OverlayDragHandle::SouthWest,
                0.0,
                1.0,
                width,
                height,
            ));
    }

    Some(layer)
}

#[expect(
    clippy::too_many_lines,
    reason = "Overlay controls are a compact declarative toolbar whose layout is clearer when kept together."
)]
pub(in crate::app) fn preview_overlay_controls(
    state: &PreviewShellState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Div> {
    let overlay = state.overlay.overlay.as_ref()?;
    if !state.overlay.overlay_mode {
        return None;
    }
    let enabled = preview_visual_controls_enabled(state);
    let media = state.media;

    let bar = div()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex(PREVIEW_TOOLBAR_BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows())
        .child(
            preview_overlay_icon_button(
                "replace",
                assets::ICON_FILE_IMAGE,
                ButtonVariant::Ghost,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                root.prompt_selected_overlay_image(cx);
            })),
        )
        .child(preview_toolbar_vertical_separator())
        .child(
            preview_overlay_icon_button(
                "decrease",
                assets::ICON_MINUS,
                ButtonVariant::Ghost,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if root.nudge_selected_overlay_size(OverlaySizeDirection::Decrease, media) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_overlay_icon_button(
                "increase",
                assets::ICON_PLUS,
                ButtonVariant::Ghost,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if root.nudge_selected_overlay_size(OverlaySizeDirection::Increase, media) {
                    cx.notify();
                }
            })),
        )
        .child(preview_overlay_opacity_slider(overlay.opacity, enabled, cx))
        .child(preview_toolbar_vertical_separator())
        .child(
            frame_icon_button(
                "preview-overlay-remove",
                assets::ICON_TRASH,
                FrameIconButtonVariant::DestructiveGhost,
                enabled,
                FrameIconButtonSize {
                    button: PREVIEW_TOOLBAR_BUTTON_SIZE,
                    icon: PREVIEW_TOOLBAR_ICON_SIZE,
                },
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.remove_selected_overlay() {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_overlay_icon_button(
                "done",
                assets::ICON_CHECK,
                ButtonVariant::Default,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.set_selected_overlay_mode(false) {
                    cx.notify();
                }
            })),
        );

    Some(
        div()
            .absolute()
            .bottom(px(16.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(bar),
    )
}

pub(super) fn overlay_drag_point_from_bounds(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
    drag: PreviewOverlayDrag,
) -> OverlayDragPoint {
    let point = normalized_point_from_bounds(position, bounds);
    OverlayDragPoint {
        x: point.x,
        y: point.y,
        width: Some(drag.width),
        height: Some(drag.height),
    }
}

fn preview_overlay_render_size(state: &PreviewShellState, overlay: &PreviewOverlay) -> (f64, f64) {
    let width = overlay.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
    let height = width * preview_overlay_height_ratio(state);
    (width, height.clamp(MIN_OVERLAY_WIDTH, 1.0))
}

fn preview_overlay_height_ratio(state: &PreviewShellState) -> f64 {
    let overlay_ratio = state
        .overlay
        .image_dimensions
        .map_or(1.0, PreviewOverlayImageDimensions::height_over_width);
    let media_ratio = state.media.map_or(1.0, |media| {
        if media.height == 0 {
            1.0
        } else {
            f64::from(media.width) / f64::from(media.height)
        }
    });
    overlay_ratio * media_ratio
}

fn preview_overlay_handle(
    handle: OverlayDragHandle,
    x: f32,
    y: f32,
    width: f64,
    height: f64,
) -> gpui::Stateful<gpui::Div> {
    overlay_handle_cursor(
        div()
            .id(format!(
                "preview-overlay-handle-{}",
                overlay_handle_id(handle)
            ))
            .absolute()
            .left(relative(x))
            .top(relative(y))
            .ml(px(-(OVERLAY_HANDLE_SIZE / 2.0)))
            .mt(px(-(OVERLAY_HANDLE_SIZE / 2.0)))
            .w(px(OVERLAY_HANDLE_SIZE))
            .h(px(OVERLAY_HANDLE_SIZE))
            .rounded_full()
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.45))
            .bg(color(theme::FOREGROUND))
            .shadow(card_surface_shadows()),
        handle,
    )
    .on_drag(
        PreviewOverlayDrag {
            handle,
            width,
            height,
        },
        |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
    )
}

fn overlay_handle_cursor(
    handle: gpui::Stateful<gpui::Div>,
    drag_handle: OverlayDragHandle,
) -> gpui::Stateful<gpui::Div> {
    match drag_handle {
        OverlayDragHandle::NorthWest | OverlayDragHandle::SouthEast => handle.cursor_nwse_resize(),
        OverlayDragHandle::NorthEast | OverlayDragHandle::SouthWest => handle.cursor_nesw_resize(),
        OverlayDragHandle::Move => handle.cursor_grab(),
    }
}

const fn overlay_handle_id(handle: OverlayDragHandle) -> &'static str {
    match handle {
        OverlayDragHandle::Move => "move",
        OverlayDragHandle::NorthWest => "nw",
        OverlayDragHandle::NorthEast => "ne",
        OverlayDragHandle::SouthEast => "se",
        OverlayDragHandle::SouthWest => "sw",
    }
}

fn preview_overlay_icon_button(
    id: &'static str,
    icon: &'static str,
    variant: ButtonVariant,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(variant, false, enabled);
    let button_id = format!("preview-overlay-{id}");
    let animated = animated_button_colors(button_id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;
    let highlighted = matches!(variant, ButtonVariant::Default);

    div()
        .id(button_id)
        .w(px(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .h(px(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(highlighted, |this| this.shadow(button_highlight_shadows()))
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| {
                    style
                        .bg(color(colors.active_background))
                        .text_color(color(colors.hover_foreground))
                })
        })
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .child(icon_svg(icon, PREVIEW_TOOLBAR_ICON_SIZE, foreground))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
}

fn preview_overlay_opacity_slider(
    value: f64,
    enabled: bool,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let value = f64_to_f32(value.clamp(0.0, 1.0));

    frame_slider("preview-overlay-opacity-slider", value, !enabled)
        .w(px(OVERLAY_OPACITY_SLIDER_WIDTH))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|root, event: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                if root.commit_preview_overlay_opacity_at_position(event.position) {
                    cx.notify();
                }
            }),
        )
        .when(enabled, |this| {
            this.cursor_ew_resize().on_drag(
                PreviewOverlayOpacityDrag,
                |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
            )
        })
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewOverlayOpacityDrag>, _window, cx| {
                let opacity =
                    timeline_slider_percent_from_bounds(event.event.position, event.bounds);
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
