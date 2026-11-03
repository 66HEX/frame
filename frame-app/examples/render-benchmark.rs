//! Measures CPU preparation of complete Frame UI frames using native fonts.
//! Draws fixture state synchronously without advancing playback tasks.
//! GPU presentation is not timed.
//!
//! FRAME_GPUI_VISUAL_FIXTURE=settings-video cargo run -p frame-app --example render-benchmark
use frame_app::{
    app::{FrameRoot, frame_window_options, init_app},
    assets::{self, FrameAssets},
    visual_fixture_from_env_value,
};
use gpui::{AppContext, Bounds, px, size};
use std::time::Instant;

const WARMUP_FRAMES: usize = 20;
const MEASURED_FRAMES: usize = 120;

fn main() {
    let fixture = std::env::var("FRAME_GPUI_VISUAL_FIXTURE")
        .expect("set FRAME_GPUI_VISUAL_FIXTURE, for example settings-video or workspace-image");
    assert!(
        visual_fixture_from_env_value(Some(&fixture)).is_some(),
        "unknown visual fixture: {fixture}"
    );
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(move |cx| {
            assets::load_frame_fonts(cx).expect("failed to load benchmark fonts");
            init_app(cx, "Frame render benchmark");
            let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
            let handle = cx
                .open_window(frame_window_options(bounds), |_, cx| {
                    // In-memory fixture state; no runtime-capability probes or update checks.
                    cx.new(|_| FrameRoot::new())
                })
                .expect("failed to create benchmark window");
            cx.defer(move |cx| {
                // Keep the loop synchronous so playback tasks cannot supply new frames
                // between measurements. Each draw rebuilds the complete UI instead.
                let mut timings = Vec::with_capacity(MEASURED_FRAMES);
                for index in 0..WARMUP_FRAMES + MEASURED_FRAMES {
                    let elapsed = cx
                        .update_window(handle.into(), |_, window, cx| {
                            window.refresh();
                            let start = Instant::now();
                            window.draw(cx).clear(cx);
                            start.elapsed()
                        })
                        .expect("benchmark window closed");
                    if index >= WARMUP_FRAMES {
                        timings.push(elapsed);
                    }
                }
                timings.sort_unstable();
                println!(
                    "{fixture}: {MEASURED_FRAMES} CPU frames; median={:.3?}, p95={:.3?}, max={:.3?}",
                    timings[MEASURED_FRAMES / 2],
                    timings[MEASURED_FRAMES * 95 / 100],
                    timings[MEASURED_FRAMES - 1],
                );
                cx.quit();
            });
        });
}
