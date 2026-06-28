use super::*;

pub(in crate::app) const FRAME_ICON_BUTTON_SM_SIZE: f32 = 24.0;
pub(in crate::app) const FRAME_ICON_SM_SIZE: f32 = 16.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameIconButtonVariant {
    Ghost,
    Destructive,
    DestructiveGhost,
}

pub(in crate::app) fn frame_choice_button(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(id, label, ButtonVariant::Secondary, selected, enabled).w_full()
}

pub(in crate::app) fn frame_text_button(
    id: impl Into<String>,
    label: impl Into<String>,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(variant, selected, enabled);
    div()
        .id(id.into())
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(label.into())
}

pub(in crate::app) fn frame_icon_button(
    id: impl Into<String>,
    icon: &'static str,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: f32,
    icon_size: f32,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let (background, hover_background, active_background, foreground, hover_foreground, opacity) =
        match (variant, enabled) {
            (FrameIconButtonVariant::Ghost, true) => (
                theme::TRANSPARENT,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_200,
                theme::FRAME_GRAY_600,
                theme::FOREGROUND,
                1.0,
            ),
            (FrameIconButtonVariant::Ghost, false) => (
                theme::TRANSPARENT,
                theme::TRANSPARENT,
                theme::TRANSPARENT,
                theme::FRAME_GRAY_600,
                theme::FRAME_GRAY_600,
                0.5,
            ),
            (FrameIconButtonVariant::Destructive, true) => (
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_200,
                theme::FRAME_GRAY_200,
                theme::FRAME_RED,
                theme::FRAME_RED,
                1.0,
            ),
            (FrameIconButtonVariant::Destructive, false) => (
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_RED.with_alpha(0.5),
                theme::FRAME_RED.with_alpha(0.5),
                1.0,
            ),
            (FrameIconButtonVariant::DestructiveGhost, true) => (
                theme::TRANSPARENT,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_200,
                theme::FRAME_RED,
                theme::FRAME_RED,
                1.0,
            ),
            (FrameIconButtonVariant::DestructiveGhost, false) => (
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_RED.with_alpha(0.5),
                theme::FRAME_RED.with_alpha(0.5),
                1.0,
            ),
        };

    div()
        .id(id.clone())
        .group(id.clone())
        .w(px(size))
        .h(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(background))
        .text_color(color(foreground))
        .opacity(opacity)
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(hover_background))
                    .text_color(color(hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(active_background)))
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(icon_svg_with_hover(
            icon,
            icon_size,
            color(foreground),
            id,
            color(hover_foreground),
        ))
}
