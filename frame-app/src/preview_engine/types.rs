use std::{path::PathBuf, sync::Arc};

use super::PreviewEngineError;

pub const DEFAULT_PREVIEW_MAX_WIDTH: u32 = 1280;
pub const DEFAULT_PREVIEW_MAX_HEIGHT: u32 = 720;
pub const DEFAULT_PREVIEW_FPS: u32 = 30;
pub const MIN_PREVIEW_DIMENSION: u32 = 16;
pub const MAX_PREVIEW_DIMENSION: u32 = 3840;
pub const MIN_PREVIEW_FPS: u32 = 1;
pub const MAX_PREVIEW_FPS: u32 = 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewSourceKind {
    Video,
    Audio,
    Image,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewPixelFormat {
    Bgra,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PreviewTransform {
    pub rotation_degrees: u16,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}

impl PreviewTransform {
    #[must_use]
    pub const fn has_side_rotation(self) -> bool {
        matches!(self.rotation_degrees, 90 | 270)
    }

    #[must_use]
    pub const fn is_identity(self) -> bool {
        self.rotation_degrees == 0 && !self.flip_horizontal && !self.flip_vertical
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub timestamp_us: u64,
    pub pixel_format: PreviewPixelFormat,
    data: Arc<[u8]>,
}

impl PreviewFrame {
    pub fn bgra(
        width: u32,
        height: u32,
        stride: u32,
        timestamp_us: u64,
        data: Vec<u8>,
    ) -> Result<Self, PreviewEngineError> {
        validate_frame_layout(width, height, stride, data.len())?;
        Ok(Self {
            width,
            height,
            stride,
            timestamp_us,
            pixel_format: PreviewPixelFormat::Bgra,
            data: Arc::from(data),
        })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    #[must_use]
    pub fn dimensions(&self) -> PreviewDimensions {
        PreviewDimensions {
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSessionConfig {
    pub file_id: String,
    pub path: PathBuf,
    pub source_kind: PreviewSourceKind,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub duration_seconds: f64,
    pub max_width: u32,
    pub max_height: u32,
    pub fps: u32,
    pub transform: PreviewTransform,
}

impl PreviewSessionConfig {
    pub fn validate(&self) -> Result<(), PreviewEngineError> {
        if self.file_id.trim().is_empty() {
            return Err(PreviewEngineError::InvalidInput(
                "Preview file id cannot be empty".to_string(),
            ));
        }
        if self.path.as_os_str().is_empty() {
            return Err(PreviewEngineError::InvalidInput(
                "Preview file path cannot be empty".to_string(),
            ));
        }
        validate_range(
            "max_width",
            self.max_width,
            MIN_PREVIEW_DIMENSION,
            MAX_PREVIEW_DIMENSION,
        )?;
        validate_range(
            "max_height",
            self.max_height,
            MIN_PREVIEW_DIMENSION,
            MAX_PREVIEW_DIMENSION,
        )?;
        validate_range("fps", self.fps, MIN_PREVIEW_FPS, MAX_PREVIEW_FPS)?;
        if !matches!(self.transform.rotation_degrees, 0 | 90 | 180 | 270) {
            return Err(PreviewEngineError::InvalidInput(
                "preview rotation must be 0, 90, 180, or 270 degrees".to_string(),
            ));
        }

        match (self.source_width, self.source_height) {
            (Some(width), Some(height)) => {
                validate_range(
                    "source_width",
                    width,
                    MIN_PREVIEW_DIMENSION,
                    MAX_PREVIEW_DIMENSION * 8,
                )?;
                validate_range(
                    "source_height",
                    height,
                    MIN_PREVIEW_DIMENSION,
                    MAX_PREVIEW_DIMENSION * 8,
                )?;
            }
            (None, None) => {}
            _ => {
                return Err(PreviewEngineError::InvalidInput(
                    "source_width and source_height must be provided together".to_string(),
                ));
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn target_dimensions(&self) -> PreviewDimensions {
        match self.transformed_source_dimensions() {
            Some(dimensions) => fit_dimensions(
                dimensions.width,
                dimensions.height,
                self.max_width,
                self.max_height,
            ),
            _ => PreviewDimensions {
                width: even_dimension(self.max_width),
                height: even_dimension(self.max_height),
            },
        }
    }

    #[must_use]
    pub fn transformed_source_dimensions(&self) -> Option<PreviewDimensions> {
        let (Some(width), Some(height)) = (self.source_width, self.source_height) else {
            return None;
        };

        Some(if self.transform.has_side_rotation() {
            PreviewDimensions {
                width: height,
                height: width,
            }
        } else {
            PreviewDimensions { width, height }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewPlaybackSnapshot {
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub playing: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewSessionStatus {
    Loading,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSessionSnapshot {
    pub file_id: String,
    pub source_kind: PreviewSourceKind,
    pub dimensions: PreviewDimensions,
    pub status: PreviewSessionStatus,
    pub playback: PreviewPlaybackSnapshot,
    pub frame_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreviewCommand {
    Play,
    Pause,
    SeekFast(f64),
    SeekPrecise(f64),
}

#[must_use]
pub fn fit_dimensions(
    source_width: u32,
    source_height: u32,
    max_width: u32,
    max_height: u32,
) -> PreviewDimensions {
    let width_scale = max_width as f64 / source_width as f64;
    let height_scale = max_height as f64 / source_height as f64;
    let scale = width_scale.min(height_scale).min(1.0);

    PreviewDimensions {
        width: even_dimension((source_width as f64 * scale).round() as u32),
        height: even_dimension((source_height as f64 * scale).round() as u32),
    }
}

fn even_dimension(value: u32) -> u32 {
    let value = value.max(MIN_PREVIEW_DIMENSION);
    if value.is_multiple_of(2) {
        value
    } else {
        value - 1
    }
}

fn validate_range(name: &str, value: u32, min: u32, max: u32) -> Result<(), PreviewEngineError> {
    if (min..=max).contains(&value) {
        return Ok(());
    }

    Err(PreviewEngineError::InvalidInput(format!(
        "{name} must be between {min} and {max}"
    )))
}

fn validate_frame_layout(
    width: u32,
    height: u32,
    stride: u32,
    byte_len: usize,
) -> Result<(), PreviewEngineError> {
    if width == 0 || height == 0 {
        return Err(PreviewEngineError::UnsupportedFrameLayout(
            "frame dimensions must be non-zero".to_string(),
        ));
    }

    let row_len = width.checked_mul(4).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout("frame row length overflow".to_string())
    })?;
    if stride < row_len {
        return Err(PreviewEngineError::UnsupportedFrameLayout(
            "frame stride is smaller than row length".to_string(),
        ));
    }

    let expected_len = usize::try_from(stride)
        .ok()
        .and_then(|stride| {
            usize::try_from(height)
                .ok()
                .and_then(|height| stride.checked_mul(height))
        })
        .ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame byte length overflow".to_string())
        })?;

    if byte_len < expected_len {
        return Err(PreviewEngineError::UnsupportedFrameLayout(
            "frame data is shorter than stride * height".to_string(),
        ));
    }

    Ok(())
}
