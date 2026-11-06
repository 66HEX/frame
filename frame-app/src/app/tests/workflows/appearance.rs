use super::*;

#[test]
fn app_settings_close_keeps_drawer_present_until_motion_finishes() {
    let mut root = FrameRoot::new();

    root.open_app_settings();
    root.close_app_settings();

    assert!(!root.settings_ui.is_open);
    assert!(root.settings_ui.is_present);

    assert!(root.finish_app_settings_close());
    assert!(!root.settings_ui.is_present);
}

#[test]
fn app_settings_drawer_motion_keeps_final_edge_inset() {
    assert_eq!(settings_drawer_right_inset(1.0), 8.0);
    assert!((settings_drawer_right_inset(0.0) + 30.4).abs() < 0.001);
}

#[test]
fn ui_scale_selection_persists() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    let mut root = FrameRoot::new_with_persistence(persistence.clone());

    assert!(root.set_ui_scale(ScalePreset::Percent125));

    assert_eq!(
        persistence
            .load()
            .expect("appearance settings should be readable")
            .appearance,
        AppearanceSettings {
            ui_scale: ScalePreset::Percent125,
            color_theme: crate::appearance::ColorTheme::Dark,
        }
    );
}

#[test]
fn color_theme_selection_persists_without_changing_ui_scale() {
    let persistence = AppPersistence::from_settings_path(test_settings_path());
    let mut root = FrameRoot::new_with_persistence(persistence.clone());
    root.appearance.ui_scale = ScalePreset::Percent150;

    assert!(root.set_color_theme(crate::appearance::ColorTheme::Light));

    assert_eq!(root.appearance.ui_scale, ScalePreset::Percent150);
    assert_eq!(
        persistence
            .load()
            .expect("appearance settings should be readable")
            .appearance,
        AppearanceSettings {
            ui_scale: ScalePreset::Percent150,
            color_theme: crate::appearance::ColorTheme::Light,
        }
    );
}

#[test]
fn ui_scale_selection_preserves_color_theme() {
    let mut root = FrameRoot::new();
    root.appearance.color_theme = crate::appearance::ColorTheme::Light;

    assert!(root.set_ui_scale(ScalePreset::Percent125));

    assert_eq!(
        root.appearance.color_theme,
        crate::appearance::ColorTheme::Light
    );
}

#[test]
fn selecting_active_theme_is_a_no_op_without_attempting_to_save() {
    let blocker = test_settings_path();
    std::fs::create_dir_all(blocker.parent().expect("test path should have a parent"))
        .expect("test settings parent should be created");
    std::fs::write(&blocker, b"not a directory").expect("blocker file should be created");
    let persistence = AppPersistence::from_settings_path(blocker.join("settings.json"));
    let mut root = FrameRoot::new_with_persistence(persistence);

    assert!(!root.set_color_theme(crate::appearance::ColorTheme::Dark));
    assert!(root.settings_ui.appearance_error.is_none());
}

#[test]
fn appearance_save_failure_rolls_runtime_state_back() {
    let blocker = test_settings_path();
    std::fs::create_dir_all(blocker.parent().expect("test path should have a parent"))
        .expect("test settings parent should be created");
    std::fs::write(&blocker, b"not a directory").expect("blocker file should be created");
    let persistence = AppPersistence::from_settings_path(blocker.join("settings.json"));
    let mut root = FrameRoot::new_with_persistence(persistence);

    assert!(!root.set_ui_scale(ScalePreset::Percent150));

    assert_eq!(root.appearance, AppearanceSettings::default());
    assert!(
        root.settings_ui
            .appearance_error
            .as_deref()
            .is_some_and(|error| error.starts_with("Failed to save settings:"))
    );
}

#[test]
fn theme_save_failure_rolls_runtime_state_back() {
    let blocker = test_settings_path();
    std::fs::create_dir_all(blocker.parent().expect("test path should have a parent"))
        .expect("test settings parent should be created");
    std::fs::write(&blocker, b"not a directory").expect("blocker file should be created");
    let persistence = AppPersistence::from_settings_path(blocker.join("settings.json"));
    let mut root = FrameRoot::new_with_persistence(persistence);

    assert!(!root.set_color_theme(crate::appearance::ColorTheme::Light));

    assert_eq!(root.appearance, AppearanceSettings::default());
    assert!(root.settings_ui.appearance_error.is_some());
}

#[test]
fn successful_appearance_save_clears_a_previous_error() {
    let blocker = test_settings_path();
    std::fs::create_dir_all(blocker.parent().expect("test path should have a parent"))
        .expect("test settings parent should be created");
    std::fs::write(&blocker, b"not a directory").expect("blocker file should be created");
    let mut root = FrameRoot::new_with_persistence(AppPersistence::from_settings_path(
        blocker.join("settings.json"),
    ));
    assert!(!root.set_color_theme(crate::appearance::ColorTheme::Light));

    root.persistence = Some(AppPersistence::from_settings_path(test_settings_path()));
    assert!(root.set_color_theme(crate::appearance::ColorTheme::Light));

    assert!(root.settings_ui.appearance_error.is_none());
}

#[test]
fn toggling_ui_scale_popover_opens_and_closes_it() {
    let mut root = FrameRoot::new();

    root.toggle_app_settings_appearance_popover(AppearancePopover::UiScale);

    assert_eq!(root.settings_ui.ui_scale_popover, PopoverState::Open);

    root.toggle_app_settings_appearance_popover(AppearancePopover::UiScale);

    assert_eq!(root.settings_ui.ui_scale_popover, PopoverState::Closing);
}

#[test]
fn opening_an_appearance_popover_closes_the_other_one() {
    let mut root = FrameRoot::new();

    root.open_app_settings_appearance_popover(AppearancePopover::Theme);
    assert!(root.settings_ui.theme_popover.is_open());
    assert!(!root.settings_ui.ui_scale_popover.is_rendered());

    root.open_app_settings_appearance_popover(AppearancePopover::UiScale);
    assert!(!root.settings_ui.theme_popover.is_rendered());
    assert!(root.settings_ui.ui_scale_popover.is_open());

    root.open_app_settings_appearance_popover(AppearancePopover::Theme);
    assert!(root.settings_ui.theme_popover.is_open());
    assert!(!root.settings_ui.ui_scale_popover.is_rendered());
}

#[test]
fn theme_popover_remains_rendered_until_close_motion_finishes() {
    let mut root = FrameRoot::new();
    root.open_app_settings_appearance_popover(AppearancePopover::Theme);

    root.close_app_settings_appearance_popover(AppearancePopover::Theme);

    assert_eq!(root.settings_ui.theme_popover, PopoverState::Closing);
    assert!(root.finish_app_settings_appearance_popover_close(AppearancePopover::Theme));
    assert_eq!(root.settings_ui.theme_popover, PopoverState::Hidden);
}

#[test]
fn appearance_popover_remains_rendered_until_close_motion_finishes() {
    let mut root = FrameRoot::new();
    root.toggle_app_settings_appearance_popover(AppearancePopover::UiScale);

    root.close_app_settings_appearance_popover(AppearancePopover::UiScale);

    assert_eq!(root.settings_ui.ui_scale_popover, PopoverState::Closing);
    assert!(root.finish_app_settings_appearance_popover_close(AppearancePopover::UiScale));
    assert_eq!(root.settings_ui.ui_scale_popover, PopoverState::Hidden);
}

#[test]
fn closing_app_settings_closes_the_active_appearance_popover() {
    let mut root = FrameRoot::new();
    root.open_app_settings();
    root.toggle_app_settings_appearance_popover(AppearancePopover::UiScale);

    root.close_app_settings();

    assert!(!root.settings_ui.ui_scale_popover.is_open());
    assert!(!root.settings_ui.is_open);
}

#[test]
fn ui_scale_step_uses_the_next_supported_preset() {
    let mut root = FrameRoot::new();
    assert!(root.set_ui_scale(ScalePreset::Percent175));

    assert!(root.set_ui_scale(root.appearance.ui_scale.next()));

    assert_eq!(root.appearance.ui_scale, ScalePreset::Percent200);
}

#[test]
fn ui_scale_steps_at_bounds_are_no_ops() {
    let mut root = FrameRoot::new();
    root.appearance.ui_scale = ScalePreset::Percent200;
    assert!(!root.set_ui_scale(root.appearance.ui_scale.next()));

    root.appearance.ui_scale = ScalePreset::Percent80;
    assert!(!root.set_ui_scale(root.appearance.ui_scale.previous()));
}

#[test]
fn appearance_changes_are_blocked_during_update_installation() {
    let mut root = FrameRoot::new();
    root.update_ui.status = UpdateStatus::Installing;

    assert!(!root.set_ui_scale(ScalePreset::Percent150));
    assert!(!root.set_color_theme(crate::appearance::ColorTheme::Light));
    assert_eq!(root.appearance, AppearanceSettings::default());
}
