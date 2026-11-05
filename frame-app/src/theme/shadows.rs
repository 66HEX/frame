//! Tailwind v4 shadow scale used by Mira. Borders and surface rings are separate.

use gpui::{BoxShadow, Rgba, point, px};

/// Logical pixels, identical on macOS, Windows and Linux (independent of UI zoom).
pub const EDGE_WIDTH: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowLevel {
    None,
    Xxs,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

pub const FLOATING: ShadowLevel = ShadowLevel::Md;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Layer {
    y: f32,
    blur: f32,
    spread: f32,
    alpha: f32,
    inset: bool,
}

const fn layer(y: f32, blur: f32, spread: f32, alpha: f32, inset: bool) -> Layer {
    Layer {
        y,
        blur,
        spread,
        alpha,
        inset,
    }
}

const XXS: &[Layer] = &[layer(1.0, 0.0, 0.0, 0.05, false)];
const XS: &[Layer] = &[layer(1.0, 2.0, 0.0, 0.05, false)];
const SM: &[Layer] = &[
    layer(1.0, 3.0, 0.0, 0.1, false),
    layer(1.0, 2.0, -1.0, 0.1, false),
];
const MD: &[Layer] = &[
    layer(4.0, 6.0, -1.0, 0.1, false),
    layer(2.0, 4.0, -2.0, 0.1, false),
];
const LG: &[Layer] = &[
    layer(10.0, 15.0, -3.0, 0.1, false),
    layer(4.0, 6.0, -4.0, 0.1, false),
];
const XL: &[Layer] = &[
    layer(20.0, 25.0, -5.0, 0.1, false),
    layer(8.0, 10.0, -6.0, 0.1, false),
];
const XXL: &[Layer] = &[layer(25.0, 50.0, -12.0, 0.25, false)];

impl ShadowLevel {
    /// CSS lists its top layer first; GPUI paints the last layer on top.
    /// GPUI separates outer and inset passes around the element's background.
    /// Keep `px` rather than `ui_rem`: these are surface edges, not layout dimensions.
    #[must_use]
    pub fn shadows(self) -> Vec<BoxShadow> {
        self.layers()
            .iter()
            .rev()
            .map(|layer| BoxShadow {
                color: Rgba::new(0.0, 0.0, 0.0, layer.alpha).into(),
                offset: point(px(0.0), px(layer.y)),
                // GPUI consumes Gaussian sigma; CSS box-shadow specifies 2 * sigma.
                blur_radius: px(layer.blur / 2.0),
                spread_radius: px(layer.spread),
                inset: layer.inset,
            })
            .collect()
    }

    const fn layers(self) -> &'static [Layer] {
        match self {
            Self::None => &[],
            Self::Xxs => XXS,
            Self::Xs => XS,
            Self::Sm => SM,
            Self::Md => MD,
            Self::Lg => LG,
            Self::Xl => XL,
            Self::Xxl => XXL,
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "Transcribed source geometry must match exactly."
)]
mod tests {
    use super::*;

    const LEVELS: [ShadowLevel; 7] = [
        ShadowLevel::Xxs,
        ShadowLevel::Xs,
        ShadowLevel::Sm,
        ShadowLevel::Md,
        ShadowLevel::Lg,
        ShadowLevel::Xl,
        ShadowLevel::Xxl,
    ];

    #[test]
    fn documented_css_matches_all_native_layers() {
        let css = include_str!("../../docs/ui-shadows/scale.css");
        for (level, name) in LEVELS
            .into_iter()
            .zip(["2xs", "xs", "sm", "md", "lg", "xl", "2xl"])
        {
            let key = format!("--shadow-{name}:");
            let value = css.split_once(&key).unwrap().1.split_once(';').unwrap().0;
            let parsed: Vec<Layer> = value
                .split(',')
                .map(|line| {
                    let fields: Vec<_> = line.split_whitespace().collect();
                    let pixels =
                        |index: usize| fields[index].trim_end_matches("px").parse::<f32>().unwrap();
                    assert_eq!(pixels(0), 0.0);
                    Layer {
                        y: pixels(1),
                        blur: pixels(2),
                        spread: pixels(3),
                        alpha: fields[8].trim_end_matches(')').parse().unwrap(),
                        inset: fields.get(9) == Some(&"inset"),
                    }
                })
                .collect();
            assert_eq!(parsed, level.layers(), "{name}");
        }
    }

    #[test]
    fn native_adapter_preserves_css_stacking_and_geometry() {
        for level in LEVELS {
            let shadows = level.shadows();
            for (native, source) in shadows.iter().rev().zip(level.layers()) {
                assert_eq!(native.color, Rgba::new(0.0, 0.0, 0.0, source.alpha).into());
                assert_eq!(native.offset, point(px(0.0), px(source.y)));
                assert_eq!(native.blur_radius, px(source.blur / 2.0));
                assert_eq!(native.spread_radius, px(source.spread));
                assert_eq!(native.inset, source.inset);
            }
            assert_eq!(shadows.len(), level.layers().len());
            assert!(shadows.iter().all(|shadow| !shadow.inset));
        }
        assert!(ShadowLevel::None.shadows().is_empty());
    }
}
