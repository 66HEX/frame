use super::{
    ASPECT_OPTIONS, ConversionConfig, CropRect, CropSettings, DEFAULT_CROP_SIZE, DEFAULT_CROP_X,
    DEFAULT_CROP_Y, PreviewControlInput, PreviewMediaKind, PreviewMetadataStatus, PreviewRotation,
    SourceMetadata, clamp_rect, preview_control_availability, preview_source_media_kind,
    transform_crop_rect,
};
use crate::numeric::rounded_f64_to_u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct CropSourceDimensions {
    pub(in crate::app) width: u32,
    pub(in crate::app) height: u32,
}

pub(in crate::app) fn preview_transform_controls_enabled(
    metadata: Option<&SourceMetadata>,
    config: &ConversionConfig,
    controls_disabled: bool,
) -> bool {
    let metadata_status = if metadata.is_some() {
        PreviewMetadataStatus::Ready
    } else {
        PreviewMetadataStatus::Idle
    };
    let availability = preview_control_availability(PreviewControlInput {
        metadata_status,
        source_media_kind: metadata.map(preview_source_media_kind),
        controls_disabled,
        processing_mode: config.processing_mode,
        container: Some(config.container.as_str()),
    });

    availability.media_kind != PreviewMediaKind::Unknown
        && !availability.hide_visual_controls
        && !controls_disabled
}

pub(in crate::app) fn preview_crop_controls_enabled(
    metadata: Option<&SourceMetadata>,
    config: &ConversionConfig,
    controls_disabled: bool,
) -> bool {
    preview_transform_controls_enabled(metadata, config, controls_disabled)
        && preview_crop_source_dimensions(metadata, &config.rotation).is_some()
}

pub(in crate::app) fn preview_crop_source_dimensions(
    metadata: Option<&SourceMetadata>,
    _rotation: &str,
) -> Option<CropSourceDimensions> {
    let metadata = metadata?;
    let (Some(width), Some(height)) = (metadata.width, metadata.height) else {
        return None;
    };
    if width == 0 || height == 0 {
        return None;
    }

    Some(CropSourceDimensions { width, height })
}

pub(in crate::app) fn crop_base_dimensions(
    metadata: Option<&SourceMetadata>,
    rotation: &str,
) -> Option<CropSourceDimensions> {
    let dimensions = preview_crop_source_dimensions(metadata, rotation)?;
    if is_side_rotation(rotation) {
        Some(CropSourceDimensions {
            width: dimensions.height,
            height: dimensions.width,
        })
    } else {
        Some(dimensions)
    }
}

pub(in crate::app) fn crop_rect_from_settings(
    crop: Option<&CropSettings>,
    config: &ConversionConfig,
) -> Option<CropRect> {
    let crop = crop.filter(|crop| crop.enabled)?;
    let (Some(source_width), Some(source_height)) = (crop.source_width, crop.source_height) else {
        return None;
    };
    if source_width == 0 || source_height == 0 {
        return None;
    }

    let raw_rect = CropRect {
        x: f64::from(crop.x) / f64::from(source_width),
        y: f64::from(crop.y) / f64::from(source_height),
        width: f64::from(crop.width) / f64::from(source_width),
        height: f64::from(crop.height) / f64::from(source_height),
    };

    Some(clamp_rect(transform_crop_rect(
        raw_rect,
        PreviewRotation::from(config.rotation.as_str()),
        config.flip_horizontal,
        config.flip_vertical,
        true,
    )))
}

pub(in crate::app) fn crop_settings_from_rect(
    rect: CropRect,
    aspect_id: &str,
    rotation: &str,
    flip_horizontal: bool,
    flip_vertical: bool,
    metadata: Option<&SourceMetadata>,
) -> Option<CropSettings> {
    let dimensions = crop_base_dimensions(metadata, rotation)?;
    let output_rect = clamp_rect(transform_crop_rect(
        rect,
        PreviewRotation::from(rotation),
        flip_horizontal,
        flip_vertical,
        false,
    ));

    Some(CropSettings {
        enabled: true,
        x: round_unit_to_u32(output_rect.x, dimensions.width),
        y: round_unit_to_u32(output_rect.y, dimensions.height),
        width: round_unit_to_u32(output_rect.width, dimensions.width),
        height: round_unit_to_u32(output_rect.height, dimensions.height),
        source_width: Some(dimensions.width),
        source_height: Some(dimensions.height),
        aspect_ratio: (aspect_id != "free").then(|| aspect_id.to_string()),
    })
}

pub(in crate::app) fn round_unit_to_u32(value: f64, scale: u32) -> u32 {
    let scaled = (value * f64::from(scale)).round();
    if scaled <= 0.0 || !scaled.is_finite() {
        0
    } else if scaled >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        rounded_f64_to_u32(scaled)
    }
}

pub(in crate::app) const fn default_crop_rect() -> CropRect {
    CropRect {
        x: DEFAULT_CROP_X,
        y: DEFAULT_CROP_Y,
        width: DEFAULT_CROP_SIZE,
        height: DEFAULT_CROP_SIZE,
    }
}

pub(in crate::app) const fn full_crop_rect() -> CropRect {
    CropRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    }
}

pub(in crate::app) fn crop_rect_is_full(rect: CropRect) -> bool {
    rect.x <= 0.001 && rect.y <= 0.001 && rect.width >= 0.999 && rect.height >= 0.999
}

pub(in crate::app) fn crop_aspect_id(crop: Option<&CropSettings>) -> &str {
    crop.and_then(|crop| crop.aspect_ratio.as_deref())
        .unwrap_or("free")
}

pub(in crate::app) fn is_known_crop_aspect(aspect_id: &str) -> bool {
    ASPECT_OPTIONS.iter().any(|option| option.id == aspect_id)
}

pub(in crate::app) fn next_rotation(rotation: &str) -> String {
    match rotation {
        "0" => "90",
        "90" => "180",
        "180" => "270",
        _ => "0",
    }
    .to_string()
}

pub(in crate::app) fn is_side_rotation(rotation: &str) -> bool {
    matches!(rotation, "90" | "270")
}
