//! Native colors compiled directly from assets/themes/theme.css by Cargo.

use super::RgbaToken;
use crate::appearance::ColorTheme;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
pub struct SourceColor {
    pub css: String,
    pub srgb: [f32; 4],
}

#[derive(Deserialize)]
struct Snapshot {
    light: BTreeMap<String, SourceColor>,
    dark: BTreeMap<String, SourceColor>,
}

static SNAPSHOT: LazyLock<Snapshot> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/theme-colors.json")))
        .expect("the CSS theme was validated by the build script")
});

/// # Panics
/// Panics if `name` is absent from the pinned Mira theme.
#[must_use]
pub fn colors(mode: ColorTheme) -> &'static BTreeMap<String, SourceColor> {
    match mode {
        ColorTheme::Light => &SNAPSHOT.light,
        ColorTheme::Dark => &SNAPSHOT.dark,
    }
}

/// # Panics
/// Panics if `name` is absent from the pinned Mira theme.
#[must_use]
pub fn color(mode: ColorTheme, name: &str) -> RgbaToken {
    let [red, green, blue, alpha] = colors(mode)
        .get(name)
        .unwrap_or_else(|| panic!("missing Mira {mode:?} token: {name}"))
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
    fn compiled_theme_contains_finite_colors_in_both_modes() {
        for mode in [ColorTheme::Light, ColorTheme::Dark] {
            for token in colors(mode).values() {
                assert!(!token.css.is_empty());
                assert!(
                    token
                        .srgb
                        .iter()
                        .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
                );
            }
        }
    }
}
