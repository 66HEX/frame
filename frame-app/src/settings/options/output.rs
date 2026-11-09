use super::audio::audio_copy_error_for_track;
use crate::settings::model::{
    AudioTrack, ConversionConfig, OutputContainerOption, OutputModeOption, ProcessingMode,
    SourceKind, SourceMetadata,
};
use crate::settings::rules::{
    is_audio_only_container, is_gif_container, is_image_container,
    is_subtitle_codec_allowed_for_container, is_video_stream_codec_allowed_for_container,
    source_kind_for,
};
use frame_core::audio_rules::audio_track_count_error;
use frame_core::media_rules;

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

    let selected_audio = selected_audio_tracks(config, metadata);
    if selected_audio.len() != config.selected_audio_tracks.len()
        || audio_track_count_error(container, selected_audio.len()).is_some()
    {
        return false;
    }
    if is_audio_only_container(container) {
        return !selected_audio.is_empty()
            && selected_audio
                .iter()
                .all(|track| audio_copy_error_for_track(container, metadata, track).is_none());
    }

    let Some(video_codec) = metadata.video_codec.as_deref() else {
        return false;
    };
    if !is_video_stream_codec_allowed_for_container(container, video_codec) {
        return false;
    }

    let audio_codecs_allowed = selected_audio
        .iter()
        .all(|track| audio_copy_error_for_track(container, metadata, track).is_none());
    let subtitle_codecs_allowed = selected_subtitle_codecs(config, metadata)
        .iter()
        .all(|codec| is_subtitle_codec_allowed_for_container(container, codec));

    audio_codecs_allowed && subtitle_codecs_allowed
}

pub(super) fn output_mode_option(
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

pub(super) fn output_container_disabled_reason(
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
        if audio_track_count_error(container, config.selected_audio_tracks.len()).is_some() {
            return Some("Select one audio track for this format");
        }
        if metadata.is_some_and(|metadata| {
            selected_audio_tracks(config, metadata).iter().any(|track| {
                frame_core::audio_rules::mp3_requires_mp4_resampling(
                    container,
                    &track.codec,
                    track.sample_rate.as_deref(),
                )
            })
        }) {
            return Some("MP4 needs MP3 audio at 16 kHz or higher");
        }
        if let Some(reason) = metadata.and_then(|metadata| {
            selected_audio_tracks(config, metadata)
                .iter()
                .find_map(|track| audio_copy_error_for_track(container, metadata, track))
        }) {
            return Some(reason);
        }
        return Some("Incompatible container");
    }

    None
}

pub(super) fn selected_audio_tracks<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a AudioTrack> {
    metadata
        .audio_tracks
        .iter()
        .filter(|track| config.selected_audio_tracks.contains(&track.index))
        .collect()
}

pub(super) fn selected_subtitle_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    metadata
        .subtitle_tracks
        .iter()
        .filter(|track| config.selected_subtitle_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}
