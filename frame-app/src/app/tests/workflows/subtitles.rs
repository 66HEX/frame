use super::*;

#[test]
fn subtitle_font_color_hex_input_expands_short_hex_and_updates_config() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.subtitle_ui.font_color_draft = "#".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::SubtitleFontColorHex)
        .selected_range = 1..1;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::SubtitleFontColorHex,
        None,
        "abc",
        None,
        false,
    ));

    assert_eq!(root.subtitle_ui.font_color_draft, "#AABBCC");
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.subtitle_font_color.as_deref()),
        Some("#aabbcc")
    );
}

#[test]
fn subtitle_outline_color_hex_input_keeps_incomplete_draft_without_committing() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.subtitle_ui.outline_color_draft = "#".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::SubtitleOutlineColorHex)
        .selected_range = 1..1;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::SubtitleOutlineColorHex,
        None,
        "f",
        None,
        false,
    ));

    assert_eq!(root.subtitle_ui.outline_color_draft, "#F");
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.subtitle_outline_color.as_deref()),
        None
    );
}

#[test]
fn subtitle_color_hsv_commit_updates_selected_config_and_draft() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));

    assert!(root.commit_subtitle_hsv_color(
        SettingsSubtitleColorTarget::Font,
        SettingsSubtitleHsv {
            h: 60.0,
            s: 1.0,
            v: 1.0,
        },
    ));

    assert_eq!(root.subtitle_ui.font_color_draft, "#FFFF00");
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.subtitle_font_color.as_deref()),
        Some("#ffff00")
    );
}

#[test]
fn subtitle_color_click_commits_from_picker_bounds() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.set_subtitle_color_picker_bounds(
        SettingsSubtitleColorTarget::Font,
        SettingsSubtitleColorDragKind::SaturationValue,
        Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(100.0))),
    );

    assert!(root.commit_subtitle_color_at_position(
        SettingsSubtitleColorTarget::Font,
        SettingsSubtitleColorDragKind::SaturationValue,
        point(px(10.0), px(20.0)),
    ));

    assert_eq!(root.subtitle_ui.font_color_draft, "#FFFFFF");
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.subtitle_font_color.as_deref()),
        Some("#ffffff")
    );
}

#[test]
fn subtitle_color_sv_drag_keeps_start_hue_after_white() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.set_subtitle_color_picker_bounds(
        SettingsSubtitleColorTarget::Font,
        SettingsSubtitleColorDragKind::SaturationValue,
        Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(100.0))),
    );
    let drag = SettingsSubtitleColorDrag {
        target: SettingsSubtitleColorTarget::Font,
        kind: SettingsSubtitleColorDragKind::SaturationValue,
        base_hsv: SettingsSubtitleHsv {
            h: 270.0,
            s: 1.0,
            v: 1.0,
        },
    };

    assert!(root.commit_subtitle_color_drag_at_position(drag, point(px(10.0), px(20.0))));
    assert_eq!(root.subtitle_ui.font_color_draft, "#FFFFFF");
    assert_eq!(
        root.subtitle_ui.font_color_hsv_draft,
        SettingsSubtitleHsv {
            h: 270.0,
            s: 0.0,
            v: 1.0,
        }
    );
    assert!(root.commit_subtitle_color_drag_at_position(drag, point(px(110.0), px(20.0))));

    assert_eq!(root.subtitle_ui.font_color_draft, "#8000FF");
    assert_eq!(root.subtitle_ui.font_color_hsv_draft.h, 270.0);
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.subtitle_font_color.as_deref()),
        Some("#8000ff")
    );
}

#[test]
fn subtitle_mode_switch_changes_only_the_visible_editor() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    let config = &mut root.file_queue.selected_file_mut().unwrap().config;
    config.subtitle_burn_path = Some("/tmp/burn.srt".to_string());
    config.external_subtitle_tracks = vec![crate::settings::ExternalSubtitleTrack {
        path: "/tmp/selectable.srt".to_string(),
        language: Some("eng".to_string()),
        ..crate::settings::ExternalSubtitleTrack::default()
    }];

    assert_eq!(root.subtitle_ui.mode, SettingsSubtitleMode::Selectable);
    assert!(!root.select_subtitle_mode(SettingsSubtitleMode::Selectable));
    assert!(root.select_subtitle_mode(SettingsSubtitleMode::BurnIn));
    root.toggle_subtitle_popover(SettingsSubtitlePopover::FontName);
    assert!(root.select_subtitle_mode(SettingsSubtitleMode::Selectable));

    let config = &root.file_queue.selected_file().unwrap().config;
    assert_eq!(config.subtitle_burn_path.as_deref(), Some("/tmp/burn.srt"));
    assert_eq!(config.external_subtitle_tracks.len(), 1);
    assert_eq!(root.subtitle_ui.popover, None);
    assert_eq!(root.subtitle_ui.rendered_popover, None);
}

#[test]
fn subtitle_popover_toggle_keeps_only_one_open_panel() {
    let mut root = FrameRoot::new();

    root.toggle_subtitle_popover(SettingsSubtitlePopover::FontName);
    assert_eq!(
        root.subtitle_ui.popover,
        Some(SettingsSubtitlePopover::FontName)
    );
    assert_eq!(
        root.subtitle_ui.rendered_popover,
        Some(SettingsSubtitlePopover::FontName)
    );

    root.toggle_subtitle_popover(SettingsSubtitlePopover::FontSize);
    assert_eq!(
        root.subtitle_ui.popover,
        Some(SettingsSubtitlePopover::FontSize)
    );
    assert_eq!(
        root.subtitle_ui.rendered_popover,
        Some(SettingsSubtitlePopover::FontSize)
    );

    root.toggle_subtitle_popover(SettingsSubtitlePopover::FontSize);
    assert_eq!(root.subtitle_ui.popover, None);
    assert_eq!(
        root.subtitle_ui.rendered_popover,
        Some(SettingsSubtitlePopover::FontSize)
    );
    assert!(root.finish_subtitle_popover_close(SettingsSubtitlePopover::FontSize));
    assert_eq!(root.subtitle_ui.rendered_popover, None);
}

#[test]
fn subtitle_color_popover_toggle_closes_active_picker() {
    let mut root = FrameRoot::new();

    root.toggle_subtitle_color_popover(
        SettingsSubtitlePopover::FontColor,
        SettingsSubtitleColorTarget::Font,
        "#ffd166",
    );
    assert_eq!(
        root.subtitle_ui.popover,
        Some(SettingsSubtitlePopover::FontColor)
    );
    assert_eq!(root.subtitle_ui.font_color_draft, "#FFD166");

    root.toggle_subtitle_color_popover(
        SettingsSubtitlePopover::FontColor,
        SettingsSubtitleColorTarget::Font,
        "#ffd166",
    );
    assert_eq!(root.subtitle_ui.popover, None);
    assert_eq!(
        root.subtitle_ui.rendered_popover,
        Some(SettingsSubtitlePopover::FontColor)
    );
}
