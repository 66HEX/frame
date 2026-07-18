use super::{
    ButtonColors, ButtonVariant, Context, FluentBuilder, FrameRoot, InteractiveElement,
    ParentElement, SETTINGS_CONTROL_HEIGHT, StatefulInteractiveElement, Styled, Window,
    animated_button_colors, apply_accessible_button, apply_accessible_button_with_focus,
    apply_accessible_toggle_button, apply_button_motion, button_colors, button_highlight_shadows,
    color, contextual_icon_motion, div, icon_svg, px, theme,
};
use gpui::{FocusHandle, Rgba, Svg, Transformation, size, svg};

pub(in crate::app) const FRAME_ICON_BUTTON_SM_SIZE: f32 = 24.0;
pub(in crate::app) const FRAME_ICON_SM_SIZE: f32 = 16.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameIconButtonVariant {
    Ghost,
    DestructiveGhost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct FrameIconButtonSize {
    pub(in crate::app) button: f32,
    pub(in crate::app) icon: f32,
}

#[derive(Clone, Copy, Debug)]
enum FrameIconButtonContent {
    Static(&'static str),
    Swap {
        inactive: &'static str,
        active: &'static str,
        progress: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ContextualIconVisuals {
    opacity: f32,
    scale: f32,
    blur_radius: f32,
}

pub(in crate::app) fn frame_choice_button(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    apply_accessible_toggle_button(
        frame_text_button(
            id,
            label.clone(),
            ButtonVariant::Secondary,
            selected,
            enabled,
            window,
            cx,
        )
        .w_full(),
        label,
        enabled,
        selected,
    )
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
    frame_text_button_inner(id, label, variant, selected, enabled, None, window, cx)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Focused buttons mirror the existing explicit button builder and add only the focus handle."
)]
pub(in crate::app) fn frame_text_button_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button_inner(
        id,
        label,
        variant,
        selected,
        enabled,
        Some(focus),
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared implementation preserves the existing explicit button builder contract."
)]
fn frame_text_button_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let display_label = theme::ui_text_owned(label.clone());
    let colors = button_colors(variant, selected, enabled);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;
    let button = div()
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
        .child(display_label);

    let button = apply_button_motion(button, motion, enabled);

    if let Some(focus) = focus {
        apply_accessible_button_with_focus(button, label, enabled, focus)
    } else {
        apply_accessible_button(button, label, enabled)
    }
}

const fn text_button_uses_highlight(variant: ButtonVariant, selected: bool) -> bool {
    !matches!(variant, ButtonVariant::Ghost) || selected
}

#[expect(
    clippy::too_many_arguments,
    reason = "Icon buttons need explicit a11y labels plus the existing visual button contract."
)]
pub(in crate::app) fn frame_icon_button(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    frame_icon_button_inner(
        id,
        FrameIconButtonContent::Static(icon),
        label,
        variant,
        enabled,
        size,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Animated icon buttons add the alternate icon and active state to the existing explicit button contract."
)]
pub(in crate::app) fn frame_icon_swap_button(
    id: impl Into<String>,
    inactive_icon: &'static str,
    active_icon: &'static str,
    active: bool,
    label: impl Into<String>,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let progress = contextual_icon_motion(format!("{id}-icon-motion"), active, window, cx);
    frame_icon_button_inner(
        id,
        FrameIconButtonContent::Swap {
            inactive: inactive_icon,
            active: active_icon,
            progress,
        },
        label.into(),
        variant,
        enabled,
        size,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared implementation preserves the existing explicit icon button contract."
)]
fn frame_icon_button_inner(
    id: String,
    content: FrameIconButtonContent,
    label: String,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
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
            (FrameIconButtonVariant::DestructiveGhost, false) => (
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
    let motion = animated.motion;
    let icon = frame_icon_button_content(content, size.icon, animated_foreground);

    let button = div()
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
        .child(icon);

    let button = apply_button_motion(button, motion, enabled);

    apply_accessible_button(button, label, enabled)
}

fn frame_icon_button_content(
    content: FrameIconButtonContent,
    icon_size: f32,
    icon_color: Rgba,
) -> gpui::Div {
    let container = div().relative().w(px(icon_size)).h(px(icon_size));
    match content {
        FrameIconButtonContent::Static(icon) => {
            container.child(icon_svg(icon, icon_size, icon_color))
        }
        FrameIconButtonContent::Swap {
            inactive,
            active,
            progress,
        } => container
            .child(contextual_icon_svg(
                inactive,
                1.0 - progress,
                icon_size,
                icon_color,
            ))
            .child(contextual_icon_svg(active, progress, icon_size, icon_color)),
    }
}

fn contextual_icon_svg(icon: &'static str, progress: f32, icon_size: f32, icon_color: Rgba) -> Svg {
    let visuals = contextual_icon_visuals(progress);
    svg()
        .absolute()
        .inset_0()
        .path(icon)
        .w(px(icon_size))
        .h(px(icon_size))
        .text_color(icon_color)
        .opacity(visuals.opacity)
        .blur(px(visuals.blur_radius))
        .with_transformation(Transformation::scale(size(visuals.scale, visuals.scale)))
}

fn contextual_icon_visuals(progress: f32) -> ContextualIconVisuals {
    let progress = progress.clamp(0.0, 1.0);
    ContextualIconVisuals {
        opacity: progress,
        scale: progress.mul_add(0.75, 0.25),
        blur_radius: (1.0 - progress) * 4.0,
    }
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

    #[test]
    fn contextual_icon_visuals_use_hidden_endpoint_for_zero_progress() {
        assert_eq!(
            contextual_icon_visuals(0.0),
            ContextualIconVisuals {
                opacity: 0.0,
                scale: 0.25,
                blur_radius: 4.0,
            }
        );
    }

    #[test]
    fn contextual_icon_visuals_use_visible_endpoint_for_full_progress() {
        assert_eq!(
            contextual_icon_visuals(1.0),
            ContextualIconVisuals {
                opacity: 1.0,
                scale: 1.0,
                blur_radius: 0.0,
            }
        );
    }
}
