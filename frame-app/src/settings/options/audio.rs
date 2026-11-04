use crate::settings::model::{
    AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, AudioChannelOption, AudioCodecOption,
    AudioTrackOption, ConversionConfig, OPTIONAL_AUDIO_CODEC_DEFINITIONS, ProcessingMode,
    SourceMetadata,
};
use crate::settings::rules::is_audio_codec_allowed_for_container;
use crate::settings::source_info::{audio_track_bitrate, audio_track_detail, display_source_value};
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
    let original_is_unsupported = mp2_original_channels_are_unsupported(config, metadata);

    AUDIO_CHANNEL_DEFINITIONS.map(|definition| AudioChannelOption {
        id: definition.id,
        label: definition.label,
        is_selected: config.audio_channels.eq_ignore_ascii_case(definition.id),
        is_disabled: disabled || (definition.id == "original" && original_is_unsupported),
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
                    bitrate: audio_track_bitrate(track),
                    is_selected: config.selected_audio_tracks.contains(&track.index),
                    is_disabled: disabled,
                })
                .collect()
        })
        .unwrap_or_default()
}
