use frame_app::{
    WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
    app::{FrameRoot, frame_window_options, init_app},
    app_info::FRAME_APP_NAME,
    assets::{self, FrameAssets},
};
use gpui::{AppContext, Bounds, px, size};

fn main() {
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(|cx| {
            assets::load_frame_fonts(cx).expect("failed to load Frame fonts");
            let bounds =
                Bounds::centered(None, size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)), cx);
            cx.open_window(frame_window_options(bounds), |_, cx| {
                cx.new(|_| FrameRoot::new_with_platform_persistence())
            })
            .expect("failed to open Frame GPUI window");

            init_app(cx, FRAME_APP_NAME);
        });
}
