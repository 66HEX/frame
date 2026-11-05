use crate::app::accessibility::{apply_accessible_button, apply_accessible_toggle_button};
use crate::app::motion::contextual_icon_motion;
use crate::app::primitives::{ButtonVariant, button_colors, button_mouse_down, color, icon_svg};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    Context, FocusHandle, InteractiveElement, Rgba, StatefulInteractiveElement, Svg,
    Transformation, Window, div, px, size, svg,
};

pub(in crate::app) const FRAME_ICON_BUTTON_SM_SIZE: f32 = 24.0;
pub(in crate::app) const FRAME_ICON_SM_SIZE: f32 = 12.0;

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

pub(in crate::app) fn frame_toggle_button_base(
    id: impl Into<String>,
    label: &str,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    _window: &mut Window,
    _cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let button = frame_control_base(
        id,
        label,
        ButtonVariant::Ghost,
        selected,
        enabled,
        theme::metrics::ControlSize::Default,
        palette,
        ControlBehavior::Toggle,
    );
    apply_accessible_toggle_button(button, label.to_owned(), enabled, selected, palette)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Toggle items share their group focus, state, palette and rendering context."
)]
pub(in crate::app) fn frame_choice_button(
    group: &super::FrameChoiceFocus,
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let focus = group.register(&id, selected, enabled, cx);
    let label = label.into();
    let button =
        frame_toggle_button_base(id.clone(), &label, selected, enabled, palette, window, cx)
            .w_full()
            .child(theme::ui_text_owned(label));
    group.apply(button, id, enabled, &focus, cx)
}

#[expect(
    clippy::too_many_arguments,
    reason = "This shared button constructor exposes its semantic state, palette, and GPUI render context explicitly."
)]
pub(in crate::app) fn frame_text_button(
    id: impl Into<String>,
    label: impl Into<String>,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button_inner(
        id, label, variant, selected, enabled, None, palette, window, cx,
    )
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
    palette: &'static theme::ThemePalette,
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
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared implementation preserves the existing explicit button builder contract."
)]
pub(in crate::app) fn frame_text_button_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let button = frame_button_base(
        id,
        &label,
        variant,
        selected,
        enabled,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    )
    .child(theme::ui_text_owned(label.clone()));
    if let Some(focus) = focus {
        button.track_focus(focus)
    } else {
        button
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "One shared button constructor owns all Mira variants and sizes."
)]
pub(in crate::app) fn frame_button_base(
    id: impl Into<String>,
    label: &str,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    size: theme::metrics::ControlSize,
    palette: &'static theme::ThemePalette,
    _window: &mut Window,
    _cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_control_base(
        id,
        label,
        variant,
        selected,
        enabled,
        size,
        palette,
        ControlBehavior::Button,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlBehavior {
    Button,
    Toggle,
}

impl ControlBehavior {
    fn transitions(self, t: gpui::StyleTransitions) -> gpui::StyleTransitions {
        let t = crate::app::motion::control_style_transitions(t);
        match self {
            // Only Button has Mira's 1px pressed offset. Drag handles use the
            // accessibility helper and must not inherit position transitions.
            Self::Button => t.top(crate::app::motion::interaction_motion()),
            Self::Toggle => t,
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Shared visual implementation for the Mira Button and Toggle contracts."
)]
fn frame_control_base(
    id: impl Into<String>,
    label: &str,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    size: theme::metrics::ControlSize,
    palette: &'static theme::ThemePalette,
    behavior: ControlBehavior,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let mut colors = button_colors(variant, selected, enabled, palette);
    if behavior == ControlBehavior::Toggle {
        colors.background = palette.fill_selected.with_alpha(if selected {
            palette.fill_selected.alpha
        } else {
            0.0
        });
        colors.hover_background = if enabled {
            palette.fill_selected
        } else {
            colors.background
        };
    }

    let button = div()
        .id(id.clone())
        .debug_selector(|| id.clone())
        .group(id)
        .h(theme::ui_rem(size.height()))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::ui_rem(theme::metrics::CONTROL_GAP))
        .rounded(theme::ui_rem(size.radius()))
        .border_1()
        .when(behavior == ControlBehavior::Toggle, gpui::Styled::border_0)
        .border_color(color(if matches!(variant, ButtonVariant::Outline) {
            palette.border_subtle
        } else {
            palette.transparent
        }))
        .px(theme::ui_rem(size.padding()))
        .bg(color(if colors.background.alpha == 0.0 {
            colors.hover_background.with_alpha(0.0)
        } else {
            colors.background
        }))
        .text_size(theme::ui_rem(size.text()))
        .line_height(gpui::relative(theme::metrics::TEXT_LINE_HEIGHT))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .whitespace_nowrap()
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .disabled(!enabled)
        .relative()
        .top_0()
        .when(enabled, |this| {
            this.cursor_default()
                .hover_style(move |style| {
                    let style = style
                        .bg(color(colors.hover_background))
                        .text_color(color(colors.hover_foreground));
                    if matches!(variant, ButtonVariant::Link) {
                        style.underline()
                    } else {
                        style
                    }
                })
                .when(behavior == ControlBehavior::Button, |button| {
                    button.active_style(move |style| style.relative().top(px(1.0)))
                })
        })
        .when(!enabled, |button| {
            button.cursor_not_allowed().relative().child(
                // Disabled GPUI callbacks do not consume native titlebar gestures.
                div().absolute().inset_0().on_mouse_down(
                    gpui::MouseButton::Left,
                    |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    },
                ),
            )
        });
    apply_accessible_button(
        button.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        }),
        label.to_string(),
        enabled,
        palette,
    )
    .transitions(|t| behavior.transitions(t))
    .when(matches!(variant, ButtonVariant::Destructive), |button| {
        button
            .ring_color(color(palette.danger.with_alpha(0.0)))
            .focus_visible_style(move |style| {
                style
                    .border_color(color(palette.danger.with_opacity(0.4)))
                    .ring(gpui::px(2.0))
                    .ring_color(color(palette.danger.with_opacity(if palette.is_light() {
                        0.2
                    } else {
                        0.4
                    })))
            })
    })
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
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    frame_icon_button_inner(
        id,
        FrameIconButtonContent::Static(icon),
        &label,
        variant,
        enabled,
        size,
        palette,
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
    palette: &'static theme::ThemePalette,
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
        &label.into(),
        variant,
        enabled,
        size,
        palette,
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
    label: &str,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = match variant {
        FrameIconButtonVariant::Ghost => ButtonVariant::Ghost,
        FrameIconButtonVariant::DestructiveGhost => ButtonVariant::Destructive,
    };
    let foreground = color(button_colors(variant, false, enabled, palette).foreground);
    frame_button_base(
        id,
        label,
        variant,
        false,
        enabled,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    )
    .w(theme::ui_rem(size.button))
    .h(theme::ui_rem(size.button))
    .p_0()
    .child(frame_icon_button_content(content, size.icon, foreground))
}

fn frame_icon_button_content(
    content: FrameIconButtonContent,
    icon_size: f32,
    icon_color: Rgba,
) -> gpui::Div {
    let container = div()
        .relative()
        .w(theme::ui_rem(icon_size))
        .h(theme::ui_rem(icon_size));
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
        .w(theme::ui_rem(icon_size))
        .h(theme::ui_rem(icon_size))
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
