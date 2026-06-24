use super::{model::*, rules::*};

#[must_use]
pub fn sanitize_output_name(value: &str) -> String {
    let candidate = value.rsplit(['/', '\\']).next().unwrap_or_default().trim();

    if candidate == "." || candidate == ".." {
        String::new()
    } else {
        candidate.to_string()
    }
}

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

pub fn apply_processing_mode(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
    mode: ProcessingMode,
) -> bool {
    if mode == ProcessingMode::Copy && source_kind_for(metadata) == SourceKind::Image {
        return false;
    }

    let changed = config.processing_mode != mode;
    config.processing_mode = mode;
    changed | normalize_output_config(config, metadata)
}

pub fn apply_output_container(config: &mut ConversionConfig, container: &str) -> bool {
    let changed = !config.container.eq_ignore_ascii_case(container);
    config.container = container.to_ascii_lowercase();

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
        normalize_audio_encoding_settings(config);
        return true;
    }

    normalize_audio_encoding_settings(config);
    changed
}

pub fn apply_trim_times(
    config: &mut ConversionConfig,
    start_time: Option<String>,
    end_time: Option<String>,
) -> bool {
    let start_time = normalize_optional_timecode(start_time);
    let end_time = normalize_optional_timecode(end_time);
    let changed = config.start_time != start_time || config.end_time != end_time;

    config.start_time = start_time;
    config.end_time = end_time;

    changed
}

fn normalize_optional_timecode(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn normalize_output_config(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    let source_kind = source_kind_for(metadata);

    if source_kind == SourceKind::Audio && !is_audio_only_container(&config.container) {
        config.container = "mp3".to_string();
    }

    if source_kind == SourceKind::Image
        && !is_image_container(&config.container)
        && !is_gif_container(&config.container)
    {
        config.container = "png".to_string();
    }

    if source_kind == SourceKind::Image {
        config.start_time = None;
        config.end_time = None;
        config.selected_audio_tracks.clear();
        reset_audio_filter_settings(config);
    }

    if source_kind == SourceKind::Audio || is_audio_only_container(&config.container) {
        config.crop = None;
    }

    if (source_kind == SourceKind::Image || is_gif_container(&config.container))
        && config.processing_mode == ProcessingMode::Copy
    {
        config.processing_mode = ProcessingMode::Reencode;
    }

    if config.processing_mode == ProcessingMode::Copy {
        reset_audio_filter_settings(config);
    }

    if !container_supports_audio(&config.container) {
        config.selected_audio_tracks.clear();
        config.audio_normalize = false;
    }

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
    }
    normalize_audio_encoding_settings(config);

    before != *config
}

fn normalize_audio_encoding_settings(config: &mut ConversionConfig) {
    if !matches!(config.audio_bitrate_mode.as_str(), "bitrate" | "vbr") {
        config.audio_bitrate_mode = DEFAULT_AUDIO_BITRATE_MODE.to_string();
    }
    if config.audio_bitrate_mode == "vbr" && !audio_codec_supports_vbr(&config.audio_codec) {
        config.audio_bitrate_mode = DEFAULT_AUDIO_BITRATE_MODE.to_string();
    }
    if !is_known_audio_channels(&config.audio_channels) {
        config.audio_channels = DEFAULT_AUDIO_CHANNELS.to_string();
    }

    config.audio_quality = normalized_audio_quality(&config.audio_codec, &config.audio_quality);
    config.audio_volume = config.audio_volume.min(MAX_AUDIO_VOLUME);
}

fn reset_audio_filter_settings(config: &mut ConversionConfig) {
    config.audio_normalize = false;
    config.audio_volume = DEFAULT_AUDIO_VOLUME;
    config.audio_bitrate_mode = DEFAULT_AUDIO_BITRATE_MODE.to_string();
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

fn normalized_audio_quality(codec: &str, quality: &str) -> String {
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

fn is_known_audio_codec(codec: &str) -> bool {
    AUDIO_CODEC_DEFINITIONS
        .iter()
        .any(|definition| definition.codec == codec)
}

fn is_known_audio_channels(channels: &str) -> bool {
    AUDIO_CHANNEL_DEFINITIONS
        .iter()
        .any(|definition| definition.id == channels)
}
