//! Shared light/dark elevation scale, transcribed from the supplied Figma CSS.
//! See frame-app/docs/ui-shadows/README.md for component assignments and border exceptions.

use gpui::{BoxShadow, point, px, rgba};

/// Logical pixels, identical on macOS, Windows and Linux (independent of UI zoom).
pub const EDGE_WIDTH: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowLevel {
    Xxs,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

pub const INDICATOR: ShadowLevel = ShadowLevel::Xxs;
pub const CONTROL: ShadowLevel = ShadowLevel::Xs;
pub const EMBEDDED_PANEL: ShadowLevel = ShadowLevel::Sm;
pub const CARD: ShadowLevel = ShadowLevel::Md;
pub const FLOATING: ShadowLevel = ShadowLevel::Lg;
pub const MODAL: ShadowLevel = ShadowLevel::Xl;

/// Shared inset treatment for fields, sliders, checkboxes and the canvas.
///
/// Reuses every xs layer. Dark shadows keep their downward offset and become
/// inset; the existing white inset highlight moves to the bottom.
#[must_use]
pub fn recessed_control() -> Vec<BoxShadow> {
    CONTROL
        .shadows()
        .into_iter()
        .map(|mut shadow| {
            if shadow.inset {
                shadow.offset.y = -shadow.offset.y;
            }
            shadow.inset = true;
            shadow
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Layer {
    y: f32,
    blur: f32,
    spread: f32,
    rgba: u32,
    inset: bool,
}

const fn layer(y: f32, blur: f32, spread: f32, rgba: u32, inset: bool) -> Layer {
    Layer {
        y,
        blur,
        spread,
        rgba,
        inset,
    }
}

const XXS: &[Layer] = &[
    layer(1.0, 0.0, 0.0, 0x03_03_02_13, false),
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
    layer(0.0, 0.0, 1.0, 0x03_03_02_12, false),
];

const XS: &[Layer] = &[
    layer(1.0, 1.0, -0.5, 0x03_03_02_30, false),
    layer(2.0, 2.0, -1.0, 0x03_03_02_0A, false),
    layer(4.0, 4.0, -2.0, 0x03_03_02_05, false),
    layer(8.0, 4.0, -3.0, 0x03_03_02_03, false),
    layer(0.0, 0.0, 1.0, 0x03_03_02_12, false),
    layer(1.0, 0.0, 0.0, 0x03_03_02_0A, false),
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
];

const SM: &[Layer] = &[
    layer(1.0, 2.0, -0.5, 0x03_03_02_30, false),
    layer(3.0, 3.0, -1.0, 0x03_03_02_0A, false),
    layer(6.0, 5.0, -2.0, 0x03_03_02_05, false),
    layer(15.0, 6.0, -3.0, 0x03_03_02_03, false),
    layer(0.0, 0.0, 1.0, 0x03_03_02_12, false),
    layer(1.0, 0.0, 0.0, 0x03_03_02_0A, false),
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
];

const MD: &[Layer] = &[
    layer(1.0, 3.0, 0.0, 0x03_03_02_0D, false),
    layer(5.0, 5.0, -1.0, 0x03_03_02_0D, false),
    layer(11.0, 6.0, -2.0, 0x03_03_02_08, false),
    layer(19.0, 8.0, -3.0, 0x03_03_02_03, false),
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
    layer(0.0, 0.0, 1.0, 0x03_03_02_12, false),
    layer(1.0, 0.0, 0.0, 0x03_03_02_0A, false),
];

const LG: &[Layer] = &[
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
    layer(0.0, 0.0, 1.0, 0x03_03_02_1A, false),
    layer(1.0, 0.0, 0.0, 0x03_03_02_1A, false),
    layer(2.0, 5.0, -1.0, 0x03_03_02_0F, false),
    layer(8.0, 12.0, -1.0, 0x03_03_02_0D, false),
    layer(22.0, 16.0, -2.0, 0x03_03_02_0A, false),
    layer(32.0, 18.0, -2.0, 0x03_03_02_03, false),
];

const XL: &[Layer] = &[
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
    layer(0.0, 0.0, 1.0, 0x03_03_02_1A, false),
    layer(1.0, 0.0, 0.0, 0x03_03_02_1A, false),
    layer(4.0, 8.0, -1.0, 0x03_03_02_0F, false),
    layer(15.0, 15.0, -1.0, 0x03_03_02_0D, false),
    layer(33.0, 20.0, -2.0, 0x03_03_02_0A, false),
    layer(58.0, 23.0, -2.0, 0x03_03_02_03, false),
];

const XXL: &[Layer] = &[
    layer(13.0, 29.0, 0.0, 0x03_03_02_12, false),
    layer(53.0, 53.0, 0.0, 0x03_03_02_0F, false),
    layer(119.0, 71.0, 0.0, 0x03_03_02_08, false),
    layer(211.0, 85.0, 0.0, 0x03_03_02_03, false),
    layer(0.0, 0.0, 1.0, 0x03_03_02_1A, false),
    layer(1.0, 0.0, 0.0, 0x03_03_02_1A, false),
    layer(0.0, 0.0, 1.0, 0xFF_FF_FF_0D, true),
    layer(1.0, 0.0, 0.0, 0xFF_FF_FF_1F, true),
];

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
                color: rgba(layer.rgba).into(),
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
                        rgba: u32::from_str_radix(fields[4].trim_start_matches('#'), 16).unwrap(),
                        inset: fields.get(5) == Some(&"inset"),
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
                assert_eq!(native.color, rgba(source.rgba).into());
                assert_eq!(native.offset, point(px(0.0), px(source.y)));
                assert_eq!(native.blur_radius, px(source.blur / 2.0));
                assert_eq!(native.spread_radius, px(source.spread));
                assert_eq!(native.inset, source.inset);
            }
            assert_eq!(shadows.len(), level.layers().len());
            assert!(
                level
                    .layers()
                    .iter()
                    .any(|layer| layer.inset && layer.y == EDGE_WIDTH)
            );
            assert!(
                level
                    .layers()
                    .iter()
                    .any(|layer| !layer.inset && layer.spread == EDGE_WIDTH)
            );
        }
        assert_eq!(ShadowLevel::Xs.layers()[0].spread, -0.5);
        assert_eq!(ShadowLevel::Sm.layers()[0].spread, -0.5);
    }

    #[test]
    fn recessed_controls_have_only_inner_layers_and_a_bottom_highlight() {
        let recessed = recessed_control();
        assert_eq!(recessed.len(), 8);
        assert!(recessed.iter().all(|layer| layer.inset));
        let highlight = recessed
            .iter()
            .find(|layer| layer.color == rgba(0xFF_FF_FF_1F).into())
            .unwrap();
        assert_eq!(highlight.offset, point(px(0.0), px(-1.0)));
        let top_shadow = recessed
            .iter()
            .find(|layer| layer.color == rgba(0x03_03_02_30).into())
            .unwrap();
        assert_eq!(top_shadow.offset, point(px(0.0), px(1.0)));
        assert_eq!(top_shadow.blur_radius, px(0.5));
        assert_eq!(top_shadow.spread_radius, px(-0.5));
        for (inner, source) in recessed.iter().zip(CONTROL.shadows()) {
            assert_eq!(inner.color, source.color);
            assert_eq!(inner.blur_radius, source.blur_radius);
            assert_eq!(inner.spread_radius, source.spread_radius);
        }
    }
}
