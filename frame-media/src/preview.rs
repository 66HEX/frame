//! Streams filtered frames and audio to the application-owned preview pipe.

use std::ffi::{CString, OsStr, OsString, c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn frame_preview(
        path: *const c_char,
        overlay: *const c_char,
        graph: *const c_char,
        tail: *const c_char,
        output: c_int,
        stream: c_int,
        start: *const c_char,
        duration: *const c_char,
        fast: c_int,
        realtime: c_int,
        threads: c_int,
        limit: i64,
        progress: c_int,
        verbose: c_int,
    ) -> c_int;
}

fn c_string(value: &OsStr) -> Result<CString, String> {
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        value.as_bytes()
    };
    #[cfg(not(unix))]
    let bytes = value.to_str().ok_or("invalid Unicode path")?.as_bytes();
    CString::new(bytes).map_err(|err| err.to_string())
}

fn optional(value: Option<&CString>) -> *const c_char {
    value.map_or(ptr::null(), |s| s.as_ptr())
}

/// Execute the preview, source PNG or frame-analysis plan emitted by frame-core.
///
/// # Errors
/// Returns a diagnostic for incomplete plans, decoding failures or closed output.
pub fn run(args: &[OsString]) -> Result<(), String> {
    let (destination, options) = args.split_last().ok_or("missing preview destination")?;
    let mut inputs = Vec::new();
    let mut args = options.iter().map(OsString::as_os_str);
    let (mut start, mut duration, mut graph) = (None, None, None);
    let (mut fast, mut realtime, mut progress, mut verbose) = (false, false, false, false);
    let (mut stream, mut threads, mut limit) = (-1, 0, i64::MAX);
    let (mut channels, mut rate) = (0_u16, 0_u32);
    let mut format = "";
    while let Some(option) = args.next() {
        let mut next = || args.next().ok_or("missing option value");
        let mut text = || next()?.to_str().ok_or("invalid option value");
        match option.to_str() {
            Some(
                "-hide_banner" | "-nostdin" | "-an" | "-sn" | "-dn" | "-vn" | "-nostats"
                | "-xerror",
            ) => (),
            Some("-loglevel") => verbose = text()? == "info",
            Some("-noaccurate_seek") => fast = true,
            Some("-readrate") if text()? == "1" => realtime = true,
            Some("-progress") if text()? == "pipe:1" => progress = true,
            Some("-i") => inputs.push(c_string(next()?)?),
            Some("-ss") => start = Some(c_string(next()?)?),
            Some("-t" | "-to") => duration = Some(c_string(next()?)?),
            Some("-map") => {
                let value = text()?;
                if let Some(index) = value.strip_prefix("0:").filter(|index| *index != "v:0") {
                    stream = index.parse().map_err(|_| "invalid stream index")?;
                } else if value != "0:v:0" && !value.starts_with('[') {
                    return Err(format!("unsupported preview map {value}"));
                }
            }
            Some("-filter_complex" | "-af" | "-vf") => graph = Some(c_string(next()?)?),
            Some("-c:v") if matches!(text()?, "png" | "wrapped_avframe") => (),
            Some("-pix_fmt") if text()? == "bgra" => (),
            Some("-fps_mode") if text()? == "passthrough" => (),
            Some("-threads" | "-filter_threads") => {
                threads = text()?.parse().map_err(|_| "invalid thread count")?
            }
            Some("-frames:v") => limit = text()?.parse().map_err(|_| "invalid frame count")?,
            Some("-ac") => channels = text()?.parse().map_err(|_| "invalid channel count")?,
            Some("-ar") => rate = text()?.parse().map_err(|_| "invalid sample rate")?,
            Some("-f") => format = text()?,
            _ => return Err(format!("unsupported preview option {}", option.display())),
        }
    }
    let (output, tail) = match (format, destination.to_str()) {
        ("rawvideo", Some("pipe:1")) => (0, "format=bgra".into()),
        ("f32le", Some("pipe:1")) if channels > 0 && rate > 0 => (
            1,
            format!("aformat=sample_fmts=flt:sample_rates={rate}:channel_layouts={channels}c"),
        ),
        ("image2pipe", Some("pipe:1")) => (
            2,
            "format=rgb24|rgba|rgb48be|rgba64be|gray|ya8|gray16be|ya16be|pal8|monob".into(),
        ),
        ("null", Some("-")) => (3, "null".into()),
        _ => {
            return Err(format!(
                "invalid preview output {format} {}",
                destination.display()
            ));
        }
    };
    if progress && output != 3 {
        return Err("progress requires an analysis operation".into());
    }
    let input = inputs.first().ok_or("missing preview input")?;
    if inputs.len() > 2 || limit < 0 || threads < 0 {
        return Err("invalid preview plan".into());
    }
    let tail = CString::new(tail).map_err(|e| e.to_string())?;
    let graph = graph
        .as_deref()
        .unwrap_or(if output == 1 { c"anull" } else { c"null" });
    // SAFETY: all strings live across this synchronous call. Null denotes an omitted option.
    let result = unsafe {
        frame_preview(
            input.as_ptr(),
            inputs.get(1).map_or(ptr::null(), |s| s.as_ptr()),
            graph.as_ptr(),
            tail.as_ptr(),
            output,
            stream,
            optional(start.as_ref()),
            optional(duration.as_ref()),
            fast.into(),
            realtime.into(),
            threads,
            limit,
            progress.into(),
            verbose.into(),
        )
    };
    if result < 0 {
        Err(format!("preview failed ({result})"))
    } else {
        Ok(())
    }
}
