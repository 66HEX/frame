use super::*;

#[test]
fn max_concurrency_defaults_to_shared_backend_limit() {
    let root = FrameRoot::new();

    assert_eq!(root.max_concurrency, DEFAULT_MAX_CONCURRENCY);
    assert_eq!(
        root.conversion_processes
            .current_max_concurrency()
            .expect("max concurrency should be readable"),
        DEFAULT_MAX_CONCURRENCY
    );
}

#[test]
fn new_with_persistence_hydrates_max_concurrency_and_custom_presets() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    persistence
        .save(&AppSettings {
            max_concurrency: 6,
            custom_presets: vec![PresetDefinition::custom(
                "custom-preset-7".to_string(),
                "Review MP4".to_string(),
                ConversionConfig::default(),
            )],
            ..AppSettings::default()
        })
        .expect("settings should be saved");

    let root = FrameRoot::new_with_persistence(persistence);

    assert_eq!(root.max_concurrency, 6);
    assert_eq!(
        root.conversion_processes
            .current_max_concurrency()
            .expect("max concurrency should be readable"),
        6
    );
    assert!(
        root.presets
            .iter()
            .any(|preset| preset.name == "Review MP4")
    );
    assert_eq!(root.settings_ui.next_custom_preset_sequence, 7);
}

#[test]
fn new_with_persistence_hydrates_appearance_before_rendering() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    persistence
        .save(&AppSettings {
            appearance: AppearanceSettings {
                ui_scale: ScalePreset::Percent175,
                color_theme: crate::appearance::ColorTheme::Light,
            },
            ..AppSettings::default()
        })
        .expect("settings should be saved");

    let root = FrameRoot::new_with_persistence(persistence);

    assert_eq!(
        root.appearance,
        AppearanceSettings {
            ui_scale: ScalePreset::Percent175,
            color_theme: crate::appearance::ColorTheme::Light,
        }
    );
}

#[test]
fn default_output_directory_persists_for_future_sessions() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    let mut root = FrameRoot::new_with_persistence(persistence.clone());

    root.set_default_output_directory(PathBuf::from("/tmp/frame-output"))
        .expect("output directory should persist");

    assert_eq!(
        persistence
            .load()
            .expect("settings should be readable")
            .default_output_directory,
        Some(PathBuf::from("/tmp/frame-output"))
    );
    assert_eq!(
        FrameRoot::new_with_persistence(persistence).default_output_directory,
        Some(PathBuf::from("/tmp/frame-output"))
    );
}

#[test]
fn output_directory_save_failure_opens_settings_with_recovery_message() {
    let blocker = test_settings_path();
    std::fs::create_dir_all(blocker.parent().expect("test path should have a parent"))
        .expect("test settings parent should be created");
    std::fs::write(&blocker, b"not a directory").expect("blocker file should be created");
    let persistence = AppPersistence::from_settings_path(blocker.join("settings.json"));
    let mut root = FrameRoot::new_with_persistence(persistence);

    assert!(!root.apply_default_output_directory(PathBuf::from("/tmp/frame-output")));

    assert!(root.settings_ui.is_open);
    assert!(root.default_output_directory.is_none());
    assert!(
        root.settings_ui
            .output_directory_error
            .as_deref()
            .is_some_and(|error| error.starts_with("Failed to save settings:"))
    );
}

#[test]
fn apply_max_concurrency_draft_updates_live_controller_limit() {
    let mut root = FrameRoot::new();
    root.settings_ui.max_concurrency_draft = "4".to_string();

    assert!(root.apply_max_concurrency_draft());

    assert_eq!(root.max_concurrency, 4);
    assert_eq!(
        root.conversion_processes
            .current_max_concurrency()
            .expect("max concurrency should be readable"),
        4
    );
}

#[test]
fn apply_max_concurrency_draft_persists_updated_limit() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    let mut root = FrameRoot::new_with_persistence(persistence.clone());
    root.settings_ui.max_concurrency_draft = "5".to_string();

    assert!(root.apply_max_concurrency_draft());

    assert_eq!(
        persistence
            .load()
            .expect("settings should be readable")
            .max_concurrency,
        5
    );
}

#[test]
fn apply_max_concurrency_draft_rejects_zero() {
    let mut root = FrameRoot::new();
    root.settings_ui.max_concurrency_draft = "0".to_string();

    assert!(!root.apply_max_concurrency_draft());

    assert_eq!(root.max_concurrency, DEFAULT_MAX_CONCURRENCY);
    assert!(root.settings_ui.max_concurrency_error.is_some());
}

#[test]
fn max_concurrency_apply_updates_live_controller_limit() {
    let mut root = FrameRoot::new();
    root.settings_ui.max_concurrency_draft = "4".to_string();

    assert!(root.apply_max_concurrency_draft());

    assert_eq!(root.max_concurrency, 4);
    assert_eq!(
        root.conversion_processes
            .current_max_concurrency()
            .expect("max concurrency should be readable"),
        4
    );
}
