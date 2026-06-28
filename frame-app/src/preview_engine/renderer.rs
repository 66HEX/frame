use std::sync::Arc;

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use smallvec::SmallVec;

use super::{
    PreviewCrop, PreviewEngineError, PreviewFrame, PreviewPixelFormat, PreviewRenderPresentation,
    PreviewTransform,
};

pub fn render_image_from_frame(
    frame: &PreviewFrame,
) -> Result<Arc<RenderImage>, PreviewEngineError> {
    render_image_from_frame_with_presentation(frame, PreviewRenderPresentation::default())
}

pub fn render_image_from_frame_with_presentation(
    frame: &PreviewFrame,
    presentation: PreviewRenderPresentation,
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
    let image = apply_frame_presentation(image, presentation)?;
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

fn apply_frame_presentation(
    image: RgbaImage,
    presentation: PreviewRenderPresentation,
) -> Result<RgbaImage, PreviewEngineError> {
    let mut image = transform_frame_image(image, presentation.transform);
    if let Some(crop) = presentation.crop {
        image = crop_frame_image(
            image,
            crop,
            presentation.crop_source_width,
            presentation.crop_source_height,
        )?;
    }
    Ok(image)
}

fn transform_frame_image(image: RgbaImage, transform: PreviewTransform) -> RgbaImage {
    if transform.is_identity() {
        return image;
    }

    let image = if transform.flip_horizontal {
        image::imageops::flip_horizontal(&image)
    } else {
        image
    };
    let image = if transform.flip_vertical {
        image::imageops::flip_vertical(&image)
    } else {
        image
    };

    match transform.rotation_degrees {
        90 => image::imageops::rotate90(&image),
        180 => image::imageops::rotate180(&image),
        270 => image::imageops::rotate270(&image),
        _ => image,
    }
}

fn crop_frame_image(
    image: RgbaImage,
    crop: PreviewCrop,
    source_width: Option<u32>,
    source_height: Option<u32>,
) -> Result<RgbaImage, PreviewEngineError> {
    let (Some(source_width), Some(source_height)) = (source_width, source_height) else {
        return Err(PreviewEngineError::InvalidInput(
            "preview render crop requires source dimensions".to_string(),
        ));
    };
    if source_width == 0 || source_height == 0 || image.width() == 0 || image.height() == 0 {
        return Err(PreviewEngineError::InvalidInput(
            "preview render crop dimensions cannot be zero".to_string(),
        ));
    }

    let crop_right = crop.x.checked_add(crop.width);
    let crop_bottom = crop.y.checked_add(crop.height);
    if crop.width == 0
        || crop.height == 0
        || crop_right.is_none_or(|right| right > source_width)
        || crop_bottom.is_none_or(|bottom| bottom > source_height)
    {
        return Err(PreviewEngineError::InvalidInput(
            "preview render crop must fit inside source dimensions".to_string(),
        ));
    }

    let image_width = image.width();
    let image_height = image.height();
    let left = scale_crop_start(crop.x, source_width, image_width);
    let top = scale_crop_start(crop.y, source_height, image_height);
    let right = scale_crop_end(
        crop_right.unwrap_or(source_width),
        source_width,
        image_width,
    )
    .max(left.saturating_add(1))
    .min(image_width);
    let bottom = scale_crop_end(
        crop_bottom.unwrap_or(source_height),
        source_height,
        image_height,
    )
    .max(top.saturating_add(1))
    .min(image_height);

    Ok(image::imageops::crop_imm(
        &image,
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    )
    .to_image())
}

fn scale_crop_start(value: u32, source_extent: u32, image_extent: u32) -> u32 {
    ((f64::from(value) / f64::from(source_extent)) * f64::from(image_extent))
        .floor()
        .clamp(0.0, f64::from(image_extent.saturating_sub(1))) as u32
}

fn scale_crop_end(value: u32, source_extent: u32, image_extent: u32) -> u32 {
    ((f64::from(value) / f64::from(source_extent)) * f64::from(image_extent))
        .ceil()
        .clamp(1.0, f64::from(image_extent)) as u32
}
