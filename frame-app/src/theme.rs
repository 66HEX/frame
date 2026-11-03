//! Frame native visual tokens and resolved color palettes.

pub mod geist;
pub mod shadows;

use std::sync::LazyLock;

use gpui::{FontWeight, Rems, rems};

use crate::appearance::BASE_REM_PX;
use crate::appearance::ColorTheme;

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
}

/// Semantic roles selected from the exact Geist sRGB palette.
/// See docs/colors/README.md for the shadcn hierarchy and component assignments.
#[derive(Debug, PartialEq)]
pub struct ThemePalette {
    color_theme: ColorTheme,
    pub background: RgbaToken,
    /// Recessed input, checkbox, slider and infinite-canvas background.
    pub canvas: RgbaToken,
    pub surface: RgbaToken,
    pub surface_elevated: RgbaToken,
    pub tooltip_surface: RgbaToken,
    pub tooltip_foreground: RgbaToken,
    pub text_primary: RgbaToken,
    pub text_muted: RgbaToken,
    pub text_disabled: RgbaToken,
    pub fill_subtle: RgbaToken,
    pub fill_selected: RgbaToken,
    pub fill_strong: RgbaToken,
    pub highlight_subtle: RgbaToken,
    pub border_subtle: RgbaToken,
    pub control_muted: RgbaToken,
    pub accent: RgbaToken,
    pub focus_ring: RgbaToken,
    pub text_selection: RgbaToken,
    pub danger: RgbaToken,
    pub danger_surface: RgbaToken,
    pub warning: RgbaToken,
    pub success: RgbaToken,
    pub overlay_backdrop: RgbaToken,
    pub primary: RgbaToken,
    pub primary_to: RgbaToken,
    pub primary_hover: RgbaToken,
    pub primary_hover_to: RgbaToken,
    pub primary_active: RgbaToken,
    pub primary_active_to: RgbaToken,
    pub primary_foreground: RgbaToken,
    pub transparent: RgbaToken,
}

static DARK_PALETTE: LazyLock<ThemePalette> =
    LazyLock::new(|| ThemePalette::resolve(ColorTheme::Dark));
static LIGHT_PALETTE: LazyLock<ThemePalette> =
    LazyLock::new(|| ThemePalette::resolve(ColorTheme::Light));

impl ThemePalette {
    fn resolve(color_theme: ColorTheme) -> Self {
        let get = |name| geist::color(color_theme, name);
        let pick = |light, dark| {
            get(if matches!(color_theme, ColorTheme::Light) {
                light
            } else {
                dark
            })
        };
        Self {
            color_theme,
            background: get("ds-background-100"),
            canvas: pick("ds-background-200", "ds-background-100"),
            surface: pick("ds-background-100", "ds-gray-100"),
            surface_elevated: pick("ds-background-100", "ds-gray-100"),
            tooltip_surface: get("ds-gray-1000"),
            tooltip_foreground: get("ds-background-100"),
            text_primary: get("ds-gray-1000"),
            text_muted: get("ds-gray-900"),
            text_disabled: get("ds-gray-700"),
            fill_subtle: pick("ds-gray-100", "ds-gray-200"),
            fill_selected: pick("ds-gray-200", "ds-gray-300"),
            fill_strong: pick("ds-gray-300", "ds-gray-400"),
            highlight_subtle: get("ds-gray-alpha-100"),
            border_subtle: get("ds-gray-400"),
            control_muted: get("ds-gray-700"),
            accent: get("ds-blue-900"),
            focus_ring: get("ds-focus-color"),
            text_selection: get("ds-blue-200"),
            danger: get("ds-red-900"),
            danger_surface: get("ds-red-100"),
            warning: get("ds-amber-900"),
            success: get("ds-green-900"),
            overlay_backdrop: get("ds-overlay-backdrop-color"),
            // Graphite raised action: the darker fill lets the shared xs
            // inset edge and top highlight remain visible in both themes.
            primary: pick("accents-6", "ds-gray-500"),
            primary_to: pick("ds-gray-1000", "ds-gray-400"),
            primary_hover: pick("accents-5", "accents-4"),
            primary_hover_to: pick("accents-6", "ds-gray-500"),
            primary_active: pick("accents-7", "ds-gray-400"),
            primary_active_to: pick("ds-gray-1000", "ds-gray-200"),
            primary_foreground: get("ds-contrast-fg"),
            transparent: get("geist-background").with_alpha(0.0),
        }
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

pub const RADIUS_BASE: f32 = 3.6;
pub const RADIUS_XS: f32 = RADIUS_BASE;
pub const RADIUS_SM: f32 = RADIUS_BASE * 2.0;
pub const RADIUS_MD: f32 = RADIUS_BASE * 3.0;
pub const RADIUS_LG: f32 = RADIUS_BASE * 4.0;
pub const RADIUS_XL: f32 = RADIUS_BASE * 6.0;

pub const TEXT_UI_BASE_SIZE: f32 = 12.0;
pub const TEXT_ROW_BASE_SIZE: f32 = 14.0;
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
    fn surface_hierarchy_matches_the_documented_shadcn_adaptation() {
        let dark = palette(ColorTheme::Dark);
        assert!(luminance(dark.background) < luminance(dark.surface));
        assert!(luminance(dark.surface) < luminance(dark.fill_subtle));
        assert!(luminance(dark.fill_subtle) < luminance(dark.fill_selected));
        assert!(luminance(dark.fill_selected) < luminance(dark.fill_strong));
        let light = palette(ColorTheme::Light);
        assert_eq!(light.background, light.surface);
        assert!(luminance(light.surface) > luminance(light.fill_subtle));
        assert!(luminance(light.fill_subtle) > luminance(light.fill_selected));
        assert!(luminance(light.fill_selected) > luminance(light.fill_strong));
        for mode in ColorTheme::ALL {
            let p = palette(mode);
            assert_eq!(p.surface, p.surface_elevated);
            assert!(luminance(p.canvas) < luminance(p.surface));
            for token in [
                p.background,
                p.canvas,
                p.surface,
                p.surface_elevated,
                p.fill_subtle,
                p.fill_selected,
                p.fill_strong,
            ] {
                assert_eq!(token.alpha, 1.0);
                assert_eq!(token.red, token.green);
                assert_eq!(token.green, token.blue);
            }
        }
    }

    #[test]
    fn semantic_text_remains_readable_on_surfaces_and_control_states() {
        for mode in ColorTheme::ALL {
            let p = palette(mode);
            for bg in [
                p.background,
                p.canvas,
                p.surface,
                p.surface_elevated,
                p.fill_subtle,
                p.fill_selected,
                p.fill_strong,
            ] {
                for fg in [p.text_primary, p.text_muted] {
                    assert!(contrast(fg, bg) >= 4.5, "{mode:?}: text {fg:?} on {bg:?}");
                }
            }
            for bg in [p.background, p.canvas, p.surface, p.surface_elevated] {
                for fg in [p.accent, p.danger, p.warning, p.success] {
                    assert!(contrast(fg, bg) >= 4.5, "{mode:?}: status {fg:?} on {bg:?}");
                }
                for fg in [p.control_muted, p.focus_ring] {
                    assert!(
                        contrast(fg, bg) >= 3.0,
                        "{mode:?}: control {fg:?} on {bg:?}"
                    );
                }
            }
            assert!(contrast(p.danger, p.danger_surface) >= 4.5);
            assert!(contrast(p.tooltip_foreground, p.tooltip_surface) >= 4.5);
            assert!(contrast(p.text_primary, p.text_selection) >= 4.5);
        }
    }

    #[test]
    fn primary_gradient_is_neutral_opaque_lighter_at_top_and_readable_in_all_states() {
        for mode in ColorTheme::ALL {
            let p = palette(mode);
            for (top, bottom) in [
                (p.primary, p.primary_to),
                (p.primary_hover, p.primary_hover_to),
                (p.primary_active, p.primary_active_to),
            ] {
                assert!(
                    luminance(top) > luminance(bottom),
                    "{mode:?}: gradient direction"
                );
                // Neutral interpolation is monotonic in sRGB and OKLab, so the
                // least contrasting endpoint bounds the whole gradient.
                for stop in [top, bottom] {
                    assert_eq!(stop.alpha, 1.0);
                    assert_eq!(stop.red, stop.green);
                    assert_eq!(stop.green, stop.blue);
                    assert!(contrast(p.primary_foreground, stop) >= 4.5);
                }
            }
        }
    }

    #[test]
    fn ui_text_preserves_case_unless_explicitly_uppercase() {
        assert_eq!(ui_text("Add source"), "Add source");
        assert_eq!(format_ui_text("Add source", true), "ADD SOURCE");
    }
}
