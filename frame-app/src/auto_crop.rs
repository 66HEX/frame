//! Bounded, cancellable black-bar detection in several parts of the source video.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod process;

#[derive(Clone, Debug, PartialEq)]
pub struct AutoCropRequest {
    pub(super) path: String,
    pub(super) video_stream_index: Option<u32>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) start_seconds: f64,
    pub(super) end_seconds: f64,
}

/// Pixels in the original source's display orientation, before user transforms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DetectedCrop {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AutoCropError {
    #[error("The video dimensions or duration are unavailable.")]
    InvalidInput,
    #[error("Auto crop was cancelled.")]
    Cancelled,
    #[error("Auto crop took too long. Try a shorter selection or crop manually.")]
    TimedOut,
    #[error("FFmpeg could not be started: {0}")]
    Unavailable(String),
    #[error("FFmpeg could not analyze this video: {0}")]
    Failed(String),
    #[error("No reliable crop was found. Adjust the crop manually.")]
    NoReliableCrop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SampleWindow {
    start: f64,
    duration: f64,
}

const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(30);
const SAMPLE_SECONDS: f64 = 2.0;

/// Analyze a bounded selection without applying any crop to the source file.
///
/// # Errors
/// Returns an error for invalid metadata, cancellation, a timed-out or failed
/// `FFmpeg` process, or samples without enough reliable visible content.
pub fn detect_auto_crop(
    request: &AutoCropRequest,
    cancelled: &AtomicBool,
) -> Result<DetectedCrop, AutoCropError> {
    detect_with_executable(
        request,
        cancelled,
        &crate::runtime_binaries::ffmpeg_executable(),
    )
}

fn detect_with_executable(
    request: &AutoCropRequest,
    cancelled: &AtomicBool,
    executable: &str,
) -> Result<DetectedCrop, AutoCropError> {
    let windows = sample_windows(request)?;
    let deadline = Instant::now() + ANALYSIS_TIMEOUT;
    let mut detected = Vec::with_capacity(windows.len());
    for window in &windows {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AutoCropError::Cancelled);
        }
        if let Some(crop) =
            process::analyze_window(executable, request, *window, cancelled, deadline)?
        {
            detected.push(crop);
        }
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(AutoCropError::Cancelled);
    }
    // A single dark scene must not decide the crop for a longer video.
    if detected.len() < windows.len().min(2) {
        return Err(AutoCropError::NoReliableCrop);
    }
    let crop = detected
        .into_iter()
        .reduce(DetectedCrop::union)
        .ok_or(AutoCropError::NoReliableCrop)?;
    Ok(crop.preserve_source_edges(request.width, request.height))
}

fn sample_windows(request: &AutoCropRequest) -> Result<Vec<SampleWindow>, AutoCropError> {
    if request.path.is_empty()
        || request.width == 0
        || request.height == 0
        || !request.start_seconds.is_finite()
        || !request.end_seconds.is_finite()
        || request.start_seconds < 0.0
        || request.end_seconds <= request.start_seconds
    {
        return Err(AutoCropError::InvalidInput);
    }
    let duration = request.end_seconds - request.start_seconds;
    let window_duration = duration.min(SAMPLE_SECONDS);
    let count = (1_u32..=5)
        .find(|count| duration <= f64::from(*count) * SAMPLE_SECONDS)
        .unwrap_or(5);
    let available = duration - window_duration;
    Ok((0..count)
        .map(|index| {
            let fraction = if count == 1 {
                0.0
            } else {
                f64::from(index) / f64::from(count - 1)
            };
            // Long films sample 10%, 30%, 50%, 70%, 90%; short clips cover the
            // complete selected range with overlapping windows when necessary.
            let offset = if duration > 5.0 * SAMPLE_SECONDS {
                fraction
                    .mul_add(0.8, 0.1)
                    .mul_add(duration, -(window_duration / 2.0))
                    .clamp(0.0, available)
            } else {
                fraction * available
            };
            SampleWindow {
                start: request.start_seconds + offset,
                duration: window_duration,
            }
        })
        .collect())
}

fn sample_args(request: &AutoCropRequest, window: SampleWindow) -> Vec<String> {
    vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-nostats".into(),
        "-loglevel".into(),
        "info".into(),
        "-threads".into(),
        "2".into(),
        "-ss".into(),
        format!("{:.6}", window.start),
        "-i".into(),
        request.path.clone(),
        "-map".into(),
        request
            .video_stream_index
            .map_or_else(|| "0:v:0".into(), |index| format!("0:{index}")),
        "-t".into(),
        format!("{:.6}", window.duration),
        "-an".into(),
        "-sn".into(),
        "-dn".into(),
        "-filter_threads".into(),
        "1".into(),
        "-vf".into(),
        "fps=5,cropdetect=mode=black:limit=0.094117647:round=2:skip=0:reset=0".into(),
        "-frames:v".into(),
        "12".into(),
        "-f".into(),
        "null".into(),
        "-".into(),
    ]
}

fn parse_crop(line: &str, width: u32, height: u32) -> Option<DetectedCrop> {
    if !line.contains("[Parsed_cropdetect_") {
        return None;
    }
    let value = line.split_once(" crop=")?.1.split_whitespace().next()?;
    let mut fields = value.split(':');
    let crop = DetectedCrop {
        width: fields.next()?.parse().ok()?,
        height: fields.next()?.parse().ok()?,
        x: fields.next()?.parse().ok()?,
        y: fields.next()?.parse().ok()?,
    };
    if fields.next().is_some()
        || crop.width < width.min(32)
        || crop.height < height.min(32)
        || crop.x.checked_add(crop.width)? > width
        || crop.y.checked_add(crop.height)? > height
        || u64::from(crop.width) * u64::from(crop.height)
            < (u64::from(width) * u64::from(height)) / 10
    {
        return None;
    }
    Some(crop)
}

impl DetectedCrop {
    fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            width: (self.x + self.width).max(other.x + other.width) - x,
            height: (self.y + self.height).max(other.y + other.height) - y,
        }
    }

    const fn preserve_source_edges(self, width: u32, height: u32) -> Self {
        // cropdetect rounds dimensions to even numbers. Do not mistake that
        // one-pixel rounding for a real border on odd-sized source material.
        let x = if self.x <= 1 { 0 } else { self.x };
        let y = if self.y <= 1 { 0 } else { self.y };
        let right = if width - (self.x + self.width) <= 1 {
            width
        } else {
            self.x + self.width
        };
        let bottom = if height - (self.y + self.height) <= 1 {
            height
        } else {
            self.y + self.height
        };
        Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }
    }
}

#[cfg(test)]
mod tests;
