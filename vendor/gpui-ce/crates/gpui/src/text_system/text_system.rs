use crate::{
    Bounds, Font, FontId, FontMetrics, ForegroundDependency, GlyphId, GlyphRenderMode, Pixels,
    PlatformTextSystem, PreparedRasterStyle, RasterStyleRequest, RenderGlyphParams, Result, Size,
    TextRenderingMode, ValidatedRasterizedGlyph, px,
};
use anyhow::{Context as _, anyhow};
use collections::FxHashMap;
use palette::Hsla;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use smallvec::SmallVec;
use std::{borrow::Cow, sync::Arc};

/// The GPUI text rendering sub system.
pub struct TextSystem {
    pub(super) platform_text_system: Arc<dyn PlatformTextSystem>,
    font_ids_by_font: RwLock<FxHashMap<Font, Result<FontId>>>,
    font_metrics: RwLock<FxHashMap<FontId, FontMetrics>>,
}

impl TextSystem {
    /// Create a new TextSystem with the given platform text system.
    pub fn new(platform_text_system: Arc<dyn PlatformTextSystem>) -> Self {
        TextSystem {
            platform_text_system,
            font_metrics: RwLock::default(),
            font_ids_by_font: RwLock::default(),
        }
    }

    /// Get a list of all available font names from the operating system.
    pub fn all_font_names(&self) -> Vec<String> {
        self.platform_text_system.all_font_names()
    }

    /// Adds font data to the text system.
    ///
    /// Font family names come from each font's embedded name table.
    pub fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        // Serialize registration with cache misses so an in-flight lookup cannot
        // repopulate a stale miss after the newly registered fonts become available.
        let mut font_ids = self.font_ids_by_font.write();
        let result = self.platform_text_system.add_fonts(fonts);
        font_ids.clear();
        result
    }

    /// Get the FontId for the configure font family and style.
    fn font_id(&self, font: &Font) -> Result<FontId> {
        fn clone_font_id_result(font_id: &Result<FontId>) -> Result<FontId> {
            match font_id {
                Ok(font_id) => Ok(*font_id),
                Err(err) => Err(anyhow!("{err}")),
            }
        }

        let font_id = self
            .font_ids_by_font
            .read()
            .get(font)
            .map(clone_font_id_result);
        if let Some(font_id) = font_id {
            font_id
        } else {
            let mut font_ids = self.font_ids_by_font.write();
            if let Some(font_id) = font_ids.get(font) {
                return clone_font_id_result(font_id);
            }
            let font_id = self.platform_text_system.font_id(font);
            font_ids.insert(font.clone(), clone_font_id_result(&font_id));
            font_id
        }
    }

    /// Resolves the specified font using backend-owned aliases and fallbacks.
    ///
    /// # Panics
    ///
    /// Panics if the backend cannot resolve the font.
    pub fn resolve_font(&self, font: &Font) -> FontId {
        self.font_id(font)
            .unwrap_or_else(|error| panic!("failed to resolve font '{}': {error}", font.family))
    }

    /// Prewarm any system font caches needed to shape text.
    ///
    /// This may be expensive, so callers should generally invoke it on a
    /// background executor. Missing entries are still populated on demand by
    /// the normal shaping path.
    pub fn prewarm_fonts(&self, fonts: &[Font]) {
        let mut font_ids = SmallVec::<[FontId; 8]>::new();
        for font in fonts {
            let font_id = self.resolve_font(font);
            if !font_ids.contains(&font_id) {
                font_ids.push(font_id);
            }
        }
        self.platform_text_system.prewarm_fonts(&font_ids);
    }

    /// Get the bounding box for the given font and font size.
    /// A font's bounding box is the smallest rectangle that could enclose all glyphs
    /// in the font. superimposed over one another.
    pub fn bounding_box(&self, font_id: FontId, font_size: Pixels) -> Bounds<Pixels> {
        self.read_metrics(font_id, |metrics| metrics.bounding_box(font_size))
    }

    /// Get the typographic bounds for the given character, in the given font and size.
    pub fn typographic_bounds(
        &self,
        font_id: FontId,
        font_size: Pixels,
        character: char,
    ) -> Result<Bounds<Pixels>> {
        let glyph_id = self
            .platform_text_system
            .glyph_for_char(font_id, character)
            .with_context(|| format!("glyph not found for character '{character}'"))?;
        let bounds = self
            .platform_text_system
            .typographic_bounds(font_id, glyph_id)?;
        Ok(self.read_metrics(font_id, |metrics| {
            (bounds / metrics.units_per_em as f32 * font_size.0).map(px)
        }))
    }

    /// Get the advance width for the given character, in the given font and size.
    pub fn advance(&self, font_id: FontId, font_size: Pixels, ch: char) -> Result<Size<Pixels>> {
        let glyph_id = self
            .platform_text_system
            .glyph_for_char(font_id, ch)
            .with_context(|| format!("glyph not found for character '{ch}'"))?;
        let result = self.platform_text_system.advance(font_id, glyph_id)?
            / self.units_per_em(font_id) as f32;

        Ok(result * font_size)
    }

    /// Get the number of font size units per 'em square',
    /// Per MDN: "an abstract square whose height is the intended distance between
    /// lines of type in the same type size"
    pub fn units_per_em(&self, font_id: FontId) -> u32 {
        self.read_metrics(font_id, |metrics| metrics.units_per_em)
    }

    /// Get the recommended distance from the baseline for the given font
    pub fn ascent(&self, font_id: FontId, font_size: Pixels) -> Pixels {
        self.read_metrics(font_id, |metrics| metrics.ascent(font_size))
    }

    /// Get the recommended distance below the baseline for the given font,
    /// in single spaced text.
    pub fn descent(&self, font_id: FontId, font_size: Pixels) -> Pixels {
        self.read_metrics(font_id, |metrics| metrics.descent(font_size))
    }

    /// Get the x-height for the given font and font size.
    pub fn x_height(&self, font_id: FontId, font_size: Pixels) -> Pixels {
        self.read_metrics(font_id, |metrics| metrics.x_height(font_size))
    }

    /// Get the recommended baseline offset for the given font and line height.
    pub fn baseline_offset(
        &self,
        font_id: FontId,
        font_size: Pixels,
        line_height: Pixels,
    ) -> Pixels {
        let ascent = self.ascent(font_id, font_size);
        let descent = self.descent(font_id, font_size);
        let padding_top = (line_height - ascent - descent) / 2.;
        padding_top + ascent
    }

    fn read_metrics<T>(&self, font_id: FontId, read: impl FnOnce(&FontMetrics) -> T) -> T {
        let lock = self.font_metrics.upgradable_read();

        if let Some(metrics) = lock.get(&font_id) {
            read(metrics)
        } else {
            let mut lock = RwLockUpgradableReadGuard::upgrade(lock);
            let metrics = lock
                .entry(font_id)
                .or_insert_with(|| self.platform_text_system.font_metrics(font_id));
            read(metrics)
        }
    }

    /// Rasterizes a glyph and validates its placement and pixel buffer.
    pub fn rasterize_glyph(&self, params: &RenderGlyphParams) -> Result<ValidatedRasterizedGlyph> {
        let glyph = self.platform_text_system.rasterize_glyph(params)?;

        ValidatedRasterizedGlyph::new(glyph)
    }

    /// Normalizes a requested scene color and render mode into the settings which affect the
    /// cached glyph raster.
    pub(crate) fn prepare_raster_style(
        &self,
        font_id: FontId,
        glyph_id: GlyphId,
        scene_color: Hsla,
        requested_mode: GlyphRenderMode,
    ) -> PreparedRasterStyle {
        self.platform_text_system
            .prepare_raster_style(RasterStyleRequest {
                font_id,
                glyph_id,
                scene_color: crate::hsla_to_rgba(scene_color),
                requested_mode,
                foreground_dependency: ForegroundDependency::Full,
            })
    }

    /// Returns the text rendering mode recommended by the platform for the given font and size.
    /// The return value will never be [`TextRenderingMode::PlatformDefault`].
    pub(crate) fn recommended_rendering_mode(
        &self,
        font_id: FontId,
        font_size: Pixels,
    ) -> TextRenderingMode {
        self.platform_text_system
            .recommended_rendering_mode(font_id, font_size)
    }
}
