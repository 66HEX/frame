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
