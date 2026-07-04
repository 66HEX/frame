use std::sync::Arc;

use gpui::{ImageId, RenderImage};
use image::{Frame, RgbaImage};
use smallvec::SmallVec;

use super::{
    PreviewEngineError, PreviewFrame, PreviewPixelFormat, PreviewRenderedFrame,
    validate_frame_layout,
};

/// Converts a preview frame into a GPUI render image.
///
/// # Errors
///
/// Returns an error when the frame is not BGRA, its stride is unsupported, or
/// the byte buffer does not match the declared image dimensions.
pub fn render_image_from_frame(
    frame: &PreviewFrame,
) -> Result<Arc<RenderImage>, PreviewEngineError> {
    if frame.pixel_format != PreviewPixelFormat::Bgra {
        return Err(PreviewEngineError::UnsupportedFrameLayout(
            "only BGRA preview frames can be rendered by the GPUI image path".to_string(),
        ));
    }

    let bytes = tight_frame_bytes(frame)?;
    render_image_from_tight_bgra(frame.width, frame.height, bytes)
}

/// Converts owned BGRA bytes from `FFmpeg` into a published preview frame.
///
/// Tight rows reuse the input allocation directly. Padded rows are compacted
/// into a tightly packed image buffer because GPUI image frames do not carry
/// stride metadata.
///
/// # Errors
///
/// Returns an error when dimensions, stride, or byte length do not describe a
/// valid BGRA image.
pub fn rendered_frame_from_bgra_payload(
    width: u32,
    height: u32,
    stride: u32,
    timestamp_us: u64,
    data: Vec<u8>,
) -> Result<PreviewRenderedFrame, PreviewEngineError> {
    rendered_frame_from_bgra_payload_with_image_id(width, height, stride, timestamp_us, None, data)
}

/// Converts owned BGRA bytes into a published preview frame using a stable
/// image identity and content version.
///
/// # Errors
///
/// Returns an error when dimensions, stride, or byte length do not describe a
/// valid BGRA image.
pub fn rendered_frame_from_bgra_payload_with_image_id(
    width: u32,
    height: u32,
    stride: u32,
    timestamp_us: u64,
    image_identity: Option<(ImageId, u64)>,
    data: Vec<u8>,
) -> Result<PreviewRenderedFrame, PreviewEngineError> {
    let byte_len = data.len();
    validate_frame_layout(width, height, stride, byte_len)?;
    let bytes = tight_bgra_payload(width, height, stride, data)?;
    let render_image = if let Some((image_id, content_version)) = image_identity {
        render_image_from_tight_bgra_with_image_id(width, height, image_id, content_version, bytes)?
    } else {
        render_image_from_tight_bgra(width, height, bytes)?
    };

    Ok(PreviewRenderedFrame::new(
        width,
        height,
        timestamp_us,
        byte_len,
        render_image,
    ))
}

fn render_image_from_tight_bgra(
    width: u32,
    height: u32,
    bytes: Vec<u8>,
) -> Result<Arc<RenderImage>, PreviewEngineError> {
    render_image_from_tight_bgra_with_builder(width, height, bytes, RenderImage::new)
}

fn render_image_from_tight_bgra_with_image_id(
    width: u32,
    height: u32,
    image_id: ImageId,
    content_version: u64,
    bytes: Vec<u8>,
) -> Result<Arc<RenderImage>, PreviewEngineError> {
    render_image_from_tight_bgra_with_builder(width, height, bytes, |frames| {
        RenderImage::new_with_id(image_id, content_version, frames)
    })
}

fn render_image_from_tight_bgra_with_builder(
    width: u32,
    height: u32,
    bytes: Vec<u8>,
    build: impl FnOnce(SmallVec<[Frame; 1]>) -> RenderImage,
) -> Result<Arc<RenderImage>, PreviewEngineError> {
    let image = RgbaImage::from_raw(width, height, bytes).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout(
            "frame bytes do not match image dimensions".to_string(),
        )
    })?;
    let mut frames = SmallVec::<[Frame; 1]>::new();
    frames.push(Frame::new(image));
    Ok(Arc::new(build(frames)))
}

fn tight_bgra_payload(
    width: u32,
    height: u32,
    stride: u32,
    mut data: Vec<u8>,
) -> Result<Vec<u8>, PreviewEngineError> {
    let row_len = checked_row_len(width)?;
    let height = checked_height(height)?;
    let stride = checked_stride(stride)?;
    let len = checked_frame_len(row_len, height)?;

    if stride == row_len {
        data.truncate(len);
        return Ok(data);
    }

    let mut bytes = Vec::with_capacity(len);
    for row in 0..height {
        let start = row.checked_mul(stride).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame row offset overflow".to_string())
        })?;
        let end = start.checked_add(row_len).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame row end overflow".to_string())
        })?;
        bytes.extend_from_slice(data.get(start..end).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame data is incomplete".to_string())
        })?);
    }
    Ok(bytes)
}

fn tight_frame_bytes(frame: &PreviewFrame) -> Result<Vec<u8>, PreviewEngineError> {
    let row_len = checked_row_len(frame.width)?;
    let height = checked_height(frame.height)?;
    let stride = checked_stride(frame.stride)?;

    if stride == row_len {
        let len = checked_frame_len(row_len, height)?;
        return frame
            .bytes()
            .get(0..len)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| {
                PreviewEngineError::UnsupportedFrameLayout("frame data is incomplete".to_string())
            });
    }

    let mut bytes = Vec::with_capacity(row_len.checked_mul(height).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout("frame byte length overflow".to_string())
    })?);
    for row in 0..height {
        let start = row.checked_mul(stride).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame row offset overflow".to_string())
        })?;
        let end = start.checked_add(row_len).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame row end overflow".to_string())
        })?;
        bytes.extend_from_slice(frame.bytes().get(start..end).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame data is incomplete".to_string())
        })?);
    }
    Ok(bytes)
}

fn checked_row_len(width: u32) -> Result<usize, PreviewEngineError> {
    usize::try_from(width.checked_mul(4).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout("frame row length overflow".to_string())
    })?)
    .map_err(|_| {
        PreviewEngineError::UnsupportedFrameLayout("frame row length is too large".to_string())
    })
}

fn checked_height(height: u32) -> Result<usize, PreviewEngineError> {
    usize::try_from(height).map_err(|_| {
        PreviewEngineError::UnsupportedFrameLayout("frame height is too large".to_string())
    })
}

fn checked_stride(stride: u32) -> Result<usize, PreviewEngineError> {
    usize::try_from(stride).map_err(|_| {
        PreviewEngineError::UnsupportedFrameLayout("frame stride is too large".to_string())
    })
}

fn checked_frame_len(row_len: usize, height: usize) -> Result<usize, PreviewEngineError> {
    row_len.checked_mul(height).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout("frame byte length overflow".to_string())
    })
}
