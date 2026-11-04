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
