use super::*;

fn metadata_with_subtitles() -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Video),
        subtitle_tracks: vec![
            SubtitleTrack {
                index: 2,
                codec: "subrip".to_string(),
                language: Some("eng".to_string()),
                label: Some("Dialogue".to_string()),
            },
            SubtitleTrack {
                index: 3,
                codec: "ass".to_string(),
                language: Some("jpn".to_string()),
                label: Some("Signs".to_string()),
            },
        ],
        ..SourceMetadata::default()
    }
}

#[test]
fn default_subtitle_fields_match_original_empty_state() {
    let config = ConversionConfig::default();

    assert_eq!(config.subtitle_burn_path, None);
    assert_eq!(config.subtitle_font_name, None);
    assert_eq!(config.subtitle_font_size, None);
    assert_eq!(config.subtitle_font_color, None);
    assert!(config.external_subtitle_tracks.is_empty());
    assert_eq!(subtitle_position(&config), SubtitlePosition::Bottom);
}

#[test]
fn subtitle_track_options_mark_selected_track() {
    let config = ConversionConfig {
        selected_subtitle_tracks: vec![3],
        ..ConversionConfig::default()
    };

    let options = subtitle_track_options(
        &config,
        Some(&metadata_with_subtitles()),
        false,
        &frame_core::capabilities::AvailableEncoders::default(),
    );

    assert!(!options[0].is_selected);
    assert!(options[1].is_selected);
}

#[test]
fn subtitle_track_options_format_language_and_label_detail() {
    let options = subtitle_track_options(
        &ConversionConfig::default(),
        Some(&metadata_with_subtitles()),
        false,
        &frame_core::capabilities::AvailableEncoders::default(),
    );

    assert_eq!(options[0].detail, "eng • Dialogue");
}

#[test]
fn m2t_bitmap_subtitle_requires_available_dvb_encoder() {
    let config = ConversionConfig {
        container: "m2t".to_string(),
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        subtitle_tracks: vec![SubtitleTrack {
            index: 4,
            codec: "hdmv_pgs_subtitle".to_string(),
            language: None,
            label: None,
        }],
        ..SourceMetadata::default()
    };

    let unavailable = subtitle_track_options(
        &config,
        Some(&metadata),
        false,
        &frame_core::capabilities::AvailableEncoders::default(),
    );
    let available = subtitle_track_options(
        &config,
        Some(&metadata),
        false,
        &frame_core::capabilities::AvailableEncoders {
            dvbsub: true,
            ..frame_core::capabilities::AvailableEncoders::default()
        },
    );

    assert!(unavailable[0].is_disabled);
    assert!(
        unavailable[0]
            .disabled_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("DVB subtitle encoder"))
    );
    assert!(!available[0].is_disabled);
    assert!(available[0].disabled_reason.is_none());
}

#[test]
fn toggle_subtitle_track_selection_removes_selected_track() {
    let mut config = ConversionConfig {
        selected_subtitle_tracks: vec![2, 3],
        ..ConversionConfig::default()
    };

    assert!(toggle_subtitle_track_selection(&mut config, 2));

    assert_eq!(config.selected_subtitle_tracks, vec![3]);
}

#[test]
fn apply_subtitle_font_size_rejects_unknown_size() {
    let mut config = ConversionConfig::default();

    assert!(!apply_subtitle_font_size(&mut config, "13"));

    assert_eq!(config.subtitle_font_size, None);
}

#[test]
fn apply_subtitle_font_color_normalizes_short_hex() {
    let mut config = ConversionConfig::default();

    assert!(apply_subtitle_font_color(&mut config, "#fff"));

    assert_eq!(config.subtitle_font_color.as_deref(), Some("#ffffff"));
}

#[test]
fn external_subtitle_updates_add_unique_files_and_edit_metadata() {
    let mut config = ConversionConfig {
        container: "mkv".to_string(),
        ..ConversionConfig::default()
    };

    let selected = add_external_subtitle_tracks(
        &mut config,
        [
            " /tmp/english.srt ".to_string(),
            "/tmp/english.srt".to_string(),
        ],
    );

    assert_eq!(selected, Some(0));
    assert_eq!(config.external_subtitle_tracks.len(), 1);
    assert_eq!(config.external_subtitle_tracks[0].path, "/tmp/english.srt");
    assert!(apply_external_subtitle_language(&mut config, 0, "eng"));
    assert!(apply_external_subtitle_title(
        &mut config,
        0,
        "English subtitles"
    ));
    assert!(apply_external_subtitle_forced(&mut config, 0, true));
    assert_eq!(
        config.external_subtitle_tracks[0].language.as_deref(),
        Some("eng")
    );
    assert_eq!(
        config.external_subtitle_tracks[0].title.as_deref(),
        Some("English subtitles")
    );
    assert!(config.external_subtitle_tracks[0].is_forced);
}

#[test]
fn external_subtitle_default_selection_is_exclusive() {
    let mut config = ConversionConfig {
        container: "mkv".to_string(),
        ..ConversionConfig::default()
    };
    assert_eq!(
        add_external_subtitle_tracks(
            &mut config,
            ["/tmp/first.srt".to_string(), "/tmp/second.ass".to_string()]
        ),
        Some(1)
    );

    assert!(apply_external_subtitle_default(&mut config, 0, true));
    assert!(apply_external_subtitle_default(&mut config, 1, true));

    assert!(!config.external_subtitle_tracks[0].is_default);
    assert!(config.external_subtitle_tracks[1].is_default);
}

#[test]
fn stream_copy_normalization_keeps_selectable_subtitles() {
    let mut config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/english.srt".to_string(),
            ..ExternalSubtitleTrack::default()
        }],
        subtitle_burn_path: Some("/tmp/burn.srt".to_string()),
        ..ConversionConfig::default()
    };

    assert!(normalize_output_config(
        &mut config,
        Some(&metadata_with_subtitles())
    ));

    assert_eq!(config.external_subtitle_tracks.len(), 1);
    assert_eq!(config.subtitle_burn_path, None);
}

#[test]
fn transport_profile_switch_keeps_compatible_subtitle_drafts_and_selections() {
    let mut config = ConversionConfig {
        container: "m2t".to_string(),
        selected_subtitle_tracks: vec![2],
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/captions.sup".to_string(),
            ..ExternalSubtitleTrack::default()
        }],
        subtitle_burn_path: Some("/tmp/burn.ass".to_string()),
        subtitle_font_name: Some("Arial".to_string()),
        subtitle_font_size: Some("24".to_string()),
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        subtitle_tracks: vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }],
        ..SourceMetadata::default()
    };

    apply_output_container(&mut config, "m2ts");
    normalize_output_config(&mut config, Some(&metadata));

    assert_eq!(config.selected_subtitle_tracks, [2]);
    assert_eq!(config.external_subtitle_tracks.len(), 1);
    assert_eq!(config.subtitle_burn_path.as_deref(), Some("/tmp/burn.ass"));
    assert_eq!(config.subtitle_font_name.as_deref(), Some("Arial"));
    assert_eq!(config.subtitle_font_size.as_deref(), Some("24"));
}

#[test]
fn removing_external_subtitle_track_preserves_remaining_order() {
    let mut config = ConversionConfig {
        container: "mkv".to_string(),
        ..ConversionConfig::default()
    };
    add_external_subtitle_tracks(
        &mut config,
        ["/tmp/first.srt".to_string(), "/tmp/second.vtt".to_string()],
    );

    assert!(remove_external_subtitle_track(&mut config, 0));
    assert!(!remove_external_subtitle_track(&mut config, 4));
    assert_eq!(config.external_subtitle_tracks[0].path, "/tmp/second.vtt");
}

#[test]
fn adding_external_subtitles_rejects_text_sidecars_for_mts() {
    let mut config = ConversionConfig {
        container: "mts".to_string(),
        ..ConversionConfig::default()
    };

    let selected = add_external_subtitle_tracks(
        &mut config,
        [
            "/tmp/captions.ass".to_string(),
            "/tmp/captions.sup".to_string(),
        ],
    );

    assert_eq!(selected, Some(0));
    assert_eq!(config.external_subtitle_tracks[0].path, "/tmp/captions.sup");
}

#[test]
fn switching_to_mts_removes_incompatible_text_sidecars() {
    let mut config = ConversionConfig {
        container: "mts".to_string(),
        external_subtitle_tracks: vec![
            ExternalSubtitleTrack {
                path: "/tmp/captions.vtt".to_string(),
                ..ExternalSubtitleTrack::default()
            },
            ExternalSubtitleTrack {
                path: "/tmp/captions.sup".to_string(),
                ..ExternalSubtitleTrack::default()
            },
        ],
        ..ConversionConfig::default()
    };

    normalize_output_config(&mut config, Some(&metadata_with_subtitles()));

    assert_eq!(config.external_subtitle_tracks.len(), 1);
    assert_eq!(config.external_subtitle_tracks[0].path, "/tmp/captions.sup");
}

#[test]
fn switching_to_mts_clears_hidden_pgs_sidecar_metadata() {
    let mut config = ConversionConfig {
        container: "mts".to_string(),
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/captions.sup".to_string(),
            language: Some("eng".to_string()),
            title: Some("English".to_string()),
            is_default: true,
            is_forced: true,
        }],
        ..ConversionConfig::default()
    };

    normalize_output_config(&mut config, Some(&metadata_with_subtitles()));

    assert_eq!(
        config.external_subtitle_tracks[0],
        ExternalSubtitleTrack {
            path: "/tmp/captions.sup".to_string(),
            ..ExternalSubtitleTrack::default()
        }
    );
}

#[test]
fn normalize_output_config_clears_subtitle_settings_for_audio_container() {
    let mut config = ConversionConfig {
        container: "mp3".to_string(),
        selected_subtitle_tracks: vec![2],
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/soft.srt".to_string(),
            ..ExternalSubtitleTrack::default()
        }],
        subtitle_burn_path: Some("/tmp/sub.srt".to_string()),
        subtitle_font_name: Some("Arial".to_string()),
        ..ConversionConfig::default()
    };

    assert!(normalize_output_config(
        &mut config,
        Some(&metadata_with_subtitles())
    ));

    assert!(config.selected_subtitle_tracks.is_empty());
    assert!(config.external_subtitle_tracks.is_empty());
    assert_eq!(config.subtitle_burn_path, None);
    assert_eq!(config.subtitle_font_name, None);
}

#[test]
fn normalize_output_config_removes_pgs_selection_for_mp4() {
    let mut config = ConversionConfig {
        container: "mp4".to_string(),
        selected_subtitle_tracks: vec![2],
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        subtitle_tracks: vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }],
        ..SourceMetadata::default()
    };

    assert!(!normalize_video_config(&mut config, Some(&metadata)));
    assert_normalizes_once(normalize_output_config, &mut config, Some(&metadata));
    assert!(config.selected_subtitle_tracks.is_empty());
}

#[test]
fn normalize_output_config_keeps_text_subtitle_selection_for_mp4() {
    let mut config = ConversionConfig {
        container: "mp4".to_string(),
        selected_subtitle_tracks: vec![2],
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        subtitle_tracks: vec![SubtitleTrack {
            index: 2,
            codec: "subrip".to_string(),
            ..SubtitleTrack::default()
        }],
        ..SourceMetadata::default()
    };

    normalize_output_config(&mut config, Some(&metadata));

    assert_eq!(config.selected_subtitle_tracks, [2]);
}
