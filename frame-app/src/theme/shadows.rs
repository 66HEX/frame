//! Guerriero surface shadows: ambient depth, a painted edge and inset highlights.
//! CSS layers retain their original order, color, opacity and logical-pixel geometry.

use super::{RgbaToken, ThemePalette};
use gpui::{Background, BoxShadow, Rgba, linear_color_stop, linear_gradient, point, px};

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

impl Layer {
    fn color(self) -> Rgba {
        let [red, green, blue, alpha] = self.rgba.to_be_bytes();
        Rgba::new(
            f32::from(red) / 255.0,
            f32::from(green) / 255.0,
            f32::from(blue) / 255.0,
            f32::from(alpha) / 255.0,
        )
    }
}

// The hexadecimal colors below are the supplied CSS values, including alpha.
const XXS: &[Layer] = &[
    layer(1.0, 0.0, 0.0, 0x0303_0213, false),
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
    layer(0.0, 0.0, 1.0, 0x0303_0212, false),
];
const XS: &[Layer] = &[
    layer(1.0, 1.0, -0.5, 0x0303_0230, false),
    layer(2.0, 2.0, -1.0, 0x0303_020A, false),
    layer(4.0, 4.0, -2.0, 0x0303_0205, false),
    layer(8.0, 4.0, -3.0, 0x0303_0203, false),
    layer(0.0, 0.0, 1.0, 0x0303_0212, false),
    layer(1.0, 0.0, 0.0, 0x0303_020A, false),
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
];
const SM: &[Layer] = &[
    layer(1.0, 2.0, -0.5, 0x0303_0230, false),
    layer(3.0, 3.0, -1.0, 0x0303_020A, false),
    layer(6.0, 5.0, -2.0, 0x0303_0205, false),
    layer(15.0, 6.0, -3.0, 0x0303_0203, false),
    layer(0.0, 0.0, 1.0, 0x0303_0212, false),
    layer(1.0, 0.0, 0.0, 0x0303_020A, false),
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
];
const MD: &[Layer] = &[
    layer(1.0, 3.0, 0.0, 0x0303_020D, false),
    layer(5.0, 5.0, -1.0, 0x0303_020D, false),
    layer(11.0, 6.0, -2.0, 0x0303_0208, false),
    layer(19.0, 8.0, -3.0, 0x0303_0203, false),
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
    layer(0.0, 0.0, 1.0, 0x0303_0212, false),
    layer(1.0, 0.0, 0.0, 0x0303_020A, false),
];
const LG: &[Layer] = &[
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
    layer(0.0, 0.0, 1.0, 0x0303_021A, false),
    layer(1.0, 0.0, 0.0, 0x0303_021A, false),
    layer(2.0, 5.0, -1.0, 0x0303_020F, false),
    layer(8.0, 12.0, -1.0, 0x0303_020D, false),
    layer(22.0, 16.0, -2.0, 0x0303_020A, false),
    layer(32.0, 18.0, -2.0, 0x0303_0203, false),
];
const XL: &[Layer] = &[
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
    layer(0.0, 0.0, 1.0, 0x0303_021A, false),
    layer(1.0, 0.0, 0.0, 0x0303_021A, false),
    layer(4.0, 8.0, -1.0, 0x0303_020F, false),
    layer(15.0, 15.0, -1.0, 0x0303_020D, false),
    layer(33.0, 20.0, -2.0, 0x0303_020A, false),
    layer(58.0, 23.0, -2.0, 0x0303_0203, false),
];
const XXL: &[Layer] = &[
    layer(13.0, 29.0, 0.0, 0x0303_0212, false),
    layer(53.0, 53.0, 0.0, 0x0303_020F, false),
    layer(119.0, 71.0, 0.0, 0x0303_0208, false),
    layer(211.0, 85.0, 0.0, 0x0303_0203, false),
    layer(0.0, 0.0, 1.0, 0x0303_021A, false),
    layer(1.0, 0.0, 0.0, 0x0303_021A, false),
    layer(0.0, 0.0, 1.0, 0xFFFF_FF0D, true),
    layer(1.0, 0.0, 0.0, 0xFFFF_FF1F, true),
];

fn native_layers(layers: &[Layer]) -> Vec<BoxShadow> {
    layers
        .iter()
        .rev()
        .map(|layer| BoxShadow {
            color: layer.color().into(),
            offset: point(px(0.0), px(layer.y)),
            // GPUI consumes Gaussian sigma; CSS box-shadow specifies 2 * sigma.
            blur_radius: px(layer.blur / 2.0),
            spread_radius: px(layer.spread),
            inset: layer.inset,
        })
        .collect()
}

/// A quiet inset well for text inputs and segmented-control tracks.
/// No outer shadow is painted: this surface belongs inside its parent.
#[must_use]
pub fn recessed(palette: &ThemePalette) -> Vec<BoxShadow> {
    let layers = if palette.is_dark() {
        [
            layer(1.0, 2.0, 0.0, 0x0303_024D, true),
            layer(0.0, 0.0, 1.0, 0x0303_0233, true),
            layer(-1.0, 0.0, 0.0, 0xFFFF_FF0F, true),
        ]
    } else {
        [
            layer(1.0, 2.0, 0.0, 0x0303_0212, true),
            layer(0.0, 0.0, 1.0, 0x0303_020F, true),
            layer(-1.0, 0.0, 0.0, 0xFFFF_FF8C, true),
        ]
    };
    native_layers(&layers)
}

/// Subtle top lighting for raised controls; badges and quiet content stay flat.
#[must_use]
pub fn surface_fill(base: RgbaToken, palette: &ThemePalette) -> Background {
    let light = if palette.is_dark() { 0.035 } else { 0.025 };
    let top = Rgba::new(
        (1.0 - base.red).mul_add(light, base.red),
        (1.0 - base.green).mul_add(light, base.green),
        (1.0 - base.blue).mul_add(light, base.blue),
        base.alpha,
    );
    let bottom = Rgba::new(base.red, base.green, base.blue, base.alpha);
    linear_gradient(
        180.0,
        linear_color_stop(top, 0.0),
        linear_color_stop(bottom, 1.0),
    )
}

impl ShadowLevel {
    /// CSS lists its top layer first; GPUI paints the last layer on top.
    /// GPUI separates outer and inset passes around the element's background.
    /// Keep `px` rather than `ui_rem`: these are surface edges, not layout dimensions.
    #[must_use]
    pub fn shadows(self) -> Vec<BoxShadow> {
        native_layers(self.layers())
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
                assert_eq!(native.color, source.color().into());
                assert_eq!(native.offset, point(px(0.0), px(source.y)));
                assert_eq!(native.blur_radius, px(source.blur / 2.0));
                assert_eq!(native.spread_radius, px(source.spread));
                assert_eq!(native.inset, source.inset);
            }
            assert_eq!(shadows.len(), level.layers().len());
            assert_eq!(shadows.iter().filter(|shadow| shadow.inset).count(), 2);
            assert!(shadows.iter().any(|shadow| !shadow.inset));
        }
        assert!(ShadowLevel::None.shadows().is_empty());
    }
}
