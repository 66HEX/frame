use std::{path::PathBuf, sync::Arc};

use frame_core::types::ConversionConfig as CoreConversionConfig;
use gpui::RenderImage;

use crate::numeric::rounded_f64_to_u32;

use super::{PreviewEngineError, PreviewFrameStats, PreviewRuntimeMetrics};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewCrop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PreviewRenderPresentation {
    pub transform: PreviewTransform,
    pub crop: Option<PreviewCrop>,
    pub crop_source_width: Option<u32>,
    pub crop_source_height: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub timestamp_us: u64,
    pub pixel_format: PreviewPixelFormat,
    data: Arc<[u8]>,
}

#[derive(Clone, Debug)]
pub struct PreviewRenderedFrame {
    pub width: u32,
    pub height: u32,
    pub timestamp_us: u64,
    pub byte_len: usize,
    render_image: Arc<RenderImage>,
}

impl PartialEq for PreviewRenderedFrame {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.timestamp_us == other.timestamp_us
            && self.byte_len == other.byte_len
            && self.render_image.id == other.render_image.id
            && self.render_image.content_version() == other.render_image.content_version()
    }
}

impl Eq for PreviewRenderedFrame {}

impl PreviewRenderedFrame {
    #[must_use]
    pub const fn new(
        width: u32,
        height: u32,
        timestamp_us: u64,
        byte_len: usize,
        render_image: Arc<RenderImage>,
    ) -> Self {
        Self {
            width,
            height,
            timestamp_us,
            byte_len,
            render_image,
        }
    }

    #[must_use]
    pub fn render_image(&self) -> Arc<RenderImage> {
        Arc::clone(&self.render_image)
    }

    #[must_use]
    pub const fn dimensions(&self) -> PreviewDimensions {
        PreviewDimensions {
            width: self.width,
            height: self.height,
        }
    }
}

impl PreviewFrame {
    /// Creates a BGRA preview frame after validating its memory layout.
    ///
    /// # Errors
    ///
    /// Returns an error when dimensions, stride, or byte length do not describe
    /// a supported BGRA frame.
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
    pub const fn dimensions(&self) -> PreviewDimensions {
        PreviewDimensions {
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreviewSessionConfig {
    pub file_id: String,
    pub path: PathBuf,
    pub source_kind: PreviewSourceKind,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub has_audio: bool,
    pub selected_audio_track: Option<u32>,
    pub duration_seconds: f64,
    pub max_width: u32,
    pub max_height: u32,
    pub fps: u32,
    pub conversion_config: CoreConversionConfig,
}

impl PreviewSessionConfig {
    /// Validates that the preview session can be started.
    ///
    /// # Errors
    ///
    /// Returns an error when required identifiers, paths, dimensions, frame
    /// rate, or crop rectangles are invalid.
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
        if self.source_kind == PreviewSourceKind::Audio && !self.has_audio {
            return Err(PreviewEngineError::InvalidInput(
                "Audio preview sources must declare an audio stream".to_string(),
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
        PreviewDimensions {
            width: even_dimension(self.max_width),
            height: even_dimension(self.max_height),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewPlaybackSnapshot {
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub playing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub frame_stats: PreviewFrameStats,
    pub runtime_metrics: PreviewRuntimeMetrics,
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
    let width_scale = f64::from(max_width) / f64::from(source_width);
    let height_scale = f64::from(max_height) / f64::from(source_height);
    let scale = width_scale.min(height_scale).min(1.0);

    PreviewDimensions {
        width: even_dimension(rounded_f64_to_u32(f64::from(source_width) * scale)),
        height: even_dimension(rounded_f64_to_u32(f64::from(source_height) * scale)),
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

pub(super) fn validate_frame_layout(
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
