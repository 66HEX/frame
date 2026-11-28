use super::*;

fn metadata_with_tracks() -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Video),
        audio_tracks: vec![
            AudioTrack {
                index: 1,
                codec: "aac".to_string(),
                channels: Some("2".to_string()),
                language: Some("eng".to_string()),
                label: Some("Main".to_string()),
                bitrate_kbps: Some(192.0),
                ..AudioTrack::default()
            },
            AudioTrack {
                index: 2,
                codec: "ac3".to_string(),
                channels: Some("6".to_string()),
                ..AudioTrack::default()
            },
        ],
        ..SourceMetadata::default()
    }
}

#[test]
fn marks_selected_audio_track() {
    let config = ConversionConfig {
        selected_audio_tracks: vec![2],
        ..ConversionConfig::default()
    };

    let options = audio_track_options(&config, Some(&metadata_with_tracks()), false);

    assert!(!options[0].is_selected);
    assert!(options[1].is_selected);
}

#[test]
fn formats_track_detail_without_trailing_bitrate() {
    let options = audio_track_options(
        &ConversionConfig::default(),
        Some(&metadata_with_tracks()),
        false,
    );

    assert_eq!(options[0].detail, "2 channels • eng • Main");
}

#[test]
fn formats_track_bitrate_as_trailing_metadata() {
    let options = audio_track_options(
        &ConversionConfig::default(),
        Some(&metadata_with_tracks()),
        false,
    );

    assert_eq!(options[0].bitrate, "192 kb/s");
    assert_eq!(
        options[0].description(),
        "2 channels • eng • Main · 192 kb/s"
    );
    assert_eq!(options[1].description(), "6 channels");
}

#[test]
fn propagates_disabled_state_to_all_tracks() {
    let options = audio_track_options(
        &ConversionConfig::default(),
        Some(&metadata_with_tracks()),
        true,
    );

    assert!(options.iter().all(|option| option.is_disabled));
}

#[test]
fn toggle_audio_track_selection_adds_missing_track() {
    let mut config = ConversionConfig::default();

    assert!(toggle_audio_track_selection(&mut config, 1));

    assert_eq!(config.selected_audio_tracks, vec![1]);
}

#[test]
fn toggle_audio_track_selection_removes_selected_track() {
    let mut config = ConversionConfig {
        selected_audio_tracks: vec![1, 2],
        ..ConversionConfig::default()
    };

    assert!(toggle_audio_track_selection(&mut config, 1));

    assert_eq!(config.selected_audio_tracks, vec![2]);
}

#[test]
fn single_track_outputs_replace_selection_only_after_an_explicit_click() {
    for container in ["mp3", "wav", "flac"] {
        let mut config = ConversionConfig {
            container: container.to_string(),
            selected_audio_tracks: vec![1, 2],
            ..ConversionConfig::default()
        };
        normalize_output_config(&mut config, Some(&metadata_with_tracks()));
        assert_eq!(config.selected_audio_tracks, [1, 2]);
        assert!(
            audio_settings_error(&config, Some(&metadata_with_tracks()))
                .unwrap()
                .contains("one audio track")
        );
        toggle_audio_track_selection(&mut config, 3);
        assert_eq!(config.selected_audio_tracks, [3]);
        toggle_audio_track_selection(&mut config, 3);
        assert!(config.selected_audio_tracks.is_empty());
    }
    for container in ["mkv", "m4a"] {
        let mut config = ConversionConfig {
            container: container.to_string(),
            selected_audio_tracks: vec![1],
            ..ConversionConfig::default()
        };
        toggle_audio_track_selection(&mut config, 2);
        assert_eq!(config.selected_audio_tracks, [1, 2]);
    }
}

#[test]
fn selected_adpcm_copy_error_matches_container_and_readiness_without_changing_selection() {
    let mut metadata = metadata_with_tracks();
    metadata.video_codec = Some("h264".into());
    metadata.source_format = Some("mov,mp4,m4a,3gp,3g2,mj2".into());
    metadata.audio_tracks[0].codec = "pcm_s16le".into();
    metadata.audio_tracks[1].codec = "adpcm_swf".into();
    metadata.audio_tracks[1].channels = Some("1".into());
    metadata.audio_tracks[1].codec_tag = Some(0x4653_736d);
    for selected in [vec![2], vec![1, 2], vec![2, 1]] {
        let mut config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "mkv".into(),
            selected_audio_tracks: selected.clone(),
            ..ConversionConfig::default()
        };
        normalize_output_config(&mut config, Some(&metadata));
        assert_eq!(config.processing_mode, ProcessingMode::Copy);
        assert_eq!(config.selected_audio_tracks, selected);
        assert!(!is_container_compatible_for_stream_copy(
            &config,
            Some(&metadata),
            "mkv"
        ));
        let reason = audio_settings_error(&config, Some(&metadata)).unwrap();
        assert!(reason.contains("Track #2"));
        let options = audio_track_options(&config, Some(&metadata), false);
        assert!(options[0].copy_error.is_none());
        assert_eq!(
            options[1].copy_error,
            Some("This audio track cannot be copied to MKV. Choose Re-encode.")
        );
        assert!(!options[1].is_disabled);
        let outputs = output_container_options(&config, Some(&metadata), false);
        assert_eq!(
            outputs
                .iter()
                .find(|option| option.container == "mkv")
                .unwrap()
                .disabled_reason,
            options[1].copy_error
        );
        toggle_audio_track_selection(&mut config, 2);
        assert!(audio_settings_error(&config, Some(&metadata)).is_none());
        assert!(is_container_compatible_for_stream_copy(
            &config,
            Some(&metadata),
            "mkv"
        ));
        assert!(
            audio_track_options(&config, Some(&metadata), false)[1]
                .copy_error
                .is_none()
        );
    }
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Reencode,
        container: "mkv".into(),
        selected_audio_tracks: vec![2],
        ..ConversionConfig::default()
    };
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
    assert!(
        audio_track_options(&config, Some(&metadata), false)[1]
            .copy_error
            .is_none()
    );
}

#[test]
fn source_audio_overrides_survive_reordering_and_deselection_and_drop_missing_sources() {
    use frame_core::types::AudioTrackOverride;
    let metadata = metadata_with_tracks();
    let mut config: ConversionConfig =
        serde_json::from_str(r#"{"container":"mkv","selectedAudioTracks":[2,1,2]}"#).unwrap();
    assert!(config.audio_track_overrides.is_empty());
    let override_for = |title: &str| AudioTrackOverride {
        title: Some(title.into()),
        ..Default::default()
    };
    config.audio_track_overrides = [
        (1, override_for("")),
        (2, override_for("Main")),
        (44, override_for("Gone")),
    ]
    .into();
    normalize_output_config(&mut config, Some(&metadata));
    assert_eq!(config.selected_audio_tracks, [2, 1, 2]);
    assert!(!config.audio_track_overrides.contains_key(&44));
    assert!(toggle_audio_track_selection(&mut config, 2));
    normalize_output_config(&mut config, Some(&metadata));
    assert_eq!(config.selected_audio_tracks, [1]);
    assert!(toggle_audio_track_selection(&mut config, 2));
    let task = crate::conversion_runner::core_config_from_gpui(&config);
    assert_eq!(task.audio_track_overrides[&1].title.as_deref(), Some(""));
    assert_eq!(
        task.audio_track_overrides[&2].title.as_deref(),
        Some("Main")
    );
}

#[test]
fn validates_only_exported_audio_tags_and_projects_default_for_the_selected_container() {
    let mut metadata = metadata_with_tracks();
    metadata.audio_tracks[0].is_default = true;
    let mut config = ConversionConfig {
        container: "mp4".into(),
        selected_audio_tracks: vec![2, 1, 2],
        ..ConversionConfig::default()
    };
    let source_default = &metadata.audio_tracks[0];
    let other = &metadata.audio_tracks[1];
    assert!(audio_track_is_default(&config, &metadata, source_default));
    assert!(!audio_track_is_default(&config, &metadata, other));
    assert!(apply_audio_track_default(&mut config, 1, false));
    assert!(audio_track_is_default(&config, &metadata, other));
    assert!(apply_audio_track_default(&mut config, 2, false));
    assert!(
        audio_settings_error(&config, Some(&metadata))
            .unwrap()
            .contains("default")
    );
    assert!(apply_audio_track_default(&mut config, 2, true));
    config.audio_track_overrides.entry(1).or_default().language = Some("English".into());
    assert!(
        audio_settings_error(&config, Some(&metadata))
            .unwrap()
            .contains("language")
    );
    let edit = config.audio_track_overrides.get_mut(&1).unwrap();
    edit.language = Some("eng".into());
    edit.title = Some("bad\0title".into());
    assert!(
        audio_settings_error(&config, Some(&metadata))
            .unwrap()
            .contains("title")
    );
    config.metadata.mode = MetadataMode::Clean;
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
    config.metadata.mode = MetadataMode::Preserve;
    config.selected_audio_tracks = vec![2];
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
}

#[test]
fn sample_rate_overrides_remain_bound_to_selected_encoded_audio_across_mode_and_codec_changes() {
    let mut config = ConversionConfig {
        container: "mkv".into(),
        audio_codec: "pcm_s16le".into(),
        selected_audio_tracks: vec![2, 1, 2],
        ..ConversionConfig::default()
    };
    assert!(apply_audio_sample_rate(&mut config, 2, Some(44_056)));
    assert!(apply_audio_sample_rate(&mut config, 1, Some(48_000)));
    assert!(!apply_audio_sample_rate(&mut config, 99, Some(44_100)));
    let metadata = metadata_with_tracks();
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
    config.processing_mode = ProcessingMode::CopyAudio;
    assert!(!apply_audio_sample_rate(&mut config, 2, None));
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
    config.processing_mode = ProcessingMode::CopyVideo;
    apply_audio_codec(&mut config, "aac");
    assert!(
        audio_settings_error(&config, Some(&metadata))
            .unwrap()
            .contains("#2")
    );
    toggle_audio_track_selection(&mut config, 2);
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
    toggle_audio_track_selection(&mut config, 2);
    assert_eq!(config.audio_track_overrides[&2].sample_rate, Some(44_056));
    assert!(apply_audio_sample_rate(&mut config, 2, None));
    assert!(!config.audio_track_overrides.contains_key(&2));
    config.audio_track_overrides.get_mut(&1).unwrap().title = Some("Main".into());
    assert!(apply_audio_sample_rate(&mut config, 1, None));
    assert_eq!(
        config.audio_track_overrides[&1].title.as_deref(),
        Some("Main")
    );
    assert!(audio_settings_error(&config, Some(&metadata)).is_none());
}
