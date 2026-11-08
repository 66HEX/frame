use crate::numeric::u32_to_u16;
use crate::settings::model::{
    ConversionConfig, FPS_OPTIONS, GIF_DITHER_OPTIONS, GIF_FPS_OPTIONS, MAX_GIF_COLORS,
    MAX_GIF_LOOP, RESOLUTION_OPTIONS, SCALING_ALGORITHM_OPTIONS, VIDEO_CODEC_DEFINITIONS,
    VIDEO_PIXEL_FORMAT_DEFINITIONS,
};
use crate::settings::options::{
    is_hardware_video_codec, is_nvenc_video_codec, is_video_preset_allowed,
    is_videotoolbox_video_codec,
};
use crate::settings::rules::{
    is_gif_container, is_video_codec_allowed_for_container,
    is_video_pixel_format_allowed_for_container,
};
use crate::settings::updates::normalize::normalize_video_config;
use crate::settings::updates::validation::sanitized_optional_number;

pub fn apply_resolution(config: &mut ConversionConfig, resolution: &str) -> bool {
    let resolution = resolution.to_ascii_lowercase();
    if !RESOLUTION_OPTIONS.contains(&resolution.as_str()) {
        return false;
    }

    if config.resolution == resolution {
        return false;
    }

    config.resolution = resolution;
    true
}

pub fn apply_custom_width(config: &mut ConversionConfig, width: &str) -> bool {
    let width = sanitized_optional_number(width);
    if config.custom_width == width {
        return false;
    }

    config.custom_width = width;
    true
}

pub fn apply_custom_height(config: &mut ConversionConfig, height: &str) -> bool {
    let height = sanitized_optional_number(height);
    if config.custom_height == height {
        return false;
    }

    config.custom_height = height;
    true
}

pub fn apply_scaling_algorithm(config: &mut ConversionConfig, algorithm: &str) -> bool {
    let algorithm = algorithm.to_ascii_lowercase();
    if !SCALING_ALGORITHM_OPTIONS.contains(&algorithm.as_str()) {
        return false;
    }

    if config.scaling_algorithm == algorithm {
        return false;
    }

    config.scaling_algorithm = algorithm;
    true
}

pub fn apply_fps(config: &mut ConversionConfig, fps: &str) -> bool {
    let valid = if is_gif_container(&config.container) {
        GIF_FPS_OPTIONS.contains(&fps)
    } else {
        FPS_OPTIONS.contains(&fps)
    };
    if !valid {
        return false;
    }

    if config.fps == fps {
        return false;
    }

    config.fps = fps.to_string();
    true
}

pub fn apply_gif_colors(config: &mut ConversionConfig, colors: u16) -> bool {
    let colors = colors.clamp(frame_core::types::MIN_GIF_COLORS, MAX_GIF_COLORS);
    if config.gif_colors == colors {
        return false;
    }

    config.gif_colors = colors;
    true
}

pub fn apply_gif_dither(config: &mut ConversionConfig, dither: &str) -> bool {
    let dither = dither.to_ascii_lowercase();
    if !GIF_DITHER_OPTIONS.contains(&dither.as_str()) {
        return false;
    }

    if config.gif_dither == dither {
        return false;
    }

    config.gif_dither = dither;
    true
}

pub fn apply_gif_loop(config: &mut ConversionConfig, loop_count: &str) -> bool {
    let parsed = loop_count
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse::<u32>()
        .map_or(0, |value| u32_to_u16(value.min(u32::from(MAX_GIF_LOOP))));

    if config.gif_loop == parsed {
        return false;
    }

    config.gif_loop = parsed;
    true
}

pub fn apply_video_codec(config: &mut ConversionConfig, codec: &str) -> bool {
    let codec = codec.to_ascii_lowercase();
    if !is_known_video_codec(&codec)
        || !is_video_codec_allowed_for_container(&config.container, &codec)
    {
        return false;
    }

    let changed = config.video_codec != codec;
    config.video_codec = codec;
    changed | normalize_video_config(config, None)
}

pub fn apply_pixel_format(config: &mut ConversionConfig, pixel_format: &str) -> bool {
    let pixel_format = pixel_format.to_ascii_lowercase();
    if !is_known_pixel_format(&pixel_format)
        || !is_video_pixel_format_allowed_for_container(
            &config.container,
            &config.video_codec,
            &pixel_format,
        )
    {
        return false;
    }

    if config.pixel_format == pixel_format {
        return false;
    }

    config.pixel_format = pixel_format;
    true
}

pub fn apply_video_preset(config: &mut ConversionConfig, preset: &str) -> bool {
    let preset = preset.to_ascii_lowercase();
    if !is_video_preset_allowed(&config.video_codec, &preset) {
        return false;
    }

    if config.preset == preset {
        return false;
    }

    config.preset = preset;
    true
}

pub fn apply_video_bitrate_mode(config: &mut ConversionConfig, mode: &str) -> bool {
    let mode = mode.to_ascii_lowercase();
    if !matches!(mode.as_str(), "crf" | "bitrate") {
        return false;
    }

    if config.video_bitrate_mode == mode {
        return false;
    }

    config.video_bitrate_mode = mode;
    true
}

pub fn apply_video_bitrate(config: &mut ConversionConfig, bitrate: &str) -> bool {
    let bitrate: String = bitrate.chars().filter(char::is_ascii_digit).collect();
    if config.video_bitrate == bitrate {
        return false;
    }

    config.video_bitrate = bitrate;
    true
}

pub fn apply_crf(config: &mut ConversionConfig, crf: u8) -> bool {
    let crf = crf.min(51);
    if config.crf == crf {
        return false;
    }

    config.crf = crf;
    true
}

pub fn apply_quality(config: &mut ConversionConfig, quality: u32) -> bool {
    let quality = quality.clamp(1, 100);
    if config.quality == quality {
        return false;
    }

    config.quality = quality;
    true
}

pub fn apply_nvenc_spatial_aq(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_nvenc_video_codec(&config.video_codec) || config.nvenc_spatial_aq == enabled {
        return false;
    }

    config.nvenc_spatial_aq = enabled;
    true
}

pub fn apply_nvenc_temporal_aq(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_nvenc_video_codec(&config.video_codec) || config.nvenc_temporal_aq == enabled {
        return false;
    }

    config.nvenc_temporal_aq = enabled;
    true
}

pub fn apply_videotoolbox_allow_sw(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_videotoolbox_video_codec(&config.video_codec) || config.videotoolbox_allow_sw == enabled
    {
        return false;
    }

    config.videotoolbox_allow_sw = enabled;
    true
}

pub fn apply_hw_decode(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_hardware_video_codec(&config.video_codec) || config.hw_decode == enabled {
        return false;
    }

    config.hw_decode = enabled;
    true
}

pub(super) fn is_known_video_codec(codec: &str) -> bool {
    VIDEO_CODEC_DEFINITIONS
        .iter()
        .any(|definition| definition.codec == codec)
}

pub(super) fn is_known_pixel_format(pixel_format: &str) -> bool {
    VIDEO_PIXEL_FORMAT_DEFINITIONS
        .iter()
        .any(|definition| definition.id == pixel_format)
}
