use super::*;

#[test]
fn max_concurrency_input_inserts_digits_at_selection() {
    let mut root = FrameRoot::new();
    root.settings_ui.max_concurrency_draft = "12".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::MaxConcurrency)
        .selected_range = 1..1;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::MaxConcurrency,
        None,
        "9",
        None,
        false,
    ));

    assert_eq!(root.settings_ui.max_concurrency_draft, "192");
    assert_eq!(
        root.text_input_runtime(FrameTextInputKind::MaxConcurrency)
            .selected_range,
        2..2
    );
}

#[test]
fn max_concurrency_input_deletes_selected_range() {
    let mut root = FrameRoot::new();
    root.settings_ui.max_concurrency_draft = "12".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::MaxConcurrency)
        .selected_range = 1..2;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::MaxConcurrency,
        None,
        "",
        None,
        false,
    ));

    assert_eq!(root.settings_ui.max_concurrency_draft, "1");
    assert_eq!(
        root.text_input_runtime(FrameTextInputKind::MaxConcurrency)
            .selected_range,
        1..1
    );
}

#[test]
fn output_name_input_appends_text_at_selection() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    let len = root
        .file_queue
        .selected_file()
        .map_or(0, |file| file.output_name.len());
    root.text_input_runtime_mut(FrameTextInputKind::OutputName)
        .selected_range = len..len;

    assert!(root.replace_text_input_range(FrameTextInputKind::OutputName, None, "x", None, false,));

    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.output_name.as_str()),
        Some("one_convertedx")
    );
}

#[test]
fn output_name_input_delete_can_leave_field_empty() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue.update_selected_output_name("a");
    root.text_input_runtime_mut(FrameTextInputKind::OutputName)
        .selected_range = 0..1;

    assert!(root.replace_text_input_range(FrameTextInputKind::OutputName, None, "", None, false,));

    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.output_name.as_str()),
        Some("")
    );
}

#[test]
fn metadata_title_input_inserts_free_text_at_selection() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .metadata
        .title = Some("Render".to_string());
    root.text_input_runtime_mut(FrameTextInputKind::MetadataTitle)
        .selected_range = 6..6;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::MetadataTitle,
        None,
        " Title",
        None,
        false,
    ));

    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.metadata.title.as_deref()),
        Some("Render Title")
    );
}

#[test]
fn external_subtitle_inputs_edit_the_selected_sidecar_only() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .external_subtitle_tracks = vec![
        crate::settings::ExternalSubtitleTrack {
            path: "/tmp/first.srt".to_string(),
            language: Some("eng".to_string()),
            ..crate::settings::ExternalSubtitleTrack::default()
        },
        crate::settings::ExternalSubtitleTrack {
            path: "/tmp/second.srt".to_string(),
            title: Some("Signs".to_string()),
            ..crate::settings::ExternalSubtitleTrack::default()
        },
    ];
    root.subtitle_ui.external_track_index = Some(1);
    root.text_input_runtime_mut(FrameTextInputKind::ExternalSubtitleTitle)
        .selected_range = 5..5;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::ExternalSubtitleTitle,
        None,
        " & songs",
        None,
        false,
    ));

    let tracks = &root
        .file_queue
        .selected_file()
        .unwrap()
        .config
        .external_subtitle_tracks;
    assert_eq!(tracks[0].language.as_deref(), Some("eng"));
    assert_eq!(tracks[0].title, None);
    assert_eq!(tracks[1].title.as_deref(), Some("Signs & songs"));
}

#[test]
fn preset_name_input_inserts_free_text_at_selection() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.settings_ui.preset_name_draft = "Review".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::PresetName)
        .selected_range = 6..6;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::PresetName,
        None,
        " MP4",
        None,
        false,
    ));

    assert_eq!(root.settings_ui.preset_name_draft, "Review MP4");
}

#[test]
fn audio_bitrate_input_inserts_digits_at_selection() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .audio_bitrate = "12".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::AudioBitrate)
        .selected_range = 1..1;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::AudioBitrate,
        None,
        "9",
        None,
        false,
    ));

    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.config.audio_bitrate.as_str()),
        Some("192")
    );
    assert_eq!(
        root.text_input_runtime(FrameTextInputKind::AudioBitrate)
            .selected_range,
        2..2
    );
}

#[test]
fn audio_bitrate_input_rejects_non_digits() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .audio_bitrate = "128".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::AudioBitrate)
        .selected_range = 3..3;

    assert!(!root.replace_text_input_range(
        FrameTextInputKind::AudioBitrate,
        None,
        "k",
        None,
        false,
    ));

    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.config.audio_bitrate.as_str()),
        Some("128")
    );
}

#[test]
fn text_input_handler_is_scoped_to_the_active_focused_field() {
    assert!(!should_handle_text_input(false, false, false));
    assert!(!should_handle_text_input(false, true, false));
    assert!(!should_handle_text_input(false, false, true));
    assert!(should_handle_text_input(false, true, true));
    assert!(!should_handle_text_input(true, true, true));
}

#[test]
fn text_input_outside_mouse_up_captures_only_while_selecting() {
    assert!(!should_capture_text_input_drag(false));
    assert!(should_capture_text_input_drag(true));
}

#[test]
fn text_input_scroll_reveals_cursor_past_right_edge() {
    let scroll_x =
        text_input_scroll_x_for_cursor(px(0.0), px(180.0), px(240.0), px(120.0), px(1.0));

    assert!(scroll_x > px(0.0));
}

#[test]
fn text_input_scroll_reveals_cursor_past_left_edge() {
    let scroll_x =
        text_input_scroll_x_for_cursor(px(80.0), px(40.0), px(240.0), px(120.0), px(1.0));

    assert_eq!(scroll_x, px(40.0));
}

#[test]
fn text_input_scroll_stays_zero_when_content_fits() {
    let scroll_x = text_input_scroll_x_for_cursor(px(0.0), px(60.0), px(90.0), px(120.0), px(1.0));

    assert_eq!(scroll_x, px(0.0));
}
