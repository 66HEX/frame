use crate::settings::model::{ConversionConfig, ProcessingMode, SourceKind, SourceMetadata};
use crate::settings::rules::{
    container_supports_audio, default_audio_codec_for_container,
    is_audio_codec_allowed_for_container, source_kind_for,
};
use crate::settings::updates::normalize::{
    normalize_audio_encoding_settings, normalize_output_config, normalize_video_config,
};

#[must_use]
pub fn sanitize_output_name(value: &str) -> String {
    let candidate = value.rsplit(['/', '\\']).next().unwrap_or_default().trim();

    if candidate == "." || candidate == ".." {
        String::new()
    } else {
        candidate.to_string()
    }
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
    let mut changed = !config.container.eq_ignore_ascii_case(container);
    config.container = container.to_ascii_lowercase();

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
        changed = true;
    }

    normalize_audio_encoding_settings(config);
    changed | normalize_video_config(config, None)
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

pub(super) fn normalize_optional_timecode(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
