use super::*;
use crate::settings::{CropSettings, OverlaySettings};

#[test]
fn preview_request_hash_tracks_only_effective_high_depth_overlay_policy() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "overlay", "/tmp/dynamic.mov");
    let entry = root.source_metadata.entry_for("overlay");
    root.update_selected_config(|config| {
        crate::settings::apply_output_container(config, "png");
        config.overlay = Some(OverlaySettings {
            enabled: true,
            path: "/tmp/logo.png".into(),
            x: 0.5,
            y: 0.5,
            width: 0.25,
            opacity: 0.145,
            anchor: "center".into(),
        });
        true
    });
    let ordinary = root.selected_preview_runtime_request(&entry).unwrap();
    root.update_selected_config(|config| {
        config.pixel_format = "rgba64be".into();
        true
    });
    let precise = root.selected_preview_runtime_request(&entry).unwrap();
    assert_ne!(ordinary.key.visual_hash, precise.key.visual_hash);
    root.update_selected_config(|config| {
        config.pixel_format = "rgb48be".into();
        true
    });
    let same_policy = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(precise.key.visual_hash, same_policy.key.visual_hash);
    root.update_selected_config(|config| {
        config.overlay.as_mut().unwrap().opacity = 0.0;
        true
    });
    let zero = root.selected_preview_runtime_request(&entry).unwrap();
    root.update_selected_config(|config| {
        config.pixel_format = "auto".into();
        true
    });
    let zero_auto = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(zero.key.visual_hash, zero_auto.key.visual_hash);
    root.update_selected_config(|config| {
        config.overlay = None;
        true
    });
    let absent = root.selected_preview_runtime_request(&entry).unwrap();
    root.update_selected_config(|config| {
        config.pixel_format = "rgba64be".into();
        true
    });
    let absent_precise = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(absent.key.visual_hash, absent_precise.key.visual_hash);
}

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
        edit_crop: false,
        edit_overlay: false,
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
        edit_crop: false,
        edit_overlay: false,
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

#[test]
fn preview_resolves_video_only_timing_and_hashes_only_an_effective_burn_clock() {
    use frame_core::source_timing::{ResolvedSourceClock, TimeBase};
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "clock", "/tmp/source.m2ts");
    let mut entry = root.source_metadata.entry_for("clock");
    let probe = frame_core::probe::parse_ffprobe_stdout("source.m2ts", serde_json::json!({
        "format":{"format_name":"mpegts","start_time":"1.400000","duration":"10"},
        "streams":[
            {"index":4,"codec_type":"video","codec_name":"h264","width":320,"height":180,"time_base":"1/90000","start_pts":145_920},
            {"index":7,"codec_type":"audio","codec_name":"aac","channels":2,"time_base":"1/90000","start_pts":126_000}
        ]
    }).to_string()).unwrap();
    entry.metadata = Some(crate::source_metadata::source_metadata_from_probe(probe));
    let video_only = ResolvedSourceClock::Known {
        video_time_base: TimeBase::new(1, 90_000).unwrap(),
        origin_microseconds: 1_400_000,
        minimum_start_microseconds: Some(1_621_333),
    };
    root.update_selected_config(|config| {
        config.selected_audio_tracks = vec![7];
        true
    });
    let without_burn = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(without_burn.config.source_clock, video_only);
    entry
        .metadata
        .as_mut()
        .unwrap()
        .source_timing
        .as_mut()
        .unwrap()
        .format_start_microseconds = Some(0);
    let changed_unused = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(without_burn.key.visual_hash, changed_unused.key.visual_hash);
    root.update_selected_config(|config| {
        config.subtitle_burn_path = Some("/tmp/sub.srt".into());
        true
    });
    let burn_zero_origin = root.selected_preview_runtime_request(&entry).unwrap();
    entry
        .metadata
        .as_mut()
        .unwrap()
        .source_timing
        .as_mut()
        .unwrap()
        .format_start_microseconds = Some(1_400_000);
    let burn_original = root.selected_preview_runtime_request(&entry).unwrap();
    assert_ne!(
        burn_zero_origin.key.visual_hash,
        burn_original.key.visual_hash
    );
    assert_eq!(burn_original.config.source_clock, video_only);
    root.update_selected_config(|config| {
        config.processing_mode = crate::settings::ProcessingMode::Copy;
        true
    });
    let copy = root.selected_preview_runtime_request(&entry).unwrap();
    entry.metadata.as_mut().unwrap().source_timing = None;
    let copy_unknown = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(copy.key.visual_hash, copy_unknown.key.visual_hash);
}

#[test]
fn preview_key_retains_canonical_unknown_distinction_and_edit_intent() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "ratio", "/tmp/ratio.mkv");
    let mut entry = root.source_metadata.entry_for("ratio");
    let unknown = root.selected_preview_runtime_request(&entry).unwrap();
    entry.metadata.as_mut().unwrap().sample_aspect_ratio =
        frame_core::aspect::SampleAspectRatio::new(1, 1);
    let square = root.selected_preview_runtime_request(&entry).unwrap();
    assert_ne!(unknown.key.visual_hash, square.key.visual_hash);
    entry.metadata.as_mut().unwrap().sample_aspect_ratio =
        frame_core::aspect::SampleAspectRatio::new(2, 2);
    let reduced = root.selected_preview_runtime_request(&entry).unwrap();
    assert_eq!(square.key, reduced.key);
    assert_eq!(
        square.config.source_sample_aspect_ratio,
        entry.metadata.as_ref().unwrap().sample_aspect_ratio
    );
    root.preview_ui.crop_mode = true;
    let editing = root.selected_preview_runtime_request(&entry).unwrap();
    assert_ne!(editing.key, square.key);
    assert!(editing.key.edit_crop);
    assert!(editing.config.conversion_config.crop.is_none());
    assert!(square.config.conversion_config.crop.is_none());
}

#[test]
fn preview_request_retains_small_source_facts_without_sanitizing_upper_bounds() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "tiny", "/tmp/tiny.mkv");
    let saved = root.file_queue.selected_file().unwrap().config.clone();
    let mut entry = root.source_metadata.entry_for("tiny");
    for (width, height) in [
        (Some(1), Some(1)),
        (Some(400), Some(4)),
        (Some(4), Some(400)),
        (Some(15), Some(100)),
        (Some(30_720), Some(1)),
        (Some(30_721), Some(16)),
        (Some(16), Some(u32::MAX)),
        (Some(0), Some(100)),
        (Some(100), Some(0)),
        (Some(1), None),
        (None, Some(1)),
        (None, None),
    ] {
        let metadata = entry.metadata.as_mut().unwrap();
        metadata.width = width;
        metadata.height = height;
        metadata.sample_aspect_ratio = frame_core::aspect::SampleAspectRatio::new(2, 1);
        let request = root.selected_preview_runtime_request(&entry).unwrap();
        let expected = if width.is_some_and(|w| w > 0) && height.is_some_and(|h| h > 0) {
            (width, height)
        } else {
            (None, None)
        };
        assert_eq!(
            (request.config.source_width, request.config.source_height),
            expected
        );
        assert_eq!(
            (request.key.source_width, request.key.source_height),
            expected
        );
        assert_eq!(
            request.config.source_sample_aspect_ratio,
            frame_core::aspect::SampleAspectRatio::new(2, 1)
        );
        let oversized =
            expected.0.is_some_and(|w| w > 30_720) || expected.1.is_some_and(|h| h > 30_720);
        assert_eq!(
            request.config.validate().is_err(),
            oversized,
            "{width:?}x{height:?}"
        );
        assert_eq!(root.file_queue.selected_file().unwrap().config, saved);
    }
}

#[test]
fn tiny_source_edit_requests_keep_full_source_facts_and_saved_geometry() {
    let mut root = FrameRoot::new();
    let mut file = FileItem::from_path("tiny-edit", "/tmp/tiny.mkv", 1);
    file.config.crop = Some(CropSettings {
        enabled: true,
        x: 1,
        y: 1,
        width: 200,
        height: 3,
        ..CropSettings::default()
    });
    file.config.overlay = Some(OverlaySettings {
        enabled: true,
        path: "/tmp/logo.png".into(),
        x: 0.5,
        y: 0.5,
        width: 0.05,
        opacity: 1.0,
        anchor: "center".into(),
    });
    let saved = file.config.clone();
    root.file_queue.add_file(file);
    root.source_metadata.mark_ready(
        "tiny-edit",
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(400),
            height: Some(4),
            sample_aspect_ratio: frame_core::aspect::SampleAspectRatio::new(2, 1),
            ..SourceMetadata::default()
        },
    );
    let overlay = saved.overlay.as_ref().unwrap();
    root.preview_ui
        .overlay
        .sync_initial_overlay(Some(&crate::preview::PreviewOverlay {
            enabled: overlay.enabled,
            path: overlay.path.clone(),
            x: overlay.x,
            y: overlay.y,
            width: overlay.width,
            opacity: overlay.opacity,
            anchor: overlay.anchor.clone(),
        }));
    let entry = root.source_metadata.entry_for("tiny-edit");
    for editing in [false, true] {
        root.preview_ui.crop_mode = editing;
        root.preview_ui.overlay.set_overlay_mode(editing, false);
        let request = root.selected_preview_runtime_request(&entry).unwrap();
        assert_eq!(
            (request.config.source_width, request.config.source_height),
            (Some(400), Some(4))
        );
        assert_eq!(
            (request.key.source_width, request.key.source_height),
            (Some(400), Some(4))
        );
        assert_eq!(
            (request.key.edit_crop, request.key.edit_overlay),
            (editing, editing)
        );
        assert_eq!(request.config.conversion_config.crop.is_none(), editing);
        assert_eq!(request.config.conversion_config.overlay.is_none(), editing);
        assert_eq!(root.file_queue.selected_file().unwrap().config, saved);
    }
}
