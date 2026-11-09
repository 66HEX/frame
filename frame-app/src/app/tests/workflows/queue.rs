use super::*;

pub(super) fn root_with_output_directory() -> FrameRoot {
    let mut root = FrameRoot::new();
    root.default_output_directory = Some(PathBuf::from("/tmp/frame-output"));
    root
}

#[test]
fn queue_selected_conversion_tasks_requires_default_output_directory() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));

    assert!(root.queue_selected_conversion_tasks().is_empty());
    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Idle)
    );
}

#[test]
fn queue_selected_conversion_tasks_marks_pending_file_as_queued() {
    let mut root = root_with_output_directory();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
    root.file_queue.toggle_batch("second", false);

    let tasks = root.queue_selected_conversion_tasks();

    assert_eq!(
        tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        ["first"]
    );
    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Queued)
    );
    assert_eq!(tasks[0].output_name.as_deref(), Some("one_converted"));
    assert_eq!(tasks[0].output_directory, "/tmp/frame-output");
}

#[test]
fn queued_retry_accepts_cancellation_before_its_background_worker_starts() {
    let mut root = root_with_output_directory();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.m4a", 1));
    // Seed only the controller's completed-publication policy. Real repaired
    // exports and delayed batch retries have separate FFmpeg integration tests.
    root.conversion_processes
        .complete_publication_for_test("first")
        .unwrap();
    root.file_queue
        .update_status("first", FileStatus::Completed, 100);
    root.conversion_processes.cancel_task("first").unwrap();
    assert!(!root.conversion_processes.is_cancelled("first"));

    assert!(root.prepare_file_for_reconversion("first"));
    let tasks = root.queue_selected_conversion_tasks();
    assert_eq!(tasks.len(), 1);
    assert!(root.cancel_conversion_task("first"));
    assert!(root.conversion_processes.is_cancelled("first"));
    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Cancelling)
    );
    // Registration at the later batch entry must preserve that pending cancel.
    root.conversion_processes
        .begin_queued_attempts(&[tasks[0].id.clone()])
        .unwrap();
    assert!(root.conversion_processes.take_cancelled("first").unwrap());
}

#[test]
fn queue_selected_conversion_tasks_assigns_and_exposes_unique_output_names() {
    let mut root = FrameRoot::new();
    root.default_output_directory = Some(test_settings_path().with_file_name("exports"));
    root.file_queue
        .add_file(FileItem::from_path("mov", "/A/clip.mov", 1));
    root.file_queue
        .add_file(FileItem::from_path("mkv", "/B/clip.mkv", 1));

    let tasks = root.queue_selected_conversion_tasks();

    assert_eq!(
        (
            tasks[0].output_name.as_deref(),
            tasks[1].output_name.as_deref(),
            root.file_queue
                .file_by_id("mov")
                .map(|file| file.output_name.as_str()),
            root.file_queue
                .file_by_id("mkv")
                .map(|file| file.output_name.as_str())
        ),
        (
            Some("clip_converted"),
            Some("clip_converted_2"),
            Some("clip_converted"),
            Some("clip_converted_2")
        )
    );
}

#[test]
fn queue_selected_conversion_tasks_normalizes_each_file_from_own_metadata() {
    let mut root = root_with_output_directory();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.file_queue
        .add_file(FileItem::from_path("image", "/tmp/two.png", 1));
    root.source_metadata.mark_ready(
        "image",
        SourceMetadata {
            media_kind: Some(SourceKind::Image),
            ..SourceMetadata::default()
        },
    );

    let tasks = root.queue_selected_conversion_tasks();

    let image_task = tasks
        .iter()
        .find(|task| task.id == "image")
        .expect("image task should be queued");
    assert_eq!(image_task.config.container, "png");
    assert_eq!(image_task.config.video_codec, "png");
}

#[test]
fn queue_selected_conversion_tasks_infers_image_config_from_extension_without_metadata() {
    let mut root = root_with_output_directory();
    root.file_queue
        .add_file(FileItem::from_path("image", "/tmp/two.PNG", 1));

    let tasks = root.queue_selected_conversion_tasks();

    assert_eq!(tasks[0].config.container, "png");
    assert_eq!(tasks[0].config.video_codec, "png");
}

#[test]
fn apply_conversion_event_updates_processing_state_from_queue() {
    let mut root = root_with_output_directory();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.queue_selected_conversion_tasks();
    root.is_processing = true;

    root.apply_conversion_event(ConversionEvent::completed("first", "/tmp/one.mp4"));

    assert!(!root.is_processing);
    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Completed)
    );
}

#[test]
fn apply_conversion_event_notifies_when_active_batch_settles() {
    let notifications = std::sync::Arc::new(Mutex::new(Vec::new()));
    let received_notifications = notifications.clone();
    let mut root = FrameRoot::new_with_notifier(AppNotifier::from_conversion_finished_handler(
        move |summary| {
            received_notifications
                .lock()
                .expect("notifications should be writable")
                .push(summary);
        },
    ));
    root.default_output_directory = Some(PathBuf::from("/tmp/frame-output"));
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
    root.queue_selected_conversion_tasks();
    root.active_conversion_task_ids = vec!["first".to_string(), "second".to_string()];
    root.is_processing = true;

    root.apply_conversion_event(ConversionEvent::completed("first", "/tmp/one.mp4"));
    root.apply_conversion_event(ConversionEvent::error("second", "ffmpeg failed"));

    assert_eq!(
        notifications
            .lock()
            .expect("notifications should be readable")
            .as_slice(),
        [ConversionNotificationSummary {
            completed_count: 1,
            error_count: 1,
        }]
    );
}

#[test]
fn apply_conversion_event_does_not_notify_when_active_batch_has_no_results() {
    let notifications = std::sync::Arc::new(Mutex::new(Vec::new()));
    let received_notifications = notifications.clone();
    let mut root = FrameRoot::new_with_notifier(AppNotifier::from_conversion_finished_handler(
        move |summary| {
            received_notifications
                .lock()
                .expect("notifications should be writable")
                .push(summary);
        },
    ));
    root.default_output_directory = Some(PathBuf::from("/tmp/frame-output"));
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.queue_selected_conversion_tasks();
    root.active_conversion_task_ids = vec!["first".to_string()];
    root.is_processing = true;

    root.apply_conversion_event(ConversionEvent::cancelled("first"));

    assert!(
        notifications
            .lock()
            .expect("notifications should be readable")
            .is_empty()
    );
}

#[test]
fn remove_file_from_queue_cancels_and_removes_paused_file() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .update_status("first", FileStatus::Paused, 30);
    root.conversion_events
        .apply_conversion_event(&mut root.file_queue, ConversionEvent::log("first", "line"));

    assert!(root.remove_file_from_queue("first"));

    assert!(root.file_queue.file_by_id("first").is_none());
    assert!(root.conversion_events.logs_for("first").is_empty());
}

#[test]
fn pause_conversion_task_keeps_status_when_process_is_missing() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .update_status("first", FileStatus::Converting, 30);

    assert!(!root.pause_conversion_task("first"));

    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Converting)
    );
    assert!(
        root.conversion_events
            .logs_for("first")
            .iter()
            .any(|line| line.contains("Failed to pause"))
    );
}

#[test]
fn cancel_conversion_task_keeps_source_until_runner_confirms_cancellation() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .update_status("first", FileStatus::Queued, 0);

    assert!(root.cancel_conversion_task("first"));
    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Cancelling)
    );

    root.apply_conversion_event(ConversionEvent::cancelled("first"));

    assert_eq!(
        root.file_queue.file_by_id("first").map(|file| file.status),
        Some(FileStatus::Idle)
    );
}

#[test]
fn prepare_file_for_reconversion_keeps_source_settings_and_enables_start() {
    let mut root = FrameRoot::new();
    let mut file = FileItem::from_path("first", "/tmp/one.mp4", 1);
    file.config.container = "mkv".to_string();
    root.file_queue.add_file(file);
    root.file_queue
        .update_status("first", FileStatus::Completed, 100);

    assert!(root.prepare_file_for_reconversion("first"));

    let file = root
        .file_queue
        .file_by_id("first")
        .expect("source should remain in queue");
    assert_eq!(file.status, FileStatus::Idle);
    assert_eq!(file.config.container, "mkv");
    root.default_output_directory = Some(PathBuf::from("/tmp/frame-output"));
    assert!(root.app_state().can_start_conversion());
}
