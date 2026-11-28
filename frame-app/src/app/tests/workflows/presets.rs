use super::*;

#[test]
fn save_preset_from_draft_adds_custom_preset() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.settings_ui.preset_name_draft = "Review MP4".to_string();

    assert!(root.save_preset_from_draft());

    assert!(
        root.presets
            .iter()
            .any(|preset| preset.name == "Review MP4")
    );
    assert!(root.settings_ui.preset_name_draft.is_empty());
}

#[test]
fn save_preset_from_draft_persists_custom_preset_with_unique_id() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    persistence
        .save(&AppSettings {
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            custom_presets: vec![PresetDefinition::custom(
                "custom-preset-3".to_string(),
                "Existing".to_string(),
                ConversionConfig::default(),
            )],
            ..AppSettings::default()
        })
        .expect("settings should be saved");
    let mut root = FrameRoot::new_with_persistence(persistence.clone());
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.settings_ui.preset_name_draft = "Review MP4".to_string();

    assert!(root.save_preset_from_draft());

    let settings = persistence.load().expect("settings should be readable");
    assert!(
        settings
            .custom_presets
            .iter()
            .any(|preset| preset.id == "custom-preset-4" && preset.name == "Review MP4")
    );
}

#[test]
fn delete_preset_persists_removed_custom_preset() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    persistence
        .save(&AppSettings {
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            custom_presets: vec![PresetDefinition::custom(
                "custom-preset-1".to_string(),
                "Review MP4".to_string(),
                ConversionConfig::default(),
            )],
            ..AppSettings::default()
        })
        .expect("settings should be saved");
    let mut root = FrameRoot::new_with_persistence(persistence.clone());

    assert!(root.delete_preset("custom-preset-1"));

    assert!(
        persistence
            .load()
            .expect("settings should be readable")
            .custom_presets
            .is_empty()
    );
}

#[test]
fn opus_presets_save_reload_and_apply_explicit_vbr_values() {
    for opus_vbr in [false, true] {
        let persistence = AppPersistence::from_settings_path(test_settings_path());
        let mut root = FrameRoot::new_with_persistence(persistence.clone());
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/input.flac", 1));
        root.update_selected_config(|config| {
            config.container = "opus".to_string();
            config.audio_codec = "libopus".to_string();
            config.opus_vbr = opus_vbr;
            config.audio_bitrate = "64".to_string();
            true
        });
        root.settings_ui.preset_name_draft = "Opus voice".to_string();
        assert!(root.save_preset_from_draft());
        let saved = persistence.load().unwrap();
        let preset = saved
            .custom_presets
            .iter()
            .find(|preset| preset.name == "Opus voice")
            .unwrap();
        assert_eq!(preset.config.opus_vbr, opus_vbr);
        let mut restored = FrameRoot::new_with_persistence(persistence);
        restored
            .file_queue
            .add_file(FileItem::from_path("next", "/tmp/next.flac", 2));
        restored.source_metadata.mark_ready(
            "next",
            crate::settings::SourceMetadata {
                media_kind: Some(crate::settings::SourceKind::Audio),
                ..crate::settings::SourceMetadata::default()
            },
        );
        restored.update_selected_config(|config| {
            config.opus_vbr = !opus_vbr;
            true
        });
        assert!(restored.apply_preset_to_selected(&preset.id));
        let config = restored.selected_config().unwrap();
        assert_eq!(config.container, "opus");
        assert_eq!(config.audio_codec, "libopus");
        assert_eq!(config.audio_bitrate, "64");
        assert_eq!(config.opus_vbr, opus_vbr);
    }
}

#[test]
fn legacy_custom_opus_preset_defaults_to_vbr_on_when_applied() {
    let mut json = serde_json::to_value(PresetDefinition::custom(
        "legacy-opus".to_string(),
        "Legacy Opus".to_string(),
        ConversionConfig {
            container: "webm".to_string(),
            audio_codec: "libopus".to_string(),
            ..ConversionConfig::default()
        },
    ))
    .unwrap();
    json["config"].as_object_mut().unwrap().remove("opusVbr");
    let preset: PresetDefinition = serde_json::from_value(json).unwrap();
    assert!(preset.config.opus_vbr);
    let mut config = ConversionConfig {
        opus_vbr: false,
        ..ConversionConfig::default()
    };
    assert!(crate::settings::apply_preset(&mut config, &preset, None));
    assert!(config.opus_vbr);
}

#[test]
fn preset_batch_skips_files_without_ready_metadata() {
    let mut root = FrameRoot::new();
    for id in ["missing", "loading", "failed"] {
        root.file_queue
            .add_file(FileItem::from_path(id, format!("/tmp/{id}.mp4"), 1));
    }
    root.source_metadata.mark_loading("loading");
    root.source_metadata.mark_error("failed", "probe failed");
    let preset = root
        .presets
        .iter()
        .find(|preset| preset.id == "archive-hq")
        .unwrap()
        .clone();
    let before = root.file_queue.files().to_vec();
    assert!(!root.apply_preset_to_all_pending(&preset));
    assert_eq!(root.file_queue.files(), before);
}

#[test]
fn preset_lineage_is_bound_to_file_identity_and_pruned_with_removed_files() {
    let mut root = FrameRoot::new();
    for id in ["first", "second"] {
        root.file_queue
            .add_file(FileItem::from_path(id, format!("/tmp/{id}.mp4"), 1));
        root.file_queue.select_existing_file(id);
        root.source_metadata.mark_ready(
            id,
            crate::settings::SourceMetadata {
                media_kind: Some(crate::settings::SourceKind::Video),
                video_codec: Some("h264".into()),
                ..crate::settings::SourceMetadata::default()
            },
        );
        assert!(root.apply_preset_to_selected("balanced-mp4"));
    }
    root.file_queue.file_by_id_mut("second").unwrap().path = "/tmp/replacement.mp4".into();
    assert_eq!(root.selected_applied_preset_id(), None);
    root.file_queue.remove_file("first");
    root.sync_inspector_availability(None);
    assert!(root.settings_ui.applied_presets.is_empty());
}

#[test]
fn current_file_preset_requires_ready_metadata_without_using_stale_probe_data() {
    for state in ["missing", "loading", "error"] {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("source", "/tmp/source.mp4", 1));
        if state != "missing" {
            root.source_metadata.mark_ready(
                "source",
                crate::settings::SourceMetadata {
                    media_kind: Some(crate::settings::SourceKind::Video),
                    video_codec: Some("h264".into()),
                    ..crate::settings::SourceMetadata::default()
                },
            );
            if state == "loading" {
                root.source_metadata.mark_loading("source");
            } else {
                root.source_metadata.mark_error("source", "probe failed");
            }
        }
        let before = root.file_queue.files().to_vec();
        assert!(!root.apply_preset_to_selected("archive-hq"), "{state}");
        assert_eq!(root.file_queue.files(), before, "{state}");
        assert!(root.settings_ui.applied_presets.is_empty());
    }
}

mod export;
