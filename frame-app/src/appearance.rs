//! Runtime appearance settings shared by persistence and the GPUI application.

/// The base size of one Frame rem at 100% UI scale.
pub const BASE_REM_PX: f32 = 16.0;

/// A user-selectable application color theme.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorTheme {
    #[default]
    Dark,
    Light,
}

impl ColorTheme {
    /// All supported themes in user-facing order.
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];

    /// Returns the label shown in appearance controls.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    /// Returns the canonical value stored in settings JSON.
    #[must_use]
    pub const fn persisted(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// Parses a persisted value, defaulting safely for missing or future values.
    #[must_use]
    pub fn from_persisted(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("light") => Self::Light,
            _ => Self::Dark,
        }
    }
}

/// Parses a fixture-only color theme override.
#[must_use]
pub fn color_theme_from_env_value(value: Option<&str>) -> Option<ColorTheme> {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("dark") => Some(ColorTheme::Dark),
        Some("light") => Some(ColorTheme::Light),
        _ => None,
    }
}

/// A supported user-facing scale percentage.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum ScalePreset {
    Percent80,
    Percent90,
    #[default]
    Percent100,
    Percent110,
    Percent125,
    Percent150,
    Percent175,
    Percent200,
}

impl ScalePreset {
    /// All supported presets in ascending order.
    pub const ALL: [Self; 8] = [
        Self::Percent80,
        Self::Percent90,
        Self::Percent100,
        Self::Percent110,
        Self::Percent125,
        Self::Percent150,
        Self::Percent175,
        Self::Percent200,
    ];

    /// Returns the integer percentage represented by this preset.
    #[must_use]
    pub const fn percent(self) -> u16 {
        match self {
            Self::Percent80 => 80,
            Self::Percent90 => 90,
            Self::Percent100 => 100,
            Self::Percent110 => 110,
            Self::Percent125 => 125,
            Self::Percent150 => 150,
            Self::Percent175 => 175,
            Self::Percent200 => 200,
        }
    }

    /// Returns the multiplier represented by this preset.
    #[must_use]
    pub const fn factor(self) -> f32 {
        self.percent() as f32 / 100.0
    }

    /// Returns the label shown in appearance controls.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            Self::Percent80 => "80%",
            Self::Percent90 => "90%",
            Self::Percent100 => "100%",
            Self::Percent110 => "110%",
            Self::Percent125 => "125%",
            Self::Percent150 => "150%",
            Self::Percent175 => "175%",
            Self::Percent200 => "200%",
        }
    }

    /// Returns the next supported preset, clamped at 200%.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Percent80 => Self::Percent90,
            Self::Percent90 => Self::Percent100,
            Self::Percent100 => Self::Percent110,
            Self::Percent110 => Self::Percent125,
            Self::Percent125 => Self::Percent150,
            Self::Percent150 => Self::Percent175,
            Self::Percent175 | Self::Percent200 => Self::Percent200,
        }
    }

    /// Returns the previous supported preset, clamped at 80%.
    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Percent80 | Self::Percent90 => Self::Percent80,
            Self::Percent100 => Self::Percent90,
            Self::Percent110 => Self::Percent100,
            Self::Percent125 => Self::Percent110,
            Self::Percent150 => Self::Percent125,
            Self::Percent175 => Self::Percent150,
            Self::Percent200 => Self::Percent175,
        }
    }

    /// Converts a persisted percentage into a supported preset.
    #[must_use]
    pub const fn from_percent(percent: u16) -> Option<Self> {
        match percent {
            80 => Some(Self::Percent80),
            90 => Some(Self::Percent90),
            100 => Some(Self::Percent100),
            110 => Some(Self::Percent110),
            125 => Some(Self::Percent125),
            150 => Some(Self::Percent150),
            175 => Some(Self::Percent175),
            200 => Some(Self::Percent200),
            _ => None,
        }
    }
}

#[must_use]
pub fn scale_preset_from_env_value(value: Option<&str>) -> Option<ScalePreset> {
    value
        .map(str::trim)
        .and_then(|value| value.trim_end_matches('%').parse::<u16>().ok())
        .and_then(ScalePreset::from_percent)
}

/// User-configurable appearance settings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AppearanceSettings {
    pub ui_scale: ScalePreset,
    pub color_theme: ColorTheme,
}

#[must_use]
pub const fn resolved_ui_pixels(base_pixels: f32, ui_scale: ScalePreset) -> f32 {
    base_pixels * ui_scale.factor()
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::float_cmp,
        reason = "Scale model tests compare exact products of supported decimal presets."
    )]

    use super::*;

    #[test]
    fn default_appearance_uses_unscaled_ui() {
        assert_eq!(
            AppearanceSettings::default(),
            AppearanceSettings {
                ui_scale: ScalePreset::Percent100,
                color_theme: ColorTheme::Dark,
            }
        );
    }

    #[test]
    fn color_theme_options_use_the_user_facing_order() {
        assert_eq!(ColorTheme::ALL, [ColorTheme::Dark, ColorTheme::Light]);
    }

    #[test]
    fn persisted_color_theme_parser_defaults_unknown_values_to_dark() {
        assert_eq!(ColorTheme::from_persisted(Some("future")), ColorTheme::Dark);
        assert_eq!(ColorTheme::from_persisted(None), ColorTheme::Dark);
        assert_eq!(ColorTheme::from_persisted(Some(" DARK ")), ColorTheme::Dark);
        assert_eq!(
            ColorTheme::from_persisted(Some(" LIGHT ")),
            ColorTheme::Light
        );
    }

    #[test]
    fn visual_theme_parser_accepts_light_case_insensitively() {
        assert_eq!(
            color_theme_from_env_value(Some(" LIGHT ")),
            Some(ColorTheme::Light)
        );
        assert_eq!(
            color_theme_from_env_value(Some(" DARK ")),
            Some(ColorTheme::Dark)
        );
        assert_eq!(color_theme_from_env_value(Some("system")), None);
        assert_eq!(color_theme_from_env_value(None), None);
    }

    #[test]
    fn all_presets_are_sorted_by_percentage() {
        assert!(
            ScalePreset::ALL
                .windows(2)
                .all(|presets| presets[0].percent() < presets[1].percent())
        );
    }

    #[test]
    fn every_preset_has_the_expected_public_representation() {
        let expected = [
            (ScalePreset::Percent80, 80, 0.8, "80%"),
            (ScalePreset::Percent90, 90, 0.9, "90%"),
            (ScalePreset::Percent100, 100, 1.0, "100%"),
            (ScalePreset::Percent110, 110, 1.1, "110%"),
            (ScalePreset::Percent125, 125, 1.25, "125%"),
            (ScalePreset::Percent150, 150, 1.5, "150%"),
            (ScalePreset::Percent175, 175, 1.75, "175%"),
            (ScalePreset::Percent200, 200, 2.0, "200%"),
        ];

        for (preset, percent, factor, display) in expected {
            assert_eq!(preset.percent(), percent);
            assert_eq!(preset.factor(), factor);
            assert_eq!(preset.display(), display);
            assert_eq!(ScalePreset::from_percent(percent), Some(preset));
        }
    }

    #[test]
    fn next_and_previous_follow_the_complete_preset_sequence() {
        for presets in ScalePreset::ALL.windows(2) {
            assert_eq!(presets[0].next(), presets[1]);
            assert_eq!(presets[1].previous(), presets[0]);
        }
    }

    #[test]
    fn next_and_previous_clamp_at_the_supported_bounds() {
        for (actual, expected, boundary) in [
            (
                ScalePreset::Percent200.next(),
                ScalePreset::Percent200,
                "maximum",
            ),
            (
                ScalePreset::Percent80.previous(),
                ScalePreset::Percent80,
                "minimum",
            ),
        ] {
            assert_eq!(actual, expected, "preset did not clamp at {boundary}");
        }
    }

    #[test]
    fn from_percent_rejects_unsupported_value() {
        assert_eq!(ScalePreset::from_percent(120), None);
    }

    #[test]
    fn visual_fixture_scale_parser_accepts_percent_or_plain_number() {
        assert_eq!(
            scale_preset_from_env_value(Some("150%")),
            Some(ScalePreset::Percent150)
        );
        assert_eq!(
            scale_preset_from_env_value(Some(" 80 ")),
            Some(ScalePreset::Percent80)
        );
    }

    #[test]
    fn visual_fixture_scale_parser_rejects_unknown_or_missing_value() {
        assert_eq!(scale_preset_from_env_value(Some("120")), None);
        assert_eq!(scale_preset_from_env_value(None), None);
    }

    #[test]
    fn factor_matches_percentage() {
        assert!((ScalePreset::Percent125.factor() - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn ui_scale_resolves_geometry_and_rem_based_text_consistently() {
        for (ui_scale, expected) in [
            (ScalePreset::Percent80, 12.8),
            (ScalePreset::Percent100, 16.0),
            (ScalePreset::Percent150, 24.0),
            (ScalePreset::Percent200, 32.0),
        ] {
            assert_eq!(resolved_ui_pixels(16.0, ui_scale), expected);
        }
    }
}
