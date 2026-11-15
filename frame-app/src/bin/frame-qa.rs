use frame_app::app::init_app;
use frame_app::app::qa::{QaProfile, open_qa_window};
use frame_app::assets::{self, FrameAssets};
use frame_core::events::ConversionEvent;
use std::io::{self, Write};
use std::path::PathBuf;

const USAGE: &str = "Usage: frame-qa --qa-directory ABSOLUTE_NEW_DIRECTORY | --plan REQUEST.json | --export REQUEST.json | --export-frame REQUEST.json";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [flag, directory] = args.as_slice() else {
        return Err(USAGE.into());
    };
    if flag == "--plan" || flag == "--export" {
        return run_headless(&PathBuf::from(directory), flag == "--export");
    }
    if flag == "--export-frame" {
        let output = frame_app::app::qa::source_frame::export(&PathBuf::from(directory))?;
        let mut stdout = io::stdout().lock();
        serde_json::to_writer(
            &mut stdout,
            &serde_json::json!({"status": "published", "outputPath": output}),
        )?;
        writeln!(stdout)?;
        return Ok(());
    }
    if flag != "--qa-directory" {
        return Err(USAGE.into());
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
            open_qa_window(cx, &profile).expect("failed to open Frame QA window");
            init_app(cx, "Frame QA");
        });
    Ok(())
}

fn run_headless(path: &std::path::Path, execute: bool) -> Result<(), Box<dyn std::error::Error>> {
    let plan = frame_app::app::qa::export::read_plan(path)?;
    let mut stdout = io::stdout().lock();
    if !execute {
        serde_json::to_writer(&mut stdout, &plan)?;
        writeln!(stdout)?;
        return Ok(());
    }
    let mut write_error = None;
    let result = frame_app::conversion_runner::run_conversion_task(plan.task, |event| {
        if write_error.is_some() {
            return;
        }
        let payload = match &event {
            ConversionEvent::Started(value) => serde_json::to_value(value),
            ConversionEvent::Cancelled(value) => serde_json::to_value(value),
            ConversionEvent::Progress(value) => serde_json::to_value(value),
            ConversionEvent::Completed(value) => serde_json::to_value(value),
            ConversionEvent::Error(value) => serde_json::to_value(value),
            ConversionEvent::Log(value) => serde_json::to_value(value),
        };
        let line = payload.and_then(|payload| {
            serde_json::to_vec(&serde_json::json!({
                "event": event.event_name(), "payload": payload
            }))
        });
        let written = line.map_err(io::Error::other).and_then(|line| {
            stdout.write_all(&line)?;
            writeln!(stdout)?;
            stdout.flush()
        });
        if let Err(error) = written {
            write_error = Some(error);
        }
    });
    if let Some(error) = write_error {
        return Err(error.into());
    }
    result.map_err(Into::into)
}
