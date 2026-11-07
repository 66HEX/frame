use crate::settings::model::{
    AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, AudioQualityRange, ConversionConfig,
    DEFAULT_AUDIO_QUALITY, MAX_AUDIO_VOLUME, ProcessingMode,
};
use crate::settings::rules::{container_supports_audio, is_audio_codec_allowed_for_container};
use crate::settings::updates::normalize::normalize_audio_encoding_settings;

pub fn toggle_audio_track_selection(config: &mut ConversionConfig, index: u32) -> bool {
    if config.selected_audio_tracks.contains(&index) {
        config
            .selected_audio_tracks
            .retain(|selected_index| *selected_index != index);
    } else {
        config.selected_audio_tracks.push(index);
    }

    true
}

pub fn apply_audio_codec(config: &mut ConversionConfig, codec: &str) -> bool {
    let codec = codec.to_ascii_lowercase();
    if config.processing_mode == ProcessingMode::Copy
        || !is_known_audio_codec(&codec)
        || !container_supports_audio(&config.container)
        || !is_audio_codec_allowed_for_container(&config.container, &codec)
    {
        return false;
    }

    if config.audio_codec.eq_ignore_ascii_case(&codec) {
        return false;
    }

    config.audio_codec = codec;
    normalize_audio_encoding_settings(config);
    true
}

pub fn apply_audio_channels(config: &mut ConversionConfig, channels: &str) -> bool {
    let channels = channels.to_ascii_lowercase();
    if config.processing_mode == ProcessingMode::Copy || !is_known_audio_channels(&channels) {
        return false;
    }

    if config.audio_channels.eq_ignore_ascii_case(&channels) {
        return false;
    }

    config.audio_channels = channels;
    true
}

pub fn apply_audio_bitrate(config: &mut ConversionConfig, bitrate: &str) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    let bitrate: String = bitrate.chars().filter(char::is_ascii_digit).collect();
    if config.audio_bitrate == bitrate {
        return false;
    }

    config.audio_bitrate = bitrate;
    true
}

pub fn apply_audio_bitrate_mode(config: &mut ConversionConfig, mode: &str) -> bool {
    let mode = mode.to_ascii_lowercase();
    if config.processing_mode == ProcessingMode::Copy
        || !matches!(mode.as_str(), "bitrate" | "vbr")
        || (mode == "vbr" && !audio_codec_supports_vbr(&config.audio_codec))
    {
        return false;
    }

    if config.audio_bitrate_mode == mode {
        return false;
    }

    config.audio_bitrate_mode = mode;
    normalize_audio_encoding_settings(config);
    true
}

pub fn apply_opus_vbr(config: &mut ConversionConfig, enabled: bool) -> bool {
    if config.processing_mode == ProcessingMode::Copy
        || config.audio_codec != "libopus"
        || config.opus_vbr == enabled
    {
        return false;
    }

    config.opus_vbr = enabled;
    true
}

pub fn apply_audio_quality(config: &mut ConversionConfig, quality: &str) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    let quality = normalized_audio_quality(&config.audio_codec, quality);
    if config.audio_quality == quality {
        return false;
    }

    config.audio_quality = quality;
    true
}

pub fn apply_audio_volume(config: &mut ConversionConfig, volume: u32) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    let volume = volume.min(MAX_AUDIO_VOLUME);
    if config.audio_volume == volume {
        return false;
    }

    config.audio_volume = volume;
    true
}

pub fn apply_audio_normalize(config: &mut ConversionConfig, enabled: bool) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    if config.audio_normalize == enabled {
        return false;
    }

    config.audio_normalize = enabled;
    true
}

#[must_use]
pub fn audio_codec_supports_vbr(codec: &str) -> bool {
    matches!(codec, "mp3" | "libfdk_aac")
}

#[must_use]
pub fn audio_quality_range(codec: &str) -> Option<AudioQualityRange> {
    match codec {
        "mp3" => Some(AudioQualityRange {
            min: 0,
            max: 9,
            lower_is_better: true,
            default_value: 4,
        }),
        "libfdk_aac" => Some(AudioQualityRange {
            min: 1,
            max: 5,
            lower_is_better: false,
            default_value: 4,
        }),
        _ => None,
    }
}

pub(super) fn normalized_audio_quality(codec: &str, quality: &str) -> String {
    let Some(range) = audio_quality_range(codec) else {
        return if quality.trim().is_empty() {
            DEFAULT_AUDIO_QUALITY.to_string()
        } else {
            quality.trim().to_string()
        };
    };

    let parsed = quality.trim().parse::<u32>().unwrap_or(range.default_value);
    parsed.clamp(range.min, range.max).to_string()
}

pub(super) fn is_known_audio_codec(codec: &str) -> bool {
    AUDIO_CODEC_DEFINITIONS
        .iter()
        .any(|definition| definition.codec == codec)
}

pub(super) fn is_known_audio_channels(channels: &str) -> bool {
    AUDIO_CHANNEL_DEFINITIONS
        .iter()
        .any(|definition| definition.id == channels)
}
