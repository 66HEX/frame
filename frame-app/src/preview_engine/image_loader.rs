use std::path::Path;

use image::RgbaImage;

use super::{PreviewCrop, PreviewEngineError, PreviewFrame, PreviewTransform};

/// Loads a still image as a BGRA preview frame.
///
/// # Errors
///
/// Returns an error when the image cannot be opened, decoded, cropped to the
/// requested rectangle, or converted into a valid preview frame layout.
pub fn load_still_image_frame(
    path: &Path,
    transform: PreviewTransform,
    crop: Option<PreviewCrop>,
) -> Result<PreviewFrame, PreviewEngineError> {
    let image = image::ImageReader::open(path)
        .map_err(|source| PreviewEngineError::ImageLoad {
            path: path.to_path_buf(),
            source: image::ImageError::IoError(source),
        })?
        .with_guessed_format()
        .map_err(|source| PreviewEngineError::ImageLoad {
            path: path.to_path_buf(),
            source: image::ImageError::IoError(source),
        })?
        .decode()
        .map_err(|source| PreviewEngineError::ImageLoad {
            path: path.to_path_buf(),
            source,
        })?;

    let mut rgba = transform_still_image(image.into_rgba8(), transform);
    if let Some(crop) = crop {
        rgba = crop_still_image(&rgba, crop)?;
    }
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    let (width, height) = rgba.dimensions();
    PreviewFrame::bgra(width, height, width.saturating_mul(4), 0, rgba.into_raw())
}

fn crop_still_image(image: &RgbaImage, crop: PreviewCrop) -> Result<RgbaImage, PreviewEngineError> {
    let crop_right = crop.x.checked_add(crop.width);
    let crop_bottom = crop.y.checked_add(crop.height);
    if crop.width == 0
        || crop.height == 0
        || crop_right.is_none_or(|right| right > image.width())
        || crop_bottom.is_none_or(|bottom| bottom > image.height())
    {
        return Err(PreviewEngineError::InvalidInput(
            "preview crop must fit inside still image dimensions".to_string(),
        ));
    }

    Ok(image::imageops::crop_imm(image, crop.x, crop.y, crop.width, crop.height).to_image())
}

fn transform_still_image(image: RgbaImage, transform: PreviewTransform) -> RgbaImage {
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
