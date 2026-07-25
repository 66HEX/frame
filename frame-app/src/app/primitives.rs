use super::{
    App, BoxShadow, Context, FluentBuilder, FrameRoot, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Rgba, StatefulInteractiveElement, Styled,
    TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_BUTTON_HEIGHT, TITLEBAR_ICON_BUTTON_SIZE,
    TITLEBAR_ICON_SIZE, Window, accessibility::apply_accessible_button, div, hover_motion,
    mix_color, point, px, retarget_hover_motion, svg, theme,
};

#[derive(Clone, Copy)]
pub(super) enum ButtonVariant {
    Default,
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
}

pub(super) struct AnimatedButtonColors {
    pub(super) background: Rgba,
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
    let active_variant = matches!(variant, ButtonVariant::Default) || selected;
    if !enabled {
        let (background, foreground, opacity) = if active_variant {
            (
                palette.border_subtle.with_alpha(0.10),
                palette.text_primary.with_alpha(0.50),
                1.0,
            )
        } else if matches!(variant, ButtonVariant::Ghost) {
            (palette.transparent, palette.text_muted, 0.5)
        } else {
            (
                palette.fill_subtle,
                palette.text_primary.with_alpha(0.50),
                0.5,
            )
        };
        return ButtonColors {
            background,
            hover_background: background,
            active_background: background,
            foreground,
            hover_foreground: foreground,
            opacity,
        };
    }

    if active_variant {
        ButtonColors {
            background: palette.border_subtle,
            hover_background: palette.border_subtle.with_alpha(0.18),
            active_background: palette.border_subtle.with_alpha(0.18),
            foreground: palette.text_primary,
            hover_foreground: palette.text_primary,
            opacity: 1.0,
        }
    } else if matches!(variant, ButtonVariant::Ghost) {
        ButtonColors {
            background: palette.transparent,
            hover_background: palette.fill_subtle,
            active_background: palette.fill_selected,
            foreground: palette.text_muted,
            hover_foreground: palette.text_primary,
            opacity: 1.0,
        }
    } else {
        ButtonColors {
            background: palette.fill_subtle,
            hover_background: palette.fill_selected,
            active_background: palette.fill_selected,
            foreground: palette.text_primary,
            hover_foreground: palette.text_primary,
            opacity: 1.0,
        }
    }
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
        background: mix_color(colors.background, colors.hover_background, hover_progress),
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
        // GPUI suppresses hover while a click is pending; pressed keeps the whole button emphasized.
        let pressed = *pressed.read(cx);
        retarget_hover_motion(
            &hover_transition,
            button_motion_is_emphasized(enabled, *hover, pressed),
            cx,
        );
    });

    let hover_transition = motion.hover_transition.clone();
    let pressed = motion.pressed.clone();
    let button = button.on_mouse_down(MouseButton::Left, move |_, window, cx| {
        if enabled {
            set_button_pressed(&pressed, true, cx);
            retarget_hover_motion(&hover_transition, true, cx);
        }
        button_mouse_down(enabled, window, cx);
    });

    let hover_transition = motion.hover_transition.clone();
    let pressed = motion.pressed.clone();
    let button = button.on_mouse_up(MouseButton::Left, move |_, _window, cx| {
        set_button_pressed(&pressed, false, cx);
        retarget_hover_motion(&hover_transition, enabled, cx);
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

const fn button_motion_is_emphasized(enabled: bool, hovered: bool, pressed: bool) -> bool {
    enabled && (hovered || pressed)
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
        .group(id)
        .min_h(theme::ui_rem(TITLEBAR_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(background)
        .shadow(button_highlight_shadows(palette))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(enabled, |this| this.hover(gpui::Styled::cursor_pointer))
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
    if cfg!(target_os = "macos") { 0.5 } else { 1.0 }
}

pub(super) fn input_highlight_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    let highlight_px = frame_highlight_px();

    if palette.is_light() {
        light_control_depth_shadows(palette, true)
    } else {
        vec![
            BoxShadow {
                color: color(palette.shadow.with_alpha(0.20)).into(),
                offset: point(px(0.0), px(highlight_px)),
                blur_radius: px(0.0),
                spread_radius: px(0.0),
                inset: true,
            },
            BoxShadow {
                color: color(palette.border_subtle).into(),
                offset: point(px(0.0), px(-highlight_px)),
                blur_radius: px(0.0),
                spread_radius: px(0.0),
                inset: true,
            },
        ]
    }
}

pub(super) fn button_highlight_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    if palette.is_light() {
        return light_control_depth_shadows(palette, true);
    }

    let highlight_px = frame_highlight_px();

    vec![
        BoxShadow {
            color: color(palette.border_subtle).into(),
            offset: point(px(0.0), px(highlight_px)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(palette.fill_subtle).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(highlight_px),
            inset: true,
        },
    ]
}

fn light_control_depth_shadows(palette: &theme::ThemePalette, inset: bool) -> Vec<BoxShadow> {
    let highlight_px = frame_highlight_px();

    vec![
        BoxShadow {
            color: color(palette.shadow.with_alpha(0.06)).into(),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(1.0),
            spread_radius: px(-0.5),
            inset,
        },
        BoxShadow {
            color: color(palette.shadow.with_alpha(0.06)).into(),
            offset: point(px(0.0), px(3.0)),
            blur_radius: px(3.0),
            spread_radius: px(-1.5),
            inset,
        },
        BoxShadow {
            color: color(palette.shadow.with_alpha(0.06)).into(),
            offset: point(px(0.0), px(6.0)),
            blur_radius: px(6.0),
            spread_radius: px(-3.0),
            inset,
        },
        BoxShadow {
            color: color(palette.surface.with_alpha(0.08)).into(),
            offset: point(px(0.0), px(-highlight_px)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset,
        },
        BoxShadow {
            color: color(palette.shadow.with_alpha(0.08)).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(highlight_px),
            inset,
        },
    ]
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
            .child(div().h_full().w(px(1.0)).bg(color(palette.fill_subtle)))
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
            .bg(color(if palette.is_light() {
                palette.surface
            } else {
                palette.fill_subtle
            }))
            .shadow(card_surface_shadows(palette))
    }
}

pub(super) fn card_surface_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    let highlight_px = frame_highlight_px();

    if palette.is_light() {
        light_control_depth_shadows(palette, false)
    } else {
        vec![
            BoxShadow {
                color: color(palette.shadow.with_alpha(0.10)).into(),
                offset: point(px(0.0), px(4.0)),
                blur_radius: px(6.0),
                spread_radius: px(-1.0),
                inset: false,
            },
            BoxShadow {
                color: color(palette.shadow.with_alpha(0.10)).into(),
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(4.0),
                spread_radius: px(-2.0),
                inset: false,
            },
            BoxShadow {
                color: color(palette.fill_selected).into(),
                offset: point(px(0.0), px(highlight_px)),
                blur_radius: px(0.0),
                spread_radius: px(0.0),
                inset: true,
            },
            BoxShadow {
                color: color(palette.fill_subtle).into(),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(highlight_px),
                inset: true,
            },
        ]
    }
}

pub(super) fn horizontal_separator_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    let (highlight, offset_y) = if palette.is_light() {
        (palette.border_subtle, frame_highlight_px())
    } else {
        // Dark separators intentionally pair their canvas base with a full-pixel light edge.
        (palette.fill_subtle, 1.0)
    };

    vec![BoxShadow {
        color: color(highlight).into(),
        offset: point(px(0.0), px(offset_y)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

pub(super) fn drop_target_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    let mut shadows = card_surface_shadows(palette);
    shadows.push(BoxShadow {
        color: color(palette.text_muted.with_alpha(0.55)).into(),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(frame_highlight_px()),
        inset: true,
    });
    shadows
}

pub(super) const fn color(token: theme::RgbaToken) -> Rgba {
    Rgba {
        r: token.red,
        g: token.green,
        b: token.blue,
        a: token.alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_motion_emphasis_follows_the_interaction_matrix() {
        let cases = [
            (true, true, false, true),
            (true, false, true, true),
            (true, false, false, false),
            (false, true, true, false),
        ];

        for (enabled, hovered, pressed, expected) in cases {
            assert_eq!(
                button_motion_is_emphasized(enabled, hovered, pressed),
                expected,
                "emphasis mismatch for enabled={enabled}, hovered={hovered}, pressed={pressed}"
            );
        }
    }
}
