use super::{
    ButtonColors, ButtonVariant, Context, FluentBuilder, FrameRoot, InteractiveElement,
    MouseButton, ParentElement, SETTINGS_CONTROL_HEIGHT, StatefulInteractiveElement, Styled,
    Window, animated_button_colors, button_colors, button_highlight_shadows, button_mouse_down,
    color, div, icon_svg, px, retarget_hover_motion, theme,
};

pub(in crate::app) const FRAME_ICON_BUTTON_SM_SIZE: f32 = 24.0;
pub(in crate::app) const FRAME_ICON_SM_SIZE: f32 = 16.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameIconButtonVariant {
    Ghost,
    Destructive,
    DestructiveGhost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct FrameIconButtonSize {
    pub(in crate::app) button: f32,
    pub(in crate::app) icon: f32,
}

pub(in crate::app) fn frame_choice_button(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        id,
        label,
        ButtonVariant::Secondary,
        selected,
        enabled,
        window,
        cx,
    )
    .w_full()
}

pub(in crate::app) fn frame_text_button(
    id: impl Into<String>,
    label: impl Into<String>,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = theme::ui_text_owned(label.into());
    let colors = button_colors(variant, selected, enabled);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;
    div()
        .id(id)
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(background)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(text_button_uses_highlight(variant, selected), |this| {
            this.shadow(button_highlight_shadows())
        })
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(label)
}

const fn text_button_uses_highlight(variant: ButtonVariant, selected: bool) -> bool {
    !matches!(variant, ButtonVariant::Ghost) || selected
}

pub(in crate::app) fn frame_icon_button(
    id: impl Into<String>,
    icon: &'static str,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
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
            (
                FrameIconButtonVariant::Destructive | FrameIconButtonVariant::DestructiveGhost,
                false,
            ) => (
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
        };
    let animated = animated_button_colors(
        id.clone(),
        ButtonColors {
            background,
            hover_background,
            active_background,
            foreground,
            hover_foreground,
            opacity,
        },
        window,
        cx,
    );
    let animated_background = animated.background;
    let animated_foreground = animated.foreground;
    let hover_transition = animated.hover_transition;

    div()
        .id(id.clone())
        .group(id)
        .w(px(size.button))
        .h(px(size.button))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(animated_background)
        .text_color(animated_foreground)
        .opacity(opacity)
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| style.bg(color(active_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(icon_svg(icon, size.icon, animated_foreground))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghost_text_button_uses_no_highlight_when_unselected() {
        assert!(!text_button_uses_highlight(ButtonVariant::Ghost, false));
    }

    #[test]
    fn ghost_text_button_uses_highlight_when_selected() {
        assert!(text_button_uses_highlight(ButtonVariant::Ghost, true));
    }

    #[test]
    fn secondary_text_button_keeps_highlight() {
        assert!(text_button_uses_highlight(ButtonVariant::Secondary, false));
    }
}
