use frame_core::media_rules;

use super::{
    model::{
        AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, AudioChannelOption, AudioCodecOption,
        AudioTrackOption, ConversionConfig, OutputContainerOption, OutputModeOption,
        ProcessingMode, SourceKind, SourceMetadata,
    },
    rules::*,
    source_info::{audio_track_detail, display_source_value},
};

#[must_use]
pub fn output_processing_mode_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> [OutputModeOption; 2] {
    let is_source_image = source_kind_for(metadata) == SourceKind::Image;
    [
        output_mode_option(ProcessingMode::Reencode, config, disabled),
        output_mode_option(ProcessingMode::Copy, config, disabled || is_source_image),
    ]
}

#[must_use]
pub fn visible_output_containers(metadata: Option<&SourceMetadata>) -> Vec<String> {
    let is_source_image = source_kind_for(metadata) == SourceKind::Image;

    media_rules::all_containers()
        .iter()
        .filter(|container| {
            if is_source_image {
                is_image_container(container) || is_gif_container(container)
            } else {
                !is_image_container(container)
            }
        })
        .cloned()
        .collect()
}

#[must_use]
pub fn output_container_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<OutputContainerOption> {
    visible_output_containers(metadata)
        .into_iter()
        .map(|container| {
            let disabled_reason =
                output_container_disabled_reason(config, metadata, &container, disabled);
            OutputContainerOption {
                is_selected: config.container.eq_ignore_ascii_case(&container),
                is_disabled: disabled_reason.is_some(),
                disabled_reason,
                container,
            }
        })
        .collect()
}

#[must_use]
pub fn audio_codec_options(config: &ConversionConfig, disabled: bool) -> Vec<AudioCodecOption> {
    let encode_controls_disabled = disabled || config.processing_mode == ProcessingMode::Copy;

    AUDIO_CODEC_DEFINITIONS
        .iter()
        .map(|definition| {
            let is_compatible =
                is_audio_codec_allowed_for_container(&config.container, definition.codec);
            AudioCodecOption {
                codec: definition.codec,
                label: definition.label,
                is_selected: config.audio_codec.eq_ignore_ascii_case(definition.codec),
                is_disabled: encode_controls_disabled || !is_compatible,
                disabled_reason: (!is_compatible).then_some("Incompatible container"),
            }
        })
        .collect()
}

#[must_use]
pub fn audio_channel_options(config: &ConversionConfig, disabled: bool) -> [AudioChannelOption; 3] {
    let disabled = disabled || config.processing_mode == ProcessingMode::Copy;

    AUDIO_CHANNEL_DEFINITIONS.map(|definition| AudioChannelOption {
        id: definition.id,
        label: definition.label,
        is_selected: config.audio_channels.eq_ignore_ascii_case(definition.id),
        is_disabled: disabled,
    })
}

#[must_use]
pub fn audio_track_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<AudioTrackOption> {
    metadata
        .map(|metadata| {
            metadata
                .audio_tracks
                .iter()
                .map(|track| AudioTrackOption {
                    index: track.index,
                    index_label: format!("#{}", track.index),
                    codec: display_source_value(Some(&track.codec)),
                    detail: audio_track_detail(track),
                    is_selected: config.selected_audio_tracks.contains(&track.index),
                    is_disabled: disabled,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[must_use]
pub fn is_container_compatible_for_stream_copy(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    container: &str,
) -> bool {
    if config.processing_mode != ProcessingMode::Copy {
        return true;
    }
    if source_kind_for(metadata) == SourceKind::Image {
        return false;
    }
    if is_image_container(container) || is_gif_container(container) {
        return false;
    }

    let Some(metadata) = metadata else {
        return true;
    };

    let selected_audio_codecs = selected_audio_codecs(config, metadata);
    if is_audio_only_container(container) {
        return !selected_audio_codecs.is_empty()
            && selected_audio_codecs
                .iter()
                .all(|codec| is_audio_stream_codec_allowed_for_container(container, codec));
    }

    let Some(video_codec) = metadata.video_codec.as_deref() else {
        return false;
    };
    if !is_video_stream_codec_allowed_for_container(container, video_codec) {
        return false;
    }

    let audio_codecs_allowed = selected_audio_codecs
        .iter()
        .all(|codec| is_audio_stream_codec_allowed_for_container(container, codec));
    let subtitle_codecs_allowed = selected_subtitle_codecs(config, metadata)
        .iter()
        .all(|codec| is_subtitle_codec_allowed_for_container(container, codec));

    audio_codecs_allowed && subtitle_codecs_allowed
}

fn output_mode_option(
    mode: ProcessingMode,
    config: &ConversionConfig,
    is_disabled: bool,
) -> OutputModeOption {
    OutputModeOption {
        mode,
        label: mode.label(),
        hint: mode.hint(),
        is_selected: config.processing_mode == mode,
        is_disabled,
    }
}

fn output_container_disabled_reason(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    container: &str,
    disabled: bool,
) -> Option<&'static str> {
    let source_kind = source_kind_for(metadata);

    if disabled {
        return Some("Locked");
    }
    if source_kind == SourceKind::Audio && !is_audio_only_container(container) {
        return Some("Video container unavailable for audio sources");
    }
    if source_kind == SourceKind::Image && is_audio_only_container(container) {
        return Some("Audio container unavailable for image sources");
    }
    if !is_container_compatible_for_stream_copy(config, metadata, container) {
        return Some("Incompatible container");
    }

    None
}

fn selected_audio_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    if metadata.audio_tracks.is_empty() {
        return Vec::new();
    }
    if config.selected_audio_tracks.is_empty() {
        return metadata
            .audio_tracks
            .iter()
            .map(|track| track.codec.as_str())
            .collect();
    }

    metadata
        .audio_tracks
        .iter()
        .filter(|track| config.selected_audio_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}

fn selected_subtitle_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    if metadata.subtitle_tracks.is_empty() {
        return Vec::new();
    }
    if config.selected_subtitle_tracks.is_empty() {
        return metadata
            .subtitle_tracks
            .iter()
            .map(|track| track.codec.as_str())
            .collect();
    }

    metadata
        .subtitle_tracks
        .iter()
        .filter(|track| config.selected_subtitle_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}
