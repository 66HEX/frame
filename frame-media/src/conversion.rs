//! Convert a validated Frame export using the linked demuxers, codecs and muxers.
use std::ffi::{CStr, CString, OsStr, OsString, c_char, c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn frame_convert(argc: c_int, argv: *const *const c_char, io: *mut c_void) -> c_int;
    fn av_strerror(error: c_int, buffer: *mut c_char, size: usize) -> c_int;
}

enum Sink {
    File(File),
    Stdout(io::Stdout),
}

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::File(file) => file.write(bytes),
            Self::Stdout(out) => out.write(bytes),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(file) => file.flush(),
            Self::Stdout(out) => out.flush(),
        }
    }
}

#[derive(Default)]
struct Output {
    media: Option<Sink>,
    progress: Option<Sink>,
    error: Option<io::Error>,
    last_report: Option<Instant>,
    quiet: bool,
    frames: i64,
    time_us: i64,
    pass: c_int,
    duration_us: i64,
    scratch: Option<(tempfile::TempDir, CString)>,
}

impl Output {
    fn checked<T>(&mut self, result: io::Result<T>, ok: impl FnOnce(T) -> i64) -> i64 {
        match result {
            Ok(value) => ok(value),
            Err(error) => {
                self.error.get_or_insert(error);
                -1
            }
        }
    }
}

// SAFETY: these private ABI functions are called synchronously by frame_convert,
// which borrows the Output, argument strings and write buffers for the call.
#[unsafe(no_mangle)]
unsafe extern "C" fn frame_output_open(
    state: &mut Output,
    name: *const c_char,
    overwrite: c_int,
    progress: c_int,
) -> c_int {
    let raw = unsafe { CStr::from_ptr(name) }.to_bytes();
    let sink = if raw == b"pipe:1" || raw == b"-" {
        Ok(Sink::Stdout(io::stdout()))
    } else {
        #[cfg(unix)]
        let path = {
            use std::os::unix::ffi::OsStrExt;
            OsStr::from_bytes(raw)
        };
        #[cfg(not(unix))]
        let Ok(path) = std::str::from_utf8(raw).map(OsStr::new) else {
            return -1;
        };
        let mut open = OpenOptions::new();
        open.write(true);
        if overwrite == 0 {
            open.create_new(true);
        } else {
            open.create(true).truncate(true);
        }
        open.open(path).map(Sink::File)
    };
    match sink {
        Ok(sink) => {
            let seekable = matches!(sink, Sink::File(_));
            if progress == 0 {
                state.media = Some(sink);
            } else {
                state.progress = Some(sink);
            }
            c_int::from(seekable)
        }
        Err(error) => {
            state.error = Some(error);
            -1
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn frame_output_write(
    state: &mut Output,
    buffer: *const u8,
    size: c_int,
) -> c_int {
    let Ok(size_usize) = usize::try_from(size) else {
        return -1;
    };
    let bytes = unsafe { std::slice::from_raw_parts(buffer, size_usize) };
    let Some(media) = state.media.as_mut() else {
        return -1;
    };
    let result = media.write_all(bytes);
    if state.checked(result, |()| 0) < 0 {
        -1
    } else {
        size
    }
}

#[unsafe(no_mangle)]
extern "C" fn frame_output_seek(state: &mut Output, offset: i64, whence: c_int) -> i64 {
    let Some(Sink::File(file)) = &mut state.media else {
        return -1;
    };
    let result = match whence {
        0 => u64::try_from(offset).map_or_else(
            |_| Err(io::ErrorKind::InvalidInput.into()),
            |value| file.seek(SeekFrom::Start(value)),
        ),
        1 => file.seek(SeekFrom::Current(offset)),
        2 => file.seek(SeekFrom::End(offset)),
        0x10000 => file.metadata().map(|value| value.len()), // AVSEEK_SIZE
        _ => return -1,
    };
    state.checked(result, |n| i64::try_from(n).unwrap_or(-1))
}

// The application may own a surrounding directory so it can remove all
// encoder sidecars even after force-terminating this process.
#[unsafe(no_mangle)]
extern "C" fn frame_pass_path(state: &mut Output) -> *const c_char {
    let workspace = || -> io::Result<_> {
        let mut builder = tempfile::Builder::new();
        builder.prefix("frame-two-pass-");
        let directory = match std::env::var_os("FRAME_PASS_DIRECTORY") {
            Some(parent) => builder.tempdir_in(parent)?,
            None => builder.tempdir()?,
        };
        let path = directory.path().join("video.stats");
        #[cfg(windows)]
        let bytes = path.to_str().ok_or(io::ErrorKind::InvalidInput)?.as_bytes();
        #[cfg(not(windows))]
        let bytes = path.as_os_str().as_encoded_bytes();
        let name = CString::new(bytes)?;
        Ok((directory, name))
    };
    match workspace() {
        Ok(pair) => state.scratch.insert(pair).1.as_ptr(),
        Err(err) => {
            state.error = Some(err);
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn frame_output_pass(state: &mut Output, pass: c_int, duration: i64) {
    state.pass = pass;
    state.duration_us = duration;
    state.last_report = None;
    state.time_us = 0;
    state.frames = 0;
    if !state.quiet {
        let mut log = io::stderr().lock();
        if duration > 0 {
            let sec = duration / 1_000_000;
            let _ = writeln!(
                log,
                "Duration: {:02}:{:02}:{:02}.{:03}",
                sec / 3600,
                sec % 3600 / 60,
                sec % 60,
                duration % 1_000_000 / 1000
            );
        }
        let _ = writeln!(log, "Video pass {pass}/2");
    }
}

#[unsafe(no_mangle)]
const extern "C" fn frame_output_quiet(state: &mut Output, quiet: c_int) {
    state.quiet = quiet != 0;
}

#[unsafe(no_mangle)]
extern "C" fn frame_output_stats(state: &mut Output, frames: i64, time_us: i64) {
    state.frames = frames;
    state.time_us = state.time_us.max(time_us);
    if state.quiet
        || state
            .last_report
            .is_some_and(|last| last.elapsed() < Duration::from_millis(250))
    {
        return;
    }
    state.last_report = Some(Instant::now());
    let time = match state.pass {
        1 => state.time_us.min(state.duration_us) / 2,
        2 => state.duration_us / 2 + state.time_us.min(state.duration_us) / 2,
        _ => state.time_us,
    };
    let sec = time / 1_000_000;
    let _ = writeln!(
        io::stderr(),
        "frame={frames} time={:02}:{:02}:{:02}.{:03}",
        sec / 3600,
        sec % 3600 / 60,
        sec % 60,
        time % 1_000_000 / 1000
    );
}

/// Executes one export and publishes its completion record after the muxer closes.
///
/// # Errors
/// Returns the library or I/O error that stopped the export.
pub fn run(args: &[OsString]) -> Result<(), String> {
    let strings: Vec<_> = args
        .iter()
        .map(|value| {
            #[cfg(windows)]
            let bytes = value
                .to_str()
                .ok_or("Media path is not representable as UTF-8")?
                .as_bytes();
            #[cfg(not(windows))]
            let bytes = value.as_encoded_bytes();
            CString::new(bytes).map_err(|_| "Media argument contains NUL")
        })
        .collect::<Result<_, _>>()?;
    let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
    let mut state = Output::default();
    let count = c_int::try_from(pointers.len()).map_err(|err| err.to_string())?;
    let result = unsafe { frame_convert(count, pointers.as_ptr(), (&raw mut state).cast()) };
    if let Some(error) = state.error.take() {
        return Err(error.to_string());
    }
    if result < 0 {
        let mut buf: [c_char; 128] = [0; 128];
        unsafe { av_strerror(result, buf.as_mut_ptr(), buf.len()) };
        return Err(format!(
            "Media export failed: {}",
            unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy()
        ));
    }
    if let Some(media) = &mut state.media {
        media.flush().map_err(|error| error.to_string())?;
    }
    state.last_report = None;
    let (frames, time_us) = (state.frames, state.time_us);
    frame_output_stats(&mut state, frames, time_us);
    if let Some(progress) = &mut state.progress {
        writeln!(
            progress,
            "frame={}\nout_time_us={}\nprogress=end",
            state.frames, state.time_us
        )
        .and_then(|()| progress.flush())
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}
