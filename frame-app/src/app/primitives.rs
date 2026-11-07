use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{ParentElement, Styled};
use gpui::{App, BoxShadow, Context, IntoElement, Rgba, Window, div, point, px, svg};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Expose every variant in the pinned Mira button contract, including link actions."
    )
)]
pub(super) enum ButtonVariant {
    Primary,
    Secondary,
    Accent,
    Outline,
    Ghost,
    Destructive,
    Link,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ButtonColors {
    pub(super) background: theme::RgbaToken,
    pub(super) hover_background: theme::RgbaToken,
    pub(super) foreground: theme::RgbaToken,
    pub(super) hover_foreground: theme::RgbaToken,
    pub(super) opacity: f32,
}

pub(super) const fn button_colors(
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
) -> ButtonColors {
    let (background, hover_background, foreground) = if selected {
        (
            palette.fill_selected,
            palette.fill_selected,
            palette.text_primary,
        )
    } else {
        match variant {
            ButtonVariant::Primary => (
                palette.primary,
                palette.primary_hover,
                palette.primary_foreground,
            ),
            ButtonVariant::Secondary => (
                palette.fill_subtle,
                palette.secondary_hover,
                palette.secondary_foreground,
            ),
            ButtonVariant::Accent => (
                palette.fill_selected,
                palette.accent_hover,
                palette.accent_foreground,
            ),
            ButtonVariant::Outline => (
                palette.fill_subtle,
                palette.secondary_hover,
                palette.text_primary,
            ),
            ButtonVariant::Ghost => (
                palette.transparent,
                palette.ghost_hover,
                palette.text_primary,
            ),
            ButtonVariant::Destructive => (
                palette.danger_surface,
                palette
                    .danger
                    .with_opacity(if palette.is_light() { 0.15 } else { 0.1 }),
                palette.danger,
            ),
            ButtonVariant::Link => (palette.transparent, palette.transparent, palette.primary),
        }
    };
    ButtonColors {
        background,
        hover_background: if enabled {
            hover_background
        } else {
            background
        },
        foreground,
        hover_foreground: foreground,
        opacity: if enabled { 1.0 } else { 0.5 },
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
    let foreground = color(button_colors(variant, false, enabled, palette).foreground);
    let button = crate::app::components::frame_button_base(
        id.clone(),
        &accessibility_label,
        variant,
        false,
        enabled,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    );
    let icon = crate::app::components::frame_animated_icon(
        id,
        Some(icon),
        theme::metrics::CONTROL_ICON,
        Some(foreground),
        window,
        cx,
    );
    if let Some(label) = label {
        button
            .pl(theme::ui_rem(6.0))
            .child(icon)
            .child(theme::ui_text(label))
    } else {
        button
            .w(theme::ui_rem(theme::metrics::CONTROL_HEIGHT))
            .p_0()
            .child(icon)
    }
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
    div()
        .h(theme::ui_rem(height))
        .w(px(frame_highlight_px()))
        .bg(color(palette.border_subtle))
}

pub(super) fn panel_bottom_separator(palette: &theme::ThemePalette) -> gpui::Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(px(1.0))
        .bg(color(palette.border_subtle))
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
            .bg(theme::shadows::surface_fill(palette.surface, palette))
            .shadow(theme::shadows::ShadowLevel::Sm.shadows())
    }
}

pub(super) fn horizontal_separator_shadows(palette: &theme::ThemePalette) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color(palette.border_subtle).into(),
        offset: point(px(0.0), px(1.0)),
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
    fn mira_variants_preserve_colors_when_disabled_and_use_flat_fills() {
        for mode in crate::appearance::ColorTheme::ALL {
            let p = theme::palette(mode);
            for variant in [
                ButtonVariant::Primary,
                ButtonVariant::Secondary,
                ButtonVariant::Accent,
                ButtonVariant::Outline,
                ButtonVariant::Ghost,
                ButtonVariant::Destructive,
                ButtonVariant::Link,
            ] {
                let enabled = button_colors(variant, false, true, p);
                let disabled = button_colors(variant, false, false, p);
                assert_eq!(enabled.background, disabled.background);
                assert_eq!(enabled.foreground, disabled.foreground);
                assert!((disabled.opacity - 0.5).abs() < f32::EPSILON);
                assert_eq!(disabled.background, disabled.hover_background);
            }
        }
    }
}
