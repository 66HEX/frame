use palette::{IntoColor, OklabHue, Oklcha, RgbHue, WithAlpha};
use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

// Re-exported for api maintenance
pub use palette::{Hsla, rgb::Rgba};

/// Convert an RGB hex color code number to a color type
pub fn rgb(hex: u32) -> Rgba {
    let [_, r, g, b] = hex.to_be_bytes().map(|b| (b as f32) / 255.0);
    Rgba {
        color: palette::rgb::Rgb::new(r, g, b),
        alpha: 1.0,
    }
}

/// Convert an RGBA hex color code number to [`Rgba`]
pub fn rgba(hex: u32) -> Rgba {
    let [r, g, b, a] = hex.to_be_bytes().map(|b| (b as f32) / 255.0);
    Rgba::new(r, g, b, a)
}

/// Convert an sRGB color to GPUI's HSL-with-alpha representation.
///
/// This is the explicit conversion boundary for APIs that store [`Hsla`]. It
/// avoids exposing Palette's conversion traits to applications and examples.
pub fn rgb_to_hsla(color: Rgba) -> Hsla {
    color.into_color()
}

/// Convert GPUI's HSL-with-alpha representation to sRGB.
pub fn hsla_to_rgba(color: Hsla) -> Rgba {
    color.into_color()
}

/// Swap from RGBA with premultiplied alpha to BGRA
pub fn swap_rgba_pa_to_bgra(color: &mut [u8]) {
    color.swap(0, 2);
    if color[3] > 0 {
        let a = color[3] as f32 / 255.;
        color[0] = (color[0] as f32 / a) as u8;
        color[1] = (color[1] as f32 / a) as u8;
        color[2] = (color[2] as f32 / a) as u8;
    }
}

/// Construct an [`Hsla`] object from plain values
pub const fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla {
        color: palette::Hsl::new_const(
            // `RgbHue` stores degrees, so the 0..1 fraction of the circle needs
            // scaling to 0..360 before it's wrapped.
            RgbHue::new(h.clamp(0., 1.) * 360.),
            s.clamp(0., 1.),
            l.clamp(0., 1.),
        ),
        alpha: a.clamp(0., 1.),
    }
}

/// Constructs a ['Oklcha'](palette::Oklcha) object from plain values.
pub fn oklcha<T>(lightness: T, chroma: T, hue: impl Into<OklabHue<T>>, alpha: T) -> Oklcha<T> {
    Oklcha::new(lightness, chroma, hue, alpha)
}

/// Pure black in [`Hsla`]
pub const fn black() -> Hsla {
    Hsla::new_const(RgbHue::new(0.), 0., 0., 1.)
}

/// Transparent black in [`Hsla`]
pub const fn transparent_black() -> Hsla {
    Hsla::new_const(RgbHue::new(0.), 0., 0., 0.)
}

/// Transparent white in [`Hsla`]
pub const fn transparent_white() -> Hsla {
    Hsla::new_const(RgbHue::new(0.), 0., 1., 0.)
}

/// Opaque grey in [`Hsla`], values must be provided in the range [0, 1]
pub const fn opaque_grey(lightness: f32, opacity: f32) -> Hsla {
    Hsla::new_const(RgbHue::new(0.), 0., lightness, opacity)
}

/// Pure white in [`Hsla`]
pub const fn white() -> Hsla {
    Hsla::new_const(RgbHue::new(0.), 0., 1., 1.)
}

/// The color red in [`Hsla`]
pub const fn red() -> Hsla {
    Hsla::new_const(RgbHue::new(0.), 1., 0.5, 1.)
}

/// The color blue in [`Hsla`]
pub const fn blue() -> Hsla {
    Hsla::new_const(RgbHue::new(240.), 1., 0.5, 1.)
}

/// The color green in [`Hsla`]
pub const fn green() -> Hsla {
    Hsla::new_const(RgbHue::new(120.), 1., 0.25, 1.)
}

/// The color yellow in [`Hsla`]
pub const fn yellow() -> Hsla {
    Hsla::new_const(RgbHue::new(60.), 1., 0.5, 1.)
}

/// Generates the JsonSchema for palette::Hsla
pub fn hsla_schemar(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    // https://github.com/Ogeon/palette/blob/9aa1ac21a7da60db398e8c044a43dbf3fdaf4855/palette/src/hsl.rs#L633-L636
    // https://github.com/Ogeon/palette/blob/9aa1ac21a7da60db398e8c044a43dbf3fdaf4855/palette/src/alpha/alpha.rs#L1197
    json_schema!({
        "type": "object",
        "properties": {
          "hue": {
              "type": "number",
              "format": "float"
          },
          "saturation": {
              "type": "number",
              "format": "float"
          },
          "lightness": {
              "type": "number",
              "format": "float"
          },
          "alpha": {
              "type": "number",
              "format": "float"
          },
        }
    })
}

/// Generates the JsonSchema for palette::Rgba
pub fn rgba_schemar(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    // https://github.com/Ogeon/palette/blob/9aa1ac21a7da60db398e8c044a43dbf3fdaf4855/palette/src/rgb/rgb.rs#L1690
    // https://github.com/Ogeon/palette/blob/9aa1ac21a7da60db398e8c044a43dbf3fdaf4855/palette/src/alpha/alpha.rs#L1197
    json_schema!({
        "type": "object",
        "properties": {
          "red": {
              "type": "number",
              "format": "float"
          },
          "green": {
              "type": "number",
              "format": "float"
          },
          "blue": {
              "type": "number",
              "format": "float"
          },
          "alpha": {
              "type": "number",
              "format": "float"
          },
        }
    })
}

/// Wrapper methods to make alpha operations more convenient
pub trait ColorExt {
    /// Performs a `(1 - DstAlpha) + DstAlpha` blend, with custom considerations for DstAlpha <=0 and >=1.
    ///
    /// This is a nonstandard blend maintained for backwards compatibility. Its strongly recommended that users interface with the [palette crate blend ops](palette::blend) directly.
    fn blend(&self, other: &Self) -> Self
    where
        Self: Sized;

    /// Fade out the color by a given factor. This factor should be between 0.0 and 1.0.
    /// Where 0.0 will leave the color unchanged, and 1.0 will completely fade out the color.
    fn fade_out(&mut self, factor: f32);

    /// Multiplies the alpha value of the color by a given factor and returns a new color.
    /// If the color was previously opaque, then this is equivalent to
    /// [`with_alpha`](palette::WithAlpha::with_alpha).
    ///
    /// Useful for transforming colors with dynamic opacity,
    /// like a color from an external source.
    ///
    /// Example:
    /// ```
    /// use gpui::ColorExt;
    /// let color = gpui::red();
    /// let faded_color = color.opacity(0.5);
    /// assert_eq!(faded_color.alpha, 0.5);
    /// ```
    ///
    /// This will return a red color with half the opacity.
    ///
    /// Example:
    /// ```
    /// use gpui::{hsla, ColorExt};
    /// let color = hsla(0.7, 1.0, 0.5, 0.7); // A saturated blue
    /// let faded_color = color.opacity(0.16);
    /// assert!((faded_color.alpha - 0.112).abs() < 1e-6);
    /// ```
    ///
    /// This will return a blue color with around ~10% opacity,
    /// suitable for an element's hover or selected state.
    ///
    fn opacity(&self, factor: f32) -> Self
    where
        Self: Sized;
}
impl ColorExt for Rgba {
    fn blend(&self, other: &Self) -> Self {
        use palette::blend::{BlendFunction, Equation, Equations, Parameter, Parameters, PreAlpha};
        let src = self.with_alpha(self.alpha.clamp(0f32, 1f32));
        let dst = other.with_alpha(other.alpha.clamp(0f32, 1f32));
        let blend_mode = Equations {
            color_equation: Equation::Add,
            alpha_equation: Equation::Add,
            // color = [src.color * (1 - dst.alpha)] + [dst.color * dst.alpha]
            color_parameters: Parameters {
                source: Parameter::OneMinusDestinationAlpha,
                destination: Parameter::DestinationAlpha,
            },
            alpha_parameters: match dst.alpha > 0f32 && dst.alpha < 1f32 {
                // alpha = [src.alpha * One] + [dst.alpha * Zero] => src.alpha
                true => Parameters {
                    source: Parameter::One,
                    destination: Parameter::Zero,
                },
                // alpha = [src.alpha * (1 - dst.alpha)] + [dst.alpha * dst.alpha]
                //		=> dst.alpha when dst.alpha == 1
                //		=> src.alpha when dst.alpha == 0
                false => Parameters {
                    source: Parameter::OneMinusDestinationAlpha,
                    destination: Parameter::DestinationAlpha,
                },
            },
        };
        // in order to match original/upstream behavior, this blend does /not/ premultiply (as palette standard is)
        let src = PreAlpha {
            color: src.color,
            alpha: src.alpha,
        };
        let dst = PreAlpha {
            color: dst.color,
            alpha: dst.alpha,
        };
        let premul = blend_mode.apply_to(src, dst);
        Rgba {
            color: premul.color,
            alpha: premul.alpha,
        }
    }

    fn fade_out(&mut self, factor: f32) {
        self.alpha *= 1.0 - factor.clamp(0., 1.);
    }

    fn opacity(&self, factor: f32) -> Self {
        let mut color = *self;
        color.alpha *= factor.clamp(0., 1.);
        color
    }
}
impl ColorExt for Hsla {
    fn blend(&self, other: &Self) -> Self {
        let this: Rgba = (*self).into_color();
        let other: Rgba = (*other).into_color();
        this.blend(&other).into_color()
    }

    fn fade_out(&mut self, factor: f32) {
        self.alpha *= 1.0 - factor.clamp(0., 1.);
    }

    fn opacity(&self, factor: f32) -> Self {
        let mut color = *self;
        color.alpha *= factor.clamp(0., 1.);
        color
    }
}

/// The storage-buffer discriminant for a [`Background`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[repr(u32)]
pub enum BackgroundTag {
    /// A flat color.
    Solid = 0,
    /// A two-stop linear gradient.
    LinearGradient = 1,
    /// Diagonal color stripes over transparency.
    PatternSlash = 2,
    /// Alternating colored and transparent squares.
    Checkerboard = 3,
}

/// A color space for color interpolation.
///
/// References:
/// - <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation-method>
/// - <https://www.w3.org/TR/css-color-4/#typedef-color-space>
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[repr(u32)]
pub enum ColorSpace {
    #[default]
    /// The sRGB color space.
    Srgb = 0,
    /// The Oklab color space.
    Oklab = 1,
}

impl Display for ColorSpace {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ColorSpace::Srgb => write!(f, "sRGB"),
            ColorSpace::Oklab => write!(f, "Oklab"),
        }
    }
}

/// A background color, which can be either a solid color or a linear gradient.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[repr(C)]
pub struct Background {
    pub(crate) tag: BackgroundTag,
    pub(crate) color_space: ColorSpace,
    pub(crate) solid: crate::SceneHsla,
    pub(crate) gradient_angle_or_pattern_height: f32,
    pub(crate) colors: [LinearColorStop; 2],
    /// Padding for alignment for repr(C) layout.
    pub(crate) padding: u32,
}

impl std::fmt::Debug for Background {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.tag {
            BackgroundTag::Solid => write!(f, "Solid({:?})", self.solid),
            BackgroundTag::LinearGradient => write!(
                f,
                "LinearGradient({}, {:?}, {:?})",
                self.gradient_angle_or_pattern_height, self.colors[0], self.colors[1]
            ),
            BackgroundTag::PatternSlash => write!(
                f,
                "PatternSlash({:?}, {})",
                self.solid, self.gradient_angle_or_pattern_height
            ),
            BackgroundTag::Checkerboard => write!(
                f,
                "Checkerboard({:?}, {})",
                self.solid, self.gradient_angle_or_pattern_height
            ),
        }
    }
}

impl Eq for Background {}
impl Default for Background {
    fn default() -> Self {
        Self {
            tag: BackgroundTag::Solid,
            solid: Hsla::default().into(),
            color_space: ColorSpace::default(),
            gradient_angle_or_pattern_height: 0.0,
            colors: [LinearColorStop::default(), LinearColorStop::default()],
            padding: 0,
        }
    }
}

/// Creates a hash pattern background
pub fn pattern_slash(color: impl IntoColor<Hsla>, width: f32, interval: f32) -> Background {
    const PATTERN_COMPONENT_SCALE: f32 = u8::MAX as f32;
    const PATTERN_PACKING_RADIX: u32 = u16::MAX as u32;

    let width_scaled = (width * PATTERN_COMPONENT_SCALE) as u32;
    let interval_scaled = (interval * PATTERN_COMPONENT_SCALE) as u32;
    let height = (width_scaled * PATTERN_PACKING_RADIX + interval_scaled) as f32;

    Background {
        tag: BackgroundTag::PatternSlash,
        solid: color.into_color().into(),
        gradient_angle_or_pattern_height: height,
        ..Default::default()
    }
}

/// Creates a checkerboard pattern background
pub fn checkerboard(color: impl IntoColor<Hsla>, size: f32) -> Background {
    Background {
        tag: BackgroundTag::Checkerboard,
        solid: color.into_color().into(),
        gradient_angle_or_pattern_height: size,
        ..Default::default()
    }
}

/// Creates a solid background color.
pub fn solid_background(color: impl IntoColor<Hsla>) -> Background {
    Background {
        solid: color.into_color().into(),
        ..Default::default()
    }
}

/// Creates a LinearGradient background color.
///
/// The gradient line's angle of direction. A value of `0.` is equivalent to top; increasing values rotate clockwise from there.
///
/// The `angle` is in degrees value in the range 0.0 to 360.0.
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient>
pub fn linear_gradient(
    angle: f32,
    from: impl Into<LinearColorStop>,
    to: impl Into<LinearColorStop>,
) -> Background {
    Background {
        tag: BackgroundTag::LinearGradient,
        gradient_angle_or_pattern_height: angle,
        colors: [from.into(), to.into()],
        color_space: ColorSpace::Oklab,
        ..Default::default()
    }
}

/// A color stop in a linear gradient.
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient#linear-color-stop>
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[repr(C)]
pub struct LinearColorStop {
    /// The color of the color stop.
    pub color: crate::SceneHsla,
    /// The percentage of the gradient, in the range 0.0 to 1.0.
    pub percentage: f32,
}

/// Creates a new linear color stop.
///
/// The percentage of the gradient, in the range 0.0 to 1.0.
pub fn linear_color_stop(color: impl IntoColor<Hsla>, percentage: f32) -> LinearColorStop {
    LinearColorStop {
        color: color.into_color().into(),
        percentage,
    }
}

impl LinearColorStop {
    /// Returns a new color stop with the same color, but with a modified alpha value.
    pub fn opacity(&self, factor: f32) -> Self {
        let color: Hsla = self.color.into();
        Self {
            percentage: self.percentage,
            color: color.opacity(factor).into(),
        }
    }
}

/// What a [`Background`] paints, decoded from its packed representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackgroundKind {
    /// A flat color.
    Solid(Hsla),
    /// A linear gradient between two color stops.
    LinearGradient {
        /// The gradient line's angle in degrees, `0.0` pointing up, increasing clockwise.
        angle: f32,
        /// The two ends of the gradient.
        stops: [LinearColorStop; 2],
    },
    /// A diagonal stripe pattern.
    PatternSlash {
        /// The stripe color.
        color: Hsla,
        /// The stripe width, in logical pixels.
        width: f32,
        /// The gap between stripes, in logical pixels.
        interval: f32,
    },
    /// Alternating squares of one color and full transparency.
    Checkerboard {
        /// The color of one set of squares. The other set is fully transparent.
        color: Hsla,
        /// The width and height of each square, in logical pixels.
        size: f32,
    },
}

impl Background {
    /// Returns the solid color if this is a solid background, None otherwise.
    pub fn as_solid(&self) -> Option<Hsla> {
        if self.tag == BackgroundTag::Solid {
            Some(self.solid.into())
        } else {
            None
        }
    }

    /// Returns the decoded form of this background.
    pub fn kind(&self) -> BackgroundKind {
        match self.tag {
            BackgroundTag::Solid => BackgroundKind::Solid(self.solid.into()),
            BackgroundTag::LinearGradient => BackgroundKind::LinearGradient {
                angle: self.gradient_angle_or_pattern_height,
                stops: self.colors,
            },
            BackgroundTag::PatternSlash => {
                const PATTERN_COMPONENT_SCALE: f32 = u8::MAX as f32;
                const PATTERN_PACKING_RADIX: f32 = u16::MAX as f32;

                // `pattern_slash` packs both values into one exactly representable f32;
                // floor + rem_euclid inverts it, since truncation and `%` give the wrong entry.
                let packed = self.gradient_angle_or_pattern_height;
                BackgroundKind::PatternSlash {
                    color: self.solid.into(),
                    width: (packed / PATTERN_PACKING_RADIX).floor() / PATTERN_COMPONENT_SCALE,
                    interval: (packed.rem_euclid(PATTERN_PACKING_RADIX)) / PATTERN_COMPONENT_SCALE,
                }
            }
            BackgroundTag::Checkerboard => BackgroundKind::Checkerboard {
                color: self.solid.into(),
                size: self.gradient_angle_or_pattern_height,
            },
        }
    }

    /// Use specified color space for color interpolation.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation-method>
    pub fn color_space(mut self, color_space: ColorSpace) -> Self {
        self.color_space = color_space;
        self
    }

    /// The color space used to interpolate this background, set by [`Background::color_space`].
    pub fn interpolation_space(&self) -> ColorSpace {
        self.color_space
    }

    /// Returns a new background color with the same hue, saturation, and lightness, but with a modified alpha value.
    pub fn opacity(&self, factor: f32) -> Self {
        let mut background = *self;
        let solid: Hsla = background.solid.into();
        background.solid = solid.opacity(factor).into();
        background.colors = [
            self.colors[0].opacity(factor),
            self.colors[1].opacity(factor),
        ];
        background
    }

    /// Returns whether the background color is transparent.
    pub fn is_transparent(&self) -> bool {
        match self.tag {
            BackgroundTag::Solid => self.solid.a == 0.,
            BackgroundTag::LinearGradient => self.colors.iter().all(|c| c.color.a == 0.),
            BackgroundTag::PatternSlash => self.solid.a == 0.,
            BackgroundTag::Checkerboard => self.solid.a == 0.,
        }
    }
}

impl<T: IntoColor<Hsla>> From<T> for Background {
    fn from(value: T) -> Self {
        Self {
            tag: BackgroundTag::Solid,
            solid: value.into_color().into(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_solid() {
        let color: Hsla = rgba(0xff0099ff).into_color();
        let mut background = Background::from(color);
        assert_eq!(background.tag, BackgroundTag::Solid);
        assert_eq!(background.solid, color.into());

        assert_eq!(background.opacity(0.5).solid, color.opacity(0.5).into());
        assert!(!background.is_transparent());
        background.solid = hsla(0.0, 0.0, 0.0, 0.0).into();
        assert!(background.is_transparent());
    }

    #[test]
    fn test_background_linear_gradient() {
        let from = linear_color_stop(rgba(0xff0099ff), 0.0);
        let to = linear_color_stop(rgba(0x00ff99ff), 1.0);
        let background = linear_gradient(90.0, from, to);
        assert_eq!(background.tag, BackgroundTag::LinearGradient);
        assert_eq!(background.colors[0], from);
        assert_eq!(background.colors[1], to);

        assert_eq!(background.opacity(0.5).colors[0], from.opacity(0.5));
        assert_eq!(background.opacity(0.5).colors[1], to.opacity(0.5));
        assert!(!background.is_transparent());
        assert!(background.opacity(0.0).is_transparent());
    }

    #[test]
    fn test_background_kind() {
        let color: Hsla = rgba(0xff0099ff).into_color();
        assert_eq!(Background::from(color).kind(), BackgroundKind::Solid(color));

        let from = linear_color_stop(rgba(0xff0099ff), 0.0);
        let to = linear_color_stop(rgba(0x00ff99ff), 1.0);
        assert_eq!(
            linear_gradient(90.0, from, to).kind(),
            BackgroundKind::LinearGradient {
                angle: 90.0,
                stops: [from, to],
            }
        );

        assert_eq!(
            checkerboard(color, 12.0).kind(),
            BackgroundKind::Checkerboard { color, size: 12.0 }
        );
    }

    #[test]
    fn test_background_kind_unpacks_pattern_slash() {
        let color: Hsla = rgba(0xff0099ff).into_color();
        // Both values survive to the 1/255 the constructor quantizes them to.
        for (width, interval) in [(1.0, 3.0), (0.5, 0.25), (2.0, 10.0)] {
            let BackgroundKind::PatternSlash {
                width: got_width,
                interval: got_interval,
                ..
            } = pattern_slash(color, width, interval).kind()
            else {
                panic!("pattern_slash did not produce a PatternSlash");
            };
            assert!((got_width - width).abs() <= 1.0 / 255.0);
            assert!((got_interval - interval).abs() <= 1.0 / 255.0);
        }
    }

    #[test]
    fn test_rgba_alpha() {
        use palette::WithAlpha;
        let color = Rgba::<palette::Srgb>::new(0.2, 0.6, 1.0, 0.8);
        assert_eq!(color.with_alpha(0.25).alpha, 0.25);
        // NOTE: diverging from upstream, where Rgba::alpha clamps. palette does not clamp alpha
        assert_eq!(color.with_alpha(1.5).alpha, 1.5);
    }

    #[test]
    fn test_rgba_opacity() {
        use super::ColorExt;
        let color = Rgba::new(0.2, 0.6, 1.0, 0.8);
        assert!((color.opacity(0.5).alpha - 0.4).abs() < 1e-6);
        assert_eq!(color.opacity(2.0).alpha, 0.8);
    }

    fn upstream_blend(src: Rgba, dst: Rgba) -> Rgba {
        // per upstream logic;
        // https://github.com/gpui-ce/gpui-ce/commit/c738623ffbcec2aeddc44a645cc6b74646d5cf97#diff-585dd80a91eb97df686c8d31dc27ebc33b0c678a4a98c2a70a3745cd36576aaaL57-L71
        // if dst.alpha >= 1: blend with OneMinusDstAlpha (= 0) + DstAlpha (= 1)
        // 		which reduces to be equivalent to dst itself
        // if dst.alpha  < 0: blend with OneMinusDstAlpha (= 1) + DstAlpha (= 0)
        // 		which reduces to be equivalent to src itself
        // otherwise, blend color by OneMinusDstAlpha + DstAlpha, and take only the src alpha
        let new_alpha = if dst.alpha > 0f32 && dst.alpha < 1f32 {
            src.alpha
        } else {
            (src.alpha * (1f32 - dst.alpha)) + (dst.alpha * dst.alpha)
        };
        Rgba::new(
            (src.red * (1f32 - dst.alpha)) + (dst.red * dst.alpha),
            (src.green * (1f32 - dst.alpha)) + (dst.green * dst.alpha),
            (src.blue * (1f32 - dst.alpha)) + (dst.blue * dst.alpha),
            new_alpha,
        )
    }

    // per original behavior, blending a dst color that is opaque should always return that destination color
    #[test]
    fn test_blend_dst_opaque() {
        let src = Rgba::new(0f32, 0f32, 0f32, 0.5f32);
        let dst = Rgba::new(1f32, 1f32, 1f32, 1f32);
        let blended = src.blend(&dst);
        assert_eq!(blended, dst);
    }

    // per original behavior, blending a dst color that is transparent should always return the source color
    #[test]
    fn test_blend_dst_transparent() {
        let src = Rgba::new(0f32, 0f32, 0f32, 0.5f32);
        let dst = Rgba::new(1f32, 1f32, 1f32, 0f32);
        let blended = src.blend(&dst);
        assert_eq!(blended, src);
    }

    #[test]
    fn test_blend_opaque_white_black() {
        let src = Rgba::new(0f32, 0f32, 0f32, 1f32);
        let dst = Rgba::new(1f32, 1f32, 1f32, 1f32);
        assert_eq!(src.blend(&dst), upstream_blend(src, dst));
    }

    #[test]
    fn test_blend_translucent_on_opaquewhite() {
        let src = Rgba::new(0f32, 0f32, 0f32, 1f32);
        let dst = Rgba::new(0.3f32, 0.6f32, 1f32, 0.5f32);
        assert_eq!(src.blend(&dst), upstream_blend(src, dst));
    }
}
