use crate::{
    Bounds, DevicePixels, FontId, ForegroundDependency, Pixels, Point, PreparedRasterStyle,
    RasterColorExt, Rgba8, Size,
};
use derive_more::Deref;
use std::hash::{Hash, Hasher};

/// An identifier for a specific glyph, as returned by [`WindowTextSystem::layout_line`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct GlyphId(pub u32);

/// Parameters for rendering a glyph, used as cache keys for glyph atlas entries.
///
/// This struct identifies a specific glyph rendering configuration including
/// font, size, subpixel positioning, and scale factor. It's used to look up
/// cached pixels, placement metadata, and sprite atlas entries.
#[derive(Clone, Debug, PartialEq)]
#[expect(missing_docs)]
pub struct RenderGlyphParams {
    pub font_id: FontId,
    pub glyph_id: GlyphId,
    pub font_size: Pixels,
    pub subpixel_variant: Point<u8>,
    pub scale_factor: f32,
    pub raster_style: PreparedRasterStyle,
}

/// The kind of glyph image requested from a rasterizer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GlyphRenderMode {
    /// A color-independent, one-channel coverage mask.
    Grayscale,
    /// A color-independent, three-channel subpixel coverage mask.
    Subpixel,
    /// A color glyph whose pixels include their final RGB values.
    Color,
}

impl GlyphRenderMode {
    /// Returns the byte format required for this render mode.
    pub const fn rasterized_format(self) -> RasterizedGlyphFormat {
        match self {
            Self::Grayscale => RasterizedGlyphFormat::AlphaMask,
            Self::Subpixel => RasterizedGlyphFormat::BgraSubpixelMask,
            Self::Color => RasterizedGlyphFormat::BgraColor,
        }
    }
}

/// A scene request before a platform rasterizer normalizes its cache-relevant settings.
#[derive(Clone, Copy, Debug)]
pub struct RasterStyleRequest {
    /// The canonical font instance containing the glyph.
    pub font_id: FontId,
    /// The glyph whose artwork will be rasterized.
    pub glyph_id: GlyphId,
    /// The exact application color. Rasterizers may only retain a normalized derivative of it.
    pub scene_color: crate::Rgba,
    /// The requested kind of glyph image.
    pub requested_mode: GlyphRenderMode,
    /// Which foreground channels can alter the selected artwork.
    pub foreground_dependency: ForegroundDependency,
}

impl RasterStyleRequest {
    /// Quantizes the foreground after removing channels unused by the artwork.
    pub fn normalized_color(self) -> Rgba8 {
        let color = self.scene_color.quantize_raster_color();
        if self.foreground_dependency == ForegroundDependency::AlphaOnly {
            return Rgba8::new(0, 0, 0, color.alpha);
        }

        color
    }
}

/// The byte layout supplied by a glyph rasterizer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RasterizedGlyphFormat {
    /// One byte of coverage per pixel.
    AlphaMask,
    /// Four bytes per pixel in blue, green, red, unused order.
    BgraSubpixelMask,
    /// Four bytes per pixel in blue, green, red, straight-alpha order.
    BgraColor,
}

/// A glyph raster produced by a platform rasterizer.
#[derive(Clone, Debug)]
pub struct RasterizedGlyph {
    /// Placement relative to the glyph's baseline origin.
    pub bounds: Bounds<DevicePixels>,
    /// Pixel dimensions of the supplied buffer.
    pub size: Size<DevicePixels>,
    /// Byte layout of `pixels`.
    pub format: RasterizedGlyphFormat,
    /// Pixel bytes. Color pixels use straight alpha.
    pub pixels: Vec<u8>,
}

impl RasterizedGlyph {
    /// Returns a successful empty glyph in the requested format.
    pub fn empty(format: RasterizedGlyphFormat) -> Self {
        Self {
            bounds: Bounds::default(),
            size: Size::default(),
            format,
            pixels: Vec::new(),
        }
    }

    /// Checks the buffer dimensions and byte count promised by `format`.
    pub fn validate(&self) -> crate::Result<()> {
        anyhow::ensure!(
            self.bounds.size == self.size,
            "glyph raster bounds size {:?} does not match buffer size {:?}",
            self.bounds.size,
            self.size
        );

        let width: usize = self
            .size
            .width
            .0
            .try_into()
            .map_err(|_| anyhow::anyhow!("glyph raster width is negative"))?;

        let height: usize = self
            .size
            .height
            .0
            .try_into()
            .map_err(|_| anyhow::anyhow!("glyph raster height is negative"))?;

        if width == 0 || height == 0 {
            anyhow::ensure!(
                width == 0 && height == 0 && self.pixels.is_empty(),
                "empty glyph raster must have zero size and no pixels"
            );
            return Ok(());
        }

        let bytes_per_pixel = match self.format {
            RasterizedGlyphFormat::AlphaMask => 1,
            RasterizedGlyphFormat::BgraSubpixelMask | RasterizedGlyphFormat::BgraColor => 4,
        };
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
            .ok_or_else(|| anyhow::anyhow!("glyph raster byte count overflow"))?;

        anyhow::ensure!(
            self.pixels.len() == expected_len,
            "glyph raster format {:?} requires {expected_len} bytes for {}x{}, got {}",
            self.format,
            width,
            height,
            self.pixels.len()
        );

        Ok(())
    }
}

/// A glyph raster whose placement and pixel buffer have passed validation.
#[derive(Clone, Debug, Deref)]
pub struct ValidatedRasterizedGlyph(RasterizedGlyph);

impl ValidatedRasterizedGlyph {
    /// Validates a glyph raster before it can be used by an atlas builder.
    pub fn new(glyph: RasterizedGlyph) -> crate::Result<Self> {
        glyph.validate()?;

        Ok(Self(glyph))
    }
}

/// A cached glyph's GPU tile and the placement metadata for its uploaded pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphAtlasEntry {
    /// The GPU tile, or `None` for a successfully empty glyph.
    pub tile: Option<crate::AtlasTile>,
    /// Placement relative to the glyph's baseline origin.
    pub bounds: Bounds<DevicePixels>,
    /// Byte layout of the cached pixels.
    pub format: RasterizedGlyphFormat,
}

impl Eq for RenderGlyphParams {}

impl Hash for RenderGlyphParams {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_id.0.hash(state);
        self.glyph_id.0.hash(state);
        self.font_size.0.to_bits().hash(state);
        self.subpixel_variant.hash(state);
        self.scale_factor.to_bits().hash(state);
        self.raster_style.hash(state);
    }
}
