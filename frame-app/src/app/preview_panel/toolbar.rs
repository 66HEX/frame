use super::{
    ButtonVariant, ClickEvent, Context, FlipAxis, FluentBuilder, FrameRoot, InteractiveElement,
    MouseButton, PREVIEW_TOOLBAR_BUTTON_SIZE, PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET,
    ParentElement, PreviewCanvasZoomDirection, PreviewShellState, PreviewToolFocuses,
    StatefulInteractiveElement, Styled, Window, animated_button_colors, apply_accessible_button,
    apply_accessible_button_with_focus, apply_accessible_toggle_button, assets, button_colors,
    button_highlight_shadows, button_mouse_down, card_surface_shadows, color, div, icon_svg,
    parse_hex, preview_visual_controls_enabled, px, relative, retarget_hover_motion, theme,
};
use gpui::FocusHandle;

pub(in crate::app) const PREVIEW_TOOLBAR_BACKGROUND: &str = "#1B1D21";

const PREVIEW_TOOLBAR_PADDING: f32 = 4.0;
const PREVIEW_TOOLBAR_GAP: f32 = 8.0;
const PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_HEIGHT: f32 = 18.0;
const PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_WIDTH: f32 = 1.0;
const PREVIEW_TOOLBAR_BUTTON_COUNT: f32 = 5.0;
const PREVIEW_TOOLBAR_GAP_COUNT: f32 = 4.0;

pub(in crate::app) const fn preview_toolbar_height() -> f32 {
    (PREVIEW_TOOLBAR_PADDING * 2.0)
        + (PREVIEW_TOOLBAR_BUTTON_SIZE * PREVIEW_TOOLBAR_BUTTON_COUNT)
        + (PREVIEW_TOOLBAR_GAP * PREVIEW_TOOLBAR_GAP_COUNT)
}

pub(in crate::app) const fn preview_toolbar_center_margin() -> f32 {
    -(preview_toolbar_height() / 2.0)
}

pub(in crate::app) fn preview_toolbar(
    state: &PreviewShellState,
    focuses: PreviewToolFocuses<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let transform_enabled = preview_visual_controls_enabled(state);
    let crop_enabled = transform_enabled && state.crop.has_crop_dimensions;
    let overlay_enabled = transform_enabled && state.availability.overlay_available;

    div()
        .absolute()
        .top(relative(0.5))
        .mt(px(preview_toolbar_center_margin()))
        .left(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .gap(px(PREVIEW_TOOLBAR_GAP))
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex(PREVIEW_TOOLBAR_BACKGROUND))
        .p(px(PREVIEW_TOOLBAR_PADDING))
        .shadow(card_surface_shadows())
        .child(
            preview_tool_button(
                "preview-tool-rotate",
                assets::ICON_ROTATE_CW,
                "Rotate preview",
                false,
                transform_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.rotate_selected_preview() {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                "preview-tool-flip-horizontal",
                assets::ICON_FLIP_HORIZONTAL,
                "Flip horizontally",
                state.crop.flip_horizontal,
                transform_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Horizontal) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                "preview-tool-flip-vertical",
                assets::ICON_FLIP_VERTICAL,
                "Flip vertically",
                state.crop.flip_vertical,
                transform_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Vertical) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button_with_focus(
                "preview-tool-crop",
                assets::ICON_CROP,
                "Crop",
                state.crop.crop_mode || state.crop.applied_crop.is_some(),
                crop_enabled,
                focuses.crop,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_crop_mode() {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button_with_focus(
                "preview-tool-overlay",
                assets::ICON_FILE_IMAGE,
                "Overlay image",
                state.overlay.overlay_mode || state.overlay.has_overlay,
                overlay_enabled,
                focuses.overlay,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                if root.trigger_selected_overlay(window, cx) {
                    cx.notify();
                }
            })),
        )
}

pub(in crate::app) fn preview_zoom_toolbar(
    state: &PreviewShellState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let enabled = preview_visual_controls_enabled(state);

    div()
        .absolute()
        .right(px(PREVIEW_TOOLBAR_OFFSET))
        .bottom(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex(PREVIEW_TOOLBAR_BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows())
        .child(
            preview_tool_button(
                "preview-zoom-out",
                assets::ICON_MINUS,
                "Zoom out",
                false,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.zoom_preview_canvas(PreviewCanvasZoomDirection::Out, cx) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                "preview-zoom-in",
                assets::ICON_PLUS,
                "Zoom in",
                false,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.zoom_preview_canvas(PreviewCanvasZoomDirection::In, cx) {
                    cx.notify();
                }
            })),
        )
}

pub(in crate::app) fn preview_toolbar_vertical_separator() -> gpui::Div {
    div()
        .flex_none()
        .h(px(PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_HEIGHT))
        .w(px(PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_WIDTH))
        .bg(color(theme::FRAME_GRAY_200))
}

pub(in crate::app) fn preview_tool_button(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_tool_button_inner(id, icon, label, selected, enabled, None, window, cx)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Preview tool buttons need explicit focus handles for modal focus restoration."
)]
pub(in crate::app) fn preview_tool_button_with_focus(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_tool_button_inner(id, icon, label, selected, enabled, Some(focus), window, cx)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared tool button builder preserves the existing visual contract and optionally wires a focus handle."
)]
fn preview_tool_button_inner(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };
    let button_id = id.into();
    let label = label.into();
    let colors = button_colors(variant, selected, enabled);
    let animated = animated_button_colors(button_id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;

    let button = div()
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
        .when(selected, |this| this.shadow(button_highlight_shadows()))
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
        });

    if let Some(focus) = focus {
        let button = apply_accessible_button_with_focus(button, label, enabled, focus);
        if selected {
            button.aria_toggled(gpui::Toggled::True)
        } else {
            button
        }
    } else if selected {
        apply_accessible_toggle_button(button, label, enabled, true)
    } else {
        apply_accessible_button(button, label, enabled)
    }
}
