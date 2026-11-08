//! Output geometry for the UI, following the export filter order.

use crate::{
    media_rules::is_image_container, types::ConversionConfig, utils::is_audio_only_container,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OutputRasterPolicy {
    Source,
    Exact,
    Even,
}

#[must_use]
pub fn output_raster_policy(container: &str, copy_mode: bool) -> OutputRasterPolicy {
    if copy_mode || is_audio_only_container(container) {
        OutputRasterPolicy::Source
    } else if is_image_container(container) || container.eq_ignore_ascii_case("gif") {
        OutputRasterPolicy::Exact
    } else {
        OutputRasterPolicy::Even
    }
}

/// Estimates the encoded raster size, using display-oriented source dimensions.
///
/// Unknown metadata or incomplete custom sizes produce no estimate.
/// Exact cropping no longer depends on the source's chroma subsampling.
#[must_use]
pub fn output_dimensions(
    config: &ConversionConfig,
    source_width: u32,
    source_height: u32,
    _source_pixel_format: Option<&str>,
) -> Option<(u32, u32)> {
    if source_width == 0 || source_height == 0 || is_audio_only_container(&config.container) {
        return None;
    }
    if config.processing_mode == "copy" {
        return Some((source_width, source_height));
    }
    let geometry = filter_geometry(config, Some((source_width, source_height)), true)?;
    let (mut width, mut height) = geometry.canvas;
    if output_raster_policy(&config.container, false) == OutputRasterPolicy::Even {
        width = width.checked_add(width % 2)?;
        height = height.checked_add(height % 2)?;
    }
    Some((width, height))
}

/// Geometry at the scale and padding boundaries of the visual filter graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FilterGeometry {
    pub scaled: (u32, u32),
    pub canvas: (u32, u32),
}

pub(crate) fn filter_geometry(
    config: &ConversionConfig,
    source_dimensions: Option<(u32, u32)>,
    include_scale: bool,
) -> Option<FilterGeometry> {
    let mut dimensions = rotated_source_dimensions(config, source_dimensions);
    if let Some(crop) = config.crop.as_ref().filter(|crop| crop.enabled) {
        let width = rounded_dimension(crop.width)?;
        let height = rounded_dimension(crop.height)?;
        dimensions = Some(dimensions.map_or((width, height), |source| {
            (width.min(source.0), height.min(source.1))
        }));
    }
    let (mut width, mut height) = dimensions?;
    if !include_scale {
        return Some(FilterGeometry {
            scaled: (width, height),
            canvas: (width, height),
        });
    }
    match config.resolution.as_str() {
        "1080p" | "720p" | "480p" => {
            let target = match config.resolution.as_str() {
                "1080p" => 1080,
                "720p" => 720,
                _ => 480,
            };
            width = scaled_dimension(width, target, height, 2)?;
            height = target;
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
                    // Match scale's force_original_aspect_ratio=decrease:
                    // each candidate dimension is rounded before taking min.
                    let scaled = (
                        w.min(scaled_dimension(width, h, height, 1)?),
                        h.min(scaled_dimension(height, w, width, 1)?),
                    );
                    return Some(FilterGeometry {
                        scaled,
                        canvas: (w, h),
                    });
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
    Some(FilterGeometry {
        scaled: (width, height),
        canvas: (width, height),
    })
}

pub(crate) fn rotated_source_dimensions(
    config: &ConversionConfig,
    source_dimensions: Option<(u32, u32)>,
) -> Option<(u32, u32)> {
    let dimensions = source_dimensions.filter(|&(width, height)| width > 0 && height > 0)?;
    Some(if matches!(config.rotation.as_str(), "90" | "270") {
        (dimensions.1, dimensions.0)
    } else {
        dimensions
    })
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
    u32::try_from(rounded.max(1) * u64::from(multiple))
        .ok()
        .filter(|value| *value > 0)
}
