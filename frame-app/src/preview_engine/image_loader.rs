use std::path::Path;

use image::RgbaImage;

use super::{PreviewEngineError, PreviewFrame, PreviewTransform};

pub fn load_still_image_frame(
    path: &Path,
    transform: PreviewTransform,
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
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    let (width, height) = rgba.dimensions();
    PreviewFrame::bgra(width, height, width.saturating_mul(4), 0, rgba.into_raw())
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
