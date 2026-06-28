use std::sync::Arc;

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use smallvec::SmallVec;

use super::{PreviewEngineError, PreviewFrame, PreviewPixelFormat};

pub fn render_image_from_frame(
    frame: &PreviewFrame,
) -> Result<Arc<RenderImage>, PreviewEngineError> {
    if frame.pixel_format != PreviewPixelFormat::Bgra {
        return Err(PreviewEngineError::UnsupportedFrameLayout(
            "only BGRA preview frames can be rendered by the GPUI image path".to_string(),
        ));
    }

    let bytes = tight_frame_bytes(frame)?;
    let image = RgbaImage::from_raw(frame.width, frame.height, bytes).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout(
            "frame bytes do not match image dimensions".to_string(),
        )
    })?;
    let mut frames = SmallVec::<[Frame; 1]>::new();
    frames.push(Frame::new(image));
    Ok(Arc::new(RenderImage::new(frames)))
}

fn tight_frame_bytes(frame: &PreviewFrame) -> Result<Vec<u8>, PreviewEngineError> {
    let row_len = usize::try_from(frame.width.checked_mul(4).ok_or_else(|| {
        PreviewEngineError::UnsupportedFrameLayout("frame row length overflow".to_string())
    })?)
    .map_err(|_| {
        PreviewEngineError::UnsupportedFrameLayout("frame row length is too large".to_string())
    })?;
    let height = usize::try_from(frame.height).map_err(|_| {
        PreviewEngineError::UnsupportedFrameLayout("frame height is too large".to_string())
    })?;
    let stride = usize::try_from(frame.stride).map_err(|_| {
        PreviewEngineError::UnsupportedFrameLayout("frame stride is too large".to_string())
    })?;

    if stride == row_len {
        let len = row_len.checked_mul(height).ok_or_else(|| {
            PreviewEngineError::UnsupportedFrameLayout("frame byte length overflow".to_string())
        })?;
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
