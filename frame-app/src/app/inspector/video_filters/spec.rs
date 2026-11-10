use crate::app::inspector::video_filters::range::VideoFilterRangeTarget;
use crate::settings::{
    ConversionConfig, DeinterlaceMode, FilterStrength, VideoScalarFilter, apply_video_scalar_filter,
};
use frame_core::capabilities::AvailableFilters;

pub(super) struct VideoFilterRangeSpec {
    pub(super) target: VideoFilterRangeTarget,
    pub(super) label: &'static str,
    pub(super) value_label: String,
    pub(super) enabled: bool,
    pub(super) available: bool,
    pub(super) value: i32,
    pub(super) default_value: i32,
    pub(super) min: i32,
    pub(super) max: i32,
}

pub(super) fn video_filter_spec(
    target: VideoFilterRangeTarget,
    enabled: bool,
    value: i32,
    available_filters: &AvailableFilters,
) -> VideoFilterRangeSpec {
    let available = video_filter_available(target, available_filters);
    match target {
        VideoFilterRangeTarget::Brightness => video_spec(
            target,
            "Brightness",
            enabled,
            available,
            value,
            0,
            -100,
            100,
            "%",
        ),
        VideoFilterRangeTarget::Contrast => video_spec(
            target, "Contrast", enabled, available, value, 100, 0, 200, "%",
        ),
        VideoFilterRangeTarget::Saturation => video_spec(
            target,
            "Saturation",
            enabled,
            available,
            value,
            100,
            0,
            300,
            "%",
        ),
        VideoFilterRangeTarget::Gamma => video_spec(
            target, "Gamma", enabled, available, value, 100, 10, 300, "%",
        ),
        VideoFilterRangeTarget::Hue => video_spec(
            target, "Hue", enabled, available, value, 0, -180, 180, " deg",
        ),
        VideoFilterRangeTarget::Temperature => video_spec(
            target,
            "Temperature",
            enabled,
            available,
            value,
            6500,
            2000,
            12_000,
            " K",
        ),
        VideoFilterRangeTarget::Sharpen => video_spec(
            target, "Sharpen", enabled, available, value, 25, 0, 100, "%",
        ),
        VideoFilterRangeTarget::GaussianBlur => video_spec(
            target,
            "Gaussian Blur",
            enabled,
            available,
            value,
            20,
            0,
            100,
            "%",
        ),
        VideoFilterRangeTarget::Deband => {
            video_spec(target, "Deband", enabled, available, value, 25, 0, 100, "%")
        }
        VideoFilterRangeTarget::Vignette => video_spec(
            target, "Vignette", enabled, available, value, 35, 0, 100, "%",
        ),
    }
}

pub(super) const fn video_filter_available(
    target: VideoFilterRangeTarget,
    available_filters: &AvailableFilters,
) -> bool {
    match target {
        VideoFilterRangeTarget::Brightness
        | VideoFilterRangeTarget::Contrast
        | VideoFilterRangeTarget::Saturation
        | VideoFilterRangeTarget::Gamma => available_filters.lutyuv,
        VideoFilterRangeTarget::Hue => available_filters.hue,
        VideoFilterRangeTarget::Temperature => available_filters.colortemperature,
        VideoFilterRangeTarget::Sharpen => available_filters.unsharp,
        VideoFilterRangeTarget::GaussianBlur => available_filters.gblur,
        VideoFilterRangeTarget::Deband => available_filters.deband,
        VideoFilterRangeTarget::Vignette => available_filters.vignette,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Filter specs are compact data literals for the settings UI."
)]
pub(super) fn video_spec(
    target: VideoFilterRangeTarget,
    label: &'static str,
    enabled: bool,
    available: bool,
    value: i32,
    default_value: i32,
    min: i32,
    max: i32,
    suffix: &'static str,
) -> VideoFilterRangeSpec {
    VideoFilterRangeSpec {
        target,
        label,
        value_label: format!("{value}{suffix}"),
        enabled,
        available,
        value,
        default_value,
        min,
        max,
    }
}

pub(super) fn apply_video_filter_range(
    config: &mut ConversionConfig,
    target: VideoFilterRangeTarget,
    enabled: bool,
    value: i32,
) -> bool {
    let filter = match target {
        VideoFilterRangeTarget::Brightness => VideoScalarFilter::Brightness,
        VideoFilterRangeTarget::Contrast => VideoScalarFilter::Contrast,
        VideoFilterRangeTarget::Saturation => VideoScalarFilter::Saturation,
        VideoFilterRangeTarget::Gamma => VideoScalarFilter::Gamma,
        VideoFilterRangeTarget::Hue => VideoScalarFilter::Hue,
        VideoFilterRangeTarget::Temperature => VideoScalarFilter::Temperature,
        VideoFilterRangeTarget::Sharpen => VideoScalarFilter::Sharpen,
        VideoFilterRangeTarget::GaussianBlur => VideoScalarFilter::GaussianBlur,
        VideoFilterRangeTarget::Deband => VideoScalarFilter::Deband,
        VideoFilterRangeTarget::Vignette => VideoScalarFilter::Vignette,
    };
    apply_video_scalar_filter(config, filter, enabled, value)
}

pub(super) const fn video_target_id(target: VideoFilterRangeTarget) -> &'static str {
    match target {
        VideoFilterRangeTarget::Brightness => "brightness",
        VideoFilterRangeTarget::Contrast => "contrast",
        VideoFilterRangeTarget::Saturation => "saturation",
        VideoFilterRangeTarget::Gamma => "gamma",
        VideoFilterRangeTarget::Hue => "hue",
        VideoFilterRangeTarget::Temperature => "temperature",
        VideoFilterRangeTarget::Sharpen => "sharpen",
        VideoFilterRangeTarget::GaussianBlur => "gaussian-blur",
        VideoFilterRangeTarget::Deband => "deband",
        VideoFilterRangeTarget::Vignette => "vignette",
    }
}

pub(super) const fn video_slider_id(target: VideoFilterRangeTarget) -> &'static str {
    match target {
        VideoFilterRangeTarget::Brightness => "settings-video-filter-brightness-slider",
        VideoFilterRangeTarget::Contrast => "settings-video-filter-contrast-slider",
        VideoFilterRangeTarget::Saturation => "settings-video-filter-saturation-slider",
        VideoFilterRangeTarget::Gamma => "settings-video-filter-gamma-slider",
        VideoFilterRangeTarget::Hue => "settings-video-filter-hue-slider",
        VideoFilterRangeTarget::Temperature => "settings-video-filter-temperature-slider",
        VideoFilterRangeTarget::Sharpen => "settings-video-filter-sharpen-slider",
        VideoFilterRangeTarget::GaussianBlur => "settings-video-filter-gaussian-blur-slider",
        VideoFilterRangeTarget::Deband => "settings-video-filter-deband-slider",
        VideoFilterRangeTarget::Vignette => "settings-video-filter-vignette-slider",
    }
}

pub(super) const fn video_handle_id(target: VideoFilterRangeTarget) -> &'static str {
    match target {
        VideoFilterRangeTarget::Brightness => "settings-video-filter-brightness-handle",
        VideoFilterRangeTarget::Contrast => "settings-video-filter-contrast-handle",
        VideoFilterRangeTarget::Saturation => "settings-video-filter-saturation-handle",
        VideoFilterRangeTarget::Gamma => "settings-video-filter-gamma-handle",
        VideoFilterRangeTarget::Hue => "settings-video-filter-hue-handle",
        VideoFilterRangeTarget::Temperature => "settings-video-filter-temperature-handle",
        VideoFilterRangeTarget::Sharpen => "settings-video-filter-sharpen-handle",
        VideoFilterRangeTarget::GaussianBlur => "settings-video-filter-gaussian-blur-handle",
        VideoFilterRangeTarget::Deband => "settings-video-filter-deband-handle",
        VideoFilterRangeTarget::Vignette => "settings-video-filter-vignette-handle",
    }
}

pub(super) const fn video_slider_label(target: VideoFilterRangeTarget) -> &'static str {
    match target {
        VideoFilterRangeTarget::Brightness => "Brightness",
        VideoFilterRangeTarget::Contrast => "Contrast",
        VideoFilterRangeTarget::Saturation => "Saturation",
        VideoFilterRangeTarget::Gamma => "Gamma",
        VideoFilterRangeTarget::Hue => "Hue",
        VideoFilterRangeTarget::Temperature => "Temperature",
        VideoFilterRangeTarget::Sharpen => "Sharpen",
        VideoFilterRangeTarget::GaussianBlur => "Gaussian blur",
        VideoFilterRangeTarget::Deband => "Deband",
        VideoFilterRangeTarget::Vignette => "Vignette",
    }
}

pub(super) const fn filter_strength_id(strength: FilterStrength) -> &'static str {
    match strength {
        FilterStrength::Low => "low",
        FilterStrength::Medium => "medium",
        FilterStrength::High => "high",
    }
}

pub(super) const fn deinterlace_id(mode: DeinterlaceMode) -> &'static str {
    match mode {
        DeinterlaceMode::Off => "off",
        DeinterlaceMode::Auto => "auto",
        DeinterlaceMode::On => "on",
    }
}
