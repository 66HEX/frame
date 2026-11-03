//! Pinned official Geist colors. Components consume semantic `ThemePalette` roles.

use std::{collections::BTreeMap, sync::LazyLock};

use serde::Deserialize;

use super::RgbaToken;
use crate::appearance::ColorTheme;

/// An original CSS color and its normalized sRGB rendering equivalent.
#[derive(Debug, Deserialize)]
pub struct SourceColor {
    pub css: String,
    pub resolved_css: String,
    pub srgb: [f32; 4],
    /// Preserved official wide-gamut value; the native adapter currently uses sRGB.
    pub p3: Option<String>,
}

#[derive(Deserialize)]
struct Snapshot {
    light: BTreeMap<String, SourceColor>,
    dark: BTreeMap<String, SourceColor>,
}

static SNAPSHOT: LazyLock<Snapshot> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../assets/themes/geist.json"))
        .expect("the checked-in Geist color snapshot must be valid")
});

/// All imported colors, including alpha scales, P3 definitions and legacy aliases.
#[must_use]
pub fn colors(mode: ColorTheme) -> &'static BTreeMap<String, SourceColor> {
    match mode {
        ColorTheme::Light => &SNAPSHOT.light,
        ColorTheme::Dark => &SNAPSHOT.dark,
    }
}

/// Resolve a source token without the CSS `--` prefix. Only the semantic layer
/// should call this; unknown names are programming errors in that layer.
#[must_use]
pub(super) fn color(mode: ColorTheme, name: &str) -> RgbaToken {
    let [red, green, blue, alpha] = colors(mode)
        .get(name)
        .unwrap_or_else(|| panic!("missing Geist {mode:?} color: {name}"))
        .srgb;
    RgbaToken {
        red,
        green,
        blue,
        alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_covers_every_scale_in_both_modes() {
        for mode in ColorTheme::ALL {
            let colors = colors(mode);
            for family in [
                "gray",
                "gray-alpha",
                "blue",
                "red",
                "amber",
                "green",
                "teal",
                "purple",
                "pink",
            ] {
                for step in 1..=10 {
                    assert!(colors.contains_key(&format!("ds-{family}-{}", step * 100)));
                }
            }
            for name in ["ds-background-100", "ds-background-200"] {
                assert!(colors.contains_key(name));
            }
            for (name, value) in colors {
                assert!(
                    value
                        .srgb
                        .iter()
                        .all(|c| c.is_finite() && (0.0..=1.0).contains(c)),
                    "{mode:?}: {name}"
                );
                assert!(!value.css.is_empty() && !value.resolved_css.contains("var("));
            }
            for family in ["blue", "red", "amber", "green", "teal", "purple", "pink"] {
                for step in 1..=10 {
                    assert!(
                        colors[&format!("ds-{family}-{}", step * 100)]
                            .p3
                            .as_ref()
                            .is_some_and(|p3| p3.starts_with("oklch("))
                    );
                }
            }
        }
    }

    #[test]
    fn official_fractional_channels_and_alpha_are_not_quantized_or_dimmed() {
        assert!((color(ColorTheme::Dark, "ds-background-100").red - 0.04).abs() < f32::EPSILON);
        assert!((color(ColorTheme::Light, "ds-background-200").red - 0.98).abs() < f32::EPSILON);
        assert!(
            (color(ColorTheme::Light, "ds-gray-alpha-100").alpha - 13.0 / 255.0).abs()
                < f32::EPSILON
        );
        assert_ne!(
            color(ColorTheme::Light, "ds-blue-900"),
            color(ColorTheme::Dark, "ds-blue-900")
        );
    }
}
