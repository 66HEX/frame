//! Frame native visual tokens and resolved color palettes.

pub mod metrics;
pub mod mira;
pub mod shadows;

#[cfg(test)]
mod css;

use crate::appearance::{BASE_REM_PX, ColorTheme};
use gpui::{FontWeight, Rems, rems};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RgbaToken {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl RgbaToken {
    #[must_use]
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red: red as f32 / 255.0,
            green: green as f32 / 255.0,
            blue: blue as f32 / 255.0,
            alpha: 1.0,
        }
    }

    #[must_use]
    pub const fn with_alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }

    /// CSS color/opacity multiplies an existing alpha instead of replacing it.
    #[must_use]
    pub const fn with_opacity(self, opacity: f32) -> Self {
        self.with_alpha(self.alpha * opacity)
    }
}

/// Graphite surface roles. Source token names are available through `token`.
/// Domain aliases below keep preview and conversion code independent of CSS.
#[derive(Debug, PartialEq)]
pub struct ThemePalette {
    color_theme: ColorTheme,
    pub background: RgbaToken,
    /// Infinite-canvas background.
    pub canvas: RgbaToken,
    pub surface: RgbaToken,
    pub surface_elevated: RgbaToken,
    pub tooltip_surface: RgbaToken,
    pub tooltip_foreground: RgbaToken,
    pub text_primary: RgbaToken,
    pub text_muted: RgbaToken,
    pub fill_subtle: RgbaToken,
    pub fill_selected: RgbaToken,
    pub fill_strong: RgbaToken,
    pub border_subtle: RgbaToken,
    pub accent: RgbaToken,
    pub focus_ring: RgbaToken,
    pub text_selection: RgbaToken,
    pub danger: RgbaToken,
    pub danger_surface: RgbaToken,
    pub warning: RgbaToken,
    pub success: RgbaToken,
    pub overlay_backdrop: RgbaToken,
    pub primary: RgbaToken,
    pub primary_hover: RgbaToken,
    pub primary_foreground: RgbaToken,
    pub transparent: RgbaToken,
    pub input: RgbaToken,
    pub input_background: RgbaToken,
    pub input_hover: RgbaToken,
    pub secondary_hover: RgbaToken,
    pub secondary_foreground: RgbaToken,
    pub ghost_hover: RgbaToken,
    pub surface_ring: RgbaToken,
    pub slider_thumb: RgbaToken,
    pub tab_active: RgbaToken,
    pub tab_border: RgbaToken,
}

static DARK_PALETTE: LazyLock<ThemePalette> =
    LazyLock::new(|| ThemePalette::resolve(ColorTheme::Dark));
static LIGHT_PALETTE: LazyLock<ThemePalette> =
    LazyLock::new(|| ThemePalette::resolve(ColorTheme::Light));

impl ThemePalette {
    #[must_use]
    pub const fn is_dark(&self) -> bool {
        matches!(self.color_theme, ColorTheme::Dark)
    }

    fn resolve(color_theme: ColorTheme) -> Self {
        let get = |name| mira::color(color_theme, name);
        let light = matches!(color_theme, ColorTheme::Light);
        let input = get("input");
        let foreground = get("foreground");
        let primary = get("primary");
        let destructive = get("destructive");
        Self {
            color_theme,
            background: get("background"),
            canvas: get("background"),
            surface: get("card"),
            surface_elevated: get("popover"),
            tooltip_surface: get("popover"),
            tooltip_foreground: foreground,
            text_primary: foreground,
            text_muted: get("muted-foreground"),
            fill_subtle: get("secondary"),
            fill_selected: get("accent"),
            fill_strong: get("muted"),
            border_subtle: get("border"),
            // Shadcn has no info/warning roles. Neutral status text is
            // paired with the existing explicit status label and action icon.
            accent: foreground,
            focus_ring: get("ring"),
            text_selection: primary.with_alpha(primary.alpha * 0.20),
            danger: destructive,
            danger_surface: destructive
                .with_alpha(destructive.alpha * if light { 0.1 } else { 0.08 }),
            warning: foreground,
            // Optional extension; standard shadcn themes retain neutral feedback.
            success: if mira::colors(color_theme).contains_key("success") {
                get("success")
            } else {
                foreground
            },
            overlay_backdrop: RgbaToken::from_rgb(0, 0, 0).with_alpha(0.8),
            primary,
            primary_hover: primary.with_alpha(primary.alpha * 0.8),
            primary_foreground: get("primary-foreground"),
            transparent: foreground.with_alpha(0.0),
            input,
            input_background: get("background"),
            input_hover: get("muted"),
            secondary_hover: get("secondary-hover"),
            secondary_foreground: get("secondary-foreground"),
            // A light overlay stays visible over the top-lit dark card. Mixing
            // muted/50 into that surface can match the card and hide open rows.
            ghost_hover: if light {
                get("muted")
            } else {
                foreground.with_opacity(0.08)
            },
            surface_ring: foreground.with_alpha(foreground.alpha * 0.1),
            slider_thumb: if light { get("popover") } else { primary },
            tab_active: get("accent"),
            tab_border: foreground.with_alpha(0.0),
        }
    }

    /// Exact shadcn CSS color token, without the `--` prefix.
    #[must_use]
    pub fn token(&self, name: &str) -> RgbaToken {
        mira::color(self.color_theme, name)
    }

    #[must_use]
    pub const fn color_theme(&self) -> ColorTheme {
        self.color_theme
    }

    #[must_use]
    pub(crate) const fn is_light(&self) -> bool {
        matches!(self.color_theme, ColorTheme::Light)
    }
}

/// Returns the immutable resolved palette for a user-selected theme.
#[must_use]
pub fn palette(color_theme: ColorTheme) -> &'static ThemePalette {
    match color_theme {
        ColorTheme::Dark => &DARK_PALETTE,
        ColorTheme::Light => &LIGHT_PALETTE,
    }
}

include!(concat!(env!("OUT_DIR"), "/theme-radius.rs"));
pub const RADIUS_XS: f32 = 4.0;
// Subtract the existing inset: changing corner styling must never change spacing.
pub const RADIUS_SM: f32 = nested_radius(RADIUS_BASE, 8.0);
pub const RADIUS_MD: f32 = nested_radius(RADIUS_BASE, metrics::MENU_PADDING);
pub const RADIUS_LG: f32 = RADIUS_BASE;
pub const RADIUS_XL: f32 = RADIUS_BASE + 4.0;
pub const RADIUS_2XL: f32 = RADIUS_BASE + 8.0;
pub const RADIUS_3XL: f32 = RADIUS_BASE + 12.0;
pub const RADIUS_4XL: f32 = RADIUS_BASE + 16.0;

const fn nested_radius(outer: f32, inset: f32) -> f32 {
    if outer > inset { outer - inset } else { 0.0 }
}

pub const RANGE_TRACK_HEIGHT: f32 = 4.0;
pub const RANGE_TRACK_INSET: f32 = metrics::SLIDER_THUMB / 2.0;

pub const TEXT_UI_BASE_SIZE: f32 = 12.0;
pub const TEXT_ROW_BASE_SIZE: f32 = 12.0;
pub const TEXT_MARKDOWN_BASE_SIZE: f32 = 14.0;
pub const TEXT_MARKDOWN_LIST_BASE_SIZE: f32 = 12.0;
pub const TEXT_INPUT_CARET_BASE_HEIGHT: f32 = 14.0;
pub const TEXT_WEIGHT_REGULAR: FontWeight = FontWeight::NORMAL;
pub const TEXT_WEIGHT_MEDIUM: FontWeight = FontWeight::MEDIUM;
pub const FORCE_UPPERCASE_UI_TEXT: bool = false;
pub const MIN_HIT_AREA: f32 = 40.0;

/// Converts a base Frame logical-pixel token into a scalable rem length.
#[must_use]
pub const fn ui_rem(base_pixels: f32) -> Rems {
    rems(base_pixels / BASE_REM_PX)
}

#[must_use]
pub fn ui_text(text: &str) -> String {
    format_ui_text(text, FORCE_UPPERCASE_UI_TEXT)
}

#[must_use]
pub fn ui_text_owned(text: String) -> String {
    if FORCE_UPPERCASE_UI_TEXT {
        text.to_uppercase()
    } else {
        text
    }
}

fn format_ui_text(text: &str, uppercase: bool) -> String {
    if uppercase {
        text.to_uppercase()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::float_cmp, reason = "Tokens are exact cached source values.")]
    #![expect(
        clippy::suboptimal_flops,
        reason = "Contrast helpers follow the published WCAG formula."
    )]

    use super::*;

    fn luminance(token: RgbaToken) -> f32 {
        fn linear(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(token.red) + 0.7152 * linear(token.green) + 0.0722 * linear(token.blue)
    }

    fn contrast(fg: RgbaToken, bg: RgbaToken) -> f32 {
        let a = luminance(fg);
        let b = luminance(bg);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn rgba_channels_and_alpha_are_independent() {
        let token = RgbaToken::from_rgb(48, 49, 54);
        assert_eq!(token.red, 48.0 / 255.0);
        assert_eq!(token.green, 49.0 / 255.0);
        assert_eq!(token.blue, 54.0 / 255.0);
        assert_eq!(
            token.with_alpha(0.4),
            RgbaToken {
                alpha: 0.4,
                ..token
            }
        );
    }

    #[test]
    fn palettes_are_cached_but_mode_does_not_depend_on_pointer_identity() {
        for mode in ColorTheme::ALL {
            assert!(std::ptr::eq(palette(mode), palette(mode)));
            assert_eq!(ThemePalette::resolve(mode).color_theme(), mode);
        }
        assert!(ThemePalette::resolve(ColorTheme::Light).is_light());
    }

    #[test]
    fn native_roles_preserve_mira_semantics_and_text_contrast() {
        for mode in ColorTheme::ALL {
            let p = palette(mode);
            assert_eq!(p.background, p.token("background"));
            assert_eq!(p.surface, p.token("card"));
            assert_eq!(p.surface_elevated, p.token("popover"));
            assert_eq!(p.fill_selected, p.token("accent"));
            assert_eq!(p.fill_subtle, p.token("secondary"));
            assert_eq!(p.focus_ring, p.token("ring"));
            assert_eq!(p.input, p.token("input"));
            for bg in [p.background, p.surface, p.surface_elevated] {
                assert!(contrast(p.text_primary, bg) >= 4.5);
            }
            assert!(contrast(p.primary_foreground, p.primary) >= 4.5);
            assert!(contrast(p.tooltip_foreground, p.tooltip_surface) >= 4.5);
        }
    }

    #[test]
    fn ui_text_preserves_case_unless_explicitly_uppercase() {
        assert_eq!(ui_text("Add source"), "Add source");
        assert_eq!(format_ui_text("Add source", true), "ADD SOURCE");
    }
}
