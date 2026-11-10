use super::*;

fn image_metadata() -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    }
}

fn audio_metadata() -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(SourceKind::Audio),
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
fn default_presets_match_original_builtin_order() {
    let presets = default_presets();

    assert_eq!(presets[0].id, "balanced-mp4");
    assert_eq!(presets[16].id, "discord");
    assert!(presets.iter().any(|preset| preset.id == "broadcast-m2t"));
    let m2ts = presets
        .iter()
        .find(|preset| preset.id == "m2ts-h264")
        .expect("M2TS H.264 preset should exist");
    assert_eq!(m2ts.name, "M2TS H.264 (192-byte)");
    let m2t = presets
        .iter()
        .find(|preset| preset.id == "broadcast-m2t")
        .expect("M2T broadcast preset should exist");
    assert_eq!(m2t.config.audio_channels, "stereo");
    assert!(presets.iter().all(|preset| preset.built_in));
}

#[test]
fn configs_match_uses_original_core_fields() {
    let config = ConversionConfig {
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: "6000".to_string(),
        ..ConversionConfig::default()
    };
    let same = ConversionConfig {
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: "6000".to_string(),
        audio_bitrate: "999".to_string(),
        ..ConversionConfig::default()
    };

    assert!(configs_match(&config, &same));
}

#[test]
fn preset_options_mark_balanced_mp4_applied_by_default() {
    let presets = default_presets();
    let options = preset_options(&ConversionConfig::default(), &presets, None);

    assert!(options[0].is_selected);
    assert_eq!(options[0].status, Some("Applied"));
}

#[test]
fn preset_compatibility_restricts_image_sources_to_image_and_gif_outputs() {
    let presets = default_presets();
    let options = preset_options(
        &ConversionConfig::default(),
        &presets,
        Some(&image_metadata()),
    );

    assert!(!options[0].is_compatible);
    assert!(
        options
            .iter()
            .find(|option| option.preset.id == "gif-web-small")
            .unwrap()
            .is_compatible
    );
}

#[test]
fn preset_compatibility_restricts_audio_sources_to_audio_only_outputs() {
    let presets = default_presets();
    let options = preset_options(
        &ConversionConfig::default(),
        &presets,
        Some(&audio_metadata()),
    );

    assert!(!options[0].is_compatible);
    assert!(
        options
            .iter()
            .find(|option| option.preset.id == "audio-only")
            .unwrap()
            .is_compatible
    );
}

#[test]
fn apply_preset_replaces_config_and_normalizes_for_metadata() {
    let preset = default_presets()
        .into_iter()
        .find(|preset| preset.id == "audio-only")
        .expect("audio preset should exist");
    let mut config = ConversionConfig::default();

    assert!(apply_preset(&mut config, &preset, Some(&audio_metadata())));

    assert_eq!(config.container, "mp3");
    assert_eq!(config.audio_codec, "mp3");
}

#[test]
fn apply_preset_preserves_explicit_source_track_selections() {
    let preset = default_presets()
        .into_iter()
        .find(|preset| preset.id == "archive-hq")
        .expect("archive preset should exist");
    let mut config = ConversionConfig {
        selected_audio_tracks: vec![1],
        selected_subtitle_tracks: vec![2],
        ..ConversionConfig::default()
    };

    apply_preset(&mut config, &preset, Some(&video_metadata()));

    assert_eq!(config.selected_audio_tracks, [1]);
    assert_eq!(config.selected_subtitle_tracks, [2]);
}

#[test]
fn apply_preset_preserves_source_subtitle_files() {
    let preset = default_presets()
        .into_iter()
        .find(|preset| preset.id == "archive-hq")
        .expect("archive preset should exist");
    let mut config = ConversionConfig {
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/captions.srt".to_string(),
            ..ExternalSubtitleTrack::default()
        }],
        subtitle_burn_path: Some("/tmp/burn.ass".to_string()),
        ..ConversionConfig::default()
    };

    apply_preset(&mut config, &preset, Some(&video_metadata()));

    assert_eq!(config.external_subtitle_tracks.len(), 1);
    assert_eq!(config.subtitle_burn_path.as_deref(), Some("/tmp/burn.ass"));
}

#[test]
fn create_custom_preset_omits_source_specific_track_state() {
    let config = ConversionConfig {
        selected_audio_tracks: vec![1],
        selected_subtitle_tracks: vec![2],
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/captions.srt".to_string(),
            ..ExternalSubtitleTrack::default()
        }],
        subtitle_burn_path: Some("/tmp/burn.ass".to_string()),
        ..ConversionConfig::default()
    };

    let preset = create_custom_preset("custom-1".to_string(), "Custom", &config);

    assert!(preset.config.selected_audio_tracks.is_empty());
    assert!(preset.config.selected_subtitle_tracks.is_empty());
    assert!(preset.config.external_subtitle_tracks.is_empty());
    assert_eq!(preset.config.subtitle_burn_path, None);
}
