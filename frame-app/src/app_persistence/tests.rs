use super::*;
use crate::settings::{ConversionConfig, PresetDefinition};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn load_returns_defaults_when_settings_file_is_missing() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());

    let settings = persistence
        .load()
        .expect("missing settings should load as defaults");

    assert_eq!(settings, AppSettings::default());
}

#[test]
fn load_error_identifies_the_read_operation_and_path() {
    let path = test_settings_path();
    fs::create_dir_all(&path).expect("settings path fixture should be a directory");

    let error = AppPersistence::from_settings_path(&path)
        .load()
        .expect_err("reading a directory as settings should fail")
        .to_string();

    assert!(error.starts_with("failed to read app settings at "));
    assert!(error.contains(path.to_string_lossy().as_ref()));
}

#[test]
fn legacy_load_error_identifies_the_file_that_failed() {
    let path = test_settings_path();
    let legacy_path = path.with_file_name(LEGACY_APP_SETTINGS_FILE_NAME);
    fs::create_dir_all(&legacy_path).expect("legacy path fixture should be a directory");

    let error = AppPersistence::from_settings_path(path)
        .load()
        .expect_err("reading a directory as legacy settings should fail")
        .to_string();

    assert!(error.starts_with("failed to read legacy app settings at "));
    assert!(error.contains(legacy_path.to_string_lossy().as_ref()));
}

#[test]
fn save_error_identifies_a_blocked_settings_directory() {
    let path = test_settings_path();
    let parent = path
        .parent()
        .expect("test path should have parent")
        .to_path_buf();
    fs::create_dir_all(parent.parent().expect("test path should have grandparent"))
        .expect("test root should be created");
    fs::write(&parent, b"not a directory").expect("directory blocker should be written");

    let error = AppPersistence::from_settings_path(path)
        .save(&AppSettings::default())
        .expect_err("a blocked settings directory should fail")
        .to_string();

    assert!(error.starts_with("failed to create the settings directory at "));
    assert!(error.contains(parent.to_string_lossy().as_ref()));
}

#[test]
fn save_error_identifies_a_blocked_temporary_file() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    let temp_path = temp_path_for(&path);
    fs::create_dir(&temp_path).expect("temporary file blocker should be created");

    let error = AppPersistence::from_settings_path(path)
        .save(&AppSettings::default())
        .expect_err("a blocked temporary file should fail")
        .to_string();

    assert!(error.starts_with("failed to create the temporary settings file at "));
    assert!(error.contains(temp_path.to_string_lossy().as_ref()));
}

#[test]
fn write_error_identifies_the_temporary_file() {
    let path = temp_path_for(&test_settings_path());
    let error = AppPersistenceError::io(
        AppPersistenceOperation::WriteTemporaryFile,
        &path,
        io::Error::new(io::ErrorKind::WriteZero, "test write failure"),
    )
    .to_string();

    assert!(error.starts_with("failed to write the temporary settings file at "));
    assert!(error.contains(path.to_string_lossy().as_ref()));
    assert!(error.ends_with(": test write failure"));
}

#[test]
fn replacement_failure_keeps_the_existing_target_and_identifies_it() {
    let path = test_settings_path();
    fs::create_dir_all(&path).expect("settings target fixture should be a directory");
    let marker_path = path.join("keep-me");
    fs::write(&marker_path, b"existing data").expect("target marker should be written");

    let error = AppPersistence::from_settings_path(&path)
        .save(&AppSettings::default())
        .expect_err("replacing a directory should fail")
        .to_string();

    assert!(error.starts_with("failed to replace the settings file at "));
    assert!(error.contains(path.to_string_lossy().as_ref()));
    assert_eq!(
        fs::read(marker_path).expect("existing target should remain untouched"),
        b"existing data"
    );
}

#[test]
fn paths_inside_the_home_directory_do_not_expose_the_home_path() {
    let home = BaseDirs::new().expect("home directory should be available");
    let path = home.home_dir().join("Library/Frame/settings.json");

    let displayed = redacted_path(&path);

    assert_eq!(
        displayed,
        Path::new("$HOME")
            .join("Library/Frame/settings.json")
            .display()
            .to_string()
    );
    assert!(!displayed.contains(home.home_dir().to_string_lossy().as_ref()));
}

#[test]
fn save_round_trips_max_concurrency_and_custom_presets() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    let settings = AppSettings {
        appearance: AppearanceSettings {
            ui_scale: ScalePreset::Percent125,
            color_theme: ColorTheme::Light,
        },
        max_concurrency: 4,
        default_output_directory: Some(PathBuf::from("/tmp/frame-output")),
        custom_presets: vec![PresetDefinition::custom(
            "custom-preset-1".to_string(),
            "Review MP4".to_string(),
            ConversionConfig {
                video_bitrate: "9000".to_string(),
                external_subtitle_tracks: vec![crate::settings::ExternalSubtitleTrack {
                    path: "/tmp/english.srt".to_string(),
                    language: Some("eng".to_string()),
                    title: Some("English".to_string()),
                    is_default: true,
                    is_forced: false,
                }],
                ..ConversionConfig::default()
            },
        )],
        auto_update_check: false,
        update_channel: UpdateChannel::Stable,
        skipped_update_version: Some("0.2.0".to_string()),
        last_update_check_at: Some(1_800_000_000),
    };

    persistence
        .save(&settings)
        .expect("settings should be saved");
    let loaded = persistence.load().expect("settings should be loaded");

    assert_eq!(loaded, settings);
}

#[test]
fn every_supported_ui_scale_round_trips() {
    for ui_scale in ScalePreset::ALL {
        let settings = AppSettings {
            appearance: AppearanceSettings {
                ui_scale,
                color_theme: ColorTheme::Dark,
            },
            ..AppSettings::default()
        };

        let loaded = PersistedAppSettings::from_app_settings(&settings).into_app_settings();

        assert_eq!(loaded.appearance, settings.appearance);
    }
}

#[test]
fn load_accepts_settings_without_default_output_directory() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
            &path,
            r#"{"version":2,"maxConcurrency":4,"customPresets":[],"autoUpdateCheck":true,"updateChannel":"stable","skippedUpdateVersion":null,"lastUpdateCheckAt":null}"#,
        )
        .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("settings should load");

    assert_eq!(settings.default_output_directory, None);
}

#[test]
fn load_replaces_zero_max_concurrency_with_default() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        &path,
        r#"{"version":1,"maxConcurrency":0,"customPresets":[]}"#,
    )
    .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("settings should load");

    assert_eq!(settings.max_concurrency, DEFAULT_MAX_CONCURRENCY);
}

#[test]
fn load_version_three_settings_defaults_appearance_without_losing_existing_values() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        &path,
        r#"{"version":3,"maxConcurrency":7,"customPresets":[],"autoUpdateCheck":false}"#,
    )
    .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("version three settings should load");

    assert_eq!(settings.appearance, AppearanceSettings::default());
    assert_eq!(settings.max_concurrency, 7);
    assert!(!settings.auto_update_check);
}

#[test]
fn load_normalizes_invalid_ui_scale_without_resetting_other_settings() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        &path,
        r#"{"version":4,"uiScalePercent":0,"maxConcurrency":5,"customPresets":[]}"#,
    )
    .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("settings should load");

    assert_eq!(settings.appearance.ui_scale, ScalePreset::Percent100);
    assert_eq!(settings.max_concurrency, 5);
}

#[test]
fn load_version_four_settings_defaults_theme_without_losing_existing_values() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        &path,
        r#"{"version":4,"uiScalePercent":125,"maxConcurrency":7,"customPresets":[]}"#,
    )
    .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("version four settings should load");

    assert_eq!(settings.appearance.color_theme, ColorTheme::Dark);
    assert_eq!(settings.appearance.ui_scale, ScalePreset::Percent125);
    assert_eq!(settings.max_concurrency, 7);
}

#[test]
fn every_supported_color_theme_round_trips() {
    for color_theme in ColorTheme::ALL {
        let settings = AppSettings {
            appearance: AppearanceSettings {
                ui_scale: ScalePreset::Percent100,
                color_theme,
            },
            ..AppSettings::default()
        };

        let persisted = PersistedAppSettings::from_app_settings(&settings);
        let json = serde_json::to_value(&persisted).expect("settings should serialize");
        assert_eq!(json["colorTheme"], color_theme.persisted());
        let loaded = persisted.into_app_settings();

        assert_eq!(loaded.appearance.color_theme, color_theme);
    }
}

#[test]
fn load_missing_or_null_color_theme_defaults_only_theme() {
    for color_field in ["", r#","colorTheme":null"#] {
        let path = test_settings_path();
        let parent = path.parent().expect("test path should have parent");
        fs::create_dir_all(parent).expect("test directory should be created");
        fs::write(
                &path,
                format!(
                    r#"{{"version":5,"uiScalePercent":175{color_field},"maxConcurrency":8,"customPresets":[]}}"#
                ),
            )
            .expect("settings fixture should be written");

        let settings = AppPersistence::from_settings_path(path)
            .load()
            .expect("settings should load");

        assert_eq!(settings.appearance.color_theme, ColorTheme::Dark);
        assert_eq!(settings.appearance.ui_scale, ScalePreset::Percent175);
        assert_eq!(settings.max_concurrency, 8);
    }
}

#[test]
fn load_unknown_color_theme_defaults_only_theme() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
            &path,
            r#"{"version":5,"uiScalePercent":150,"colorTheme":"future","maxConcurrency":6,"customPresets":[]}"#,
        )
        .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("settings should load");

    assert_eq!(settings.appearance.color_theme, ColorTheme::Dark);
    assert_eq!(settings.appearance.ui_scale, ScalePreset::Percent150);
    assert_eq!(settings.max_concurrency, 6);
}

#[test]
fn load_wrong_type_color_theme_defaults_only_theme() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
            &path,
            r#"{"version":5,"uiScalePercent":110,"colorTheme":42,"maxConcurrency":3,"customPresets":[]}"#,
        )
        .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("settings should load");

    assert_eq!(settings.appearance.color_theme, ColorTheme::Dark);
    assert_eq!(settings.appearance.ui_scale, ScalePreset::Percent110);
    assert_eq!(settings.max_concurrency, 3);
}

#[test]
fn load_reads_camel_case_presets_and_fills_missing_config_defaults() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        &path,
        r#"{
                "version": 1,
                "maxConcurrency": 3,
                "customPresets": [{
                    "id": "custom-preset-2",
                    "name": "Legacy",
                    "builtIn": true,
                    "config": {
                        "container": "webm",
                        "metadata": { "mode": "clean" }
                    }
                }]
            }"#,
    )
    .expect("settings fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("settings should load");

    assert_eq!(settings.max_concurrency, 3);
    assert_eq!(settings.custom_presets[0].config.container, "webm");
    assert_eq!(
        settings.custom_presets[0].config.metadata.mode,
        crate::settings::MetadataMode::Clean
    );
    assert!(!settings.custom_presets[0].built_in);
}

#[test]
fn load_falls_back_to_legacy_tauri_store_files_when_new_settings_are_missing() {
    let path = test_settings_path();
    let parent = path.parent().expect("test path should have parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        path.with_file_name(LEGACY_APP_SETTINGS_FILE_NAME),
        r#"{"maxConcurrency":5,"autoUpdateCheck":true}"#,
    )
    .expect("legacy app settings fixture should be written");
    fs::write(
        path.with_file_name(LEGACY_PRESETS_FILE_NAME),
        r#"{"presets":[{
                "id":"custom-preset-8",
                "name":"Legacy Review",
                "builtIn":false,
                "config":{"container":"mkv"}
            }]}"#,
    )
    .expect("legacy presets fixture should be written");

    let settings = AppPersistence::from_settings_path(path)
        .load()
        .expect("legacy settings should load");

    assert_eq!(settings.max_concurrency, 5);
    assert!(settings.auto_update_check);
    assert_eq!(settings.custom_presets[0].id, "custom-preset-8");
    assert_eq!(settings.custom_presets[0].config.container, "mkv");
}

#[test]
fn from_runtime_persists_only_custom_presets() {
    let settings = AppSettings::from_runtime(
        AppearanceSettings::default(),
        3,
        Some(PathBuf::from("/tmp/frame-output")),
        &[
            PresetDefinition::built_in("balanced-mp4", "Balanced MP4", ConversionConfig::default()),
            PresetDefinition::custom(
                " custom-preset-1 ".to_string(),
                " Review MP4 ".to_string(),
                ConversionConfig::default(),
            ),
        ],
        true,
        UpdateChannel::Stable,
        None,
        Some(1_800_000_000),
    );

    assert_eq!(settings.custom_presets.len(), 1);
    assert_eq!(settings.custom_presets[0].id, "custom-preset-1");
    assert_eq!(settings.custom_presets[0].name, "Review MP4");
    assert!(!settings.custom_presets[0].built_in);
}

fn test_settings_path() -> PathBuf {
    let sequence = TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();

    std::env::temp_dir()
        .join("frame-app-persistence-tests")
        .join(format!("{}-{millis}-{sequence}", std::process::id()))
        .join(SETTINGS_FILE_NAME)
}
