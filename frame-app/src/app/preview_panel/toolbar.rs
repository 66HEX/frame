use super::{
    ButtonVariant, ClickEvent, Context, FlipAxis, FluentBuilder, FrameRoot, InteractiveElement,
    PREVIEW_TOOLBAR_BUTTON_SIZE, PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET, ParentElement,
    PreviewCanvasZoomDirection, PreviewShellState, PreviewToolFocuses, StatefulInteractiveElement,
    Styled, Window, animated_button_colors, apply_accessible_button,
    apply_accessible_button_with_focus, apply_accessible_toggle_button, apply_button_motion,
    assets, button_colors, button_highlight_shadows, card_surface_shadows, color, div, icon_svg,
    preview_visual_controls_enabled, relative, theme,
};
use gpui::FocusHandle;

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

#[expect(
    clippy::too_many_lines,
    reason = "The declarative toolbar layout keeps its mutually dependent controls in visual order."
)]
pub(in crate::app) fn preview_toolbar(
    state: &PreviewShellState,
    focuses: PreviewToolFocuses<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let transform_enabled = preview_visual_controls_enabled(state);
    let crop_enabled = transform_enabled && state.crop.has_crop_dimensions;
    let overlay_enabled = transform_enabled && state.availability.overlay_available;

    div()
        .absolute()
        .top(relative(0.5))
        .mt(theme::ui_rem(preview_toolbar_center_margin()))
        .left(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .gap(theme::ui_rem(PREVIEW_TOOLBAR_GAP))
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.surface))
        .p(theme::ui_rem(PREVIEW_TOOLBAR_PADDING))
        .shadow(card_surface_shadows(palette))
        .child(
            preview_tool_button(
                "preview-tool-rotate",
                assets::ICON_ROTATE_CW,
                "Rotate preview",
                false,
                transform_enabled,
                palette,
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
                palette,
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
                palette,
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
                palette,
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
                palette,
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
    let palette = state.palette;
    let enabled = preview_visual_controls_enabled(state);

    div()
        .absolute()
        .right(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .bottom(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.surface))
        .p(theme::ui_rem(4.0))
        .shadow(card_surface_shadows(palette))
        .child(
            preview_tool_button(
                "preview-zoom-out",
                assets::ICON_MINUS,
                "Zoom out",
                false,
                enabled,
                palette,
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
                palette,
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

pub(in crate::app) fn preview_toolbar_vertical_separator(
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex_none()
        .h(theme::ui_rem(PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_HEIGHT))
        .w(theme::ui_rem(PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_WIDTH))
        .bg(color(palette.fill_selected))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Toolbar buttons keep semantics, interaction state, palette, and render context explicit."
)]
pub(in crate::app) fn preview_tool_button(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_tool_button_inner(
        id, icon, label, selected, enabled, None, palette, window, cx,
    )
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
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_tool_button_inner(
        id,
        icon,
        label,
        selected,
        enabled,
        Some(focus),
        palette,
        window,
        cx,
    )
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
    palette: &'static theme::ThemePalette,
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
    let colors = button_colors(variant, selected, enabled, palette);
    let animated = animated_button_colors(button_id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;

    let button = div()
        .id(button_id)
        .w(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .h(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(selected, |this| {
            this.shadow(button_highlight_shadows(palette))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| {
                    style
                        .bg(color(colors.active_background))
                        .text_color(color(colors.hover_foreground))
                })
        })
        .child(icon_svg(icon, PREVIEW_TOOLBAR_ICON_SIZE, foreground));

    let button = apply_button_motion(button, motion, enabled);

    if let Some(focus) = focus {
        let button = apply_accessible_button_with_focus(button, label, enabled, focus, palette);
        if selected {
            button.aria_toggled(gpui::Toggled::True)
        } else {
            button
        }
    } else if selected {
        apply_accessible_toggle_button(button, label, enabled, true, palette)
    } else {
        apply_accessible_button(button, label, enabled, palette)
    }
}
