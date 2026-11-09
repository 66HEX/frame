use super::*;

#[test]
fn tiff_preview_propagates_palette_source_and_invalidates_only_effective_format_changes() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "palette", "/tmp/dynamic.mov");
    let mut entry = root.source_metadata.entry_for("palette");
    entry.metadata.as_mut().unwrap().pixel_format = Some("pal8".into());
    root.update_selected_config(|config| crate::settings::apply_output_container(config, "png"));
    let png = root.selected_preview_runtime_request(&entry).unwrap();
    assert!(png.config.source_is_palette);
    root.update_selected_config(|config| crate::settings::apply_output_container(config, "tiff"));
    let tiff = root.selected_preview_runtime_request(&entry).unwrap();
    assert!(tiff.config.source_is_palette);
    assert_ne!(png.key.visual_hash, tiff.key.visual_hash);
    root.update_selected_config(|config| {
        config.image_tiff_compression = "raw".into();
        true
    });
    let raw = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(raw.key.visual_hash, tiff.key.visual_hash);
    entry.metadata.as_mut().unwrap().pixel_format = Some("rgba64le".into());
    let true_color = root.selected_preview_runtime_request(&entry).unwrap();
    assert!(!true_color.config.source_is_palette);
    assert_ne!(true_color.key.visual_hash, tiff.key.visual_hash);
    root.update_selected_config(|config| crate::settings::apply_output_container(config, "png"));
    let png_true_color = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(png.key.visual_hash, png_true_color.key.visual_hash);
}

#[test]
fn preview_request_tracks_output_raster_policy_without_encoder_quality() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(320),
            height: Some(180),
            pixel_format: Some("yuv420p".to_string()),
            ..SourceMetadata::default()
        },
    );
    let entry = root.source_metadata.entry_for("video");
    let request = root.selected_preview_runtime_request(&entry).unwrap();
    root.update_selected_config(|config| {
        config.video_bitrate = "9000".to_string();
        true
    });
    let bitrate_only = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(request.key.visual_hash, bitrate_only.key.visual_hash);
    for container in ["gif", "png", "mp4"] {
        root.update_selected_config(|config| {
            crate::settings::apply_output_container(config, container)
        });
        let changed = root.selected_preview_runtime_request(&entry).unwrap();
        assert_eq!(
            changed.key.visual_hash == request.key.visual_hash,
            container == "mp4"
        );
    }
    root.update_selected_config(|config| {
        crate::settings::apply_processing_mode(config, None, crate::settings::ProcessingMode::Copy)
    });
    let copy = root.selected_preview_runtime_request(&entry).unwrap();
    assert_ne!(copy.key.visual_hash, request.key.visual_hash);
}

#[test]
fn preview_runtime_request_is_silent_without_explicit_audio_selection() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            audio_tracks: vec![crate::settings::AudioTrack {
                index: 1,
                codec: "aac".to_string(),
                ..crate::settings::AudioTrack::default()
            }],
            ..SourceMetadata::default()
        },
    );
    let metadata_entry = root.source_metadata.entry_for("video");

    let request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("preview request");

    assert_eq!(request.config.selected_audio_track, None);
}

#[test]
fn preview_runtime_request_uses_first_valid_explicit_audio_selection() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            audio_tracks: vec![crate::settings::AudioTrack {
                index: 1,
                codec: "aac".to_string(),
                ..crate::settings::AudioTrack::default()
            }],
            ..SourceMetadata::default()
        },
    );
    root.update_selected_config(|config| {
        config.selected_audio_tracks = vec![99, 1];
        true
    });
    let metadata_entry = root.source_metadata.entry_for("video");

    let request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("preview request");

    assert_eq!(request.config.selected_audio_track, Some(1));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This scenario verifies preview runtime hashing across visual, audio, trim, and transform config."
)]
fn preview_runtime_key_tracks_visual_config_but_ignores_encoder_quality() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            duration: Some("12.5".to_string()),
            audio_codec: Some("aac".to_string()),
            audio_tracks: vec![crate::settings::AudioTrack {
                index: 1,
                codec: "aac".to_string(),
                codec_tag: None,
                channels: Some("2".to_string()),
                channel_layout: Some("stereo".to_string()),
                language: None,
                label: None,
                bitrate_kbps: None,
                sample_rate: Some("48000".to_string()),
            }],
            ..SourceMetadata::default()
        },
    );
    let metadata_entry = root.source_metadata.entry_for("video");
    let initial_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("initial request");

    root.update_selected_config(|config| {
        config.crf = 18;
        true
    });
    let quality_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("quality request");

    assert_eq!(quality_request.key, initial_request.key);

    root.update_selected_config(|config| {
        config.start_time = Some("00:00:02.000".to_string());
        config.end_time = Some("00:00:08.000".to_string());
        true
    });
    let trim_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("trim request");

    assert_eq!(trim_request.key, initial_request.key);
    assert_eq!(
        trim_request.config.conversion_config.start_time.as_deref(),
        Some("00:00:02.000")
    );
    assert_eq!(
        trim_request.config.conversion_config.end_time.as_deref(),
        Some("00:00:08.000")
    );

    root.update_selected_config(|config| {
        config.video_filters.temperature.value = 3200;
        true
    });
    let disabled_video_filter_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("disabled video filter request");

    assert_eq!(disabled_video_filter_request.key, initial_request.key);

    root.update_selected_config(|config| {
        config.video_filters.color.brightness.enabled = true;
        config.video_filters.color.brightness.value = 20;
        true
    });
    let video_filter_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("video filter request");

    assert_ne!(
        video_filter_request.key.visual_hash,
        initial_request.key.visual_hash
    );
    assert_eq!(
        video_filter_request.key.audio_hash,
        initial_request.key.audio_hash
    );

    root.update_selected_config(|config| {
        config.video_filters.color.brightness.enabled = false;
        config.video_filters.color.brightness.value = 80;
        true
    });
    let disabled_video_value_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("disabled video filter value request");

    assert_eq!(
        disabled_video_value_request.key.visual_hash,
        disabled_video_filter_request.key.visual_hash
    );

    root.update_selected_config(|config| {
        config.audio_filters.bass.value = 12;
        true
    });
    let disabled_audio_filter_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("disabled audio filter request");

    assert_eq!(
        disabled_audio_filter_request.key.audio_hash,
        initial_request.key.audio_hash
    );

    root.update_selected_config(|config| {
        config.audio_filters.limiter.enabled = true;
        config.audio_filters.limiter.value = -3;
        true
    });
    let audio_filter_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("audio filter request");

    assert_eq!(
        audio_filter_request.key.visual_hash,
        disabled_audio_filter_request.key.visual_hash
    );
    assert_ne!(
        audio_filter_request.key.audio_hash,
        initial_request.key.audio_hash
    );

    root.update_selected_config(|config| {
        config.video_filters = crate::settings::VideoFiltersConfig::default();
        config.audio_filters = crate::settings::AudioFiltersConfig::default();
        true
    });

    root.update_selected_config(|config| {
        config.audio_volume = 80;
        true
    });
    let audio_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("audio request");

    assert_ne!(audio_request.key, initial_request.key);

    assert!(root.rotate_selected_preview());
    assert!(root.toggle_selected_flip(FlipAxis::Horizontal));
    root.preview_ui.crop_mode = true;
    root.preview_ui.draft_crop = Some(CropRect {
        x: 0.25,
        y: 0.25,
        width: 0.5,
        height: 0.5,
    });
    assert!(root.apply_selected_crop());
    let transformed_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("transformed request");

    assert_ne!(transformed_request.key, initial_request.key);
    assert_eq!(
        transformed_request.presentation,
        PreviewRenderPresentation::default()
    );
    assert_eq!(transformed_request.config.conversion_config.rotation, "90");
    assert!(transformed_request.config.conversion_config.flip_horizontal);
    assert!(
        transformed_request
            .config
            .conversion_config
            .crop
            .as_ref()
            .is_some_and(|crop| crop.enabled)
    );
}

#[test]
fn preview_runtime_key_reconfigures_only_same_source_identity() {
    let current = PreviewRuntimeKey {
        file_id: "video".to_string(),
        path: "/tmp/one.mp4".to_string(),
        source_kind: EnginePreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        duration_millis: 12_500,
        preview_dimensions: PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720,
        },
        visual_hash: 1,
        audio_hash: 1,
    };
    let mut next = current.clone();
    next.visual_hash = 2;
    next.audio_hash = 3;

    assert!(current.can_reconfigure_to(&next));

    next.path = "/tmp/two.mp4".to_string();
    assert!(!current.can_reconfigure_to(&next));
}

#[test]
fn preview_runtime_key_changes_when_adaptive_dimensions_change() {
    let current = PreviewRuntimeKey {
        file_id: "video".to_string(),
        path: "/tmp/one.mp4".to_string(),
        source_kind: EnginePreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        duration_millis: 12_500,
        preview_dimensions: PreviewRuntimeDimensions {
            max_width: 960,
            max_height: 540,
        },
        visual_hash: 1,
        audio_hash: 1,
    };
    let mut next = current.clone();
    next.preview_dimensions = PreviewRuntimeDimensions {
        max_width: 1280,
        max_height: 720,
    };

    assert_ne!(current, next);
    assert!(current.can_reconfigure_to(&next));
}
