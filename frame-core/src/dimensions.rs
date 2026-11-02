//! Output geometry for the UI, following the export filter order.

use crate::{
    media_rules::is_image_container, types::ConversionConfig, utils::is_audio_only_container,
};

/// Estimates the encoded raster size, using display-oriented source dimensions.
/// Unknown metadata or incomplete custom sizes produce no estimate.
#[must_use]
pub fn output_dimensions(
    config: &ConversionConfig,
    source_width: u32,
    source_height: u32,
    source_pixel_format: Option<&str>,
) -> Option<(u32, u32)> {
    if source_width == 0 || source_height == 0 || is_audio_only_container(&config.container) {
        return None;
    }
    let (mut width, mut height) = (source_width, source_height);
    if config.processing_mode == "copy" {
        return Some((width, height));
    }
    if matches!(config.rotation.as_str(), "90" | "270") {
        (width, height) = (height, width);
    }
    if let Some(crop) = config.crop.as_ref().filter(|crop| crop.enabled) {
        width = rounded_dimension(crop.width)?.min(width);
        height = rounded_dimension(crop.height)?.min(height);
        // FFmpeg crop uses exact=0 and rounds down to chroma boundaries.
        let format = source_pixel_format.unwrap_or_default();
        let (horizontal, vertical) = if format.contains("420")
            || matches!(format, "nv12" | "nv21" | "p010le" | "p010be" | "p016le")
        {
            (2, 2)
        } else if format.contains("422") || matches!(format, "yuyv422" | "uyvy422" | "nv16") {
            (2, 1)
        } else {
            (1, 1)
        };
        width = width / horizontal * horizontal;
        height = height / vertical * vertical;
    }
    if width == 0 || height == 0 {
        return None;
    }
    match config.resolution.as_str() {
        "1080p" => {
            width = scaled_dimension(width, 1080, height, 2)?;
            height = 1080;
        }
        "720p" => {
            width = scaled_dimension(width, 720, height, 2)?;
            height = 720;
        }
        "480p" => {
            width = scaled_dimension(width, 480, height, 2)?;
            height = 480;
        }
        "custom" => {
            let parse = |value: Option<&str>| -> Option<Option<u32>> {
                match value {
                    None | Some("-1") => Some(None),
                    Some(value) => value
                        .parse::<u32>()
                        .ok()
                        .filter(|value| *value > 0)
                        .map(Some),
                }
            };
            match (
                parse(config.custom_width.as_deref())?,
                parse(config.custom_height.as_deref())?,
            ) {
                (Some(w), Some(h)) => {
                    // The current scale+pad filter can reject odd video target sizes.
                    // Do not present a successful size for this unsupported configuration.
                    if !is_image_container(&config.container) && (w % 2 != 0 || h % 2 != 0) {
                        return None;
                    }
                    width = w;
                    height = h;
                }
                (Some(w), None) => {
                    height = scaled_dimension(height, w, width, 1)?;
                    width = w;
                }
                (None, Some(h)) => {
                    width = scaled_dimension(width, h, height, 1)?;
                    height = h;
                }
                (None, None) => {}
            }
        }
        _ => {}
    }
    if !is_image_container(&config.container) && !config.container.eq_ignore_ascii_case("gif") {
        width = width.checked_add(width % 2)?;
        height = height.checked_add(height % 2)?;
    }
    Some((width, height))
}

fn rounded_dimension(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 1.0 || value > f64::from(u32::MAX) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Finite positive dimensions are bounded to u32 before rounding."
    )]
    Some(value.round() as u32)
}

fn scaled_dimension(value: u32, target: u32, reference: u32, multiple: u32) -> Option<u32> {
    let denominator = u64::from(reference) * u64::from(multiple);
    let rounded = (u64::from(value) * u64::from(target) + denominator / 2) / denominator;
    u32::try_from(rounded * u64::from(multiple))
        .ok()
        .filter(|value| *value > 0)
}
