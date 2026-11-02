use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use gpui::{
    FontFallbacks, GlyphRenderMode, PlatformTextSystem, PreparedRasterStyle, RenderGlyphParams,
    TextAlign, TextLayoutOptions, TextLayoutRequest, TextRun, TextSystem, font, px,
};
use gpui_ce_parley::{ParleyTextSystem, SystemFonts};
use gpui_fonts::{IBM_PLEX, LILEX, SOURCE_SERIF};
use std::borrow::Cow;
use std::sync::Arc;

fn text_system() -> (ParleyTextSystem, TextRun) {
    let system = ParleyTextSystem::new_with_system_font(SystemFonts::Skip, IBM_PLEX.family);
    system
        .add_fonts(vec![
            Cow::Borrowed(IBM_PLEX.data),
            Cow::Borrowed(LILEX.data),
            Cow::Borrowed(SOURCE_SERIF.data),
        ])
        .unwrap();
    let mut descriptor = font(IBM_PLEX.family);
    descriptor.fallbacks = Some(FontFallbacks::from_fonts(vec![
        SOURCE_SERIF.family.to_string(),
        LILEX.family.to_string(),
    ]));
    (
        system,
        TextRun {
            len: 0,
            font: descriptor,
            letter_spacing: None,
            ..Default::default()
        },
    )
}

fn code_text() -> String {
    "fn layout(text: &str, width: Pixels) -> LineLayout { cache.get(text, width) } ".repeat(64)
}

fn raster_case() -> (TextSystem, RenderGlyphParams) {
    let (system, mut run) = text_system();
    let text = "A";
    run.len = text.len();
    let layout = system.layout_text(TextLayoutRequest {
        text,
        font_size: px(16.0),
        runs: &[run],
        options: TextLayoutOptions {
            text_align: TextAlign::Left,
            ..Default::default()
        },
    });

    let fragment = &layout.paint_fragments[0];
    let glyph = &fragment.glyphs[0];
    let params = RenderGlyphParams {
        font_id: fragment.font_id,
        glyph_id: glyph.id,
        font_size: layout.font_size,
        subpixel_variant: Default::default(),
        scale_factor: 1.0,
        raster_style: PreparedRasterStyle::independent(if glyph.is_emoji {
            GlyphRenderMode::Color
        } else {
            GlyphRenderMode::Grayscale
        }),
    };

    (TextSystem::new(Arc::new(system)), params)
}

fn bench_text_pipeline(criterion: &mut Criterion) {
    let (system, base_run) = text_system();
    let code = code_text();
    let multilingual =
        "office cafe\u{301} العربية אבג 日本語 ไทย 👩🏽‍💻 🇬🇧 one two three four".repeat(16);

    let mut group = criterion.benchmark_group("parley_text_pipeline");
    for (name, text, font_size, wrap_width) in [
        ("layout_code_warm", code.as_str(), 14.0, None),
        (
            "layout_multilingual_warm",
            multilingual.as_str(),
            16.0,
            None,
        ),
        (
            "wrap_multilingual_warm",
            multilingual.as_str(),
            16.0,
            Some(px(480.0)),
        ),
    ] {
        let runs = [TextRun {
            len: text.len(),
            ..base_run.clone()
        }];

        group.bench_function(name, |bencher| {
            bencher.iter(|| {
                system.layout_text(TextLayoutRequest {
                    text,
                    font_size: px(font_size),
                    runs: &runs,
                    options: TextLayoutOptions {
                        wrap_width,
                        text_align: TextAlign::Left,
                        ..Default::default()
                    },
                })
            });
        });
    }

    group.bench_function("layout_multilingual_cold", |bencher| {
        bencher.iter_batched(
            text_system,
            |(system, run)| {
                let runs = [TextRun {
                    len: multilingual.len(),
                    ..run
                }];

                system.layout_text(TextLayoutRequest {
                    text: &multilingual,
                    font_size: px(16.0),
                    runs: &runs,
                    options: TextLayoutOptions {
                        text_align: TextAlign::Left,
                        ..Default::default()
                    },
                })
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("first_glyph_rasterization", |bencher| {
        bencher.iter_batched(
            raster_case,
            |(system, params)| system.rasterize_glyph(&params).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("cached_glyph_rasterization", |bencher| {
        let (system, params) = raster_case();
        system.rasterize_glyph(&params).unwrap();
        bencher.iter(|| system.rasterize_glyph(&params).unwrap());
    });

    group.finish();
}

criterion_group!(benches, bench_text_pipeline);
criterion_main!(benches);
