//! Frame native visual tokens and resolved color palettes.

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

/// A source color expressed in OKLCH with an alpha channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OklchToken {
    pub lightness: f32,
    pub chroma: f32,
    pub hue: f32,
    pub alpha: f32,
}

impl OklchToken {
    #[must_use]
    pub const fn new(lightness: f32, chroma: f32, hue: f32) -> Self {
        Self {
            lightness,
            chroma,
            hue,
            alpha: 1.0,
        }
    }

    #[must_use]
    pub const fn with_alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }

    #[must_use]
    pub fn to_rgba(self) -> RgbaToken {
        oklch_to_rgba(self)
    }
}

#[derive(Debug)]
struct ThemeDefinition {
    canvas: OklchToken,
    surface: OklchToken,
    surface_elevated: OklchToken,
    text_primary: OklchToken,
    text_muted: OklchToken,
    fill_subtle: OklchToken,
    fill_selected: OklchToken,
    border_subtle: OklchToken,
    control_muted: OklchToken,
    accent: OklchToken,
    danger: OklchToken,
    warning: OklchToken,
    shadow: OklchToken,
    transparent: OklchToken,
}

/// Resolved semantic colors consumed by GPUI rendering code.
#[derive(Debug, PartialEq)]
pub struct ThemePalette {
    color_theme: ColorTheme,
    pub canvas: RgbaToken,
    pub surface: RgbaToken,
    pub surface_elevated: RgbaToken,
    pub text_primary: RgbaToken,
    pub text_muted: RgbaToken,
    pub fill_subtle: RgbaToken,
    pub fill_selected: RgbaToken,
    pub border_subtle: RgbaToken,
    pub control_muted: RgbaToken,
    pub accent: RgbaToken,
    pub danger: RgbaToken,
    pub warning: RgbaToken,
    /// Opaque neutral used as the source color for individually tuned shadow layers.
    pub shadow: RgbaToken,
    pub transparent: RgbaToken,
}

const DARK_CANVAS: OklchToken = OklchToken::new(0.199_806, 0.008_602, 264.360);
const DARK_SURFACE: OklchToken = OklchToken::new(0.230_343, 0.008_292, 264.399);
const DARK_SURFACE_ELEVATED: OklchToken = OklchToken::new(0.272_751, 0.009_683, 268.319);
const DARK_TEXT_PRIMARY: OklchToken = OklchToken::new(0.946_003, 0.026_830, 285.876);

const DARK_DEFINITION: ThemeDefinition = ThemeDefinition {
    canvas: DARK_CANVAS,
    surface: DARK_SURFACE,
    surface_elevated: DARK_SURFACE_ELEVATED,
    text_primary: DARK_TEXT_PRIMARY,
    text_muted: DARK_TEXT_PRIMARY.with_alpha(0.52),
    fill_subtle: DARK_TEXT_PRIMARY.with_alpha(0.05),
    fill_selected: DARK_TEXT_PRIMARY.with_alpha(0.10),
    border_subtle: DARK_TEXT_PRIMARY.with_alpha(0.20),
    control_muted: DARK_TEXT_PRIMARY.with_alpha(0.40),
    accent: OklchToken::new(0.700, 0.170, 264.4),
    danger: OklchToken::new(0.680, 0.180, 27.5),
    warning: OklchToken::new(0.768_590, 0.164_659, 70.080),
    shadow: OklchToken::new(0.0, 0.0, 0.0),
    transparent: OklchToken::new(0.0, 0.0, 0.0).with_alpha(0.0),
};

const LIGHT_TEXT_PRIMARY: OklchToken = OklchToken::new(0.240, 0.020, 264.0);

const LIGHT_DEFINITION: ThemeDefinition = ThemeDefinition {
    canvas: OklchToken::new(0.975, 0.003, 264.0),
    surface: OklchToken::new(1.0, 0.0, 0.0),
    surface_elevated: OklchToken::new(1.0, 0.0, 0.0),
    text_primary: LIGHT_TEXT_PRIMARY,
    text_muted: LIGHT_TEXT_PRIMARY.with_alpha(0.63),
    fill_subtle: OklchToken::new(0.975, 0.003, 264.0),
    fill_selected: OklchToken::new(0.95, 0.003, 264.0),
    border_subtle: LIGHT_TEXT_PRIMARY.with_alpha(0.16),
    control_muted: LIGHT_TEXT_PRIMARY.with_alpha(0.50),
    accent: OklchToken::new(0.488_198, 0.217_165, 264.376),
    danger: OklchToken::new(0.505_420, 0.190_493, 27.518),
    warning: OklchToken::new(0.540, 0.140, 70.0),
    shadow: OklchToken::new(0.0, 0.0, 0.0),
    transparent: OklchToken::new(0.0, 0.0, 0.0).with_alpha(0.0),
};

static DARK_PALETTE: LazyLock<ThemePalette> =
    LazyLock::new(|| ThemePalette::resolve(ColorTheme::Dark, &DARK_DEFINITION));
static LIGHT_PALETTE: LazyLock<ThemePalette> =
    LazyLock::new(|| ThemePalette::resolve(ColorTheme::Light, &LIGHT_DEFINITION));

impl ThemePalette {
    fn resolve(color_theme: ColorTheme, definition: &ThemeDefinition) -> Self {
        Self {
            color_theme,
            canvas: definition.canvas.to_rgba(),
            surface: definition.surface.to_rgba(),
            surface_elevated: definition.surface_elevated.to_rgba(),
            text_primary: definition.text_primary.to_rgba(),
            text_muted: definition.text_muted.to_rgba(),
            fill_subtle: definition.fill_subtle.to_rgba(),
            fill_selected: definition.fill_selected.to_rgba(),
            border_subtle: definition.border_subtle.to_rgba(),
            control_muted: definition.control_muted.to_rgba(),
            accent: definition.accent.to_rgba(),
            danger: definition.danger.to_rgba(),
            warning: definition.warning.to_rgba(),
            shadow: definition.shadow.to_rgba(),
            transparent: definition.transparent.to_rgba(),
        }
    }

    /// Returns the user-facing theme represented by this resolved palette.
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

#[expect(
    clippy::suboptimal_flops,
    reason = "The reference OKLab matrices retain their published operation order for stable cross-build color rounding."
)]
fn oklch_to_rgba(token: OklchToken) -> RgbaToken {
    let lightness = finite_or_zero(token.lightness);
    let chroma = finite_or_zero(token.chroma);
    let hue_radians = finite_or_zero(token.hue).rem_euclid(360.0).to_radians();
    let oklab_a = chroma * hue_radians.cos();
    let oklab_b = chroma * hue_radians.sin();

    let l_root = lightness + 0.396_337_78 * oklab_a + 0.215_803_76 * oklab_b;
    let m_root = lightness - 0.105_561_346 * oklab_a - 0.063_854_17 * oklab_b;
    let s_root = lightness - 0.089_484_18 * oklab_a - 1.291_485_5 * oklab_b;
    let l_linear = l_root * l_root * l_root;
    let m_linear = m_root * m_root * m_root;
    let s_linear = s_root * s_root * s_root;

    let red = 4.076_741_7 * l_linear - 3.307_711_6 * m_linear + 0.230_969_94 * s_linear;
    let green = -1.268_438 * l_linear + 2.609_757_4 * m_linear - 0.341_319_4 * s_linear;
    let blue = -0.004_196_086_3 * l_linear - 0.703_418_6 * m_linear + 1.707_614_7 * s_linear;

    RgbaToken {
        red: linear_to_srgb(red),
        green: linear_to_srgb(green),
        blue: linear_to_srgb(blue),
        alpha: finite_or_zero(token.alpha).clamp(0.0, 1.0),
    }
}

const fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[expect(
    clippy::suboptimal_flops,
    reason = "The standard sRGB transfer function keeps its canonical operation order for stable rounding."
)]
fn linear_to_srgb(channel: f32) -> f32 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
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
    #![expect(
        clippy::float_cmp,
        reason = "Theme tests compare exact color channel constants."
    )]
    #![expect(
        clippy::suboptimal_flops,
        reason = "Contrast helpers mirror the canonical WCAG formulas for auditability."
    )]

    use super::*;

    fn assert_token_matches_srgb(token: OklchToken, red: u8, green: u8, blue: u8) {
        assert_rgba_matches_srgb(token.to_rgba(), red, green, blue);
    }

    fn assert_rgba_matches_srgb(actual: RgbaToken, red: u8, green: u8, blue: u8) {
        let expected = RgbaToken::from_rgb(red, green, blue);
        let tolerance = 0.5 / 255.0;

        assert!((actual.red - expected.red).abs() <= tolerance);
        assert!((actual.green - expected.green).abs() <= tolerance);
        assert!((actual.blue - expected.blue).abs() <= tolerance);
    }

    fn composite(foreground: RgbaToken, background: RgbaToken) -> RgbaToken {
        let alpha = foreground.alpha;
        RgbaToken {
            red: foreground.red * alpha + background.red * (1.0 - alpha),
            green: foreground.green * alpha + background.green * (1.0 - alpha),
            blue: foreground.blue * alpha + background.blue * (1.0 - alpha),
            alpha: 1.0,
        }
    }

    fn relative_luminance(token: RgbaToken) -> f32 {
        fn linear(channel: f32) -> f32 {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * linear(token.red) + 0.7152 * linear(token.green) + 0.0722 * linear(token.blue)
    }

    fn contrast_ratio(foreground: RgbaToken, background: RgbaToken) -> f32 {
        let foreground = composite(foreground, background);
        let foreground_luminance = relative_luminance(foreground);
        let background_luminance = relative_luminance(background);
        let lighter = foreground_luminance.max(background_luminance);
        let darker = foreground_luminance.min(background_luminance);
        (lighter + 0.05) / (darker + 0.05)
    }

    mod rgba_token {
        use super::*;

        #[test]
        fn from_rgb_normalizes_8_bit_channels() {
            let color = RgbaToken::from_rgb(48, 49, 54);

            assert_eq!(
                color,
                RgbaToken {
                    red: 48.0 / 255.0,
                    green: 49.0 / 255.0,
                    blue: 54.0 / 255.0,
                    alpha: 1.0,
                }
            );
        }

        #[test]
        fn with_alpha_preserves_rgb_channels() {
            let base = RgbaToken::from_rgb(235, 235, 255);
            let color = base.with_alpha(0.4);

            assert_eq!(color, RgbaToken { alpha: 0.4, ..base });
        }
    }

    mod oklch_tokens {
        use super::*;

        #[test]
        fn current_dark_solid_colors_round_trip_from_oklch() {
            let references = [
                (DARK_CANVAS, (20, 22, 26)),
                (DARK_TEXT_PRIMARY, (235, 235, 255)),
                (DARK_SURFACE, (27, 29, 33)),
                (DARK_SURFACE_ELEVATED, (37, 39, 44)),
                (
                    OklchToken::new(0.488_198, 0.217_165, 264.376),
                    (29, 78, 216),
                ),
                (OklchToken::new(0.505_420, 0.190_493, 27.518), (185, 28, 28)),
                (
                    OklchToken::new(0.768_590, 0.164_659, 70.080),
                    (245, 158, 11),
                ),
            ];

            for (token, (red, green, blue)) in references {
                assert_token_matches_srgb(token, red, green, blue);
            }
        }

        #[test]
        fn hue_is_normalized_before_conversion() {
            assert_eq!(
                OklchToken::new(0.7, 0.1, -96.0).to_rgba(),
                OklchToken::new(0.7, 0.1, 264.0).to_rgba()
            );
        }

        #[test]
        fn reference_vectors_cover_neutral_and_chromatic_colors() {
            assert_token_matches_srgb(OklchToken::new(1.0, 0.0, 123.0), 255, 255, 255);
            assert_token_matches_srgb(OklchToken::new(0.488_198, 0.217_165, 264.376), 29, 78, 216);
        }

        #[test]
        fn conversion_preserves_alpha_and_clamps_out_of_gamut_channels() {
            let converted = OklchToken::new(0.7, 1.0, 30.0).with_alpha(1.5).to_rgba();

            assert_eq!(converted.alpha, 1.0);
            assert!((0.0..=1.0).contains(&converted.red));
            assert!((0.0..=1.0).contains(&converted.green));
            assert!((0.0..=1.0).contains(&converted.blue));
            assert_eq!(
                OklchToken::new(0.7, 0.1, 30.0)
                    .with_alpha(-0.5)
                    .to_rgba()
                    .alpha,
                0.0
            );
        }

        #[test]
        fn non_finite_input_is_sanitized_before_conversion() {
            let converted = OklchToken {
                lightness: f32::NAN,
                chroma: f32::INFINITY,
                hue: f32::NEG_INFINITY,
                alpha: f32::NAN,
            }
            .to_rgba();

            assert!(converted.red.is_finite());
            assert!(converted.green.is_finite());
            assert!(converted.blue.is_finite());
            assert_eq!(converted.alpha, 0.0);
        }

        #[test]
        fn resolved_palettes_are_cached_per_theme() {
            for color_theme in ColorTheme::ALL {
                assert!(std::ptr::eq(palette(color_theme), palette(color_theme)));
            }
            assert!(!std::ptr::eq(
                palette(ColorTheme::Dark),
                palette(ColorTheme::Light)
            ));
        }

        #[test]
        fn palette_mode_is_resolved_from_value_instead_of_allocation_identity() {
            let independently_resolved =
                ThemePalette::resolve(ColorTheme::Light, &LIGHT_DEFINITION);

            assert_eq!(independently_resolved.color_theme(), ColorTheme::Light);
            assert!(independently_resolved.is_light());
        }

        #[test]
        fn shadow_source_is_opaque_for_every_theme() {
            for color_theme in ColorTheme::ALL {
                assert_eq!(palette(color_theme).shadow.alpha, 1.0);
            }
        }

        #[test]
        fn resolved_solid_roles_match_the_approved_palette_values() {
            let dark = palette(ColorTheme::Dark);
            for (actual, expected) in [
                (dark.canvas, (20, 22, 26)),
                (dark.surface, (27, 29, 33)),
                (dark.surface_elevated, (37, 39, 44)),
                (dark.text_primary, (235, 235, 255)),
                (dark.accent, (104, 153, 255)),
                (dark.danger, (243, 99, 87)),
                (dark.warning, (245, 158, 11)),
            ] {
                assert_rgba_matches_srgb(actual, expected.0, expected.1, expected.2);
            }

            let light = palette(ColorTheme::Light);
            for (actual, expected) in [
                (light.canvas, (246, 247, 249)),
                (light.surface, (255, 255, 255)),
                (light.surface_elevated, (255, 255, 255)),
                (light.text_primary, (26, 31, 41)),
                (light.accent, (29, 78, 216)),
                (light.danger, (185, 28, 28)),
                (light.warning, (161, 92, 0)),
            ] {
                assert_rgba_matches_srgb(actual, expected.0, expected.1, expected.2);
            }
        }

        #[test]
        fn resolved_palettes_contain_only_finite_channels() {
            for color_theme in ColorTheme::ALL {
                let palette = palette(color_theme);
                let tokens = [
                    palette.canvas,
                    palette.surface,
                    palette.surface_elevated,
                    palette.text_primary,
                    palette.text_muted,
                    palette.fill_subtle,
                    palette.fill_selected,
                    palette.border_subtle,
                    palette.control_muted,
                    palette.accent,
                    palette.danger,
                    palette.warning,
                    palette.shadow,
                    palette.transparent,
                ];

                assert!(tokens.into_iter().all(|token| {
                    token.red.is_finite()
                        && token.green.is_finite()
                        && token.blue.is_finite()
                        && token.alpha.is_finite()
                }));
            }
        }
    }

    mod palette_contrast {
        use super::*;

        #[test]
        fn normal_text_roles_meet_wcag_on_every_surface() {
            for color_theme in ColorTheme::ALL {
                let palette = palette(color_theme);
                for surface in [palette.canvas, palette.surface, palette.surface_elevated] {
                    for text in [
                        palette.text_primary,
                        palette.text_muted,
                        palette.danger,
                        palette.warning,
                    ] {
                        let ratio = contrast_ratio(text, surface);
                        assert!(
                            ratio >= 4.5,
                            "{color_theme:?} text contrast was {ratio:.2}:1"
                        );
                    }
                }
            }
        }

        #[test]
        fn focus_accent_meets_non_text_contrast_on_every_surface() {
            for color_theme in ColorTheme::ALL {
                let palette = palette(color_theme);
                for surface in [palette.canvas, palette.surface, palette.surface_elevated] {
                    let ratio = contrast_ratio(palette.accent, surface);
                    assert!(
                        ratio >= 3.0,
                        "{color_theme:?} focus contrast was {ratio:.2}:1"
                    );
                }
            }
        }

        #[test]
        fn state_controls_meet_non_text_contrast_on_every_surface() {
            for color_theme in ColorTheme::ALL {
                let palette = palette(color_theme);
                for surface in [palette.canvas, palette.surface, palette.surface_elevated] {
                    let ratio = contrast_ratio(palette.control_muted, surface);
                    assert!(
                        ratio >= 3.0,
                        "{color_theme:?} control contrast was {ratio:.2}:1"
                    );
                }
            }
        }
    }

    mod typography_tokens {
        use super::*;

        #[test]
        fn ui_text_preserves_natural_case_by_default() {
            assert_eq!(ui_text("Add source"), "Add source");
        }

        #[test]
        fn ui_text_formatter_can_force_uppercase() {
            assert_eq!(format_ui_text("Add source", true), "ADD SOURCE");
        }
    }
}
