use super::*;

fn audio_metadata(codec: &str) -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Audio),
        audio_tracks: vec![AudioTrack {
            index: 0,
            codec: codec.to_string(),
            ..AudioTrack::default()
        }],
        ..SourceMetadata::default()
    }
}

fn image_metadata() -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Image),
        video_codec: Some("png".to_string()),
        ..SourceMetadata::default()
    }
}

fn video_metadata() -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Video),
        video_codec: Some("h264".to_string()),
        audio_tracks: vec![AudioTrack {
            index: 1,
            codec: "aac".to_string(),
            ..AudioTrack::default()
        }],
        subtitle_tracks: vec![SubtitleTrack {
            index: 2,
            codec: "subrip".to_string(),
            ..SubtitleTrack::default()
        }],
        ..SourceMetadata::default()
    }
}

#[test]
fn visible_output_containers_for_video_exclude_image_formats() {
    assert_eq!(
        visible_output_containers(None),
        vec![
            "mp4", "mkv", "webm", "mov", "m2t", "mts", "m2ts", "gif", "mp3", "m4a", "wav", "flac",
            "opus"
        ]
    );
}

#[test]
fn visible_output_containers_for_images_match_original_image_and_gif_set() {
    assert_eq!(
        visible_output_containers(Some(&image_metadata())),
        vec!["gif", "png", "jpg", "webp", "bmp", "tiff"]
    );
}

#[test]
fn processing_mode_options_disable_copy_for_image_sources() {
    let options = output_processing_mode_options(
        &ConversionConfig::default(),
        Some(&image_metadata()),
        false,
    );

    assert!(options[1].is_disabled);
}

#[test]
fn processing_mode_options_disable_all_when_settings_are_locked() {
    let options =
        output_processing_mode_options(&ConversionConfig::default(), Some(&video_metadata()), true);

    assert!(options.iter().all(|option| option.is_disabled));
}

#[test]
fn initialize_output_config_selects_first_audio_track() {
    let mut config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![
            AudioTrack {
                index: 2,
                codec: "aac".to_string(),
                ..AudioTrack::default()
            },
            AudioTrack {
                index: 4,
                codec: "ac3".to_string(),
                ..AudioTrack::default()
            },
        ],
        ..SourceMetadata::default()
    };

    assert!(initialize_output_config(&mut config, Some(&metadata)));
    assert_eq!(config.selected_audio_tracks, [2]);
}

#[test]
fn initialize_output_config_preserves_existing_audio_selection() {
    let mut config = ConversionConfig {
        selected_audio_tracks: vec![4],
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![
            AudioTrack {
                index: 2,
                codec: "aac".to_string(),
                ..AudioTrack::default()
            },
            AudioTrack {
                index: 4,
                codec: "ac3".to_string(),
                ..AudioTrack::default()
            },
        ],
        ..SourceMetadata::default()
    };

    assert!(!initialize_output_config(&mut config, Some(&metadata)));
    assert_eq!(config.selected_audio_tracks, [4]);
}

#[test]
fn initialize_output_config_keeps_audio_empty_without_source_tracks() {
    let mut config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        ..SourceMetadata::default()
    };

    assert!(!initialize_output_config(&mut config, Some(&metadata)));
    assert!(config.selected_audio_tracks.is_empty());
}

#[test]
fn initialize_output_config_does_not_select_audio_for_images() {
    let mut config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        audio_tracks: vec![AudioTrack {
            index: 1,
            codec: "aac".to_string(),
            ..AudioTrack::default()
        }],
        ..SourceMetadata::default()
    };

    assert!(initialize_output_config(&mut config, Some(&metadata)));
    assert!(config.selected_audio_tracks.is_empty());
}

#[test]
fn output_container_options_disable_video_targets_for_audio_sources() {
    let options = output_container_options(
        &ConversionConfig::default(),
        Some(&audio_metadata("aac")),
        false,
    );
    let mp4 = options
        .iter()
        .find(|option| option.container == "mp4")
        .expect("mp4 option should be visible for audio sources");

    assert_eq!(
        mp4.disabled_reason,
        Some("Video container unavailable for audio sources")
    );
}

#[test]
fn output_container_options_disable_all_when_settings_are_locked() {
    let options =
        output_container_options(&ConversionConfig::default(), Some(&video_metadata()), true);

    assert!(
        options
            .iter()
            .all(|option| option.disabled_reason == Some("Locked"))
    );
}

#[test]
fn stream_copy_audio_target_requires_compatible_audio_codec() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        container: "mp3".to_string(),
        ..ConversionConfig::default()
    };

    assert!(!is_container_compatible_for_stream_copy(
        &config,
        Some(&audio_metadata("aac")),
        "mp3"
    ));
}

#[test]
fn stream_copy_video_target_rejects_incompatible_subtitles() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        selected_subtitle_tracks: vec![2],
        ..ConversionConfig::default()
    };
    let mut metadata = video_metadata();
    metadata.subtitle_tracks[0].codec = "hdmv_pgs_subtitle".to_string();

    assert!(!is_container_compatible_for_stream_copy(
        &config,
        Some(&metadata),
        "mp4"
    ));
}

#[test]
fn stream_copy_video_target_accepts_supported_mkv_streams() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        selected_subtitle_tracks: vec![2],
        ..ConversionConfig::default()
    };

    assert!(is_container_compatible_for_stream_copy(
        &config,
        Some(&video_metadata()),
        "mkv"
    ));
}

#[test]
fn stream_copy_without_metadata_keeps_non_image_containers_selectable() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        ..ConversionConfig::default()
    };

    assert!(is_container_compatible_for_stream_copy(
        &config, None, "mp4"
    ));
}

#[test]
fn opus_output_normalizes_flac_source_and_only_copies_opus() {
    let metadata = audio_metadata("flac");
    let mut config = ConversionConfig {
        container: "flac".to_string(),
        audio_codec: "flac".to_string(),
        opus_vbr: false,
        selected_audio_tracks: vec![0],
        ..ConversionConfig::default()
    };
    assert!(visible_output_containers(Some(&metadata)).contains(&"opus".to_string()));
    assert!(apply_output_container(&mut config, "opus"));
    normalize_output_config(&mut config, Some(&metadata));
    assert_eq!(config.container, "opus");
    assert_eq!(config.audio_codec, "libopus");
    assert_eq!(config.audio_bitrate_mode, "bitrate");
    assert!(!config.opus_vbr);
    assert!(!normalize_output_config(&mut config, Some(&metadata)));
    config.processing_mode = ProcessingMode::Copy;
    for (codec, compatible) in [("opus", true), ("flac", false), ("aac", false)] {
        assert_eq!(
            is_container_compatible_for_stream_copy(&config, Some(&audio_metadata(codec)), "opus"),
            compatible,
            "{codec}"
        );
    }
}
