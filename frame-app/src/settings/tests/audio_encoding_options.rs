use super::*;

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
