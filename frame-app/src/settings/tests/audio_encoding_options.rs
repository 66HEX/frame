use super::*;
use frame_core::capabilities::AvailableEncoders;

#[test]
fn mp2_normalization_preserves_explicit_mono_for_surround_sources() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![AudioTrack {
            index: 1,
            channels: Some("6".to_string()),
            ..AudioTrack::default()
        }],
        ..SourceMetadata::default()
    };
    for (channels, expected) in [("original", "stereo"), ("mono", "mono")] {
        let mut config = ConversionConfig {
            container: "m2t".to_string(),
            audio_codec: "mp2".to_string(),
            audio_channels: channels.to_string(),
            selected_audio_tracks: vec![1],
            ..ConversionConfig::default()
        };
        normalize_output_config(&mut config, Some(&metadata));
        assert_eq!(config.audio_channels, expected);
    }
}

#[test]
fn opus_ui_validates_each_selected_source_and_preserves_invalid_bitrate() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![
            AudioTrack {
                index: 1,
                channels: Some("2".to_string()),
                ..AudioTrack::default()
            },
            AudioTrack {
                index: 2,
                channels: Some("1".to_string()),
                ..AudioTrack::default()
            },
        ],
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig {
        container: "mkv".to_string(),
        audio_codec: "libopus".to_string(),
        audio_bitrate: "320".to_string(),
        selected_audio_tracks: vec![1, 2],
        ..ConversionConfig::default()
    };
    for tracks in [vec![1, 2], vec![2, 1]] {
        config.selected_audio_tracks = tracks;
        normalize_output_config(&mut config, Some(&metadata));
        assert_eq!(config.audio_bitrate, "320");
        assert!(
            audio_settings_error(&config, Some(&metadata))
                .unwrap()
                .contains("256")
        );
    }
    config.audio_channels = "stereo".to_string();
    assert_eq!(audio_settings_error(&config, Some(&metadata)), None);
    config.selected_audio_tracks.clear();
    config.audio_channels = "mono".to_string();
    assert_eq!(audio_settings_error(&config, Some(&metadata)), None);
}

#[test]
fn stereo_width_deactivation_keeps_the_draft_value() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![AudioTrack {
            index: 1,
            channels: Some("6".to_string()),
            ..AudioTrack::default()
        }],
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig {
        container: "mkv".to_string(),
        audio_codec: "flac".to_string(),
        selected_audio_tracks: vec![1],
        ..ConversionConfig::default()
    };
    config.audio_filters.stereo_width.enabled = true;
    config.audio_filters.stereo_width.value = 150;
    assert!(!stereo_width_available(&config, Some(&metadata)));
    normalize_output_config(&mut config, Some(&metadata));
    assert!(!config.audio_filters.stereo_width.enabled);
    assert_eq!(config.audio_filters.stereo_width.value, 150);
    config.audio_channels = "stereo".to_string();
    assert!(stereo_width_available(&config, Some(&metadata)));
    config.audio_channels = "original".to_string();
    config.audio_filters.stereo_width.enabled = true;
    config.audio_filters.stereo_width.value = 100;
    normalize_output_config(&mut config, Some(&metadata));
    assert!(config.audio_filters.stereo_width.enabled);
}

#[test]
fn available_fdk_aac_can_be_selected_and_keep_its_vbr_quality() {
    let available = AvailableEncoders {
        libfdk_aac: true,
        ..AvailableEncoders::default()
    };
    let mut config = ConversionConfig::default();
    assert!(
        audio_codec_options(&config, &available, false)
            .iter()
            .any(|option| option.codec == "libfdk_aac" && !option.is_disabled)
    );
    assert!(apply_audio_codec(&mut config, "libfdk_aac"));
    assert!(apply_audio_bitrate_mode(&mut config, "vbr"));
    apply_audio_quality(&mut config, "5");
    normalize_output_config(&mut config, None);
    assert_eq!(config.audio_codec, "libfdk_aac");
    assert_eq!(config.audio_bitrate_mode, "vbr");
    assert_eq!(config.audio_quality, "5");
}

fn channel_option<'a>(options: &'a [AudioChannelOption], id: &str) -> &'a AudioChannelOption {
    options
        .iter()
        .find(|option| option.id == id)
        .unwrap_or_else(|| panic!("{id} channel option should exist"))
}

#[test]
fn default_config_matches_original_audio_defaults() {
    let config = ConversionConfig::default();

    assert_eq!(config.audio_bitrate, "128");
    assert_eq!(config.audio_bitrate_mode, "bitrate");
    assert_eq!(config.audio_quality, "4");
    assert!(config.opus_vbr);
    assert_eq!(config.audio_channels, "original");
    assert_eq!(config.audio_volume, 100);
    assert!(!config.audio_normalize);
}

#[test]
fn audio_channel_options_mark_original_selected_by_default() {
    let options = audio_channel_options(&ConversionConfig::default(), None, false);

    assert!(channel_option(&options, "original").is_selected);
}

#[test]
fn audio_channel_options_disable_in_stream_copy_mode() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        ..ConversionConfig::default()
    };

    let options = audio_channel_options(&config, None, false);

    assert!(options.iter().all(|option| option.is_disabled));
}

#[test]
fn mp2_disables_original_channels_for_selected_multichannel_track() {
    let config = ConversionConfig {
        container: "m2t".to_string(),
        audio_codec: "mp2".to_string(),
        selected_audio_tracks: vec![1],
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![AudioTrack {
            index: 1,
            channels: Some("6".to_string()),
            ..AudioTrack::default()
        }],
        ..SourceMetadata::default()
    };

    let options = audio_channel_options(&config, Some(&metadata), false);

    assert!(channel_option(&options, "original").is_disabled);
    assert!(!channel_option(&options, "stereo").is_disabled);
}

#[test]
fn normalize_output_config_downmixes_selected_multichannel_track_for_mp2() {
    let mut config = ConversionConfig {
        container: "m2t".to_string(),
        video_codec: "mpeg2video".to_string(),
        audio_codec: "mp2".to_string(),
        audio_channels: "original".to_string(),
        selected_audio_tracks: vec![1],
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![AudioTrack {
            index: 1,
            channels: Some("6".to_string()),
            ..AudioTrack::default()
        }],
        ..SourceMetadata::default()
    };

    normalize_output_config(&mut config, Some(&metadata));

    assert_eq!(config.audio_channels, "stereo");
}

#[test]
fn apply_audio_channels_updates_known_channel() {
    let mut config = ConversionConfig::default();

    assert!(apply_audio_channels(&mut config, "stereo"));

    assert_eq!(config.audio_channels, "stereo");
}

#[test]
fn apply_audio_channels_rejects_unknown_channel() {
    let mut config = ConversionConfig::default();

    assert!(!apply_audio_channels(&mut config, "surround"));

    assert_eq!(config.audio_channels, "original");
}

#[test]
fn apply_audio_bitrate_keeps_digits_only() {
    let mut config = ConversionConfig::default();

    assert!(apply_audio_bitrate(&mut config, " 192k "));

    assert_eq!(config.audio_bitrate, "192");
}

#[test]
fn apply_audio_bitrate_mode_rejects_vbr_for_native_aac() {
    let mut config = ConversionConfig::default();

    assert!(!apply_audio_bitrate_mode(&mut config, "vbr"));

    assert_eq!(config.audio_bitrate_mode, "bitrate");
}

#[test]
fn apply_audio_bitrate_mode_accepts_vbr_for_mp3() {
    let mut config = ConversionConfig {
        audio_codec: "mp3".to_string(),
        ..ConversionConfig::default()
    };

    assert!(apply_audio_bitrate_mode(&mut config, "vbr"));

    assert_eq!(config.audio_bitrate_mode, "vbr");
}

#[test]
fn apply_audio_quality_clamps_mp3_quality_range() {
    let mut config = ConversionConfig {
        audio_codec: "mp3".to_string(),
        ..ConversionConfig::default()
    };

    assert!(apply_audio_quality(&mut config, "42"));

    assert_eq!(config.audio_quality, "9");
}

#[test]
fn apply_audio_volume_clamps_to_original_slider_range() {
    let mut config = ConversionConfig::default();

    assert!(apply_audio_volume(&mut config, 250));

    assert_eq!(config.audio_volume, 200);
}

#[test]
fn apply_audio_normalize_updates_filter_flag() {
    let mut config = ConversionConfig::default();

    assert!(apply_audio_normalize(&mut config, true));

    assert!(config.audio_normalize);
}

#[test]
fn apply_audio_normalize_rejects_stream_copy_mode() {
    let mut config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        ..ConversionConfig::default()
    };

    assert!(!apply_audio_normalize(&mut config, true));

    assert!(!config.audio_normalize);
}

#[test]
fn opus_vbr_is_independent_of_quality_mode_and_rejects_copy_or_other_codecs() {
    let mut config = ConversionConfig {
        container: "opus".to_string(),
        audio_codec: "libopus".to_string(),
        ..ConversionConfig::default()
    };
    assert!(!audio_codec_supports_vbr("libopus"));
    assert_eq!(audio_quality_range("libopus"), None);
    assert!(!apply_audio_bitrate_mode(&mut config, "vbr"));
    assert!(apply_opus_vbr(&mut config, false));
    assert!(!apply_opus_vbr(&mut config, false));
    normalize_output_config(&mut config, None);
    assert!(!config.opus_vbr);
    assert_eq!(config.audio_bitrate, "128");
    assert!(apply_processing_mode(
        &mut config,
        None,
        ProcessingMode::Copy
    ));
    assert!(!apply_opus_vbr(&mut config, true));
    assert!(apply_processing_mode(
        &mut config,
        None,
        ProcessingMode::Reencode
    ));
    assert!(apply_output_container(&mut config, "mkv"));
    assert!(apply_audio_codec(&mut config, "aac"));
    assert!(!apply_opus_vbr(&mut config, true));
    assert!(apply_audio_codec(&mut config, "libopus"));
    assert!(!config.opus_vbr);
}
