use std::io::{self, Read};
use std::process::ChildStdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(super) const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

/// The startup owner is the only reader until the first frame is complete.
/// Readiness checks keep a silent or partial child from blocking cancellation.
pub(super) struct StartupPipe<'a> {
    stdout: &'a mut ChildStdout,
    cancelled: &'a AtomicBool,
    deadline: Instant,
}

impl<'a> StartupPipe<'a> {
    pub(super) const fn new(
        stdout: &'a mut ChildStdout,
        cancelled: &'a AtomicBool,
        deadline: Instant,
    ) -> Self {
        Self {
            stdout,
            cancelled,
            deadline,
        }
    }
}

impl Read for StartupPipe<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(io::Error::other("preview startup cancelled"));
            }
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "preview first frame timed out; retry the preview",
                ));
            }
            if let Some(available) = available_bytes(self.stdout)? {
                let count = buffer.len().min(available);
                return self.stdout.read(&mut buffer[..count]);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(unix)]
fn available_bytes(stdout: &ChildStdout) -> io::Result<Option<usize>> {
    use std::os::fd::AsRawFd;
    let mut descriptor = libc::pollfd {
        fd: stdout.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: the descriptor borrows a live pipe, and poll receives one valid entry.
    let result = unsafe { libc::poll(&raw mut descriptor, 1, 0) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(None);
        }
        return Err(error);
    }
    // With one reader, a ready pipe read returns the available bytes without
    // waiting to fill the supplied buffer. HUP is also readable as EOF.
    Ok((result > 0).then_some(usize::MAX))
}

#[cfg(windows)]
fn available_bytes(stdout: &ChildStdout) -> io::Result<Option<usize>> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, HANDLE};
    use windows::Win32::System::Pipes::PeekNamedPipe;
    let mut available = 0;
    // SAFETY: stdout retains its pipe handle and the count points to live storage.
    let result = unsafe {
        PeekNamedPipe(
            HANDLE(stdout.as_raw_handle()),
            None,
            0,
            None,
            Some(&raw mut available),
            None,
        )
    };
    match result {
        Ok(()) => Ok((available > 0).then_some(available as usize)),
        Err(error) if error.code() == ERROR_BROKEN_PIPE.to_hresult() => Ok(Some(0)),
        Err(error) => Err(io::Error::other(error)),
    }
}

#[cfg(not(any(unix, windows)))]
fn available_bytes(_stdout: &ChildStdout) -> io::Result<Option<usize>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "bounded preview startup is unavailable on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Child, Command, Stdio};

    fn fixture(mode: &str) -> Child {
        crate::process_launch::spawn(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "preview_engine::ffmpeg_backend::startup_pipe::tests::fake_pipe",
                    "--ignored",
                    "--nocapture",
                ])
                .env("FRAME_PREVIEW_PIPE_MODE", mode)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null()),
        )
        .unwrap()
    }

    #[test]
    #[ignore = "native preview pipe fixture"]
    fn fake_pipe() {
        let Ok(mode) = std::env::var("FRAME_PREVIEW_PIPE_MODE") else {
            return;
        };
        if mode == "partial" || mode == "complete" {
            std::io::stdout()
                .write_all(&vec![37; if mode == "partial" { 64 } else { 64 * 1024 }])
                .unwrap();
            std::io::stdout().flush().unwrap();
        }
        std::thread::sleep(Duration::from_secs(2));
        std::process::exit(0);
    }

    #[test]
    fn incomplete_first_frame_obeys_the_deadline() {
        for mode in ["silent", "partial"] {
            let mut child = fixture(mode);
            let mut stdout = child.stdout.take().unwrap();
            let started = Instant::now();
            let result = StartupPipe::new(
                &mut stdout,
                &AtomicBool::new(false),
                started + Duration::from_millis(200),
            )
            .read_exact(&mut vec![0; 4096]);
            child.kill().unwrap();
            child.wait().unwrap();
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
            assert!(started.elapsed() < Duration::from_secs(1));
        }
    }

    #[test]
    fn complete_first_frame_does_not_wait_for_stream_exit() {
        let mut child = fixture("complete");
        let mut stdout = child.stdout.take().unwrap();
        let started = Instant::now();
        let result = StartupPipe::new(
            &mut stdout,
            &AtomicBool::new(false),
            started + Duration::from_secs(1),
        )
        .read_exact(&mut vec![0; 4096]);
        child.kill().unwrap();
        child.wait().unwrap();
        result.unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
