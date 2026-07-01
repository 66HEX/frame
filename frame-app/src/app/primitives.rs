use super::{
    App, BoxShadow, Context, ElementId, FluentBuilder, FrameRoot, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Rgba, StatefulInteractiveElement, Styled,
    TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_BUTTON_HEIGHT, TITLEBAR_ICON_BUTTON_SIZE,
    TITLEBAR_ICON_SIZE, Window, div, hover_motion, hsla, mix_color, point, px,
    retarget_hover_motion, svg, theme,
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
    pub(super) hover_transition: gpui::Transition<f32>,
}

pub(super) const fn button_colors(
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
) -> ButtonColors {
    let active_variant = matches!(variant, ButtonVariant::Default) || selected;
    if !enabled {
        let (background, foreground, opacity) = if active_variant {
            (
                theme::FRAME_GRAY_400.with_alpha(0.10),
                theme::FOREGROUND.with_alpha(0.50),
                1.0,
            )
        } else if matches!(variant, ButtonVariant::Ghost) {
            (theme::TRANSPARENT, theme::FRAME_GRAY_600, 0.5)
        } else {
            (
                theme::FRAME_GRAY_100,
                theme::FOREGROUND.with_alpha(0.50),
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
            background: theme::FRAME_GRAY_400,
            hover_background: theme::FRAME_GRAY_400.with_alpha(0.18),
            active_background: theme::FRAME_GRAY_400.with_alpha(0.18),
            foreground: theme::FOREGROUND,
            hover_foreground: theme::FOREGROUND,
            opacity: 1.0,
        }
    } else if matches!(variant, ButtonVariant::Ghost) {
        ButtonColors {
            background: theme::TRANSPARENT,
            hover_background: theme::FRAME_GRAY_100,
            active_background: theme::FRAME_GRAY_200,
            foreground: theme::FRAME_GRAY_600,
            hover_foreground: theme::FOREGROUND,
            opacity: 1.0,
        }
    } else {
        ButtonColors {
            background: theme::FRAME_GRAY_100,
            hover_background: theme::FRAME_GRAY_200,
            active_background: theme::FRAME_GRAY_200,
            foreground: theme::FOREGROUND,
            hover_foreground: theme::FOREGROUND,
            opacity: 1.0,
        }
    }
}

pub(super) fn animated_button_colors(
    id: impl Into<ElementId>,
    colors: ButtonColors,
    window: &mut Window,
    cx: &mut App,
) -> AnimatedButtonColors {
    let hover_transition = hover_motion(id, window, cx);
    let hover_progress = *hover_transition.evaluate(window, cx);
    AnimatedButtonColors {
        background: mix_color(colors.background, colors.hover_background, hover_progress),
        foreground: mix_color(colors.foreground, colors.hover_foreground, hover_progress),
        hover_transition,
    }
}

pub(super) fn button_mouse_down(enabled: bool, window: &mut Window, cx: &mut App) {
    if enabled {
        window.prevent_default();
    } else {
        cx.stop_propagation();
    }
}

pub(super) fn action_button(
    id: impl Into<String>,
    icon: &'static str,
    label: Option<&'static str>,
    variant: ButtonVariant,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let is_icon_only = label.is_none();
    let colors = button_colors(variant, false, enabled);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;

    let button = div()
        .id(id.clone())
        .group(id)
        .h(px(TITLEBAR_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .bg(background)
        .shadow(button_highlight_shadows())
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(enabled, |this| this.hover(gpui::Styled::cursor_pointer))
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        });

    if is_icon_only {
        button.w(px(TITLEBAR_ICON_BUTTON_SIZE)).child(icon_svg(
            icon,
            TITLEBAR_ACTION_ICON_SIZE,
            foreground,
        ))
    } else {
        button
            .px(px(10.0))
            .child(icon_svg(icon, TITLEBAR_ICON_SIZE, foreground))
            .child(theme::ui_text(label.unwrap_or_default()))
    }
}

pub(super) fn icon_svg(path: &'static str, size: f32, icon_color: Rgba) -> impl IntoElement {
    svg()
        .path(path)
        .w(px(size))
        .h(px(size))
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

pub(super) fn input_highlight_shadows() -> Vec<BoxShadow> {
    let highlight_px = frame_highlight_px();

    vec![
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.20),
            offset: point(px(0.0), px(highlight_px)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_400).into(),
            offset: point(px(0.0), px(-highlight_px)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
    ]
}

pub(super) fn button_highlight_shadows() -> Vec<BoxShadow> {
    let highlight_px = frame_highlight_px();

    vec![
        BoxShadow {
            color: color(theme::FRAME_GRAY_400).into(),
            offset: point(px(0.0), px(highlight_px)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_200).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(highlight_px),
            inset: true,
        },
    ]
}

pub(super) fn vertical_separator(height: f32) -> gpui::Div {
    div()
        .flex()
        .h(px(height))
        .w(px(2.0))
        .child(div().h_full().w(px(1.0)).bg(color(theme::BACKGROUND)))
        .child(div().h_full().w(px(1.0)).bg(color(theme::FRAME_GRAY_100)))
}

pub(super) fn panel_bottom_separator() -> gpui::Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(px(1.0))
        .bg(color(theme::BACKGROUND))
        .shadow(horizontal_separator_shadows())
}

pub(super) fn element_id(prefix: &str, id: &str) -> String {
    format!("{prefix}-{id}")
}

pub(super) trait FrameSurface {
    fn card_surface(self) -> Self;
}

impl FrameSurface for gpui::Div {
    fn card_surface(self) -> Self {
        self.rounded(px(theme::RADIUS_LG))
            .bg(color(theme::FRAME_GRAY_100))
            .shadow(card_surface_shadows())
    }
}

pub(super) fn card_surface_shadows() -> Vec<BoxShadow> {
    let highlight_px = frame_highlight_px();

    vec![
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.10),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(6.0),
            spread_radius: px(-1.0),
            inset: false,
        },
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.10),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(4.0),
            spread_radius: px(-2.0),
            inset: false,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_200).into(),
            offset: point(px(0.0), px(highlight_px)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_100).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(highlight_px),
            inset: true,
        },
    ]
}

pub(super) fn horizontal_separator_shadows() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color(theme::FRAME_GRAY_100).into(),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

pub(super) fn drop_target_shadows() -> Vec<BoxShadow> {
    let mut shadows = card_surface_shadows();
    shadows.push(BoxShadow {
        color: color(theme::FRAME_GRAY_600.with_alpha(0.55)).into(),
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
