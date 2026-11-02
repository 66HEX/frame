use crate::{GlyphRenderMode, RasterStyleRequest, Rgba8};
use std::hash::{Hash, Hasher};

/// Describes which foreground channels can change rasterized color artwork.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ForegroundDependency {
    /// The artwork or its fallback may use the complete foreground color.
    Full,
    /// Fixed RGB artwork uses only the foreground alpha.
    AlphaOnly,
}

/// The cache-relevant raster settings chosen by a platform rasterizer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PreparedRasterStyle {
    /// The normalized render mode.
    pub mode: GlyphRenderMode,
    /// Any normalized color effect baked into coverage or color pixels.
    pub color_effect: RasterColorEffect,
    /// Which foreground channels can alter this raster or its fallback.
    pub foreground_dependency: ForegroundDependency,
}

impl PreparedRasterStyle {
    /// Creates a color-independent raster style.
    pub const fn independent(mode: GlyphRenderMode) -> Self {
        Self {
            mode,
            color_effect: RasterColorEffect::Independent,
            foreground_dependency: ForegroundDependency::Full,
        }
    }

    /// Creates a native preblended style from the request's relevant color channels.
    pub fn preblend(request: RasterStyleRequest) -> Self {
        Self {
            mode: request.requested_mode,
            color_effect: RasterColorEffect::Preblend(request.normalized_color()),
            foreground_dependency: request.foreground_dependency,
        }
    }
}

/// The part of the requested color, if any, which changes rasterized pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RasterColorEffect {
    /// Coverage and color pixels do not depend on the scene color.
    Independent,
    /// CoreGraphics' five-level font-smoothing dilation.
    Dilation(u8),
    /// A quantized color consumed by a native preblending or `currentColor` path.
    Preblend(Rgba8),
}

impl Hash for RasterColorEffect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        match self {
            Self::Independent => {}
            Self::Dilation(dilation) => dilation.hash(state),
            Self::Preblend(color) => <[u8; 4]>::from(*color).hash(state),
        }
    }
}
