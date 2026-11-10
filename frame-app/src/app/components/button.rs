use super::{FrameTooltip, frame_animated_icon};
use crate::app::accessibility::apply_accessible_button;
#[cfg(test)]
use crate::app::accessibility::apply_accessible_toggle_button;
use crate::app::primitives::{ButtonVariant, button_colors, button_mouse_down, color};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    Context, FocusHandle, InteractiveElement, Rgba, StatefulInteractiveElement, Window, div, px,
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

#[cfg(test)]
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

fn control_colors(
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    behavior: ControlBehavior,
) -> crate::app::primitives::ButtonColors {
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
    colors
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
    let colors = control_colors(variant, selected, enabled, palette, behavior);
    let raised = selected
        || matches!(
            variant,
            ButtonVariant::Primary
                | ButtonVariant::Secondary
                | ButtonVariant::Accent
                | ButtonVariant::Outline
        );
    let background = if colors.background.alpha == 0.0 {
        colors.hover_background.with_alpha(0.0)
    } else {
        colors.background
    };

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
        // Keep the existing border allocation; the edge is painted by shadows.
        .border_color(color(palette.transparent))
        .px(theme::ui_rem(size.padding()))
        .bg(color(background))
        .when(raised, |button| {
            button.shadow(theme::shadows::ShadowLevel::Xs.shadows())
        })
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
                    button.active_style(move |style| {
                        let style = style.relative().top(px(1.0));
                        if raised {
                            style.shadow(theme::shadows::recessed(palette))
                        } else {
                            style
                        }
                    })
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
    frame_icon_button_with_motion_key(
        id.clone(),
        id,
        icon,
        label,
        variant,
        enabled,
        size,
        None,
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Queue action slots retain animation identity while their accessible action IDs change."
)]
pub(in crate::app) fn frame_icon_button_with_motion_key(
    id: impl Into<String>,
    motion_key: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let content = frame_animated_icon(motion_key, Some(icon), size.icon, None, window, cx);
    frame_icon_button_inner(
        FrameTooltip::new(id, label),
        content,
        variant,
        enabled,
        size,
        focus,
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
    active_color: Rgba,
    label: impl Into<String>,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let icon = if active { active_icon } else { inactive_icon };
    let content = frame_animated_icon(
        id.clone(),
        Some(icon),
        size.icon,
        active.then_some(active_color),
        window,
        cx,
    );
    frame_icon_button_inner(
        FrameTooltip::new(id, label).keep_open_on_click(),
        content,
        variant,
        enabled,
        size,
        None,
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
    tooltip: FrameTooltip,
    content: gpui::Div,
    variant: FrameIconButtonVariant,
    enabled: bool,
    size: FrameIconButtonSize,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = match variant {
        FrameIconButtonVariant::Ghost => ButtonVariant::Ghost,
        FrameIconButtonVariant::DestructiveGhost => ButtonVariant::Destructive,
    };
    let button = frame_button_base(
        tooltip.id.clone(),
        &tooltip.label,
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
    .child(content);
    tooltip.for_control(button, focus, enabled, palette, window, cx)
}
