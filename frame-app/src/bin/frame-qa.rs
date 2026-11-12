use frame_app::app::init_app;
use frame_app::app::qa::{QaProfile, open_qa_window};
use frame_app::assets::{self, FrameAssets};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [flag, directory] = args.as_slice() else {
        return Err("Usage: frame-qa --qa-directory ABSOLUTE_NEW_DIRECTORY".into());
    };
    if flag != "--qa-directory" {
        return Err("Usage: frame-qa --qa-directory ABSOLUTE_NEW_DIRECTORY".into());
    }
    let profile = QaProfile::create(&PathBuf::from(directory))?;
    eprintln!(
        "QA settings: {}",
        profile.persistence().settings_path().display()
    );
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(move |cx| {
            assets::load_frame_fonts(cx).expect("failed to load Frame fonts");
            open_qa_window(cx, profile);
            init_app(cx, "Frame QA");
        });
    Ok(())
}
