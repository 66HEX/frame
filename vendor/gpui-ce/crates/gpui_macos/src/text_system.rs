use std::borrow::Cow;

use gpui::{
    Bounds, Font, FontId, FontMetrics, GlyphId, InlineLayout, InlineLayoutRequest, LineLayout,
    Pixels, PlatformTextSystem, PreparedRasterStyle, RasterStyleRequest, RasterizedGlyph,
    RenderGlyphParams, Result, Size, TextLayoutRequest, TextRenderingMode,
};
use gpui_parley::{BitmapFallbackGlyphRasterizer, ParleyTextSystem, SystemFonts};

use self::renderer::MacGlyphRenderer;

/// The macOS text system, with Parley layout and native glyph rasterization.
pub struct MacTextSystem {
    parley: ParleyTextSystem,
}

impl MacTextSystem {
    /// Creates the macOS text system.
    pub fn new() -> Self {
        Self {
            parley: ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Load,
                ".AppleSystemUIFont",
                BitmapFallbackGlyphRasterizer::new(MacGlyphRenderer::new()),
            )
            .with_automatic_optical_sizing()
            .with_fallback_families(["Lilex", "IBM Plex Sans", "Helvetica", "Arial"]),
        }
    }
}

impl Default for MacTextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTextSystem for MacTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.parley.add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        self.parley.all_font_names()
    }

    fn font_generation(&self) -> u64 {
        self.parley.font_generation()
    }

    fn font_id(&self, descriptor: &Font) -> Result<FontId> {
        self.parley.font_id(descriptor)
    }

    fn prewarm_fonts(&self, font_ids: &[FontId]) {
        self.parley.prewarm_fonts(font_ids);
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        self.parley.font_metrics(font_id)
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        self.parley.typographic_bounds(font_id, glyph_id)
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        self.parley.advance(font_id, glyph_id)
    }

    fn glyph_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId> {
        self.parley.glyph_for_char(font_id, character)
    }

    fn rasterize_glyph(&self, params: &RenderGlyphParams) -> Result<RasterizedGlyph> {
        self.parley.rasterize_glyph(params)
    }

    fn prepare_raster_style(&self, request: RasterStyleRequest) -> PreparedRasterStyle {
        self.parley.prepare_raster_style(request)
    }

    fn layout_text(&self, request: TextLayoutRequest<'_>) -> LineLayout {
        self.parley.layout_text(request)
    }

    fn layout_inline(&self, request: InlineLayoutRequest<'_>) -> InlineLayout {
        self.parley.layout_inline(request)
    }

    fn recommended_rendering_mode(&self, font_id: FontId, font_size: Pixels) -> TextRenderingMode {
        self.parley.recommended_rendering_mode(font_id, font_size)
    }
}

mod renderer {
    use core_foundation_sys::{
        array::CFArrayRef, data::CFDataRef, preferences::CFPreferencesCopyAppValue,
        preferences::kCFPreferencesCurrentApplication, string::CFStringRef,
    };

    #[cfg(test)]
    use core_foundation_sys::dictionary::CFDictionaryRef;

    #[cfg(test)]
    use gpui::{
        GlyphId, PlatformTextSystem, RasterizedGlyphFormat, Rgba8, TextLayoutRequest, TextRun,
        font as gpui_font, px, rgba,
    };

    #[cfg(test)]
    use gpui_parley::{FontVariation, ParleyTextSystem, SystemFonts};

    #[cfg(test)]
    use std::borrow::Cow;

    use anyhow::{Context as _, Result, anyhow, ensure};
    use core_foundation::{
        array::CFArray,
        base::{CFType, TCFType},
        data::CFData,
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    };
    use core_graphics::{
        base::{CGFloat, kCGImageAlphaPremultipliedLast},
        color_space::CGColorSpace,
        context::{CGContext, CGLineJoin, CGTextDrawingMode},
        display::CGPoint,
        geometry::CGAffineTransform,
    };
    use core_text::{
        font,
        font_descriptor::{self, CTFontDescriptor, kCTFontOrientationDefault},
    };
    use font_kit::{
        canvas::RasterizationOptions, font::Font as FontKitFont, hinting::HintingOptions,
    };
    use gpui::{
        Bounds, DevicePixels, FontId, GlyphRenderMode, Pixels, PreparedRasterStyle,
        RasterColorEffect, RasterStyleRequest, RasterizedGlyph, RenderGlyphParams,
        SUBPIXEL_VARIANTS_X, SUBPIXEL_VARIANTS_Y, TextRenderingMode, point, size,
    };
    use gpui_parley::{ColorGlyphKind, FontDataBlob, FontSynthesis, GlyphRasterizer, RasterFace};
    use objc2::rc::autoreleasepool;
    use pathfinder_geometry::{rect::RectI, transform2d::Transform2F};
    use skrifa::{FontRef, Tag, raw::MinByteRange};
    use std::{
        collections::{HashMap, hash_map::Entry},
        f64::consts::PI,
        sync::{Arc, OnceLock},
    };

    const TTC_TAG: &[u8; 4] = b"ttcf";
    const CHECKSUM_MAGIC: u32 = 0xb1b0_afba;

    #[link(name = "CoreText", kind = "framework")]
    unsafe extern "C" {
        fn CTFontManagerCreateFontDescriptorsFromData(data: CFDataRef) -> CFArrayRef;
        #[cfg(test)]
        fn CTFontCopyVariation(font: core_text::font::CTFontRef) -> CFDictionaryRef;
        static kCTFontOpticalSizeAttribute: CFStringRef;
    }

    #[allow(non_upper_case_globals)]
    const kCGImageAlphaOnly: u32 = 7;

    /// CoreText and CoreGraphics rasterization for the exact face selected by Parley.
    pub(crate) struct MacGlyphRenderer {
        faces: HashMap<FontId, NativeFace>,
        sources: HashMap<u64, Arc<SendCFData>>,
    }

    struct NativeFace {
        descriptor: CTFontDescriptor,
        font: FontKitFont,
        synthesis: FontSynthesis,
        has_default_variations: bool,
        // CoreText may defer reading tables from descriptors created from in-memory data until a
        // sized CTFont first draws. Keep the descriptor's source alive for the full cached-face
        // lifetime, as the pre-Parley backend did through its retained CGFont.
        _source_data: Arc<SendCFData>,
    }

    /// An immutable Core Foundation data object retained by the serialized macOS rasterizer.
    struct SendCFData {
        data: CFData,
    }

    // SAFETY: CFData is immutable, and MacGlyphRenderer only accesses native faces while its
    // enclosing mutex is held. The value is retained solely to extend the source data's lifetime.
    unsafe impl Send for SendCFData {}
    // SAFETY: CFData is immutable, so retaining it from multiple native-face entries is safe.
    unsafe impl Sync for SendCFData {}

    impl MacGlyphRenderer {
        pub(crate) fn new() -> Self {
            Self {
                faces: HashMap::default(),
                sources: HashMap::default(),
            }
        }

        fn load_face(&mut self, face: RasterFace<'_>) -> Result<()> {
            let Entry::Vacant(entry) = self.faces.entry(face.font_id) else {
                return Ok(());
            };
            let source = self
                .sources
                .get(&face.source_id)
                .cloned()
                .unwrap_or_else(|| source_from_blob(face.source.clone()));
            let native =
                autoreleasepool(|_| NativeFace::new(&face, source.clone())).with_context(|| {
                    format!(
                        "CoreText could not create FontId {:?}, face index {}, variations {:?}",
                        face.font_id, face.face_index, face.variations
                    )
                })?;

            if Arc::ptr_eq(&native._source_data, &source) {
                self.sources.entry(face.source_id).or_insert(source);
            }

            entry.insert(native);

            Ok(())
        }

        fn raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
            let native = &self.faces[&params.font_id];

            if params.raster_style.mode != GlyphRenderMode::Color
                && native.synthesis == FontSynthesis::default()
                && native.has_default_variations
            {
                let scale = Transform2F::from_scale(params.scale_factor);
                let rect = native.font.raster_bounds(
                    params.glyph_id.0,
                    params.font_size.into(),
                    scale,
                    HintingOptions::None,
                    RasterizationOptions::GrayscaleAa,
                )?;
                let bounds = bounds_from_rect_i(rect);

                if bounds.size.width.0 == 0 || bounds.size.height.0 == 0 {
                    return Ok(bounds);
                }

                return Ok(bounds.dilate(DevicePixels(1)));
            }

            self.core_text_raster_bounds(native, params)
        }

        fn core_text_raster_bounds(
            &self,
            native: &NativeFace,
            params: &RenderGlyphParams,
        ) -> Result<Bounds<DevicePixels>> {
            let font_size = f64::from(params.font_size);
            let scale_factor = f64::from(params.scale_factor);
            let font = font::new_from_descriptor(&native.descriptor, font_size);
            let glyph: u16 = params
                .glyph_id
                .0
                .try_into()
                .context("CoreText glyph IDs are 16-bit")?;
            let skew = native
                .synthesis
                .skew_degrees
                .map_or(0.0, |degrees| f64::from(degrees) * PI / 180.0)
                .tan();
            let text_matrix = CGAffineTransform::new(1.0, 0.0, skew, 1.0, 0.0, 0.0);
            let rect = font
                .get_bounding_rects_for_glyphs(kCTFontOrientationDefault, &[glyph])
                .apply_transform(&text_matrix);

            if rect.is_empty() || rect.size.width <= 0.0 || rect.size.height <= 0.0 {
                return Ok(Bounds::default());
            }

            let embolden = if native.synthesis.embolden {
                font_size / 48.0
            } else {
                0.0
            };
            let padding = (embolden * scale_factor).ceil() + 1.0;
            let left = (rect.origin.x * scale_factor - padding).floor() as i32;
            let right = ((rect.origin.x + rect.size.width) * scale_factor + padding).ceil() as i32;
            let top = (-(rect.origin.y + rect.size.height) * scale_factor - padding).floor() as i32;
            let bottom = (-rect.origin.y * scale_factor + padding).ceil() as i32;

            Ok(Bounds {
                origin: point(DevicePixels(left), DevicePixels(top)),
                size: size(DevicePixels(right - left), DevicePixels(bottom - top)),
            })
        }

        fn rasterize_native(
            &self,
            params: &RenderGlyphParams,
            glyph_bounds: Bounds<DevicePixels>,
        ) -> Result<(gpui::Size<DevicePixels>, Vec<u8>)> {
            let font_size = f64::from(params.font_size);
            let scale_factor = f64::from(params.scale_factor);
            ensure!(
                font_size.is_finite() && font_size >= 0.0,
                "invalid font size"
            );
            ensure!(
                scale_factor.is_finite() && scale_factor > 0.0,
                "invalid raster scale factor"
            );

            ensure!(font_size > 0.0, "glyph font size is empty");
            ensure!(
                params.raster_style.mode != GlyphRenderMode::Subpixel,
                "macOS rasterization only supports grayscale and color modes"
            );

            let native = &self.faces[&params.font_id];
            let mut bitmap_size = glyph_bounds.size;

            if params.subpixel_variant.x > 0 {
                bitmap_size.width += DevicePixels(1);
            }

            if params.subpixel_variant.y > 0 {
                bitmap_size.height += DevicePixels(1);
            }

            let is_color = params.raster_style.mode == GlyphRenderMode::Color;
            let bytes_per_pixel = if is_color { 4 } else { 1 };
            let mut pixels =
                vec![
                    0;
                    bitmap_size.width.0 as usize * bitmap_size.height.0 as usize * bytes_per_pixel
                ];
            let color_space = if is_color {
                CGColorSpace::create_device_rgb()
            } else {
                CGColorSpace::create_device_gray()
            };
            let context = CGContext::create_bitmap_context(
                Some(pixels.as_mut_ptr().cast()),
                bitmap_size.width.0 as usize,
                bitmap_size.height.0 as usize,
                8,
                bitmap_size.width.0 as usize * bytes_per_pixel,
                &color_space,
                if is_color {
                    kCGImageAlphaPremultipliedLast
                } else {
                    kCGImageAlphaOnly
                },
            );

            context.translate(
                -glyph_bounds.origin.x.0 as CGFloat,
                (glyph_bounds.origin.y.0 + glyph_bounds.size.height.0) as CGFloat,
            );
            context.scale(scale_factor, scale_factor);

            let skew = native
                .synthesis
                .skew_degrees
                .map_or(0.0, |degrees| f64::from(degrees) * PI / 180.0)
                .tan();
            let text_matrix = CGAffineTransform::new(1.0, 0.0, skew, 1.0, 0.0, 0.0);
            let embolden = if native.synthesis.embolden {
                font_size / 48.0
            } else {
                0.0
            };

            configure_context(
                &context,
                params.raster_style.color_effect,
                native.synthesis.embolden,
                embolden,
                text_matrix,
            );

            let offset = CGPoint::new(
                f64::from(params.subpixel_variant.x)
                    / f64::from(SUBPIXEL_VARIANTS_X)
                    / scale_factor,
                f64::from(params.subpixel_variant.y)
                    / f64::from(SUBPIXEL_VARIANTS_Y)
                    / scale_factor,
            );
            let font = font::new_from_descriptor(&native.descriptor, font_size);
            font.draw_glyphs(&[params.glyph_id.0 as u16], &[offset], context);

            if is_color {
                for pixel in pixels.chunks_exact_mut(4) {
                    gpui::swap_rgba_pa_to_bgra(pixel);
                }
            }

            Ok((bitmap_size, pixels))
        }
    }

    impl GlyphRasterizer for MacGlyphRenderer {
        fn supports_color_glyph(&self, kind: ColorGlyphKind) -> bool {
            kind != ColorGlyphKind::Cbdt
        }

        fn prepare_style(&self, request: RasterStyleRequest) -> PreparedRasterStyle {
            if request.requested_mode == GlyphRenderMode::Color {
                return PreparedRasterStyle::preblend(request);
            }

            let color_effect = if font_smoothing_allowed_by_user() {
                let color = request.scene_color;
                let luminance = 0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue;
                let dilation = ((4.0 * luminance) + 0.5).floor().clamp(0.0, 4.0) as u8;
                RasterColorEffect::Dilation(dilation)
            } else {
                RasterColorEffect::Independent
            };

            PreparedRasterStyle {
                mode: GlyphRenderMode::Grayscale,
                color_effect,
                foreground_dependency: request.foreground_dependency,
            }
        }

        fn rasterize(
            &mut self,
            face: RasterFace<'_>,
            params: &RenderGlyphParams,
        ) -> Result<RasterizedGlyph> {
            autoreleasepool(|_| {
                let format = params.raster_style.mode.rasterized_format();

                if params.font_size == Pixels::ZERO {
                    return Ok(RasterizedGlyph::empty(format));
                }

                self.load_face(face)?;
                let bounds = self.raster_bounds(params)?;

                if bounds.size.width.0 == 0 || bounds.size.height.0 == 0 {
                    return Ok(RasterizedGlyph::empty(format));
                }

                let (bitmap_size, pixels) = self.rasterize_native(params, bounds)?;

                Ok(RasterizedGlyph {
                    bounds: Bounds {
                        origin: bounds.origin,
                        size: bitmap_size,
                    },
                    size: bitmap_size,
                    format,
                    pixels,
                })
            })
        }

        fn recommended_mode(&self) -> TextRenderingMode {
            TextRenderingMode::Grayscale
        }
    }

    impl NativeFace {
        fn new(face: &RasterFace<'_>, shared_source: Arc<SendCFData>) -> Result<Self> {
            let (mut descriptor, source_data) = if face.data().get(..4) == Some(TTC_TAG) {
                match collection_descriptor(&shared_source.data, face.data(), face.face_index) {
                    Ok(descriptor) => (descriptor, shared_source),
                    Err(error) => {
                        log::debug!(
                            "CoreText could not select collection face {}; using a compact SFNT: {error:#}",
                            face.face_index
                        );

                        let source =
                            source_from_bytes(compact_sfnt_for_face(face.data(), face.face_index)?);
                        let descriptor = core_text::font_manager::create_font_descriptor_with_data(
                            source.data.clone(),
                        )
                        .map_err(|()| anyhow!("CoreText rejected the extracted font face"))?;
                        (descriptor, source)
                    }
                }
            } else {
                ensure!(
                    face.face_index == 0,
                    "single font contains only face 0, requested {}",
                    face.face_index
                );
                let descriptor = core_text::font_manager::create_font_descriptor_with_data(
                    shared_source.data.clone(),
                )
                .map_err(|()| anyhow!("CoreText rejected the selected font face"))?;
                (descriptor, shared_source)
            };

            let optical_size_key =
                unsafe { CFString::wrap_under_get_rule(kCTFontOpticalSizeAttribute) };
            let mut attributes = vec![(optical_size_key, CFString::new("none").into_CFType())];

            if !face.variations.is_empty() {
                let variations = face
                    .variations
                    .iter()
                    .map(|variation| {
                        let tag = u32::from_be_bytes(variation.tag.to_be_bytes());
                        (
                            CFNumber::from(i64::from(tag)),
                            CFNumber::from(f64::from(variation.value)),
                        )
                    })
                    .collect::<Vec<_>>();
                let variations = CFDictionary::from_CFType_pairs(&variations);
                let variation_key = unsafe {
                    CFString::wrap_under_get_rule(font_descriptor::kCTFontVariationAttribute)
                };

                attributes.push((variation_key, variations.into_CFType()));
            }

            descriptor = descriptor
                .create_copy_with_attributes(
                    CFDictionary::from_CFType_pairs(&attributes).into_untyped(),
                )
                .map_err(|()| anyhow!("CoreText rejected the selected instance attributes"))?;

            let has_default_variations = face.has_default_variations()?;
            let core_text_font = font::new_from_descriptor(&descriptor, 0.0);
            let font = FontKitFont::from_core_graphics_font(core_text_font.copy_to_CGFont());

            Ok(Self {
                descriptor,
                font,
                synthesis: face.synthesis,
                has_default_variations,
                _source_data: source_data,
            })
        }
    }

    fn source_from_bytes(bytes: Vec<u8>) -> Arc<SendCFData> {
        source_from_blob(FontDataBlob::from(bytes))
    }

    fn source_from_blob(bytes: FontDataBlob<u8>) -> Arc<SendCFData> {
        Arc::new(SendCFData {
            data: CFData::from_arc(Arc::new(bytes)),
        })
    }

    fn collection_descriptor(
        source: &CFData,
        data: &[u8],
        face_index: u32,
    ) -> Result<CTFontDescriptor> {
        let descriptors_ref =
            unsafe { CTFontManagerCreateFontDescriptorsFromData(source.as_concrete_TypeRef()) };
        ensure!(
            !descriptors_ref.is_null(),
            "CoreText rejected the font collection"
        );
        let descriptors =
            unsafe { CFArray::<CTFontDescriptor>::wrap_under_create_rule(descriptors_ref) };
        let font = FontRef::from_index(data, face_index)
            .context("cannot parse the selected font collection face")?;

        for descriptor in &descriptors {
            if descriptor_matches_face(&descriptor, &font) {
                return Ok(descriptor.clone());
            }
        }

        Err(anyhow!(
            "none of CoreText's {} descriptors matched physical face {face_index}",
            descriptors.len()
        ))
    }

    fn descriptor_matches_face(descriptor: &CTFontDescriptor, font_ref: &FontRef<'_>) -> bool {
        let native_font = font::new_from_descriptor(descriptor, 0.0);

        // CoreText expands variable faces into named-instance descriptors, so descriptor array
        // positions are not collection face indexes. These raw tables identify the physical face.
        for tag in [b"name", b"head", b"maxp"] {
            let Some(expected) = font_ref.table_data(Tag::new(tag)) else {
                return false;
            };
            let Some(actual) = native_font.get_font_table(u32::from_be_bytes(*tag)) else {
                return false;
            };

            if !identity_table_matches(tag, actual.bytes(), expected.as_bytes()) {
                return false;
            }
        }

        true
    }

    fn identity_table_matches(tag: &[u8; 4], actual: &[u8], expected: &[u8]) -> bool {
        if tag != b"head" {
            return actual == expected;
        }

        // CoreText clears checkSumAdjustment when exposing a face from a collection.
        actual.len() >= 12
            && actual.len() == expected.len()
            && actual[..8] == expected[..8]
            && actual[12..] == expected[12..]
    }

    fn compact_sfnt_for_face(data: &[u8], face_index: u32) -> Result<Vec<u8>> {
        ensure!(
            data.get(..4) == Some(TTC_TAG),
            "compact extraction requires a font collection"
        );
        let font = FontRef::from_index(data, face_index)
            .context("cannot parse the selected font collection face")?;
        let directory = font.table_directory();
        let records = directory.table_records();
        ensure!(
            records.len() == usize::from(directory.num_tables()),
            "truncated selected SFNT directory"
        );

        let mut sfnt = directory.min_table_bytes().to_vec();
        let mut head_offset = None;

        for (table_idx, record) in records.iter().enumerate() {
            let record_position = 12 + table_idx * 16;
            let source_offset = record.offset() as usize;
            let table_len = record.length() as usize;
            let table = data
                .get(
                    source_offset
                        ..source_offset
                            .checked_add(table_len)
                            .context("selected SFNT table end overflow")?,
                )
                .context("truncated selected SFNT table")?;

            pad_to_u32(&mut sfnt);
            let target_offset = sfnt.len();
            let target_offset_u32 = u32::try_from(target_offset)
                .context("extracted SFNT exceeds the OpenType offset range")?;
            sfnt[record_position + 8..record_position + 12]
                .copy_from_slice(&target_offset_u32.to_be_bytes());
            sfnt.extend_from_slice(table);

            if record.tag() == Tag::new(b"head") {
                ensure!(table_len >= 12, "truncated selected SFNT head table");
                head_offset = Some(target_offset);
            }
        }

        pad_to_u32(&mut sfnt);

        let head_offset = head_offset.context("selected SFNT has no head table")?;
        sfnt[head_offset + 8..head_offset + 12].fill(0);
        let adjustment = CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&sfnt));
        sfnt[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());

        Ok(sfnt)
    }

    fn pad_to_u32(data: &mut Vec<u8>) {
        let padding = (4 - data.len() % 4) % 4;
        data.resize(data.len() + padding, 0);
    }

    fn sfnt_checksum(data: &[u8]) -> u32 {
        data.chunks_exact(4).fold(0, |checksum, bytes| {
            checksum.wrapping_add(u32::from_be_bytes(
                bytes.try_into().expect("four-byte checksum chunk"),
            ))
        })
    }

    fn configure_context(
        context: &CGContext,
        color_effect: RasterColorEffect,
        embolden: bool,
        embolden_amount: CGFloat,
        text_matrix: CGAffineTransform,
    ) {
        context.set_text_drawing_mode(if embolden {
            CGTextDrawingMode::CGTextFillStroke
        } else {
            CGTextDrawingMode::CGTextFill
        });

        context.set_text_matrix(&text_matrix);
        context.set_allows_antialiasing(true);
        context.set_should_antialias(true);
        context.set_allows_font_subpixel_positioning(true);
        context.set_should_subpixel_position_fonts(true);
        context.set_allows_font_subpixel_quantization(false);
        context.set_should_subpixel_quantize_fonts(false);
        context.set_line_join(CGLineJoin::CGLineJoinRound);
        context.set_line_width(embolden_amount * 2.0);

        match color_effect {
            RasterColorEffect::Preblend(color) => {
                let channels: [u8; 4] = color.into();
                let [red, green, blue, alpha] = channels.map(|channel| f64::from(channel) / 255.0);
                context.set_alpha(alpha);
                context.set_rgb_fill_color(red, green, blue, 1.0);
                context.set_rgb_stroke_color(red, green, blue, 1.0);
            }
            RasterColorEffect::Dilation(dilation) if dilation > 0 => {
                let luminance = f64::from(dilation) * 0.25;
                context.set_should_smooth_fonts(true);
                context.set_gray_fill_color(luminance, 1.0);
                context.set_rgb_stroke_color(luminance, luminance, luminance, 1.0);
            }
            _ => {
                context.set_should_smooth_fonts(false);
                context.set_gray_fill_color(0.0, 1.0);
                context.set_rgb_stroke_color(0.0, 0.0, 0.0, 1.0);
            }
        }
    }

    fn bounds_from_rect_i(rect: RectI) -> Bounds<DevicePixels> {
        Bounds {
            origin: point(DevicePixels(rect.origin_x()), DevicePixels(rect.origin_y())),
            size: size(DevicePixels(rect.width()), DevicePixels(rect.height())),
        }
    }

    fn font_smoothing_allowed_by_user() -> bool {
        static ALLOWED: OnceLock<bool> = OnceLock::new();
        *ALLOWED.get_or_init(|| {
            let key = CFString::new("AppleFontSmoothing");
            let value_ref = unsafe {
                CFPreferencesCopyAppValue(
                    key.as_concrete_TypeRef(),
                    kCFPreferencesCurrentApplication,
                )
            };

            if value_ref.is_null() {
                return true;
            }

            let value = unsafe { CFType::wrap_under_create_rule(value_ref) };

            value
                .downcast_into::<CFNumber>()
                .and_then(|number| number.to_i64())
                != Some(0)
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use gpui::FontWidth;

        #[test]
        fn collection_faces_are_selected_and_compacted_by_physical_index() {
            let collection = test_collection(&[
                *gpui_fonts::SOURCE_SERIF,
                *gpui_fonts::IBM_PLEX,
                *gpui_fonts::IBM_PLEX_ITALIC,
            ]);
            let collection_source = FontDataBlob::from(collection.clone());
            let source = source_from_blob(collection_source.clone());
            let descriptor = collection_descriptor(&source.data, &collection, 1).unwrap();
            let font = font::new_from_descriptor(&descriptor, 16.0);

            assert_eq!(font.postscript_name(), "IBMPlexSans");

            let character = ['A' as u16];
            let mut glyph = [0];
            let mapped = unsafe {
                font.get_glyphs_for_characters(character.as_ptr(), glyph.as_mut_ptr(), 1)
            };

            assert!(mapped);
            assert_ne!(glyph[0], 0);

            let sfnt = compact_sfnt_for_face(&collection, 1).unwrap();
            assert!(sfnt.len() < collection.len());
            assert_eq!(sfnt_checksum(&sfnt), CHECKSUM_MAGIC);

            let face_offset = u32::from_be_bytes(collection[16..20].try_into().unwrap()) as usize;
            let table_count = FontRef::from_index(&collection, 1)
                .unwrap()
                .table_directory()
                .table_records()
                .len();
            let truncated_directory = &collection[..face_offset + 12 + table_count * 16 - 1];
            assert!(compact_sfnt_for_face(truncated_directory, 1).is_err());

            let mut invalid_span = collection.clone();
            invalid_span[face_offset + 20..face_offset + 24]
                .copy_from_slice(&u32::MAX.to_be_bytes());
            assert!(compact_sfnt_for_face(&invalid_span, 1).is_err());

            let data = CFData::from_arc(Arc::new(sfnt));
            let descriptor =
                core_text::font_manager::create_font_descriptor_with_data(data).unwrap();
            let font = font::new_from_descriptor(&descriptor, 16.0);
            assert_eq!(font.postscript_name(), "IBMPlexSans");
            drop(font);
            drop(descriptor);
            drop(source);

            let retained_before_rasterizer = collection_source.strong_count();
            let mut rasterizer = MacGlyphRenderer::new();

            for (font_id, face_index) in [(FontId(1), 1), (FontId(2), 2)] {
                rasterizer
                    .load_face(RasterFace {
                        font_id,
                        source_id: 1,
                        source: &collection_source,
                        face_index,
                        normalized_coords: &[],
                        variations: &[],
                        synthesis: FontSynthesis::default(),
                        has_color_glyphs: false,
                    })
                    .unwrap();
            }

            assert_eq!(rasterizer.sources.len(), 1);
            assert!(Arc::ptr_eq(
                &rasterizer.faces[&FontId(1)]._source_data,
                &rasterizer.faces[&FontId(2)]._source_data,
            ));
            assert_eq!(
                collection_source.strong_count(),
                retained_before_rasterizer + 1
            );
        }

        fn test_collection(faces: &[&[u8]]) -> Vec<u8> {
            let header_len = 12 + faces.len() * 4;
            let mut collection = vec![0; header_len];
            collection[..4].copy_from_slice(TTC_TAG);
            collection[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
            collection[8..12].copy_from_slice(&(faces.len() as u32).to_be_bytes());

            for (face_idx, face) in faces.iter().enumerate() {
                while !collection.len().is_multiple_of(4) {
                    collection.push(0);
                }

                let face_offset = collection.len();
                let offset_position = 12 + face_idx * 4;
                collection[offset_position..offset_position + 4]
                    .copy_from_slice(&(face_offset as u32).to_be_bytes());
                collection.extend_from_slice(face);

                let font = FontRef::from_index(face, 0).unwrap();

                for (table_idx, record) in font.table_directory().table_records().iter().enumerate()
                {
                    let table_offset_position = face_offset + 12 + table_idx * 16 + 8;
                    let collection_offset = record.offset() + face_offset as u32;
                    collection[table_offset_position..table_offset_position + 4]
                        .copy_from_slice(&collection_offset.to_be_bytes());
                }
            }

            collection
        }

        #[test]
        fn in_memory_variable_font_renders_stably_across_glyphs_and_sizes() {
            let system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Skip,
                "Source Serif 4",
                MacGlyphRenderer::new(),
            );
            system
                .add_fonts(vec![
                    Cow::Borrowed(*gpui_fonts::SOURCE_SERIF),
                    Cow::Borrowed(*gpui_fonts::NOTO_SANS),
                ])
                .unwrap();
            let render_pass = |descriptor: &gpui::Font| {
                let font_id = system.font_id(descriptor).unwrap();

                "Ag&"
                    .chars()
                    .enumerate()
                    .map(|(idx, character)| {
                        let step = idx as u8;
                        let glyph = system
                            .rasterize_glyph(&RenderGlyphParams {
                                font_id,
                                glyph_id: system.glyph_for_char(font_id, character).unwrap(),
                                font_size: px(12.0 * f32::from(step + 1)),
                                subpixel_variant: point(step, step),
                                scale_factor: 1.0 + f32::from(step) * 0.5,
                                raster_style: PreparedRasterStyle {
                                    mode: GlyphRenderMode::Grayscale,
                                    color_effect: RasterColorEffect::Dilation(step * 2),
                                    foreground_dependency: gpui::ForegroundDependency::Full,
                                },
                            })
                            .unwrap();
                        glyph.validate().unwrap();
                        assert!(
                            glyph.pixels.iter().any(|&coverage| coverage != 0),
                            "'{character}' produced an empty coverage mask"
                        );

                        glyph
                    })
                    .collect::<Vec<_>>()
            };

            let descriptor = gpui_font("Source Serif 4");
            let first_pass = render_pass(&descriptor);
            let second_pass = render_pass(&descriptor);

            for (character, (expected, actual)) in
                "Ag&".chars().zip(first_pass.iter().zip(&second_pass))
            {
                assert_eq!(
                    actual.bounds, expected.bounds,
                    "bounds changed for '{character}'"
                );
                assert_eq!(
                    actual.pixels, expected.pixels,
                    "pixels changed for '{character}'"
                );
            }

            let normal = render_pass(&gpui_font("Noto Sans"));
            let condensed = render_pass(&gpui::Font {
                width: FontWidth::CONDENSED,
                ..gpui_font("Noto Sans")
            });

            for (normal, condensed) in normal.iter().zip(&condensed) {
                assert!(condensed.size.width < normal.size.width);
                assert_ne!(condensed.pixels, normal.pixels);
            }
        }

        #[test]
        fn core_text_preserves_default_and_nondefault_optical_sizes() {
            let source = FontDataBlob::from(gpui_fonts::SOURCE_SERIF.to_vec());
            let source_data = source_from_blob(source.clone());
            let default_variations = [
                FontVariation::new(*b"opsz", 20.0),
                FontVariation::new(*b"wght", 400.0),
            ];
            let default_face = RasterFace {
                font_id: FontId(1),
                source_id: source.id(),
                source: &source,
                face_index: 0,
                normalized_coords: &[],
                variations: &default_variations,
                synthesis: FontSynthesis::default(),
                has_color_glyphs: false,
            };
            let default_native = NativeFace::new(&default_face, source_data.clone()).unwrap();

            let mut glyph = [0];
            let default_font = font::new_from_descriptor(&default_native.descriptor, 12.0);
            let mapped = unsafe {
                default_font.get_glyphs_for_characters(['A' as u16].as_ptr(), glyph.as_mut_ptr(), 1)
            };
            assert!(mapped);

            for font_size in [12.0, 48.0] {
                let sized_font = font::new_from_descriptor(&default_native.descriptor, font_size);

                assert_eq!(core_text_variation(&sized_font, *b"opsz"), None);
            }

            let nondefault_variations = [
                FontVariation::new(*b"opsz", 12.0),
                FontVariation::new(*b"wght", 400.0),
            ];
            let nondefault_face = RasterFace {
                font_id: FontId(2),
                variations: &nondefault_variations,
                ..default_face
            };
            let nondefault_native = NativeFace::new(&nondefault_face, source_data).unwrap();
            let nondefault_font = font::new_from_descriptor(&nondefault_native.descriptor, 12.0);

            assert_eq!(core_text_variation(&nondefault_font, *b"opsz"), Some(12.0));
            assert_ne!(
                glyph_outline(&default_font, glyph[0]),
                glyph_outline(&nondefault_font, glyph[0])
            );
        }

        #[test]
        fn shaped_optical_instances_reach_core_text_unchanged() {
            let system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Skip,
                "Source Serif 4",
                MacGlyphRenderer::new(),
            )
            .with_automatic_optical_sizing();
            system
                .add_fonts(vec![Cow::Borrowed(*gpui_fonts::SOURCE_SERIF)])
                .unwrap();

            let shaped_glyph = |font_size| {
                let layout = system.layout_text(TextLayoutRequest {
                    text: "A",
                    font_size,
                    runs: &[TextRun {
                        len: 1,
                        font: gpui_font("Source Serif 4"),
                        ..Default::default()
                    }],
                    options: gpui::TextLayoutOptions {
                        text_align: gpui::TextAlign::Left,
                        ..Default::default()
                    },
                });
                let fragment = &layout.paint_fragments[0];

                (fragment.font_id, fragment.glyphs[0].id)
            };
            let small = shaped_glyph(px(12.0));
            let large = shaped_glyph(px(48.0));

            assert_ne!(small.0, large.0);
            assert_eq!(small.1, large.1);

            let rasterize = |(font_id, glyph_id)| {
                system
                    .rasterize_glyph(&RenderGlyphParams {
                        font_id,
                        glyph_id,
                        font_size: px(32.0),
                        subpixel_variant: point(0, 0),
                        scale_factor: 1.0,
                        raster_style: PreparedRasterStyle::independent(GlyphRenderMode::Grayscale),
                    })
                    .unwrap()
            };
            let small_raster = rasterize(small);
            let large_raster = rasterize(large);

            small_raster.validate().unwrap();
            large_raster.validate().unwrap();
            assert_ne!(
                (small_raster.bounds, small_raster.pixels),
                (large_raster.bounds, large_raster.pixels)
            );
        }

        #[test]
        fn explicit_optical_outlines_match_core_texts_automatic_reference() {
            let source = FontDataBlob::from(gpui_fonts::SOURCE_SERIF.to_vec());
            let source_data = source_from_blob(source.clone());
            let descriptor =
                core_text::font_manager::create_font_descriptor_with_data(source_data.data.clone())
                    .unwrap();
            let weight_tag = CFNumber::from(i64::from(u32::from_be_bytes(*b"wght")));
            let weight_value = CFNumber::from(340.0f64);
            let weight_variations = CFDictionary::from_CFType_pairs(&[(weight_tag, weight_value)]);
            let variation_key = unsafe {
                CFString::wrap_under_get_rule(font_descriptor::kCTFontVariationAttribute)
            };
            let variation_value =
                unsafe { CFType::wrap_under_get_rule(weight_variations.as_CFTypeRef()) };
            let automatic_descriptor = descriptor
                .create_copy_with_attributes(
                    CFDictionary::from_CFType_pairs(&[(variation_key, variation_value)])
                        .into_untyped(),
                )
                .unwrap();
            let glyph_for_a = |font: &core_text::font::CTFont| {
                let mut glyph = [0];
                let mapped = unsafe {
                    font.get_glyphs_for_characters(['A' as u16].as_ptr(), glyph.as_mut_ptr(), 1)
                };
                assert!(mapped);

                glyph[0]
            };

            for font_size in [12.0, 48.0] {
                let automatic_font = font::new_from_descriptor(&automatic_descriptor, font_size);
                let variations = [
                    FontVariation::new(*b"wght", 340.0),
                    FontVariation::new(*b"opsz", font_size as f32),
                ];
                let explicit_face = RasterFace {
                    font_id: FontId(1),
                    source_id: source.id(),
                    source: &source,
                    face_index: 0,
                    normalized_coords: &[],
                    variations: &variations,
                    synthesis: FontSynthesis::default(),
                    has_color_glyphs: false,
                };
                let explicit_native = NativeFace::new(&explicit_face, source_data.clone()).unwrap();
                let explicit_font =
                    font::new_from_descriptor(&explicit_native.descriptor, font_size);
                let automatic_glyph = glyph_for_a(&automatic_font);
                let explicit_glyph = glyph_for_a(&explicit_font);

                assert_eq!(explicit_glyph, automatic_glyph);
                assert_eq!(
                    glyph_outline(&explicit_font, explicit_glyph),
                    glyph_outline(&automatic_font, automatic_glyph),
                    "explicit opsz={font_size} changed the CoreText reference outline"
                );
            }
        }

        #[test]
        fn system_optical_families_shape_and_rasterize() {
            let system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Load,
                ".AppleSystemUIFont",
                MacGlyphRenderer::new(),
            )
            .with_automatic_optical_sizing();
            let text = "Hamburgefontsiv";

            for family in [".AppleSystemUIFont", "New York"] {
                for font_size in [px(12.0), px(48.0)] {
                    let layout = system.layout_text(TextLayoutRequest {
                        text,
                        font_size,
                        runs: &[TextRun {
                            len: text.len(),
                            font: gpui_font(family),
                            ..Default::default()
                        }],
                        options: gpui::TextLayoutOptions {
                            text_align: gpui::TextAlign::Left,
                            ..Default::default()
                        },
                    });
                    let fragment = layout
                        .paint_fragments
                        .first()
                        .unwrap_or_else(|| panic!("{family} produced no shaped text"));
                    let glyph = fragment
                        .glyphs
                        .first()
                        .unwrap_or_else(|| panic!("{family} produced no shaped glyphs"));

                    for scale_factor in [1.0, 1.5, 2.0] {
                        let raster = system
                            .rasterize_glyph(&RenderGlyphParams {
                                font_id: fragment.font_id,
                                glyph_id: glyph.id,
                                font_size,
                                subpixel_variant: point(0, 0),
                                scale_factor,
                                raster_style: PreparedRasterStyle::independent(
                                    GlyphRenderMode::Grayscale,
                                ),
                            })
                            .unwrap_or_else(|error| {
                                panic!(
                                    "failed to rasterize {family} at {font_size:?} and scale {scale_factor}: {error:#}"
                                )
                            });
                        raster.validate().unwrap();
                        assert!(
                            raster.pixels.iter().any(|coverage| *coverage != 0),
                            "{family} produced an empty glyph at {font_size:?} and scale {scale_factor}"
                        );
                    }
                }
            }
        }

        #[test]
        fn synthetic_bold_keeps_stroked_tips_inside_the_raster() {
            let system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Skip,
                "IBM Plex Sans",
                MacGlyphRenderer::new(),
            );
            system
                .add_fonts(vec![Cow::Borrowed(*gpui_fonts::IBM_PLEX)])
                .unwrap();
            let font_id = system.font_id(&gpui_font("IBM Plex Sans").bold()).unwrap();
            let glyph_id = system.glyph_for_char(font_id, 'A').unwrap();
            let raster = system
                .rasterize_glyph(&RenderGlyphParams {
                    font_id,
                    glyph_id,
                    font_size: px(48.0),
                    subpixel_variant: point(0, 0),
                    scale_factor: 2.0,
                    raster_style: PreparedRasterStyle {
                        mode: GlyphRenderMode::Grayscale,
                        color_effect: RasterColorEffect::Dilation(0),
                        foreground_dependency: gpui::ForegroundDependency::Full,
                    },
                })
                .unwrap();
            let width = raster.size.width.0 as usize;
            let height = raster.size.height.0 as usize;

            assert!(raster.pixels[..width].iter().all(|&pixel| pixel == 0));
            assert!(
                raster.pixels[(height - 1) * width..]
                    .iter()
                    .all(|&pixel| pixel == 0)
            );
            assert!(
                raster
                    .pixels
                    .chunks_exact(width)
                    .all(|row| { row.first() == Some(&0) && row.last() == Some(&0) })
            );
        }

        fn core_text_variation(font: &core_text::font::CTFont, tag: [u8; 4]) -> Option<f64> {
            let variations_ref = unsafe { CTFontCopyVariation(font.as_concrete_TypeRef()) };
            if variations_ref.is_null() {
                return None;
            }

            let variations = unsafe {
                CFDictionary::<CFNumber, CFNumber>::wrap_under_create_rule(variations_ref)
            };
            let tag = CFNumber::from(i64::from(u32::from_be_bytes(tag)));

            variations.find(tag).and_then(|value| value.to_f64())
        }

        fn glyph_outline(
            font: &core_text::font::CTFont,
            glyph: u16,
        ) -> Vec<(i32, Vec<(u64, u64)>)> {
            let transform = CGAffineTransform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
            let path = font.create_path_for_glyph(glyph, &transform).unwrap();
            let mut outline = Vec::new();
            path.apply(&|element| {
                outline.push((
                    element.element_type as i32,
                    element
                        .points()
                        .iter()
                        .map(|point| (point.x.to_bits(), point.y.to_bits()))
                        .collect(),
                ));
            });

            outline
        }

        #[test]
        fn core_text_obeys_platform_style_mask_color_baseline_and_empty_glyph_behavior() {
            let system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Skip,
                "Source Serif 4",
                MacGlyphRenderer::new(),
            );
            system
                .add_fonts(vec![Cow::Borrowed(*gpui_fonts::SOURCE_SERIF)])
                .unwrap();
            let font_id = system
                .font_id(&gpui_font("Source Serif 4").bold().italic())
                .unwrap();

            let render_style = |glyph_id: GlyphId, raster_style, variant| {
                system
                    .rasterize_glyph(&RenderGlyphParams {
                        font_id,
                        glyph_id,
                        font_size: px(24.0),
                        subpixel_variant: variant,
                        scale_factor: 2.0,
                        raster_style,
                    })
                    .unwrap()
            };

            let render = |glyph_id: GlyphId, mode, color, variant| {
                render_style(
                    glyph_id,
                    system.prepare_raster_style(RasterStyleRequest {
                        font_id,
                        glyph_id,
                        scene_color: color,
                        requested_mode: mode,
                        foreground_dependency: gpui::ForegroundDependency::Full,
                    }),
                    variant,
                )
            };

            let letter = system.glyph_for_char(font_id, 'A').unwrap();
            let normalized_subpixel = system.prepare_raster_style(RasterStyleRequest {
                font_id,
                glyph_id: letter,
                scene_color: rgba(0x303030ff),
                requested_mode: GlyphRenderMode::Subpixel,
                foreground_dependency: gpui::ForegroundDependency::Full,
            });

            assert_eq!(normalized_subpixel.mode, GlyphRenderMode::Grayscale);

            let light_style = system.prepare_raster_style(RasterStyleRequest {
                font_id,
                glyph_id: letter,
                scene_color: rgba(0xffffffff),
                requested_mode: GlyphRenderMode::Grayscale,
                foreground_dependency: gpui::ForegroundDependency::Full,
            });

            let expected_light_effect = if font_smoothing_allowed_by_user() {
                RasterColorEffect::Dilation(4)
            } else {
                RasterColorEffect::Independent
            };
            assert_eq!(light_style.color_effect, expected_light_effect);

            for (color, expected_level) in [(0x000000ff, 0), (0x1e1e1eff, 0), (0x222222ff, 1)] {
                let style = system.prepare_raster_style(RasterStyleRequest {
                    font_id,
                    glyph_id: letter,
                    scene_color: rgba(color),
                    requested_mode: GlyphRenderMode::Grayscale,
                    foreground_dependency: gpui::ForegroundDependency::Full,
                });
                let expected_effect = if font_smoothing_allowed_by_user() {
                    RasterColorEffect::Dilation(expected_level)
                } else {
                    RasterColorEffect::Independent
                };

                assert_eq!(style.color_effect, expected_effect);
            }

            let undilated = render_style(
                letter,
                PreparedRasterStyle {
                    mode: GlyphRenderMode::Grayscale,
                    color_effect: RasterColorEffect::Dilation(0),
                    foreground_dependency: gpui::ForegroundDependency::Full,
                },
                point(0, 0),
            );
            let dilated = render_style(
                letter,
                PreparedRasterStyle {
                    mode: GlyphRenderMode::Grayscale,
                    color_effect: RasterColorEffect::Dilation(4),
                    foreground_dependency: gpui::ForegroundDependency::Full,
                },
                point(0, 0),
            );
            assert_ne!(undilated.pixels, dilated.pixels);

            let smoothing_disabled = render_style(
                letter,
                PreparedRasterStyle::independent(GlyphRenderMode::Grayscale),
                point(0, 0),
            );
            assert_eq!(undilated.pixels, smoothing_disabled.pixels);

            for subpixel_x in 1..SUBPIXEL_VARIANTS_X {
                let shifted = render_style(
                    letter,
                    PreparedRasterStyle {
                        mode: GlyphRenderMode::Grayscale,
                        color_effect: RasterColorEffect::Dilation(0),
                        foreground_dependency: gpui::ForegroundDependency::Full,
                    },
                    point(subpixel_x, 0),
                );
                assert_eq!(shifted.bounds.origin, undilated.bounds.origin);
                assert_eq!(shifted.size.height, undilated.size.height);
                assert_eq!(shifted.size.width.0, undilated.size.width.0 + 1);
            }

            let mask = render(
                letter,
                GlyphRenderMode::Grayscale,
                rgba(0x303030ff),
                point(3, 0),
            );
            assert_eq!(mask.format, RasterizedGlyphFormat::AlphaMask);
            assert_eq!(mask.bounds.size, mask.size);
            assert!(mask.bounds.origin.y.0 < 0);
            assert!(mask.size.width.0 > 0 && mask.size.height.0 > 0);
            mask.validate().unwrap();

            let color = render(
                letter,
                GlyphRenderMode::Color,
                rgba(0xe02010ff),
                point(1, 0),
            );
            assert_eq!(color.format, RasterizedGlyphFormat::BgraColor);
            color.validate().unwrap();
            let colored_pixel = color
                .pixels
                .chunks_exact(4)
                .find(|pixel| pixel[3] > 128)
                .expect("colored glyph pixel");
            assert!(colored_pixel[2] > colored_pixel[0], "{colored_pixel:?}");

            let space = system.glyph_for_char(font_id, ' ').unwrap();
            let empty = render(
                space,
                GlyphRenderMode::Grayscale,
                rgba(0x000000ff),
                point(0, 0),
            );
            assert_eq!(empty.size, gpui::Size::default());
            assert!(empty.pixels.is_empty());

            let zero_size = system
                .rasterize_glyph(&RenderGlyphParams {
                    font_id,
                    glyph_id: letter,
                    font_size: px(0.0),
                    subpixel_variant: point(0, 0),
                    scale_factor: 2.0,
                    raster_style: PreparedRasterStyle::independent(GlyphRenderMode::Grayscale),
                })
                .unwrap();
            assert_eq!(zero_size.size, gpui::Size::default());
            assert!(zero_size.pixels.is_empty());

            let emoji_system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Load,
                ".AppleSystemUIFont",
                MacGlyphRenderer::new(),
            );
            let emoji_font = emoji_system
                .font_id(&gpui_font("Apple Color Emoji"))
                .expect("Apple Color Emoji is available on macOS");
            let emoji_glyph = emoji_system.glyph_for_char(emoji_font, '😀').unwrap();
            let emoji = emoji_system
                .rasterize_glyph(&RenderGlyphParams {
                    font_id: emoji_font,
                    glyph_id: emoji_glyph,
                    font_size: px(24.0),
                    subpixel_variant: point(2, 0),
                    scale_factor: 2.0,
                    raster_style: emoji_system.prepare_raster_style(RasterStyleRequest {
                        font_id: emoji_font,
                        glyph_id: emoji_glyph,
                        scene_color: rgba(0xffffffff),
                        requested_mode: GlyphRenderMode::Color,
                        foreground_dependency: gpui::ForegroundDependency::Full,
                    }),
                })
                .unwrap();
            assert_eq!(emoji.format, RasterizedGlyphFormat::BgraColor);
            emoji.validate().unwrap();
            assert!(emoji.pixels.chunks_exact(4).any(|pixel| {
                pixel[3] > 128
                    && (pixel[0].abs_diff(pixel[1]) > 20
                        || pixel[1].abs_diff(pixel[2]) > 20
                        || pixel[0].abs_diff(pixel[2]) > 20)
            }));

            let transparent_style = emoji_system.prepare_raster_style(RasterStyleRequest {
                font_id: emoji_font,
                glyph_id: emoji_glyph,
                scene_color: rgba(0xff000000),
                requested_mode: GlyphRenderMode::Color,
                foreground_dependency: gpui::ForegroundDependency::Full,
            });
            assert_eq!(
                transparent_style.color_effect,
                RasterColorEffect::Preblend(Rgba8::new(0, 0, 0, 0))
            );
            let transparent_emoji = emoji_system
                .rasterize_glyph(&RenderGlyphParams {
                    font_id: emoji_font,
                    glyph_id: emoji_glyph,
                    font_size: px(24.0),
                    subpixel_variant: point(2, 0),
                    scale_factor: 2.0,
                    raster_style: transparent_style,
                })
                .unwrap();
            transparent_emoji.validate().unwrap();
            assert!(
                transparent_emoji
                    .pixels
                    .chunks_exact(4)
                    .all(|pixel| pixel[3] == 0)
            );

            let directional_text = "🏃‍➡️";
            let directional_layout = emoji_system.layout_text(TextLayoutRequest {
                text: directional_text,
                font_size: px(24.0),
                runs: &[TextRun {
                    len: directional_text.len(),
                    font: gpui_font("Apple Color Emoji"),
                    ..Default::default()
                }],
                options: gpui::TextLayoutOptions {
                    text_align: gpui::TextAlign::Left,
                    ..Default::default()
                },
            });
            let directional_glyph = directional_layout
                .paint_fragments
                .iter()
                .flat_map(|fragment| {
                    fragment
                        .glyphs
                        .iter()
                        .map(move |glyph| (fragment.font_id, glyph))
                })
                .find(|(_, glyph)| glyph.is_emoji)
                .expect("directional emoji should select native bitmap artwork");
            let directional = emoji_system
                .rasterize_glyph(&RenderGlyphParams {
                    font_id: directional_glyph.0,
                    glyph_id: directional_glyph.1.id,
                    font_size: px(24.0),
                    subpixel_variant: point(0, 0),
                    scale_factor: 2.0,
                    raster_style: emoji_system.prepare_raster_style(RasterStyleRequest {
                        font_id: directional_glyph.0,
                        glyph_id: directional_glyph.1.id,
                        scene_color: rgba(0xffffffff),
                        requested_mode: GlyphRenderMode::Color,
                        foreground_dependency: gpui::ForegroundDependency::Full,
                    }),
                })
                .unwrap();
            assert_eq!(directional.format, RasterizedGlyphFormat::BgraColor);
            directional.validate().unwrap();
            assert!(directional.pixels.chunks_exact(4).any(|pixel| {
                pixel[3] > 128
                    && (pixel[0].abs_diff(pixel[1]) > 20
                        || pixel[1].abs_diff(pixel[2]) > 20
                        || pixel[0].abs_diff(pixel[2]) > 20)
            }));
        }

        #[test]
        fn native_color_bounds_follow_visible_artwork_at_the_requested_size() {
            let system = ParleyTextSystem::new_with_rasterizer(
                SystemFonts::Load,
                ".AppleSystemUIFont",
                MacGlyphRenderer::new(),
            );
            let font_id = system
                .font_id(&gpui_font("Apple Color Emoji"))
                .expect("Apple Color Emoji is available on macOS");
            let glyph_id = system.glyph_for_char(font_id, '😀').unwrap();

            for font_size in [16.0, 24.0, 32.0] {
                for scale_factor in [1.0, 1.5, 2.0] {
                    let raster = system
                        .rasterize_glyph(&RenderGlyphParams {
                            font_id,
                            glyph_id,
                            font_size: px(font_size),
                            subpixel_variant: point(0, 0),
                            scale_factor,
                            raster_style: system.prepare_raster_style(RasterStyleRequest {
                                font_id,
                                glyph_id,
                                scene_color: rgba(0xffffffff),
                                requested_mode: GlyphRenderMode::Color,
                                foreground_dependency: gpui::ForegroundDependency::Full,
                            }),
                        })
                        .unwrap();
                    raster.validate().unwrap();
                    let width = raster.size.width.0 as usize;
                    let height = raster.size.height.0 as usize;
                    let mut left = width;
                    let mut top = height;
                    let mut right = 0;
                    let mut bottom = 0;

                    for (pixel_idx, pixel) in raster.pixels.chunks_exact(4).enumerate() {
                        if pixel[3] == 0 {
                            continue;
                        }

                        let pixel_x = pixel_idx % width;
                        let pixel_y = pixel_idx / width;
                        left = left.min(pixel_x);
                        top = top.min(pixel_y);
                        right = right.max(pixel_x + 1);
                        bottom = bottom.max(pixel_y + 1);
                    }

                    assert!(right > left && bottom > top);
                    let horizontal_padding = width - (right - left);
                    let vertical_padding = height - (bottom - top);
                    let maximum_padding = (3.0 * scale_factor).ceil() as usize;
                    assert!(
                        horizontal_padding <= maximum_padding,
                        "font size {font_size} at scale {scale_factor} left {horizontal_padding}px of horizontal transparent padding"
                    );
                    assert!(
                        vertical_padding <= maximum_padding,
                        "font size {font_size} at scale {scale_factor} left {vertical_padding}px of vertical transparent padding"
                    );
                }
            }
        }
    }
}
