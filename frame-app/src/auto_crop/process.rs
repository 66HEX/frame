use super::{AutoCropError, AutoCropRequest, DetectedCrop, SampleWindow, parse_crop, sample_args};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_LINE_BYTES: usize = 8_192;
const MAX_DIAGNOSTIC_BYTES: usize = 4_096;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Default)]
struct Capture {
    crop: Option<DetectedCrop>,
    diagnostic: String,
}

pub(super) fn analyze_window(
    executable: &str,
    request: &AutoCropRequest,
    window: SampleWindow,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<Option<DetectedCrop>, AutoCropError> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(AutoCropError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(AutoCropError::TimedOut);
    }
    let mut command = Command::new(executable);
    command
        .args(sample_args(request, window))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = ChildGuard(
        command
            .spawn()
            .map_err(|error| AutoCropError::Unavailable(error.to_string()))?,
    );
    let stderr = child
        .0
        .stderr
        .take()
        .ok_or_else(|| AutoCropError::Failed("No analysis output.".into()))?;
    let (width, height) = (request.width, request.height);
    let reader = std::thread::spawn(move || capture(stderr, width, height));
    let status = loop {
        if cancelled.load(Ordering::Relaxed) {
            break Err(AutoCropError::Cancelled);
        }
        if Instant::now() >= deadline {
            break Err(AutoCropError::TimedOut);
        }
        match child.0.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => break Err(AutoCropError::Failed(error.to_string())),
        }
    };
    // Reap before joining the pipe reader, including timeout and cancellation.
    drop(child);
    let captured = reader
        .join()
        .map_err(|_| AutoCropError::Failed("Could not read analysis output.".into()))?;
    let status = status?;
    if !status.success() {
        return Err(AutoCropError::Failed(if captured.diagnostic.is_empty() {
            format!("FFmpeg exited with {status}")
        } else {
            captured.diagnostic
        }));
    }
    Ok(captured.crop)
}

fn capture(mut reader: impl Read, width: u32, height: u32) -> Capture {
    let mut result = Capture::default();
    let mut buffer = [0_u8; 4_096];
    let mut line = Vec::with_capacity(512);
    let mut overflow = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                for byte in &buffer[..count] {
                    if *byte == b'\n' || *byte == b'\r' {
                        if !overflow {
                            result.add_line(&line, width, height);
                        }
                        line.clear();
                        overflow = false;
                    } else if line.len() < MAX_LINE_BYTES {
                        line.push(*byte);
                    } else {
                        overflow = true;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    if !overflow {
        result.add_line(&line, width, height);
    }
    result
}

impl Capture {
    fn add_line(&mut self, line: &[u8], width: u32, height: u32) {
        let line = String::from_utf8_lossy(line);
        if let Some(crop) = parse_crop(&line, width, height) {
            self.crop = Some(self.crop.map_or(crop, |previous| previous.union(crop)));
        }
        // Retain only a small tail for errors while continuing to drain stderr.
        if !line.is_empty() {
            self.diagnostic.push_str(&line);
            self.diagnostic.push('\n');
            if self.diagnostic.len() > MAX_DIAGNOSTIC_BYTES {
                let mut boundary = self.diagnostic.len() - MAX_DIAGNOSTIC_BYTES;
                while !self.diagnostic.is_char_boundary(boundary) {
                    boundary += 1;
                }
                self.diagnostic.drain(..boundary);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_bounded_without_losing_later_crop_lines() {
        let source = format!(
            "{}\n[Parsed_cropdetect_1 @ 0x1] crop=320:180:0:30\n",
            "x".repeat(100_000)
        );
        let result = capture(source.as_bytes(), 320, 240);
        assert_eq!(
            result.crop,
            Some(DetectedCrop {
                x: 0,
                y: 30,
                width: 320,
                height: 180
            })
        );
        assert!(result.diagnostic.len() <= MAX_DIAGNOSTIC_BYTES);
    }
}
