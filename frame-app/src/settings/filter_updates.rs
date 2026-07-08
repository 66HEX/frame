use super::model::{
    AudioFiltersConfig, ConversionConfig, DeinterlaceMode, FilterStrength, FilterValue,
    VideoFiltersConfig,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VideoScalarFilter {
    Brightness,
    Contrast,
    Saturation,
    Gamma,
    Hue,
    Temperature,
    Sharpen,
    GaussianBlur,
    Deband,
    Vignette,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioScalarFilter {
    Limiter,
    Bass,
    Treble,
    HighPass,
    LowPass,
    NoiseReduction,
    DeEsser,
    StereoWidth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassFilterEdited {
    HighPass,
    LowPass,
}

pub fn apply_video_scalar_filter(
    config: &mut ConversionConfig,
    filter: VideoScalarFilter,
    enabled: bool,
    value: i32,
) -> bool {
    let before = config.video_filters;
    match filter {
        VideoScalarFilter::Brightness => {
            apply_i32(
                &mut config.video_filters.color.brightness,
                enabled,
                value,
                -100,
                100,
            );
        }
        VideoScalarFilter::Contrast => {
            apply_u32(
                &mut config.video_filters.color.contrast,
                enabled,
                value,
                0,
                200,
            );
        }
        VideoScalarFilter::Saturation => {
            apply_u32(
                &mut config.video_filters.color.saturation,
                enabled,
                value,
                0,
                300,
            );
        }
        VideoScalarFilter::Gamma => {
            apply_u32(
                &mut config.video_filters.color.gamma,
                enabled,
                value,
                10,
                300,
            );
        }
        VideoScalarFilter::Hue => {
            apply_i32(&mut config.video_filters.hue, enabled, value, -180, 180);
        }
        VideoScalarFilter::Temperature => {
            apply_u32(
                &mut config.video_filters.temperature,
                enabled,
                value,
                2000,
                12_000,
            );
        }
        VideoScalarFilter::Sharpen => {
            apply_u32(&mut config.video_filters.sharpen, enabled, value, 0, 100);
        }
        VideoScalarFilter::GaussianBlur => {
            apply_u32(
                &mut config.video_filters.gaussian_blur,
                enabled,
                value,
                0,
                100,
            );
        }
        VideoScalarFilter::Deband => {
            apply_u32(&mut config.video_filters.deband, enabled, value, 0, 100);
        }
        VideoScalarFilter::Vignette => {
            apply_u32(&mut config.video_filters.vignette, enabled, value, 0, 100);
        }
    }
    before != config.video_filters
}

pub fn apply_video_denoise(
    config: &mut ConversionConfig,
    enabled: bool,
    strength: FilterStrength,
) -> bool {
    let before = config.video_filters;
    config.video_filters.denoise_enabled = enabled;
    config.video_filters.denoise_strength = strength;
    before != config.video_filters
}

pub const fn apply_video_grayscale(config: &mut ConversionConfig, enabled: bool) -> bool {
    let before = config.video_filters.grayscale;
    config.video_filters.grayscale = enabled;
    before != enabled
}

pub fn apply_video_deinterlace(config: &mut ConversionConfig, mode: DeinterlaceMode) -> bool {
    let before = config.video_filters.deinterlace;
    config.video_filters.deinterlace = mode;
    before != mode
}

pub fn reset_video_filters(config: &mut ConversionConfig) -> bool {
    let before = config.video_filters;
    config.video_filters = VideoFiltersConfig::default();
    before != config.video_filters
}

pub fn apply_audio_scalar_filter(
    config: &mut ConversionConfig,
    filter: AudioScalarFilter,
    enabled: bool,
    value: i32,
) -> bool {
    let before = config.audio_filters;
    match filter {
        AudioScalarFilter::Limiter => {
            apply_i32(&mut config.audio_filters.limiter, enabled, value, -12, 0);
        }
        AudioScalarFilter::Bass => {
            apply_i32(&mut config.audio_filters.bass, enabled, value, -20, 20);
        }
        AudioScalarFilter::Treble => {
            apply_i32(&mut config.audio_filters.treble, enabled, value, -20, 20);
        }
        AudioScalarFilter::HighPass => {
            apply_u32(
                &mut config.audio_filters.high_pass,
                enabled,
                value,
                20,
                2000,
            );
            normalize_pass_gap(&mut config.audio_filters, PassFilterEdited::HighPass);
        }
        AudioScalarFilter::LowPass => {
            apply_u32(
                &mut config.audio_filters.low_pass,
                enabled,
                value,
                1000,
                20_000,
            );
            normalize_pass_gap(&mut config.audio_filters, PassFilterEdited::LowPass);
        }
        AudioScalarFilter::NoiseReduction => {
            apply_u32(
                &mut config.audio_filters.noise_reduction,
                enabled,
                value,
                1,
                30,
            );
        }
        AudioScalarFilter::DeEsser => {
            apply_u32(&mut config.audio_filters.de_esser, enabled, value, 0, 100);
        }
        AudioScalarFilter::StereoWidth => {
            apply_u32(
                &mut config.audio_filters.stereo_width,
                enabled,
                value,
                0,
                200,
            );
        }
    }
    before != config.audio_filters
}

pub fn apply_audio_compressor(
    config: &mut ConversionConfig,
    enabled: bool,
    strength: FilterStrength,
) -> bool {
    let before = config.audio_filters;
    config.audio_filters.compressor_enabled = enabled;
    config.audio_filters.compressor_strength = strength;
    before != config.audio_filters
}

pub fn reset_audio_filters(config: &mut ConversionConfig) -> bool {
    let before_filters = config.audio_filters;
    let before_volume = config.audio_volume;
    let before_normalize = config.audio_normalize;
    config.audio_filters = AudioFiltersConfig::default();
    config.audio_volume = super::model::DEFAULT_AUDIO_VOLUME;
    config.audio_normalize = false;
    before_filters != config.audio_filters
        || before_volume != config.audio_volume
        || before_normalize != config.audio_normalize
}

#[must_use]
pub fn has_active_video_filters(config: &ConversionConfig) -> bool {
    let filters = &config.video_filters;
    filters.color.brightness.enabled
        || filters.color.contrast.enabled
        || filters.color.saturation.enabled
        || filters.color.gamma.enabled
        || filters.hue.enabled
        || filters.temperature.enabled
        || filters.sharpen.enabled
        || filters.gaussian_blur.enabled
        || filters.denoise_enabled
        || filters.deband.enabled
        || filters.vignette.enabled
        || filters.grayscale
        || filters.deinterlace != DeinterlaceMode::Off
}

#[must_use]
pub const fn has_active_audio_filters(config: &ConversionConfig) -> bool {
    config.audio_volume != super::model::DEFAULT_AUDIO_VOLUME
        || config.audio_normalize
        || config.audio_filters.compressor_enabled
        || config.audio_filters.limiter.enabled
        || config.audio_filters.bass.enabled
        || config.audio_filters.treble.enabled
        || config.audio_filters.high_pass.enabled
        || config.audio_filters.low_pass.enabled
        || config.audio_filters.noise_reduction.enabled
        || config.audio_filters.de_esser.enabled
        || config.audio_filters.stereo_width.enabled
}

fn apply_i32(target: &mut FilterValue<i32>, enabled: bool, value: i32, min: i32, max: i32) {
    target.enabled = enabled;
    target.value = value.clamp(min, max);
}

fn apply_u32(target: &mut FilterValue<u32>, enabled: bool, value: i32, min: u32, max: u32) {
    target.enabled = enabled;
    let min_i32 = i32::try_from(min).unwrap_or(i32::MAX);
    let max_i32 = i32::try_from(max).unwrap_or(i32::MAX);
    let clamped = value.clamp(min_i32, max_i32);
    target.value = u32::try_from(clamped).unwrap_or(min);
}

fn normalize_pass_gap(filters: &mut AudioFiltersConfig, edited: PassFilterEdited) {
    const MIN_GAP: u32 = 100;

    if !filters.high_pass.enabled || !filters.low_pass.enabled {
        return;
    }

    if filters.high_pass.value + MIN_GAP <= filters.low_pass.value {
        return;
    }

    match edited {
        PassFilterEdited::HighPass => {
            filters.low_pass.value = (filters.high_pass.value + MIN_GAP).clamp(1000, 20_000);
            if filters.high_pass.value + MIN_GAP > filters.low_pass.value {
                filters.high_pass.value = filters.low_pass.value.saturating_sub(MIN_GAP).max(20);
            }
        }
        PassFilterEdited::LowPass => {
            filters.high_pass.value = filters
                .low_pass
                .value
                .saturating_sub(MIN_GAP)
                .clamp(20, 2000);
            if filters.high_pass.value + MIN_GAP > filters.low_pass.value {
                filters.low_pass.value = (filters.high_pass.value + MIN_GAP).min(20_000);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_audio_filters_preserves_audio_bitrate_mode() {
        let mut config = ConversionConfig {
            audio_bitrate_mode: "vbr".to_string(),
            audio_normalize: true,
            audio_volume: 140,
            ..ConversionConfig::default()
        };

        assert!(reset_audio_filters(&mut config));

        assert_eq!(config.audio_bitrate_mode, "vbr");
    }

    #[test]
    fn high_pass_update_preserves_minimum_gap() {
        let mut config = ConversionConfig::default();
        assert!(apply_audio_scalar_filter(
            &mut config,
            AudioScalarFilter::LowPass,
            true,
            1000,
        ));

        assert!(apply_audio_scalar_filter(
            &mut config,
            AudioScalarFilter::HighPass,
            true,
            980,
        ));

        assert_eq!(config.audio_filters.low_pass.value, 1080);
    }
}
