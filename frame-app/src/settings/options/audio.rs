use crate::settings::model::{
    AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, AudioChannelOption, AudioCodecOption,
    AudioTrack, AudioTrackOption, ConversionConfig, OPTIONAL_AUDIO_CODEC_DEFINITIONS,
    ProcessingMode, SourceMetadata,
};
use crate::settings::rules::is_audio_codec_allowed_for_container;
use crate::settings::source_info::{audio_track_bitrate, audio_track_detail, display_source_value};
use frame_core::audio_rules::{
    AudioStreamCopySource, audio_stream_copy_error_with_source, audio_track_count_error,
    effective_audio_channels, opus_channels_error, opus_encoding_error,
    stereo_width_supports_channels,
};
use frame_core::capabilities::AvailableEncoders;

#[must_use]
pub fn audio_codec_options(
    config: &ConversionConfig,
    available_encoders: &AvailableEncoders,
    disabled: bool,
) -> Vec<AudioCodecOption> {
    let encode_controls_disabled = disabled || config.processing_mode == ProcessingMode::Copy;

    AUDIO_CODEC_DEFINITIONS
        .iter()
        .filter(|definition| definition.codec != "mp2" || available_encoders.mp2)
        .filter(|definition| definition.codec != "pcm_bluray" || available_encoders.pcm_bluray)
        .map(|definition| (definition.codec, definition.label))
        .chain(
            OPTIONAL_AUDIO_CODEC_DEFINITIONS
                .iter()
                .filter(|definition| {
                    definition.codec != "libfdk_aac" || available_encoders.libfdk_aac
                })
                .map(|definition| (definition.codec, definition.label)),
        )
        .map(|definition| {
            let is_compatible =
                is_audio_codec_allowed_for_container(&config.container, definition.0);
            AudioCodecOption {
                codec: definition.0,
                label: definition.1,
                is_selected: config.audio_codec.eq_ignore_ascii_case(definition.0),
                is_disabled: encode_controls_disabled || !is_compatible,
                disabled_reason: (!is_compatible).then_some("Incompatible container"),
            }
        })
        .collect()
}

#[must_use]
pub fn audio_channel_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> [AudioChannelOption; 3] {
    let disabled = disabled || config.processing_mode == ProcessingMode::Copy;
    let original_is_unsupported = mp2_original_channels_are_unsupported(config, metadata)
        || (config.audio_codec == "libopus"
            && metadata.is_some_and(|metadata| {
                metadata
                    .audio_tracks
                    .iter()
                    .filter(|track| config.selected_audio_tracks.contains(&track.index))
                    .any(|track| {
                        opus_channels_error(
                            track
                                .channels
                                .as_deref()
                                .and_then(|value| value.parse().ok()),
                            track.channel_layout.as_deref(),
                        )
                        .is_some()
                    })
            }));

    AUDIO_CHANNEL_DEFINITIONS.map(|definition| AudioChannelOption {
        id: definition.id,
        label: definition.label,
        is_selected: config.audio_channels.eq_ignore_ascii_case(definition.id),
        is_disabled: disabled || (definition.id == "original" && original_is_unsupported),
    })
}

#[must_use]
pub fn audio_settings_error(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> Option<String> {
    if !frame_core::media_rules::container_supports_audio(&config.container) {
        return None;
    }
    if let Some(reason) =
        audio_track_count_error(&config.container, config.selected_audio_tracks.len())
    {
        return Some(reason);
    }
    let metadata = metadata?;
    for track in metadata
        .audio_tracks
        .iter()
        .filter(|track| config.selected_audio_tracks.contains(&track.index))
    {
        let reason = if config.processing_mode == ProcessingMode::Copy {
            audio_copy_error_for_track(&config.container, metadata, track).map(str::to_string)
        } else if config.audio_codec == "libopus" {
            let channels = effective_audio_channels(
                &config.audio_channels,
                track
                    .channels
                    .as_deref()
                    .and_then(|value| value.parse().ok()),
            );
            let layout = if config.audio_channels == "original" {
                track.channel_layout.as_deref()
            } else {
                None
            };
            opus_encoding_error(&config.audio_bitrate, channels, layout)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Some(format!("Track #{}: {reason}", track.index));
        }
    }
    None
}

pub(super) fn audio_copy_error_for_track(
    container: &str,
    metadata: &SourceMetadata,
    track: &AudioTrack,
) -> Option<&'static str> {
    audio_stream_copy_error_with_source(
        container,
        &track.codec,
        track.sample_rate.as_deref(),
        AudioStreamCopySource {
            format: metadata.source_format.as_deref(),
            channels: track
                .channels
                .as_deref()
                .and_then(|value| value.parse().ok()),
            codec_tag: track.codec_tag,
        },
    )
}

#[must_use]
pub fn stereo_width_available(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    if config.selected_audio_tracks.is_empty() {
        return false;
    }
    if config.audio_channels == "stereo" {
        return true;
    }
    metadata.is_some_and(|metadata| {
        config.selected_audio_tracks.iter().all(|index| {
            metadata
                .audio_tracks
                .iter()
                .find(|track| track.index == *index)
                .is_some_and(|track| {
                    stereo_width_supports_channels(
                        &config.audio_channels,
                        track
                            .channels
                            .as_deref()
                            .and_then(|value| value.parse().ok()),
                    )
                })
        })
    })
}

#[must_use]
pub fn mp2_original_channels_are_unsupported(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    config.audio_codec == "mp2"
        && metadata.is_some_and(|metadata| {
            metadata.audio_tracks.iter().any(|track| {
                config.selected_audio_tracks.contains(&track.index)
                    && track
                        .channels
                        .as_deref()
                        .and_then(|channels| channels.parse::<u16>().ok())
                        .is_some_and(|channels| channels > 2)
            })
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
                    copy_error: (config.processing_mode == ProcessingMode::Copy
                        && config.selected_audio_tracks.contains(&track.index))
                    .then(|| audio_copy_error_for_track(&config.container, metadata, track))
                    .flatten(),
                    bitrate: audio_track_bitrate(track),
                    is_selected: config.selected_audio_tracks.contains(&track.index),
                    is_disabled: disabled,
                })
                .collect()
        })
        .unwrap_or_default()
}
