use super::*;

fn root_with_timecode_preview(duration: f64) -> FrameRoot {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.preview_ui.playback_file_id = Some("first".to_string());
    root.preview_ui.playback =
        preview_playback_state(PreviewMediaKind::Video, duration, None, None);
    root
}

#[test]
fn preview_timecode_input_types_digits_across_fixed_separators() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;

    for digit in "001500000".chars() {
        let mut encoded = [0; 4];
        assert!(root.replace_text_input_range(
            FrameTextInputKind::PreviewStartTime,
            None,
            digit.encode_utf8(&mut encoded),
            None,
            false,
        ));
    }

    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewStartTime),
        "00:15:00.000"
    );
    assert_eq!(
        root.text_input_runtime(FrameTextInputKind::PreviewStartTime)
            .selected_range,
        12..12
    );
}

#[test]
fn preview_timecode_input_does_not_commit_while_typing() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        "001500000",
        None,
        false,
    ));

    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.start_time.as_deref()),
        None
    );
    assert_eq!(root.preview_ui.playback.start_value(), 0.0);
}

#[test]
fn preview_timecode_input_ignores_manually_typed_separator() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 3..3;

    assert!(!root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        ":",
        None,
        false,
    ));

    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewStartTime),
        "00:00:00.000"
    );
    assert_eq!(
        root.text_input_runtime(FrameTextInputKind::PreviewStartTime)
            .selected_range,
        3..3
    );
}

#[test]
fn preview_timecode_input_accepts_formatted_paste_without_editing_separators() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewEndTime)
        .selected_range = 0..12;

    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewEndTime,
        None,
        "01:15:00.000",
        None,
        false,
    ));

    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewEndTime),
        "01:15:00.000"
    );
}

#[test]
fn preview_timecode_input_commits_draft_on_request() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;
    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        "001500000",
        None,
        false,
    ));

    assert!(root.commit_preview_timecode_input(FrameTextInputKind::PreviewStartTime, None));

    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.start_time.as_deref()),
        Some("00:15:00.000")
    );
    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewStartTime),
        "00:15:00.000"
    );
}

#[test]
fn preview_timecode_input_clamps_end_to_duration_only_when_committed() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewEndTime)
        .selected_range = 0..12;
    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewEndTime,
        None,
        "013500000",
        None,
        false,
    ));
    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewEndTime),
        "01:35:00.000"
    );

    assert!(root.commit_preview_timecode_input(FrameTextInputKind::PreviewEndTime, None));

    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.end_time.as_deref()),
        None
    );
    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewEndTime),
        "01:30:00.000"
    );
}

#[test]
fn preview_timecode_input_clamps_start_below_end_when_committed() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;
    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        "020000000",
        None,
        false,
    ));

    assert!(root.commit_preview_timecode_input(FrameTextInputKind::PreviewStartTime, None));

    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.start_time.as_deref()),
        Some("01:29:59.999")
    );
}

#[test]
fn preview_timecode_input_normalizes_overflowing_segments_when_committed() {
    let mut root = root_with_timecode_preview(10_000.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;
    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        "007500000",
        None,
        false,
    ));

    assert!(root.commit_preview_timecode_input(FrameTextInputKind::PreviewStartTime, None));

    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewStartTime),
        "01:15:00.000"
    );
}

#[test]
fn preview_timecode_input_commits_to_draft_file_after_selection_changes() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;
    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        "001500000",
        None,
        false,
    ));
    assert!(root.file_queue.select_existing_file("second"));

    assert!(root.commit_preview_timecode_input(FrameTextInputKind::PreviewStartTime, None));

    assert_eq!(
        root.file_queue
            .file_by_id("first")
            .and_then(|file| file.config.start_time.as_deref()),
        Some("00:15:00.000")
    );
    assert_eq!(
        root.file_queue
            .file_by_id("second")
            .and_then(|file| file.config.start_time.as_deref()),
        None
    );
}

#[test]
fn preview_timecode_input_escape_discards_uncommitted_draft() {
    let mut root = root_with_timecode_preview(5_400.0);
    root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
        .selected_range = 0..12;
    assert!(root.replace_text_input_range(
        FrameTextInputKind::PreviewStartTime,
        None,
        "001500000",
        None,
        false,
    ));

    assert!(root.cancel_preview_timecode_input(FrameTextInputKind::PreviewStartTime));

    assert_eq!(
        root.text_input_value(FrameTextInputKind::PreviewStartTime),
        "00:00:00.000"
    );
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.start_time.as_deref()),
        None
    );
}
