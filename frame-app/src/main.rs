use frame_app::{
    app::{init_app, open_frame_window},
    app_info::FRAME_APP_NAME,
    assets::{self, FrameAssets},
};

fn main() {
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(|cx| {
            assets::load_frame_fonts(cx).expect("failed to load Frame fonts");
            open_frame_window(cx);
            init_app(cx, FRAME_APP_NAME);
        });
}
