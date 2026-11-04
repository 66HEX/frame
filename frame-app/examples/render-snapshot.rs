//! Captures a settled native fixture without advancing asynchronous media work.
//! Set `FRAME_GPUI_VISUAL_FIXTURE`, `FRAME_VISUAL_THEME` and `FRAME_GPUI_UI_SCALE`.
//! Pass the destination PNG path as the first argument.
//! Build with `--features gpui_platform/test-support` for native GPU readback.

use frame_app::{
    app::{FrameRoot, frame_window_options, init_app},
    assets::{self, FrameAssets},
    visual_fixture_from_env_value,
};
use gpui::{AppContext, Bounds, MouseMoveEvent, PlatformInput, point, px, size};

fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("pass the destination PNG path");
    let fixture = std::env::var("FRAME_GPUI_VISUAL_FIXTURE").expect("set a visual fixture");
    assert!(
        visual_fixture_from_env_value(Some(&fixture)).is_some(),
        "unknown fixture"
    );

    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(move |cx| {
            assets::load_frame_fonts(cx).expect("load fixture fonts");
            init_app(cx, "Frame visual snapshot");
            cx.set_reduce_motion(true);
            let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
            let mut options = frame_window_options(bounds);
            options.show = false;
            options.focus = false;
            let handle = cx
                .open_window(options, |_, cx| cx.new(|_| FrameRoot::new()))
                .expect("open fixture window");
            cx.defer(move |cx| {
                cx.update_window(handle.into(), |_, window, cx| {
                    // Keep physical pointer position from changing group-hover styles.
                    window.dispatch_event(
                        PlatformInput::MouseMove(MouseMoveEvent {
                            position: point(px(-1000.0), px(-1000.0)),
                            ..Default::default()
                        }),
                        cx,
                    );
                    // Resolve layout and focus synchronously, without advancing timers.
                    for _ in 0..3 {
                        window.refresh();
                        window.draw(cx).clear(cx);
                    }
                    window
                        .render_to_image()
                        .expect("render fixture")
                        .save(&output)
                        .expect("save PNG");
                })
                .expect("draw fixture");
                cx.quit();
            });
        });
}
