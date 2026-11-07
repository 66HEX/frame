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
