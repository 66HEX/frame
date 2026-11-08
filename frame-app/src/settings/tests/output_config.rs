use super::*;

#[test]
fn output_transitions_disable_overlay_without_losing_its_draft() {
    let overlay = OverlaySettings {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.4,
        y: 0.6,
        width: 0.25,
        opacity: 0.75,
        anchor: "custom".to_string(),
    };
    for container in ["gif", "mp3", "wav"] {
        let mut config = ConversionConfig {
            overlay: Some(overlay.clone()),
            ..ConversionConfig::default()
        };
        assert!(apply_output_container(&mut config, container));
        let disabled = config.overlay.as_ref().unwrap();
        assert_eq!(
            disabled,
            &OverlaySettings {
                enabled: false,
                ..overlay.clone()
            }
        );
        normalize_output_config(&mut config, None);
        assert!(!config.overlay.as_ref().unwrap().enabled);
        apply_output_container(&mut config, "mp4");
        assert_eq!(config.overlay.as_ref().unwrap().path, overlay.path);
        assert!(!config.overlay.as_ref().unwrap().enabled);
    }
    let mut config = ConversionConfig {
        overlay: Some(overlay.clone()),
        ..ConversionConfig::default()
    };
    assert!(apply_processing_mode(
        &mut config,
        None,
        ProcessingMode::Copy
    ));
    assert!(!config.overlay.as_ref().unwrap().enabled);
    assert!(apply_processing_mode(
        &mut config,
        None,
        ProcessingMode::Reencode
    ));
    assert_eq!(
        config.overlay,
        Some(OverlaySettings {
            enabled: false,
            ..overlay
        })
    );
}

#[test]
fn gif_palette_minimum_reserves_a_transparent_entry() {
    let mut config = ConversionConfig {
        container: "gif".to_string(),
        gif_colors: 2,
        ..ConversionConfig::default()
    };
    normalize_output_config(&mut config, None);
    assert_eq!(config.gif_colors, 3);
    assert!(!apply_gif_colors(&mut config, 2));
    assert_eq!(
        crate::conversion_runner::core_config_from_gpui(&config).gif_colors,
        3
    );
    config.gif_colors = 2;
    assert_eq!(
        crate::conversion_runner::core_config_from_gpui(&config).gif_colors,
        3
    );
}

#[test]
fn default_config_has_no_trim_times() {
    let config = ConversionConfig::default();

    assert_eq!(config.start_time, None);
    assert_eq!(config.end_time, None);
}

#[test]
fn default_config_has_neutral_transform_and_no_crop() {
    let config = ConversionConfig::default();

    assert_eq!(config.rotation, "0");
    assert!(!config.flip_horizontal);
    assert!(!config.flip_vertical);
    assert_eq!(config.crop, None);
}

#[test]
fn normalize_output_config_resets_audio_filter_controls_in_copy_mode() {
    let mut config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        audio_bitrate_mode: "vbr".to_string(),
        audio_volume: 150,
        audio_normalize: true,
        ..ConversionConfig::default()
    };

    assert!(!normalize_video_config(&mut config, None));
    assert_normalizes_once(normalize_output_config, &mut config, None);

    assert_eq!(config.audio_bitrate_mode, "bitrate");
    assert_eq!(config.audio_volume, 100);
    assert!(!config.audio_normalize);
}

#[test]
fn apply_trim_times_stores_trim_bounds() {
    let mut config = ConversionConfig::default();

    assert!(apply_trim_times(
        &mut config,
        Some(" 00:00:05.000 ".to_string()),
        Some("00:00:30.250".to_string())
    ));

    assert_eq!(config.start_time.as_deref(), Some("00:00:05.000"));
    assert_eq!(config.end_time.as_deref(), Some("00:00:30.250"));
}

#[test]
fn apply_trim_times_clears_blank_trim_bounds() {
    let mut config = ConversionConfig {
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:30.250".to_string()),
        ..ConversionConfig::default()
    };

    assert!(apply_trim_times(
        &mut config,
        Some(" ".to_string()),
        Some(String::new())
    ));

    assert_eq!(config.start_time, None);
    assert_eq!(config.end_time, None);
}

#[test]
fn apply_trim_times_reports_no_change_for_same_bounds() {
    let mut config = ConversionConfig {
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:30.250".to_string()),
        ..ConversionConfig::default()
    };

    assert!(!apply_trim_times(
        &mut config,
        Some("00:00:05.000".to_string()),
        Some("00:00:30.250".to_string())
    ));
}

#[test]
fn normalize_output_config_forces_audio_sources_to_audio_container() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Audio),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig::default();

    assert_normalizes_once(normalize_output_config, &mut config, Some(&metadata));

    assert_eq!(config.container, "mp3");
}

#[test]
fn normalize_output_config_forces_image_sources_to_image_container() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig::default();

    assert_normalizes_once(normalize_output_config, &mut config, Some(&metadata));

    assert_eq!(config.container, "png");
    assert_eq!(config.video_codec, "png");
}

#[test]
fn normalize_output_config_clears_trim_for_image_sources() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig {
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:30.250".to_string()),
        ..ConversionConfig::default()
    };

    assert!(normalize_output_config(&mut config, Some(&metadata)));

    assert_eq!(config.start_time, None);
    assert_eq!(config.end_time, None);
}

#[test]
fn normalize_output_config_preserves_trim_for_video_sources() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig {
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:30.250".to_string()),
        ..ConversionConfig::default()
    };

    normalize_output_config(&mut config, Some(&metadata));

    assert_eq!(config.start_time.as_deref(), Some("00:00:05.000"));
    assert_eq!(config.end_time.as_deref(), Some("00:00:30.250"));
}

#[test]
fn normalize_output_config_clears_crop_for_audio_sources() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Audio),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig {
        crop: Some(CropSettings {
            enabled: true,
            x: 100,
            y: 100,
            width: 200,
            height: 200,
            source_width: Some(1920),
            source_height: Some(1080),
            aspect_ratio: None,
        }),
        ..ConversionConfig::default()
    };

    assert!(normalize_output_config(&mut config, Some(&metadata)));

    assert_eq!(config.crop, None);
}

#[test]
fn normalize_output_config_reencodes_image_sources() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        container: "png".to_string(),
        ..ConversionConfig::default()
    };

    normalize_output_config(&mut config, Some(&metadata));

    assert_eq!(config.processing_mode, ProcessingMode::Reencode);
}

#[test]
fn normalize_output_config_reencodes_gif_outputs() {
    let mut config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        container: "gif".to_string(),
        ..ConversionConfig::default()
    };

    assert_normalizes_once(normalize_output_config, &mut config, None);

    assert_eq!(config.processing_mode, ProcessingMode::Reencode);
}

#[test]
fn normalize_output_config_reports_video_only_changes() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        ..SourceMetadata::default()
    };
    for metadata in [None, Some(&metadata)] {
        let mut config = ConversionConfig {
            pixel_format: "invalid".to_string(),
            ..ConversionConfig::default()
        };
        let mut expected = config.clone();
        assert_normalizes_once(normalize_video_config, &mut expected, metadata);
        assert_normalizes_once(normalize_output_config, &mut config, metadata);
        assert_eq!(config, expected);
    }
}

#[test]
fn apply_output_container_falls_back_to_default_audio_codec_when_needed() {
    let mut config = ConversionConfig {
        audio_codec: "flac".to_string(),
        ..ConversionConfig::default()
    };

    apply_output_container(&mut config, "webm");

    assert_eq!(config.audio_codec, "libopus");
}

#[test]
fn apply_processing_mode_rejects_copy_for_image_sources() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig::default();

    assert!(!apply_processing_mode(
        &mut config,
        Some(&metadata),
        ProcessingMode::Copy
    ));
}
