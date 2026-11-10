use super::*;
use frame_updater::{PlatformAssetKey, UpdateAssetKind, UpdateChannel, UpdatePackage};
use semver::Version;
use std::path::PathBuf;

#[test]
fn current_titlebar_platform_matches_compile_target() {
    let expected = if cfg!(target_os = "macos") {
        FrameTitlebarPlatform::Macos
    } else if cfg!(target_os = "windows") {
        FrameTitlebarPlatform::Windows
    } else {
        FrameTitlebarPlatform::Linux
    };

    assert_eq!(FrameTitlebarPlatform::current(), expected);
}

#[test]
fn titlebar_view_for_key_switches_between_main_tabs() {
    assert_eq!(
        titlebar_view_for_key(ActiveView::Workspace, "right"),
        Some(ActiveView::Logs)
    );
    assert_eq!(
        titlebar_view_for_key(ActiveView::Logs, "left"),
        Some(ActiveView::Workspace)
    );
    assert_eq!(
        titlebar_view_for_key(ActiveView::Logs, "home"),
        Some(ActiveView::Workspace)
    );
    assert_eq!(titlebar_view_for_key(ActiveView::Logs, "space"), None);
}

#[test]
fn titlebar_workspace_controls_are_hidden_without_files() {
    assert!(!titlebar_shows_workspace_controls(FrameAppState::default()));
}

#[test]
fn titlebar_workspace_controls_are_visible_with_files() {
    let state = FrameAppState {
        file_count: 1,
        ..FrameAppState::default()
    };

    assert!(titlebar_shows_workspace_controls(state));
}

#[test]
fn release_note_emphasis_strips_markers_and_highlights_range() {
    let palette = theme::palette(crate::appearance::ColorTheme::Dark);
    let (text, highlights) =
        parse_update_release_note_emphasis("• **Native GPUI Application:** Rebuilt Frame", palette);

    assert_eq!(text, "• Native GPUI Application: Rebuilt Frame");
    assert_eq!(highlights.len(), 1);
    assert_eq!(&text[highlights[0].0.clone()], "Native GPUI Application:");
    assert_eq!(
        highlights[0].1.color,
        Some(gpui::rgb_to_hsla(color(palette.text_primary)))
    );
    assert_eq!(highlights[0].1.font_weight, Some(theme::TEXT_WEIGHT_MEDIUM));
}

#[test]
fn release_note_emphasis_keeps_unclosed_markers_literal() {
    let (text, highlights) = parse_update_release_note_emphasis(
        "• **Native GPUI Application: Rebuilt Frame",
        theme::palette(crate::appearance::ColorTheme::Dark),
    );

    assert_eq!(text, "• **Native GPUI Application: Rebuilt Frame");
    assert!(highlights.is_empty());
}

#[test]
fn ready_update_explains_why_install_is_blocked() {
    let status = UpdateStatus::ReadyToInstall(Box::new(test_update_package()));

    assert_eq!(
        update_status_text(&status, false),
        UPDATE_INSTALL_WAIT_MESSAGE
    );
    assert_eq!(
        update_dialog_summary(&status, false, false).as_deref(),
        Some(UPDATE_INSTALL_WAIT_MESSAGE)
    );
}

fn test_update_package() -> UpdatePackage {
    UpdatePackage {
        version: Version::new(0, 32, 0),
        channel: UpdateChannel::Stable,
        asset_key: PlatformAssetKey::MacosAarch64,
        kind: UpdateAssetKind::MacosAppZip,
        file_name: "Frame.zip".to_string(),
        path: PathBuf::from("/tmp/Frame.zip"),
        size_bytes: 1,
        sha256: "00".repeat(32),
        installer_args: Vec::new(),
    }
}
