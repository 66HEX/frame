//! Inspect the source through libavformat and libavcodec in the bounded worker.

use std::ffi::{CString, OsStr, OsString, c_char, c_int};

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
enum Mode {
    Metadata,
    OpusTiming,
    ManualAlac,
    NormalAlac,
    PhysicalAudio,
}

unsafe extern "C" {
    fn frame_probe_file(
        path: *const c_char,
        mode: c_int,
        stream_index: c_int,
        start_us: i64,
        end_us: i64,
    ) -> c_int;
}

/// Execute one Frame inspection request, writing its JSON result to stdout.
///
/// # Errors
/// Returns an error for an invalid request, unreadable input or failed scan.
pub fn run(args: &[OsString]) -> Result<(), String> {
    let (path, options) = args.split_last().ok_or("probe requires an input path")?;
    let request = Request::parse(options)?;
    #[cfg(unix)]
    let path_bytes = {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes()
    };
    #[cfg(not(unix))]
    let path_bytes = path.to_str().ok_or("invalid Unicode path")?.as_bytes();
    let path = CString::new(path_bytes).map_err(|_| "invalid NUL in input path")?;
    // SAFETY: the C call borrows the terminated path for this synchronous invocation.
    let result = unsafe {
        frame_probe_file(
            path.as_ptr(),
            request.mode as c_int,
            request.stream_index,
            request.start_us,
            request.end_us,
        )
    };
    if result < 0 {
        Err(format!("media inspection failed ({result})"))
    } else {
        Ok(())
    }
}

struct Request {
    mode: Mode,
    stream_index: c_int,
    start_us: i64,
    end_us: i64,
}

impl Request {
    fn parse(options: &[OsString]) -> Result<Self, String> {
        let stream = |value: &OsStr| {
            value
                .to_str()
                .and_then(|s| s.parse::<c_int>().ok())
                .filter(|index| *index >= 0)
                .ok_or("invalid stream index")
        };
        let (mode, stream_index, start_us, end_us) = match options {
            [kind] if kind == "metadata" => (Mode::Metadata, -1, i64::MIN, i64::MIN),
            [kind] if kind == "physical-audio" => (Mode::PhysicalAudio, -1, i64::MIN, i64::MIN),
            [kind, index] if kind == "alac-source" => {
                (Mode::ManualAlac, stream(index)?, i64::MIN, i64::MIN)
            }
            [kind, index] if kind == "alac-output" => {
                (Mode::NormalAlac, stream(index)?, i64::MIN, i64::MIN)
            }
            [kind, index, start, end] if kind == "opus-timing" => (
                Mode::OpusTiming,
                stream(index)?,
                micros(start)?,
                micros(end)?,
            ),
            _ => return Err("invalid Frame probe operation or arguments".into()),
        };
        Ok(Self {
            mode,
            stream_index,
            start_us,
            end_us,
        })
    }
}

fn micros(value: &OsStr) -> Result<i64, String> {
    if value.is_empty() {
        return Ok(i64::MIN);
    }
    value
        .to_str()
        .and_then(|s| s.parse().ok())
        .filter(|time| *time != i64::MIN)
        .ok_or_else(|| "invalid microsecond boundary".into())
}
