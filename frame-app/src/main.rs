#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use frame_app::app::{init_app, open_frame_window};
use frame_app::app_info::FRAME_APP_NAME;
use frame_app::assets;
use frame_app::assets::FrameAssets;

fn main() {
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(|cx| {
            assets::load_frame_fonts(cx).expect("failed to load Frame fonts");
            open_frame_window(cx);
            init_app(cx, FRAME_APP_NAME);
        });
}
