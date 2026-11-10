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
