use std::ffi::OsStr;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let operation = args.next();
    let args: Vec<_> = args.collect();
    let result = match operation.as_deref().and_then(OsStr::to_str) {
        Some("convert") => frame_media::conversion::run(&args),
        Some("preview") => frame_media::preview::run(&args),
        Some("probe") => frame_media::probe::run(&args),
        Some("capabilities") => frame_media::capabilities::run(&args),
        _ => Err("Expected a Frame media operation (convert, preview, probe, capabilities)".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
