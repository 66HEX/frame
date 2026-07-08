//! `FFmpeg` media filter builders for user-facing video and audio effects.

use crate::{
    error::ConversionError,
    media_rules::{container_supports_audio, is_image_container},
    types::{
        AudioFiltersConfig, ConversionConfig, DeinterlaceMode, FilterStrength, FilterValue,
        VideoFiltersConfig,
    },
};

/// Formats `FFmpeg` floating point values with deterministic precision.
#[must_use]
pub fn format_filter_float(value: f64) -> String {
    format!("{value:.3}")
}

/// Returns true when any video/image effect is active.
#[must_use]
pub fn has_active_video_filters(filters: &VideoFiltersConfig) -> bool {
    let color = filters.color;
    color.brightness.enabled
        || color.contrast.enabled
        || color.saturation.enabled
        || color.gamma.enabled
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

/// Returns true when any audio effect beyond legacy volume/normalize is active.
#[must_use]
pub const fn has_active_audio_effect_filters(filters: &AudioFiltersConfig) -> bool {
    filters.compressor_enabled
        || filters.limiter.enabled
        || filters.bass.enabled
        || filters.treble.enabled
        || filters.high_pass.enabled
        || filters.low_pass.enabled
        || filters.noise_reduction.enabled
        || filters.de_esser.enabled
        || filters.stereo_width.enabled
}

/// Returns true when any audio filter, including legacy volume/normalize, is active.
#[must_use]
pub fn has_active_audio_filters(config: &ConversionConfig) -> bool {
    config.audio_normalize
        || (config.audio_volume - 100.0).abs() > crate::types::VOLUME_EPSILON
        || has_active_audio_effect_filters(&config.audio_filters)
}

/// Builds filters that must run before resolution scaling.
#[must_use]
pub fn build_video_pre_scale_filters(config: &VideoFiltersConfig, is_image: bool) -> Vec<String> {
    let mut filters = Vec::new();

    if let Some(eq) = build_eq_filter(config) {
        filters.push(eq);
    }
    if config.hue.enabled {
        filters.push(format!("hue=h={}", config.hue.value));
    }
    if config.temperature.enabled {
        filters.push(format!(
            "colortemperature=temperature={}:mix={}:pl={}",
            config.temperature.value,
            format_filter_float(1.0),
            format_filter_float(1.0)
        ));
    }
    if config.denoise_enabled {
        filters.push(build_denoise_filter(config.denoise_strength, is_image));
    }
    if matches!(
        config.deinterlace,
        DeinterlaceMode::Auto | DeinterlaceMode::On
    ) && !is_image
    {
        let deint = match config.deinterlace {
            DeinterlaceMode::Auto => "interlaced",
            DeinterlaceMode::On => "all",
            DeinterlaceMode::Off => unreachable!("guarded by matches"),
        };
        filters.push(format!("bwdif=mode=send_frame:parity=auto:deint={deint}"));
    }

    filters
}

/// Builds filters that must run after resolution scaling.
#[must_use]
pub fn build_video_post_scale_filters(config: &VideoFiltersConfig) -> Vec<String> {
    let mut filters = Vec::new();

    if config.sharpen.enabled {
        filters.push(format!(
            "unsharp=luma_msize_x=5:luma_msize_y=5:luma_amount={}:chroma_amount={}",
            format_filter_float(f64::from(config.sharpen.value) / 40.0),
            format_filter_float(0.0)
        ));
    }
    if config.gaussian_blur.enabled {
        filters.push(format!(
            "gblur=sigma={}:steps=1",
            format_filter_float(f64::from(config.gaussian_blur.value).mul_add(0.199, 0.1))
        ));
    }
    if config.deband.enabled {
        let threshold =
            format_filter_float(f64::from(config.deband.value).mul_add(0.000_78, 0.002));
        filters.push(format!(
            "deband=1thr={threshold}:2thr={threshold}:3thr={threshold}:4thr={threshold}:range=16:blur=1:coupling=0"
        ));
    }
    if config.vignette.enabled {
        let denominator = 8.0_f64.mul_add(-(f64::from(config.vignette.value) / 100.0), 12.0);
        filters.push(format!(
            "vignette=angle=PI/{}:mode=forward:dither=1",
            format_filter_float(denominator)
        ));
    }
    if config.grayscale {
        filters.push("hue=s=0".to_string());
    }

    filters
}

/// Builds the audio effects chain in the approved order.
#[must_use]
pub fn build_audio_effect_filters(config: &ConversionConfig) -> Vec<String> {
    let filters = &config.audio_filters;
    let mut chain = Vec::new();

    if filters.high_pass.enabled {
        chain.push(format!(
            "highpass=frequency={}:poles=2:width_type=q:width={}",
            format_filter_float(f64::from(filters.high_pass.value)),
            format_filter_float(0.707)
        ));
    }
    if filters.low_pass.enabled {
        chain.push(format!(
            "lowpass=frequency={}:poles=2:width_type=q:width={}",
            format_filter_float(f64::from(filters.low_pass.value)),
            format_filter_float(0.707)
        ));
    }
    if filters.noise_reduction.enabled {
        chain.push(format!(
            "afftdn=noise_reduction={}:noise_floor={}:residual_floor={}:noise_type=white:track_noise=0",
            format_filter_float(f64::from(filters.noise_reduction.value)),
            format_filter_float(-50.0),
            format_filter_float(-38.0)
        ));
    }
    if filters.de_esser.enabled {
        chain.push(format!(
            "deesser=i={}:m={}:f={}:s=o",
            format_filter_float(f64::from(filters.de_esser.value) / 100.0),
            format_filter_float(0.5),
            format_filter_float(0.5)
        ));
    }
    if filters.bass.enabled {
        chain.push(format!(
            "bass=gain={}:frequency={}:width_type=q:width={}",
            format_filter_float(f64::from(filters.bass.value)),
            format_filter_float(100.0),
            format_filter_float(0.5)
        ));
    }
    if filters.treble.enabled {
        chain.push(format!(
            "treble=gain={}:frequency={}:width_type=q:width={}",
            format_filter_float(f64::from(filters.treble.value)),
            format_filter_float(3000.0),
            format_filter_float(0.5)
        ));
    }
    if filters.compressor_enabled {
        chain.push(build_compressor_filter(filters.compressor_strength));
    }
    if config.audio_normalize {
        chain.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
    }
    if (config.audio_volume - 100.0).abs() > crate::types::VOLUME_EPSILON {
        chain.push(format!(
            "volume={}",
            format_filter_float(config.audio_volume / 100.0)
        ));
    }
    if filters.stereo_width.enabled && filters.stereo_width.value != 100 {
        let side_level = (f64::from(filters.stereo_width.value) / 100.0).clamp(0.015_625, 2.0);
        chain.push(format!(
            "stereotools=mode=lr>ms:slev={}:mlev={}",
            format_filter_float(side_level),
            format_filter_float(1.0)
        ));
        chain.push("stereotools=mode=ms>lr".to_string());
    }
    if filters.limiter.enabled {
        let limit = 10_f64.powf(f64::from(filters.limiter.value) / 20.0);
        chain.push(format!(
            "alimiter=limit={}:attack={}:release={}:level=0:latency=1",
            format_filter_float(limit),
            format_filter_float(5.0),
            format_filter_float(50.0)
        ));
    }

    chain
}

/// Validates active media filters against UI ranges and output constraints.
///
/// # Errors
///
/// Returns [`ConversionError`] when a filter is active for an incompatible
/// output mode or when a value is outside the accepted UI range.
pub fn validate_media_filters(config: &ConversionConfig) -> Result<(), ConversionError> {
    let is_copy = config.processing_mode == "copy";
    let is_audio_only = crate::utils::is_audio_only_container(&config.container);
    let is_image = is_image_container(&config.container);
    let supports_audio = container_supports_audio(&config.container);

    if is_copy
        && (has_active_video_filters(&config.video_filters) || has_active_audio_filters(config))
    {
        return Err(ConversionError::InvalidInput(
            "Media filters require re-encode mode; disable filters before stream copy".to_string(),
        ));
    }
    if is_audio_only && has_active_video_filters(&config.video_filters) {
        return Err(ConversionError::InvalidInput(
            "Video filters cannot be used with audio-only output".to_string(),
        ));
    }
    if !supports_audio && has_active_audio_filters(config) {
        return Err(ConversionError::InvalidInput(
            "Audio filters cannot be used with an output that has no audio stream".to_string(),
        ));
    }
    if is_image && config.video_filters.deinterlace != DeinterlaceMode::Off {
        return Err(ConversionError::InvalidInput(
            "Deinterlace cannot be used for image output".to_string(),
        ));
    }

    validate_video_filters(&config.video_filters)?;
    validate_audio_filters(&config.audio_filters)
}

fn build_eq_filter(config: &VideoFiltersConfig) -> Option<String> {
    let color = config.color;
    let mut parts = Vec::new();

    if color.brightness.enabled {
        parts.push(format!(
            "brightness={}",
            format_filter_float(f64::from(color.brightness.value) / 100.0)
        ));
    }
    if color.contrast.enabled {
        parts.push(format!(
            "contrast={}",
            format_filter_float(f64::from(color.contrast.value) / 100.0)
        ));
    }
    if color.saturation.enabled {
        parts.push(format!(
            "saturation={}",
            format_filter_float(f64::from(color.saturation.value) / 100.0)
        ));
    }
    if color.gamma.enabled {
        parts.push(format!(
            "gamma={}",
            format_filter_float(f64::from(color.gamma.value) / 100.0)
        ));
    }

    (!parts.is_empty()).then(|| format!("eq={}", parts.join(":")))
}

fn build_denoise_filter(strength: FilterStrength, is_image: bool) -> String {
    let (luma_spatial, chroma_spatial, luma_tmp, chroma_tmp) = match strength {
        FilterStrength::Low => (1.5, 1.0, 3.0, 2.0),
        FilterStrength::Medium => (3.0, 2.25, 6.0, 4.5),
        FilterStrength::High => (6.0, 4.5, 9.0, 6.75),
    };
    let (luma_tmp, chroma_tmp) = if is_image {
        (0.0, 0.0)
    } else {
        (luma_tmp, chroma_tmp)
    };

    format!(
        "hqdn3d={}:{}:{}:{}",
        format_filter_float(luma_spatial),
        format_filter_float(chroma_spatial),
        format_filter_float(luma_tmp),
        format_filter_float(chroma_tmp)
    )
}

fn build_compressor_filter(strength: FilterStrength) -> String {
    let (threshold, ratio, attack, release, makeup) = match strength {
        FilterStrength::Low => (0.250, 2.0, 20.0, 250.0, 1.0),
        FilterStrength::Medium => (0.125, 4.0, 10.0, 200.0, 1.5),
        FilterStrength::High => (0.0625, 8.0, 5.0, 150.0, 2.0),
    };

    format!(
        "acompressor=mode=downward:threshold={}:ratio={}:attack={}:release={}:makeup={}:knee={}:link=average:detection=rms:mix={}",
        format_filter_float(threshold),
        format_filter_float(ratio),
        format_filter_float(attack),
        format_filter_float(release),
        format_filter_float(makeup),
        format_filter_float(2.828),
        format_filter_float(1.0)
    )
}

fn validate_video_filters(filters: &VideoFiltersConfig) -> Result<(), ConversionError> {
    validate_i32(filters.color.brightness, -100, 100, "Brightness")?;
    validate_u32(filters.color.contrast, 0, 200, "Contrast")?;
    validate_u32(filters.color.saturation, 0, 300, "Saturation")?;
    validate_u32(filters.color.gamma, 10, 300, "Gamma")?;
    validate_i32(filters.hue, -180, 180, "Hue")?;
    validate_u32(filters.temperature, 2000, 12_000, "Temperature")?;
    validate_u32(filters.sharpen, 0, 100, "Sharpen")?;
    validate_u32(filters.gaussian_blur, 0, 100, "Gaussian blur")?;
    validate_u32(filters.deband, 0, 100, "Deband")?;
    validate_u32(filters.vignette, 0, 100, "Vignette")
}

fn validate_audio_filters(filters: &AudioFiltersConfig) -> Result<(), ConversionError> {
    validate_i32(filters.limiter, -12, 0, "Limiter")?;
    validate_i32(filters.bass, -20, 20, "Bass")?;
    validate_i32(filters.treble, -20, 20, "Treble")?;
    validate_u32(filters.high_pass, 20, 2000, "High-pass")?;
    validate_u32(filters.low_pass, 1000, 20_000, "Low-pass")?;
    if filters.high_pass.enabled
        && filters.low_pass.enabled
        && filters.high_pass.value + 100 > filters.low_pass.value
    {
        return Err(ConversionError::InvalidInput(
            "High-pass and low-pass filters require at least 100 Hz of separation".to_string(),
        ));
    }
    validate_u32(filters.noise_reduction, 1, 30, "Noise reduction")?;
    validate_u32(filters.de_esser, 0, 100, "De-esser")?;
    validate_u32(filters.stereo_width, 0, 200, "Stereo width")
}

fn validate_i32(
    filter: FilterValue<i32>,
    min: i32,
    max: i32,
    label: &str,
) -> Result<(), ConversionError> {
    if filter.enabled && !(min..=max).contains(&filter.value) {
        return Err(ConversionError::InvalidInput(format!(
            "{label} must be between {min} and {max}"
        )));
    }
    Ok(())
}

fn validate_u32(
    filter: FilterValue<u32>,
    min: u32,
    max: u32,
    label: &str,
) -> Result<(), ConversionError> {
    if filter.enabled && !(min..=max).contains(&filter.value) {
        return Err(ConversionError::InvalidInput(format!(
            "{label} must be between {min} and {max}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudioFiltersConfig, VideoColorFiltersConfig};

    #[test]
    fn default_video_filters_emit_empty_chains() {
        let config = VideoFiltersConfig::default();

        assert!(build_video_pre_scale_filters(&config, false).is_empty());
        assert!(build_video_post_scale_filters(&config).is_empty());
    }

    #[test]
    fn color_adjustments_are_combined_in_stable_order() {
        let config = VideoFiltersConfig {
            color: VideoColorFiltersConfig {
                brightness: FilterValue {
                    enabled: true,
                    value: 10,
                },
                contrast: FilterValue {
                    enabled: true,
                    value: 115,
                },
                saturation: FilterValue {
                    enabled: true,
                    value: 90,
                },
                gamma: FilterValue {
                    enabled: true,
                    value: 105,
                },
            },
            ..VideoFiltersConfig::default()
        };

        assert_eq!(
            build_video_pre_scale_filters(&config, false),
            vec!["eq=brightness=0.100:contrast=1.150:saturation=0.900:gamma=1.050"]
        );
    }

    #[test]
    fn image_denoise_zeroes_temporal_values() {
        let config = VideoFiltersConfig {
            denoise_enabled: true,
            denoise_strength: FilterStrength::Medium,
            ..VideoFiltersConfig::default()
        };

        assert_eq!(
            build_video_pre_scale_filters(&config, true),
            vec!["hqdn3d=3.000:2.250:0.000:0.000"]
        );
    }

    #[test]
    fn limiter_is_last_audio_filter() {
        let config = ConversionConfig {
            audio_filters: AudioFiltersConfig {
                bass: FilterValue {
                    enabled: true,
                    value: 6,
                },
                limiter: FilterValue {
                    enabled: true,
                    value: -1,
                },
                ..AudioFiltersConfig::default()
            },
            ..test_config()
        };

        let chain = build_audio_effect_filters(&config);

        assert_eq!(
            chain.last().map(String::as_str),
            Some("alimiter=limit=0.891:attack=5.000:release=50.000:level=0:latency=1")
        );
    }

    #[test]
    fn validation_rejects_invalid_brightness() {
        let config = ConversionConfig {
            video_filters: VideoFiltersConfig {
                color: VideoColorFiltersConfig {
                    brightness: FilterValue {
                        enabled: true,
                        value: 101,
                    },
                    ..VideoColorFiltersConfig::default()
                },
                ..VideoFiltersConfig::default()
            },
            ..test_config()
        };

        assert!(validate_media_filters(&config).is_err());
    }

    fn test_config() -> ConversionConfig {
        ConversionConfig {
            processing_mode: "reencode".to_string(),
            container: "mp4".to_string(),
            video_codec: "libx264".to_string(),
            video_bitrate_mode: "crf".to_string(),
            video_bitrate: "5000".to_string(),
            audio_codec: "aac".to_string(),
            audio_bitrate: "192".to_string(),
            audio_bitrate_mode: "bitrate".to_string(),
            audio_quality: "4".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            video_filters: VideoFiltersConfig::default(),
            audio_filters: AudioFiltersConfig::default(),
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
            subtitle_burn_path: None,
            subtitle_font_name: None,
            subtitle_font_size: None,
            subtitle_font_color: None,
            subtitle_outline_color: None,
            subtitle_position: None,
            resolution: "original".to_string(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "lanczos".to_string(),
            fps: "original".to_string(),
            crf: 23,
            quality: 50,
            preset: "medium".to_string(),
            start_time: None,
            end_time: None,
            metadata: crate::types::MetadataConfig::default(),
            rotation: "0".to_string(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
            overlay: None,
            nvenc_spatial_aq: false,
            nvenc_temporal_aq: false,
            videotoolbox_allow_sw: false,
            hw_decode: false,
            pixel_format: "auto".to_string(),
            image_jpeg_quality: 85,
            image_jpeg_huffman: "optimal".to_string(),
            image_webp_lossless: false,
            image_webp_quality: 75,
            image_webp_compression: 4,
            image_webp_preset: "default".to_string(),
            image_png_compression: 9,
            image_png_prediction: "paeth".to_string(),
            image_tiff_compression: "packbits".to_string(),
            gif_colors: 256,
            gif_dither: "sierra2_4a".to_string(),
            gif_loop: 0,
        }
    }
}
