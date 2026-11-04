use super::*;

#[test]
fn update_dialog_close_keeps_dialog_present_until_motion_finishes() {
    let mut root = FrameRoot::new();

    assert!(root.open_update_dialog());
    assert!(root.close_update_dialog());

    assert!(!root.update_ui.dialog_open);
    assert!(root.update_ui.dialog_present);
    assert!(root.finish_update_dialog_close());
    assert!(!root.update_ui.dialog_present);
    assert!(!root.finish_update_dialog_close());
}

#[test]
fn update_dialog_open_is_stable_without_reopening() {
    let mut root = FrameRoot::new();

    assert!(root.open_update_dialog());
    assert!(!root.open_update_dialog());
    assert!(root.update_ui.dialog_open);
    assert!(root.update_ui.dialog_present);
}

#[test]
fn update_dialog_close_preserves_status_for_settings() {
    let mut root = FrameRoot::new();
    root.update_ui.status = UpdateStatus::UpToDate;

    root.open_update_dialog();
    root.close_update_dialog();

    assert!(matches!(root.update_ui.status, UpdateStatus::UpToDate));
}

#[test]
fn dismiss_update_status_closes_dialog() {
    let mut root = FrameRoot::new();
    root.update_ui.status = UpdateStatus::UpToDate;
    root.open_update_dialog();

    root.dismiss_update_status();

    assert!(matches!(root.update_ui.status, UpdateStatus::Idle));
    assert!(!root.update_ui.dialog_open);
}
