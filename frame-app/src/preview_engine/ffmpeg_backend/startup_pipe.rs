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
            #[cfg(not(unix))]
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
    // Wait for the producer instead of sleeping after observing an empty pipe.
    // Every bounded wait returns to the cancellation/deadline checks above.
    // SAFETY: the descriptor borrows a live pipe, and poll receives one valid entry.
    let result = unsafe { libc::poll(&raw mut descriptor, 1, 5) };
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
                .stdin(Stdio::piped())
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
        if mode == "cancel-silent" || mode == "cancel-partial" {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(b"FRAME_PIPE_READY\n").unwrap();
            stdout.flush().unwrap();
            std::io::stdin().read_exact(&mut [0]).unwrap();
            if mode == "cancel-partial" {
                stdout.write_all(&[37; 64]).unwrap();
                stdout.flush().unwrap();
            }
            std::thread::sleep(Duration::from_secs(5));
            std::process::exit(0);
        }
        if mode == "paced-frame" {
            let mut stdout = std::io::stdout().lock();
            let mut stdin = std::io::stdin().lock();
            stdout.write_all(b"FRAME_PIPE_READY\n").unwrap();
            stdout.flush().unwrap();
            for chunk in vec![37; 1920 * 1080 * 4].chunks(8192) {
                stdin.read_exact(&mut [0]).unwrap();
                std::thread::sleep(Duration::from_millis(1));
                stdout.write_all(chunk).unwrap();
                stdout.flush().unwrap();
            }
            drop(stdin);
            drop(stdout);
            std::process::exit(0);
        }
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

    #[test]
    fn cancellation_interrupts_a_silent_or_partial_frame_and_allows_a_fresh_read() {
        for mode in ["cancel-silent", "cancel-partial"] {
            let mut child = fixture(mode);
            let mut stdout = child.stdout.take().unwrap();
            let cancelled = AtomicBool::new(false);
            let mut pipe = StartupPipe::new(
                &mut stdout,
                &cancelled,
                Instant::now() + Duration::from_secs(3),
            );
            let result = (|| -> io::Result<(io::Result<()>, Duration, Vec<u8>)> {
                // The native child is running and waiting for our acknowledgement;
                // neither startup scheduling nor libtest text counts as frame data.
                let mut line = Vec::new();
                loop {
                    let mut byte = [0];
                    pipe.read_exact(&mut byte)?;
                    line.push(byte[0]);
                    if byte[0] == b'\n' {
                        if line == b"FRAME_PIPE_READY\n" {
                            break;
                        }
                        line.clear();
                    }
                }
                child.stdin.as_mut().unwrap().write_all(&[1])?;
                let mut pixels = vec![0; 4096];
                let offset = if mode == "cancel-partial" {
                    pipe.read_exact(&mut pixels[..64])?;
                    64
                } else {
                    0
                };
                let started = Instant::now();
                let read = std::thread::scope(|scope| {
                    scope.spawn(|| {
                        std::thread::sleep(Duration::from_millis(50));
                        cancelled.store(true, Ordering::Release);
                    });
                    pipe.read_exact(&mut pixels[offset..])
                });
                Ok((read, started.elapsed(), pixels))
            })();
            // Reap before every assertion, including when the cancellation fails.
            let _ = child.kill();
            let status = child.wait().unwrap();
            assert_eq!(child.try_wait().unwrap(), Some(status));
            let (read, elapsed, pixels) = result.unwrap();
            let error = read.expect_err("cancellation must interrupt an incomplete frame");
            assert!(error.to_string().contains("cancelled"), "{mode}: {error}");
            assert!(elapsed < Duration::from_secs(1), "{mode}: {elapsed:?}");
            if mode == "cancel-partial" {
                assert_eq!(&pixels[..64], &[37; 64]);
            }

            cancelled.store(false, Ordering::Release);
            let mut retry = fixture("complete");
            let mut stdout = retry.stdout.take().unwrap();
            let read = StartupPipe::new(
                &mut stdout,
                &cancelled,
                Instant::now() + Duration::from_secs(1),
            )
            .read_exact(&mut vec![0; 4096]);
            let _ = retry.kill();
            retry.wait().unwrap();
            read.expect("a fresh read must succeed after cancellation cleanup");
        }
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "native throughput timing probe; run without competing benchmarks"]
    fn full_resolution_frame_does_not_pay_a_polling_sleep_for_each_producer_burst() {
        let mut child = fixture("paced-frame");
        let mut stdout = child.stdout.take().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let cancelled = AtomicBool::new(false);
        let mut pipe = StartupPipe::new(
            &mut stdout,
            &cancelled,
            Instant::now() + Duration::from_secs(15),
        );
        let mut pixels = vec![0; 1920 * 1080 * 4];
        let result = (|| -> io::Result<Duration> {
            // Discard the test harness preamble before requesting any pixels.
            // The child waits for each request, so this never consumes frame data.
            let mut line = Vec::new();
            loop {
                let mut byte = [0];
                pipe.read_exact(&mut byte)?;
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    if line == b"FRAME_PIPE_READY\n" {
                        break;
                    }
                    line.clear();
                }
            }
            let started = Instant::now();
            for chunk in pixels.chunks_mut(8192) {
                stdin.write_all(&[1])?;
                pipe.read_exact(chunk)?;
            }
            Ok(started.elapsed())
        })();
        // Even a failed timing/read assertion leaves no native child behind.
        let _ = child.kill();
        child.wait().unwrap();
        let elapsed = result.unwrap();
        assert!(pixels.iter().all(|byte| *byte == 37));
        eprintln!("full-resolution burst delivery: {elapsed:?}");
        assert!(
            elapsed < Duration::from_millis(3500),
            "first-frame pipe added a polling sleep to each producer burst: {elapsed:?}"
        );
    }
}
