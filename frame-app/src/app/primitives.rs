use crate::app::accessibility::apply_accessible_button;
use crate::app::layout::{
    TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_BUTTON_HEIGHT, TITLEBAR_ICON_BUTTON_SIZE,
    TITLEBAR_ICON_SIZE,
};
use crate::app::motion::{hover_motion, mix_color, retarget_hover_motion};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    App, BoxShadow, Context, InteractiveElement, IntoElement, MouseButton, Rgba,
    StatefulInteractiveElement, Window, div, point, px, svg,
};

#[derive(Clone, Copy)]
pub(super) enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ButtonColors {
    pub(super) background: theme::RgbaToken,
    pub(super) hover_background: theme::RgbaToken,
    pub(super) active_background: theme::RgbaToken,
    pub(super) foreground: theme::RgbaToken,
    pub(super) hover_foreground: theme::RgbaToken,
    pub(super) opacity: f32,
    /// Bottom stops for normal, hovered and pressed primary fills.
    pub(super) gradient_to: Option<[theme::RgbaToken; 3]>,
}

pub(super) struct AnimatedButtonColors {
    pub(super) background: gpui::Background,
    pub(super) foreground: Rgba,
    pub(super) motion: ButtonMotion,
}

pub(super) struct ButtonMotion {
    pub(super) hover_transition: gpui::Transition<f32>,
    pressed: gpui::Entity<bool>,
}

pub(super) const fn button_colors(
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
) -> ButtonColors {
    if !enabled {
        let background = if matches!(variant, ButtonVariant::Ghost) && !selected {
            palette.transparent
        } else {
            palette.fill_subtle
        };
        return ButtonColors {
            background,
            hover_background: background,
            active_background: background,
            foreground: palette.text_disabled,
            hover_foreground: palette.text_disabled,
            opacity: 1.0,
            gradient_to: None,
        };
    }

    if selected {
        ButtonColors {
            background: palette.fill_strong,
            hover_background: palette.fill_strong,
            active_background: palette.fill_selected,
            foreground: palette.text_primary,
            hover_foreground: palette.text_primary,
            opacity: 1.0,
            gradient_to: None,
        }
    } else if matches!(variant, ButtonVariant::Primary) {
        ButtonColors {
            background: palette.primary,
            hover_background: palette.primary_hover,
            active_background: palette.primary_active,
            foreground: palette.primary_foreground,
            hover_foreground: palette.primary_foreground,
            opacity: 1.0,
            gradient_to: Some([
                palette.primary_to,
                palette.primary_hover_to,
                palette.primary_active_to,
            ]),
        }
    } else if matches!(variant, ButtonVariant::Ghost) {
        ButtonColors {
            background: palette.transparent,
            hover_background: palette.fill_subtle,
            active_background: palette.fill_selected,
            foreground: palette.text_muted,
            hover_foreground: palette.text_primary,
            opacity: 1.0,
            gradient_to: None,
        }
    } else {
        ButtonColors {
            background: palette.fill_subtle,
            hover_background: palette.fill_selected,
            active_background: palette.fill_strong,
            foreground: palette.text_primary,
            hover_foreground: palette.text_primary,
            opacity: 1.0,
            gradient_to: None,
        }
    }
}

impl ButtonColors {
    pub(super) fn hover_fill(self, progress: f32) -> gpui::Background {
        let top = mix_color(self.background, self.hover_background, progress);
        self.gradient_to.map_or_else(
            || top.into(),
            |[normal, hover, _]| primary_gradient(top, mix_color(normal, hover, progress)),
        )
    }

    pub(super) fn active_fill(self) -> gpui::Background {
        let top = color(self.active_background);
        self.gradient_to.map_or_else(
            || top.into(),
            |[_, _, bottom]| primary_gradient(top, color(bottom)),
        )
    }
}

fn primary_gradient(top: Rgba, bottom: Rgba) -> gpui::Background {
    gpui::linear_gradient(
        180.0,
        gpui::linear_color_stop(top, 0.0),
        gpui::linear_color_stop(bottom, 1.0),
    )
}

pub(super) fn animated_button_colors(
    id: impl Into<String>,
    colors: ButtonColors,
    window: &mut Window,
    cx: &mut App,
) -> AnimatedButtonColors {
    let motion = button_motion(id, window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    AnimatedButtonColors {
        background: colors.hover_fill(hover_progress),
        foreground: mix_color(colors.foreground, colors.hover_foreground, hover_progress),
        motion,
    }
}

pub(super) fn button_motion(
    id: impl Into<String>,
    window: &mut Window,
    cx: &mut App,
) -> ButtonMotion {
    let id = id.into();
    ButtonMotion {
        hover_transition: hover_motion(id.clone(), window, cx),
        // GPUI refreshes after mouse-down, so pressed state must outlive render closures.
        pressed: window.use_keyed_state(format!("{id}-pressed"), cx, |_window, _cx| false),
    }
}

pub(super) fn apply_button_motion(
    button: gpui::Stateful<gpui::Div>,
    motion: ButtonMotion,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let hover_transition = motion.hover_transition.clone();
    let pressed = motion.pressed.clone();
    let button = button.on_hover(move |hover, _window, cx| {
        let pressed = *pressed.read(cx);
        if let Some(target) = button_motion_hover_target(enabled, *hover, pressed) {
            retarget_hover_motion(&hover_transition, target, cx);
        }
    });

    let pressed = motion.pressed.clone();
    let button = button.on_mouse_down(MouseButton::Left, move |_, window, cx| {
        if enabled {
            set_button_pressed(&pressed, true, cx);
        }
        button_mouse_down(enabled, window, cx);
    });

    let pressed = motion.pressed.clone();
    let button = button.capture_any_mouse_up(move |event, _window, cx| {
        if event.button == MouseButton::Left {
            set_button_pressed(&pressed, false, cx);
            // Release before click listeners run: they may stop propagation while updating the
            // selected state, which would prevent a bubble-phase mouse-up handler from running.
        }
    });

    let hover_transition = motion.hover_transition;
    let pressed = motion.pressed;
    button.on_mouse_up_out(MouseButton::Left, move |_, _window, cx| {
        set_button_pressed(&pressed, false, cx);
        retarget_hover_motion(&hover_transition, false, cx);
    })
}

fn set_button_pressed(pressed: &gpui::Entity<bool>, is_pressed: bool, cx: &mut App) {
    if *pressed.read(cx) != is_pressed {
        pressed.update(cx, |pressed, cx| {
            *pressed = is_pressed;
            cx.notify();
        });
    }
}

const fn button_motion_hover_target(enabled: bool, hovered: bool, pressed: bool) -> Option<bool> {
    if pressed {
        None
    } else {
        Some(enabled && hovered)
    }
}

pub(super) fn button_mouse_down(enabled: bool, window: &mut Window, cx: &mut App) {
    if enabled {
        window.prevent_default();
    } else {
        cx.stop_propagation();
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Titlebar action buttons need explicit a11y labels plus the existing visual contract."
)]
pub(super) fn action_button(
    id: impl Into<String>,
    icon: &'static str,
    label: Option<&'static str>,
    accessibility_label: impl Into<String>,
    variant: ButtonVariant,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let accessibility_label = accessibility_label.into();
    let is_icon_only = label.is_none();
    let colors = button_colors(variant, false, enabled, palette);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;

    let button = div()
        .id(id.clone())
        .debug_selector(|| id.clone())
        .group(id)
        .min_h(theme::ui_rem(TITLEBAR_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(background)
        .shadow(theme::shadows::CONTROL.shadows())
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(enabled, |this| {
            this.hover_style(gpui::Styled::cursor_pointer)
                .active_style(move |style| style.bg(colors.active_fill()))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed);

    let button = apply_button_motion(button, motion, enabled);

    let button = if is_icon_only {
        button
            .w(theme::ui_rem(TITLEBAR_ICON_BUTTON_SIZE))
            .child(icon_svg(icon, TITLEBAR_ACTION_ICON_SIZE, foreground))
    } else {
        button
            .px(theme::ui_rem(10.0))
            .child(icon_svg(icon, TITLEBAR_ICON_SIZE, foreground))
            .child(theme::ui_text(label.unwrap_or_default()))
    };

    apply_accessible_button(button, accessibility_label, enabled, palette)
}

pub(super) fn icon_svg(path: &'static str, size: f32, icon_color: Rgba) -> impl IntoElement {
    svg()
        .path(path)
        .w(theme::ui_rem(size))
        .h(theme::ui_rem(size))
        .text_color(icon_color)
}

pub(super) fn parse_hex(hex: &str) -> Rgba {
    let hex = hex.trim_start_matches('#');
    let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    color(theme::RgbaToken::from_rgb(red, green, blue))
}

pub(super) const fn frame_highlight_px() -> f32 {
    theme::shadows::EDGE_WIDTH
}

pub(super) fn vertical_separator(height: f32, palette: &theme::ThemePalette) -> gpui::Div {
    if palette.is_light() {
        div()
            .h(theme::ui_rem(height))
            .w(px(frame_highlight_px()))
            .bg(color(palette.border_subtle))
    } else {
        div()
            .flex()
            .h(theme::ui_rem(height))
            .w(px(2.0))
            .child(div().h_full().w(px(1.0)).bg(color(palette.canvas)))
            .child(
                div()
                    .h_full()
                    .w(px(1.0))
                    .bg(color(palette.highlight_subtle)),
            )
    }
}

pub(super) fn panel_bottom_separator(palette: &theme::ThemePalette) -> gpui::Div {
    let separator = div().absolute().left_0().right_0().bottom_0();

    if palette.is_light() {
        separator
            .h(px(frame_highlight_px()))
            .bg(color(palette.border_subtle))
    } else {
        separator
            .h(px(1.0))
            .bg(color(palette.canvas))
            .shadow(horizontal_separator_shadows(palette))
    }
}

pub(super) fn element_id(prefix: &str, id: &str) -> String {
    format!("{prefix}-{id}")
}

pub(super) trait FrameSurface {
    fn card_surface(self, palette: &theme::ThemePalette) -> Self;
}

impl FrameSurface for gpui::Div {
    fn card_surface(self, palette: &theme::ThemePalette) -> Self {
        self.rounded(theme::ui_rem(theme::RADIUS_LG))
            // Main UI cards keep a stable opaque fill.
            .bg(color(palette.surface))
            .shadow(theme::shadows::CARD.shadows())
    }
}

pub(super) fn horizontal_separator_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    let (highlight, offset_y) = if palette.is_light() {
        (palette.border_subtle, frame_highlight_px())
    } else {
        // Dark separators intentionally pair their canvas base with a full-pixel light edge.
        (palette.highlight_subtle, 1.0)
    };

    vec![BoxShadow {
        color: color(highlight).into(),
        offset: point(px(0.0), px(offset_y)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

pub(super) const fn color(token: theme::RgbaToken) -> Rgba {
    Rgba::new(token.red, token.green, token.blue, token.alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_fills_remain_gradients_during_hover_and_press() {
        for mode in crate::appearance::ColorTheme::ALL {
            let p = theme::palette(mode);
            let colors = button_colors(ButtonVariant::Primary, false, true, p);
            for fill in [
                colors.hover_fill(0.0),
                colors.hover_fill(0.5),
                colors.hover_fill(1.0),
                colors.active_fill(),
            ] {
                let gpui::BackgroundKind::LinearGradient { angle, stops } = fill.kind() else {
                    panic!("primary fill lost its gradient");
                };
                assert!((angle - 180.0).abs() < f32::EPSILON);
                assert!((stops[0].percentage).abs() < f32::EPSILON);
                assert!((stops[1].percentage - 1.0).abs() < f32::EPSILON);
                let top: gpui::Hsla = stops[0].color.into();
                let bottom: gpui::Hsla = stops[1].color.into();
                assert!(top.lightness > bottom.lightness);
            }
            assert_eq!(colors.foreground, p.primary_foreground);
        }
    }

    #[test]
    fn selection_and_disabled_states_do_not_inherit_primary_action_colors() {
        for mode in crate::appearance::ColorTheme::ALL {
            let p = theme::palette(mode);
            for variant in [
                ButtonVariant::Primary,
                ButtonVariant::Secondary,
                ButtonVariant::Ghost,
            ] {
                let selected = button_colors(variant, true, true, p);
                assert_eq!(selected.gradient_to, None);
                assert_eq!(selected.background, p.fill_strong);
                assert_eq!(selected.foreground, p.text_primary);
                let disabled = button_colors(variant, false, false, p);
                assert_eq!(disabled.gradient_to, None);
                assert_eq!(disabled.background, disabled.hover_background);
                assert_eq!(disabled.background, disabled.active_background);
                assert_eq!(disabled.foreground, p.text_disabled);
            }
        }
    }

    #[test]
    fn button_motion_hover_target_ignores_synthetic_changes_while_pressed() {
        let cases = [
            (true, true, false, Some(true)),
            (true, false, false, Some(false)),
            (false, true, false, Some(false)),
            (true, true, true, None),
            (true, false, true, None),
        ];

        for (enabled, hovered, pressed, expected) in cases {
            assert_eq!(
                button_motion_hover_target(enabled, hovered, pressed),
                expected,
                "hover target mismatch for enabled={enabled}, hovered={hovered}, pressed={pressed}"
            );
        }
    }
}
