mod font;
mod font_fallbacks;
mod font_features;
mod font_metrics;
mod font_style;
mod font_weight;
mod font_width;
mod glyph;
mod inline;
mod line;
mod line_layout;
mod raster;
mod text_color;
mod text_layout;
mod text_run;
mod text_system;
mod window_text_system;

pub use font::*;
pub use font_fallbacks::*;
pub use font_features::*;
pub use font_metrics::*;
pub use font_style::*;
pub use font_weight::*;
pub use font_width::*;
pub use glyph::*;
pub use inline::*;
pub use line::*;
pub use line_layout::*;
pub use raster::*;
pub use text_color::*;
pub use text_layout::*;
pub use text_run::*;
pub use text_system::*;
pub use window_text_system::*;

/// Number of subpixel glyph variants along the X axis.
pub const SUBPIXEL_VARIANTS_X: u8 = 4;

/// Number of subpixel glyph variants along the Y axis.
pub const SUBPIXEL_VARIANTS_Y: u8 = 1;

#[cfg(test)]
mod text_range_tests {
    use crate::util::TextRangeExt;

    #[test]
    fn contained_ranges_are_valid_utf8_slices() {
        let text = "aé日";
        #[allow(
            clippy::reversed_empty_ranges,
            reason = "The reversed range is intentional invalid input for contains_range."
        )]
        let cases = [
            ("valid ASCII", 0..1, true),
            ("valid multibyte", 1..3, true),
            ("empty", 1..1, true),
            ("out of bounds", 0..text.len() + 1, false),
            ("reversed", 3..1, false),
            ("split UTF-8 code point", 2..3, false),
        ];

        for (name, range, expected) in cases {
            assert_eq!(text.contains_range(&range), expected, "{name}");
        }
    }
}

#[cfg(test)]
mod layout_cache_tests {
    use super::*;
    use crate::{TestTextSystem, TextAlign, hsla, px};
    use gpui_shared_string::SharedString;
    use std::sync::Arc;

    fn window_text_system() -> WindowTextSystem {
        WindowTextSystem::new(Arc::new(TextSystem::new(Arc::new(TestTextSystem))))
    }

    #[test]
    fn paint_changes_reuse_shaped_geometry() {
        let system = window_text_system();
        let text = SharedString::from("paint cache");
        let mut run = TextRun {
            len: text.len(),
            color: hsla(0.0, 0.8, 0.4, 1.0),
            ..Default::default()
        };
        let first = system
            .shape_text(text.clone(), px(16.0), &[run.clone()], None, None)
            .unwrap();

        run.color = hsla(0.6, 0.8, 0.4, 1.0);
        run.underline = Some(crate::UnderlineStyle {
            thickness: px(1.5),
            color: None,
            wavy: false,
        });
        let second = system
            .shape_text(text, px(16.0), std::slice::from_ref(&run), None, None)
            .unwrap();

        assert!(Arc::ptr_eq(&first.platform_layout, &second.platform_layout));
        assert_eq!(second.paint_fragments[0].style, PaintStyle::from(&run));
        assert!(second.paint_fragments[0].underline_offset.is_some());
    }

    #[test]
    fn inline_layouts_reuse_all_valid_widths_across_frames() {
        let system = window_text_system();
        let text = "cached inline paragraph";
        let runs = [TextRun {
            len: text.len(),
            ..Default::default()
        }];
        let request = |wrap_width| InlineLayoutRequest {
            text,
            runs: &runs,
            text_styles: &[],
            boxes: &[],
            font_size: px(16.0),
            line_height: px(20.0),
            text_metrics: InlineTextMetrics::default(),
            options: TextLayoutOptions {
                wrap_width: Some(wrap_width),
                text_align: TextAlign::Left,
                ..Default::default()
            },
            bidi_scopes: &[],
        };

        let wide = system.layout_inline(request(px(200.0)));
        let narrow = system.layout_inline(request(px(80.0)));
        assert!(!Arc::ptr_eq(&wide, &narrow));

        let painted_runs = [TextRun {
            color: hsla(0.5, 0.8, 0.4, 1.0),
            ..runs[0].clone()
        }];
        let changed_paint = system.layout_inline(InlineLayoutRequest {
            runs: &painted_runs,
            ..request(px(200.0))
        });
        let changed_size = system.layout_inline(InlineLayoutRequest {
            font_size: px(18.0),
            ..request(px(200.0))
        });
        assert!(!Arc::ptr_eq(&wide, &changed_paint));
        assert!(!Arc::ptr_eq(&wide, &changed_size));
        assert!(Arc::ptr_eq(
            &wide,
            &system.layout_inline(request(px(200.0)))
        ));

        system.finish_frame();
        assert!(Arc::ptr_eq(
            &wide,
            &system.layout_inline(request(px(200.0)))
        ));
        assert!(Arc::ptr_eq(
            &narrow,
            &system.layout_inline(request(px(80.0)))
        ));
    }
}

#[cfg(test)]
mod raster_contract_tests {
    use super::*;
    use crate::{
        AtlasKey, AtlasTextureKind, Bounds, DevicePixels, PlatformAtlas, PlatformTextSystem, Point,
        Result, Size, TestAtlas, TestTextSystem, hsla, point, px, size,
    };
    use std::{
        borrow::Cow,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    fn params(style: PreparedRasterStyle) -> RenderGlyphParams {
        RenderGlyphParams {
            font_id: FontId(7),
            glyph_id: GlyphId(42),
            font_size: px(16.0),
            subpixel_variant: point(1, 0),
            scale_factor: 2.0,
            raster_style: style,
        }
    }

    #[test]
    fn raster_buffers_enforce_the_documented_format_contract() {
        let cases = [
            (
                "alpha mask",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: point(DevicePixels(-1), DevicePixels(-2)),
                        size: size(DevicePixels(2), DevicePixels(1)),
                    },
                    size: size(DevicePixels(2), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: vec![0, 255],
                },
                true,
            ),
            (
                "subpixel mask",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: point(DevicePixels(1), DevicePixels(-2)),
                        size: size(DevicePixels(1), DevicePixels(2)),
                    },
                    size: size(DevicePixels(1), DevicePixels(2)),
                    format: RasterizedGlyphFormat::BgraSubpixelMask,
                    pixels: vec![0; 8],
                },
                true,
            ),
            (
                "straight-alpha color",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: point(DevicePixels(0), DevicePixels(-1)),
                        size: size(DevicePixels(1), DevicePixels(1)),
                    },
                    size: size(DevicePixels(1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::BgraColor,
                    pixels: vec![1, 2, 3, 4],
                },
                true,
            ),
            (
                "empty glyph",
                RasterizedGlyph::empty(RasterizedGlyphFormat::AlphaMask),
                true,
            ),
            (
                "wrong alpha stride",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(2), DevicePixels(1)),
                    },
                    size: size(DevicePixels(2), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: vec![0],
                },
                false,
            ),
            (
                "wrong BGRA stride",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(1), DevicePixels(1)),
                    },
                    size: size(DevicePixels(1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::BgraColor,
                    pixels: vec![0; 3],
                },
                false,
            ),
            (
                "partially empty dimensions",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(0), DevicePixels(1)),
                    },
                    size: size(DevicePixels(0), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: Vec::new(),
                },
                false,
            ),
            (
                "negative dimensions",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(-1), DevicePixels(1)),
                    },
                    size: size(DevicePixels(-1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: Vec::new(),
                },
                false,
            ),
            (
                "bounds and buffer size disagree",
                RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(2), DevicePixels(1)),
                    },
                    size: size(DevicePixels(1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: vec![255],
                },
                false,
            ),
        ];

        for (description, raster, valid) in cases {
            assert_eq!(raster.validate().is_ok(), valid, "{description}");
        }
    }

    #[test]
    fn prepared_style_and_format_drive_atlas_keys_without_caching_pixels() {
        let backend = Arc::new(SequencedRasterizer::default());
        let text_system = TextSystem::new(backend.clone());
        let style_a = text_system.prepare_raster_style(
            FontId(7),
            GlyphId(42),
            hsla(0.0, 0.0, 0.20, 1.0),
            GlyphRenderMode::Grayscale,
        );
        let style_b = text_system.prepare_raster_style(
            FontId(7),
            GlyphId(42),
            hsla(0.0, 0.0, 0.24, 1.0),
            GlyphRenderMode::Grayscale,
        );
        let style_c = text_system.prepare_raster_style(
            FontId(7),
            GlyphId(42),
            hsla(0.0, 0.0, 0.30, 1.0),
            GlyphRenderMode::Grayscale,
        );
        assert_eq!(style_a, style_b, "nearby scene colors share a mask style");
        assert_ne!(
            style_a, style_c,
            "different dilation levels need distinct rasters"
        );

        let first = params(style_a);
        let second = params(style_b);
        let third = params(style_c);
        assert_eq!(first, second);
        let first_raster = text_system.rasterize_glyph(&first).unwrap();
        let reused_raster = text_system.rasterize_glyph(&second).unwrap();
        let distinct_raster = text_system.rasterize_glyph(&third).unwrap();
        assert_eq!(first_raster.pixels, reused_raster.pixels);
        assert_eq!(first_raster.pixels, distinct_raster.pixels);
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 3);
        for (format, expected) in [
            (
                RasterizedGlyphFormat::AlphaMask,
                AtlasTextureKind::Monochrome,
            ),
            (
                RasterizedGlyphFormat::BgraSubpixelMask,
                AtlasTextureKind::Subpixel,
            ),
            (
                RasterizedGlyphFormat::BgraColor,
                AtlasTextureKind::Polychrome,
            ),
        ] {
            assert_eq!(
                AtlasKey::from((first.clone(), format)).texture_kind(),
                expected
            );
        }
    }

    #[test]
    fn atlas_entries_own_their_raster_metadata_and_retry_failures() {
        let backend = Arc::new(SequencedRasterizer {
            attempts: AtomicUsize::new(0),
            fail_until_valid: true,
        });
        let text_system = TextSystem::new(backend.clone());
        let params = params(PreparedRasterStyle::independent(GlyphRenderMode::Grayscale));
        let atlas = TestAtlas::new();

        assert!(
            atlas
                .get_or_insert_glyph_with(&params, &mut || { text_system.rasterize_glyph(&params) })
                .is_err()
        );
        assert!(
            atlas
                .get_or_insert_glyph_with(&params, &mut || { text_system.rasterize_glyph(&params) })
                .is_err()
        );
        let first_success = atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        let second_success = atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();

        assert_eq!(first_success, second_success);
        assert_eq!(
            first_success.bounds.size,
            size(DevicePixels(1), DevicePixels(1))
        );
        assert_eq!(first_success.format, RasterizedGlyphFormat::AlphaMask);
        assert!(first_success.tile.is_some());
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 3);

        atlas.clear();
        atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn separate_atlases_keep_the_metadata_for_their_own_pixels() {
        let backend = Arc::new(ChangingRasterizer::default());
        let text_system = TextSystem::new(backend.clone());
        let params = params(PreparedRasterStyle::independent(GlyphRenderMode::Color));
        let first_atlas = TestAtlas::new();
        let second_atlas = TestAtlas::new();

        let silhouette = first_atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        let silhouette_hit = first_atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        assert_eq!(silhouette_hit, silhouette);
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 1);

        let color = second_atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        assert_ne!(color.bounds, silhouette.bounds);
        assert_ne!(color.format, silhouette.format);
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 2);

        let original = first_atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        assert_eq!(original, silhouette);
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 2);

        first_atlas.clear();
        let recovered = first_atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))
            .unwrap();
        assert_ne!(recovered.bounds, silhouette.bounds);
        assert_eq!(recovered.format, RasterizedGlyphFormat::BgraColor);
        assert_eq!(backend.attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn successful_empty_glyphs_are_cached_without_a_tile() {
        let atlas = TestAtlas::new();
        let params = params(PreparedRasterStyle::independent(GlyphRenderMode::Grayscale));
        let mut builds = 0;
        let mut build = || {
            builds += 1;

            ValidatedRasterizedGlyph::new(RasterizedGlyph::empty(RasterizedGlyphFormat::AlphaMask))
        };

        let first = atlas.get_or_insert_glyph_with(&params, &mut build).unwrap();
        let second = atlas.get_or_insert_glyph_with(&params, &mut build).unwrap();
        assert_eq!(first, second);
        assert!(first.tile.is_none());
        assert_eq!(builds, 1);
    }

    #[test]
    fn custom_glyph_builders_validate_before_caching() {
        let atlas = TestAtlas::new();
        let params = params(PreparedRasterStyle::independent(GlyphRenderMode::Grayscale));
        let mut builds = 0;
        let mut build = || {
            builds += 1;
            let size = size(DevicePixels(1), DevicePixels(1));
            let pixels = if builds == 1 { Vec::new() } else { vec![255] };

            ValidatedRasterizedGlyph::new(RasterizedGlyph {
                bounds: Bounds {
                    origin: Point::default(),
                    size,
                },
                size,
                format: RasterizedGlyphFormat::AlphaMask,
                pixels,
            })
        };

        assert!(atlas.get_or_insert_glyph_with(&params, &mut build).is_err());

        let first = atlas.get_or_insert_glyph_with(&params, &mut build).unwrap();
        let second = atlas.get_or_insert_glyph_with(&params, &mut build).unwrap();
        assert_eq!(first, second);
        assert!(first.tile.is_some());
        assert_eq!(builds, 2);
    }

    #[derive(Default)]
    struct ChangingRasterizer {
        attempts: AtomicUsize,
    }

    impl PlatformTextSystem for ChangingRasterizer {
        fn add_fonts(&self, _fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
            Ok(())
        }

        fn all_font_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn font_id(&self, descriptor: &Font) -> Result<FontId> {
            PlatformTextSystem::font_id(&TestTextSystem, descriptor)
        }

        fn font_metrics(&self, font_id: FontId) -> FontMetrics {
            PlatformTextSystem::font_metrics(&TestTextSystem, font_id)
        }

        fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
            PlatformTextSystem::typographic_bounds(&TestTextSystem, font_id, glyph_id)
        }

        fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
            PlatformTextSystem::advance(&TestTextSystem, font_id, glyph_id)
        }

        fn glyph_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId> {
            PlatformTextSystem::glyph_for_char(&TestTextSystem, font_id, character)
        }

        fn rasterize_glyph(&self, _params: &RenderGlyphParams) -> Result<RasterizedGlyph> {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
            let (origin, size, format) = match attempt {
                0 => (
                    point(DevicePixels(-2), DevicePixels(-7)),
                    size(DevicePixels(2), DevicePixels(3)),
                    RasterizedGlyphFormat::AlphaMask,
                ),
                1 => (
                    point(DevicePixels(1), DevicePixels(-9)),
                    size(DevicePixels(4), DevicePixels(5)),
                    RasterizedGlyphFormat::BgraColor,
                ),
                _ => (
                    point(DevicePixels(3), DevicePixels(-8)),
                    size(DevicePixels(3), DevicePixels(4)),
                    RasterizedGlyphFormat::BgraColor,
                ),
            };
            let bytes_per_pixel = if format == RasterizedGlyphFormat::AlphaMask {
                1
            } else {
                4
            };

            Ok(RasterizedGlyph {
                bounds: Bounds { origin, size },
                size,
                format,
                pixels: vec![
                    0x7f;
                    size.width.0 as usize * size.height.0 as usize * bytes_per_pixel
                ],
            })
        }

        fn layout_text(&self, request: TextLayoutRequest<'_>) -> LineLayout {
            PlatformTextSystem::layout_text(&TestTextSystem, request)
        }

        fn layout_inline(&self, request: InlineLayoutRequest<'_>) -> InlineLayout {
            PlatformTextSystem::layout_inline(&TestTextSystem, request)
        }
    }

    #[derive(Default)]
    struct SequencedRasterizer {
        attempts: AtomicUsize,
        fail_until_valid: bool,
    }

    impl PlatformTextSystem for SequencedRasterizer {
        fn add_fonts(&self, _fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
            Ok(())
        }

        fn all_font_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn font_id(&self, descriptor: &Font) -> Result<FontId> {
            PlatformTextSystem::font_id(&TestTextSystem, descriptor)
        }

        fn font_metrics(&self, font_id: FontId) -> FontMetrics {
            PlatformTextSystem::font_metrics(&TestTextSystem, font_id)
        }

        fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
            PlatformTextSystem::typographic_bounds(&TestTextSystem, font_id, glyph_id)
        }

        fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
            PlatformTextSystem::advance(&TestTextSystem, font_id, glyph_id)
        }

        fn glyph_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId> {
            PlatformTextSystem::glyph_for_char(&TestTextSystem, font_id, character)
        }

        fn rasterize_glyph(&self, _params: &RenderGlyphParams) -> Result<RasterizedGlyph> {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
            if !self.fail_until_valid {
                return Ok(RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(1), DevicePixels(1)),
                    },
                    size: size(DevicePixels(1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: vec![0x7f],
                });
            }
            match attempt {
                0 => Err(anyhow::anyhow!("transient native failure")),
                1 => Ok(RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(1), DevicePixels(1)),
                    },
                    size: size(DevicePixels(1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: Vec::new(),
                }),
                _ => Ok(RasterizedGlyph {
                    bounds: Bounds {
                        origin: Point::default(),
                        size: size(DevicePixels(1), DevicePixels(1)),
                    },
                    size: size(DevicePixels(1), DevicePixels(1)),
                    format: RasterizedGlyphFormat::AlphaMask,
                    pixels: vec![0x7f],
                }),
            }
        }

        fn prepare_raster_style(&self, request: RasterStyleRequest) -> PreparedRasterStyle {
            PreparedRasterStyle {
                mode: request.requested_mode,
                color_effect: RasterColorEffect::Dilation(
                    (request.scene_color.red * 4.0).floor() as u8
                ),
                foreground_dependency: request.foreground_dependency,
            }
        }

        fn layout_text(&self, request: TextLayoutRequest<'_>) -> LineLayout {
            PlatformTextSystem::layout_text(&TestTextSystem, request)
        }

        fn layout_inline(&self, request: InlineLayoutRequest<'_>) -> InlineLayout {
            PlatformTextSystem::layout_inline(&TestTextSystem, request)
        }
    }
}
