use crate::app::inspector::audio_filters::range::AudioFilterRangeTarget;
use crate::settings::{
    AudioScalarFilter, ConversionConfig, FilterStrength, apply_audio_scalar_filter,
    apply_audio_volume,
};
use frame_core::capabilities::AvailableFilters;

pub(super) struct AudioFilterRangeSpec {
    pub(super) target: AudioFilterRangeTarget,
    pub(super) label: &'static str,
    pub(super) value_label: String,
    pub(super) enabled: bool,
    pub(super) available: bool,
    pub(super) value: i32,
    pub(super) default_value: i32,
    pub(super) min: i32,
    pub(super) max: i32,
}

pub(super) fn audio_filter_spec(
    target: AudioFilterRangeTarget,
    enabled: bool,
    value: i32,
    available_filters: &AvailableFilters,
) -> AudioFilterRangeSpec {
    let available = audio_filter_available(target, available_filters);
    match target {
        AudioFilterRangeTarget::Volume => {
            audio_spec(target, "Volume", true, available, value, 100, 0, 200, "%")
        }
        AudioFilterRangeTarget::Limiter => audio_spec(
            target, "Limiter", enabled, available, value, -1, -12, 0, " dB",
        ),
        AudioFilterRangeTarget::Bass => {
            audio_spec(target, "Bass", enabled, available, value, 0, -20, 20, " dB")
        }
        AudioFilterRangeTarget::Treble => audio_spec(
            target, "Treble", enabled, available, value, 0, -20, 20, " dB",
        ),
        AudioFilterRangeTarget::HighPass => audio_spec(
            target,
            "High-pass",
            enabled,
            available,
            value,
            80,
            20,
            2000,
            " Hz",
        ),
        AudioFilterRangeTarget::LowPass => audio_spec(
            target, "Low-pass", enabled, available, value, 16_000, 1000, 20_000, " Hz",
        ),
        AudioFilterRangeTarget::NoiseReduction => audio_spec(
            target,
            "Noise Reduction",
            enabled,
            available,
            value,
            12,
            1,
            30,
            " dB",
        ),
        AudioFilterRangeTarget::DeEsser => audio_spec(
            target, "De-esser", enabled, available, value, 35, 0, 100, "%",
        ),
        AudioFilterRangeTarget::StereoWidth => audio_spec(
            target,
            "Stereo Width",
            enabled,
            available,
            value,
            100,
            0,
            200,
            "%",
        ),
    }
}

pub(super) const fn audio_filter_available(
    target: AudioFilterRangeTarget,
    available_filters: &AvailableFilters,
) -> bool {
    match target {
        AudioFilterRangeTarget::Volume => available_filters.volume,
        AudioFilterRangeTarget::Limiter => available_filters.alimiter,
        AudioFilterRangeTarget::Bass => available_filters.bass,
        AudioFilterRangeTarget::Treble => available_filters.treble,
        AudioFilterRangeTarget::HighPass => available_filters.highpass,
        AudioFilterRangeTarget::LowPass => available_filters.lowpass,
        AudioFilterRangeTarget::NoiseReduction => available_filters.afftdn,
        AudioFilterRangeTarget::DeEsser => available_filters.deesser,
        AudioFilterRangeTarget::StereoWidth => available_filters.stereotools,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Filter specs are compact data literals for the settings UI."
)]
pub(super) fn audio_spec(
    target: AudioFilterRangeTarget,
    label: &'static str,
    enabled: bool,
    available: bool,
    value: i32,
    default_value: i32,
    min: i32,
    max: i32,
    suffix: &'static str,
) -> AudioFilterRangeSpec {
    AudioFilterRangeSpec {
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

pub(super) fn apply_audio_filter_range(
    config: &mut ConversionConfig,
    target: AudioFilterRangeTarget,
    enabled: bool,
    value: i32,
) -> bool {
    match target {
        AudioFilterRangeTarget::Volume => {
            let Ok(value) = u32::try_from(value) else {
                return false;
            };
            apply_audio_volume(config, value)
        }
        AudioFilterRangeTarget::Limiter => {
            apply_audio_scalar_filter(config, AudioScalarFilter::Limiter, enabled, value)
        }
        AudioFilterRangeTarget::Bass => {
            apply_audio_scalar_filter(config, AudioScalarFilter::Bass, enabled, value)
        }
        AudioFilterRangeTarget::Treble => {
            apply_audio_scalar_filter(config, AudioScalarFilter::Treble, enabled, value)
        }
        AudioFilterRangeTarget::HighPass => {
            apply_audio_scalar_filter(config, AudioScalarFilter::HighPass, enabled, value)
        }
        AudioFilterRangeTarget::LowPass => {
            apply_audio_scalar_filter(config, AudioScalarFilter::LowPass, enabled, value)
        }
        AudioFilterRangeTarget::NoiseReduction => {
            apply_audio_scalar_filter(config, AudioScalarFilter::NoiseReduction, enabled, value)
        }
        AudioFilterRangeTarget::DeEsser => {
            apply_audio_scalar_filter(config, AudioScalarFilter::DeEsser, enabled, value)
        }
        AudioFilterRangeTarget::StereoWidth => {
            apply_audio_scalar_filter(config, AudioScalarFilter::StereoWidth, enabled, value)
        }
    }
}

pub(super) const fn audio_target_id(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "volume",
        AudioFilterRangeTarget::Limiter => "limiter",
        AudioFilterRangeTarget::Bass => "bass",
        AudioFilterRangeTarget::Treble => "treble",
        AudioFilterRangeTarget::HighPass => "high-pass",
        AudioFilterRangeTarget::LowPass => "low-pass",
        AudioFilterRangeTarget::NoiseReduction => "noise-reduction",
        AudioFilterRangeTarget::DeEsser => "de-esser",
        AudioFilterRangeTarget::StereoWidth => "stereo-width",
    }
}

pub(super) const fn audio_slider_id(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "settings-audio-filter-volume-slider",
        AudioFilterRangeTarget::Limiter => "settings-audio-filter-limiter-slider",
        AudioFilterRangeTarget::Bass => "settings-audio-filter-bass-slider",
        AudioFilterRangeTarget::Treble => "settings-audio-filter-treble-slider",
        AudioFilterRangeTarget::HighPass => "settings-audio-filter-high-pass-slider",
        AudioFilterRangeTarget::LowPass => "settings-audio-filter-low-pass-slider",
        AudioFilterRangeTarget::NoiseReduction => "settings-audio-filter-noise-reduction-slider",
        AudioFilterRangeTarget::DeEsser => "settings-audio-filter-de-esser-slider",
        AudioFilterRangeTarget::StereoWidth => "settings-audio-filter-stereo-width-slider",
    }
}

pub(super) const fn audio_handle_id(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "settings-audio-filter-volume-handle",
        AudioFilterRangeTarget::Limiter => "settings-audio-filter-limiter-handle",
        AudioFilterRangeTarget::Bass => "settings-audio-filter-bass-handle",
        AudioFilterRangeTarget::Treble => "settings-audio-filter-treble-handle",
        AudioFilterRangeTarget::HighPass => "settings-audio-filter-high-pass-handle",
        AudioFilterRangeTarget::LowPass => "settings-audio-filter-low-pass-handle",
        AudioFilterRangeTarget::NoiseReduction => "settings-audio-filter-noise-reduction-handle",
        AudioFilterRangeTarget::DeEsser => "settings-audio-filter-de-esser-handle",
        AudioFilterRangeTarget::StereoWidth => "settings-audio-filter-stereo-width-handle",
    }
}

pub(super) const fn audio_slider_label(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "Volume",
        AudioFilterRangeTarget::Limiter => "Limiter",
        AudioFilterRangeTarget::Bass => "Bass",
        AudioFilterRangeTarget::Treble => "Treble",
        AudioFilterRangeTarget::HighPass => "High-pass",
        AudioFilterRangeTarget::LowPass => "Low-pass",
        AudioFilterRangeTarget::NoiseReduction => "Noise reduction",
        AudioFilterRangeTarget::DeEsser => "De-esser",
        AudioFilterRangeTarget::StereoWidth => "Stereo width",
    }
}

pub(super) const fn filter_strength_id(strength: FilterStrength) -> &'static str {
    match strength {
        FilterStrength::Low => "low",
        FilterStrength::Medium => "medium",
        FilterStrength::High => "high",
    }
}
