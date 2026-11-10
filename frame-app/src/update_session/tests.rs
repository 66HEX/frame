use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn capture_and_restore_preserves_queue_order_selection_and_config() {
    let first_path = create_test_source("first.mp4");
    let second_path = create_test_source("second.mov");
    let mut first = FileItem::from_os_path("old-8", &first_path);
    first.output_name = "review-cut".to_string();
    first.is_selected_for_conversion = false;
    first.config.video_bitrate = "9000".to_string();
    first.config.subtitle_burn_path = Some("/missing/captions.srt".to_string());
    first.config.external_subtitle_tracks = vec![crate::settings::ExternalSubtitleTrack {
        path: "/missing/selectable.srt".to_string(),
        language: Some("eng".to_string()),
        title: Some("English".to_string()),
        is_default: true,
        is_forced: false,
    }];
    first.config.crop = Some(crate::settings::CropSettings {
        enabled: true,
        x: 12,
        y: 24,
        width: 640,
        height: 360,
        source_width: Some(1920),
        source_height: Some(1080),
        aspect_ratio: Some("16:9".to_string()),
    });
    first.config.selected_audio_tracks = vec![0, 2];
    let expected_first_config = first.config.clone();
    let mut second = FileItem::from_os_path("old-2", &second_path);
    second.status = FileStatus::Completed;
    second.progress_percent = 100;
    let mut queue = FileQueue::new();
    queue.add_files([first, second]);
    assert!(queue.select_existing_file("old-2"));

    let restored = UpdateSessionSnapshot::capture(&queue, ActiveView::Logs, "0.31.1", "0.32.0", 42)
        .and_then(UpdateSessionSnapshot::restore)
        .expect("settled queue should round-trip");

    assert_eq!(restored.active_view, ActiveView::Logs);
    assert_eq!(restored.next_file_sequence, 2);
    assert_eq!(restored.queue.selected_file_id(), Some("file-2"));
    assert_eq!(restored.queue.files()[0].path, first_path.to_string_lossy());
    assert_eq!(restored.queue.files()[0].output_name, "review-cut");
    assert!(!restored.queue.files()[0].is_selected_for_conversion);
    assert_eq!(restored.queue.files()[0].config, expected_first_config);
    assert_eq!(restored.queue.files()[1].status, FileStatus::Completed);
    assert_eq!(restored.probe_targets.len(), 2);

    remove_test_source(&first_path);
    remove_test_source(&second_path);
}

#[test]
fn capture_rejects_every_runtime_conversion_state() {
    for status in [
        FileStatus::Queued,
        FileStatus::Converting,
        FileStatus::Paused,
        FileStatus::Cancelling,
    ] {
        let mut queue = FileQueue::new();
        let mut file = FileItem::from_path("file", "/tmp/source.mp4", 10);
        file.status = status;
        queue.add_file(file);

        let error =
            UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.31.1", "0.32.0", 42)
                .expect_err("runtime conversion states must not be persisted");

        assert!(error.to_string().contains(status.label()));
    }
}

#[test]
fn stable_statuses_restore_with_normalized_progress_and_errors() {
    let idle_path = create_test_source("idle.mp4");
    let completed_path = create_test_source("completed.mp4");
    let error_path = create_test_source("error.mp4");
    let mut idle = FileItem::from_os_path("idle", &idle_path);
    idle.progress_percent = 37;
    let mut completed = FileItem::from_os_path("completed", &completed_path);
    completed.status = FileStatus::Completed;
    completed.progress_percent = 12;
    let mut error = FileItem::from_os_path("error", &error_path);
    error.status = FileStatus::Error;
    error.progress_percent = 91;
    error.conversion_error = Some("encoder failed".to_string());
    let mut queue = FileQueue::new();
    queue.add_files([idle, completed, error]);

    let restored =
        UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.31.1", "0.32.0", 42)
            .and_then(UpdateSessionSnapshot::restore)
            .expect("stable statuses should restore");

    assert_eq!(restored.queue.files()[0].status, FileStatus::Idle);
    assert_eq!(restored.queue.files()[0].progress_percent, 0);
    assert_eq!(restored.queue.files()[1].status, FileStatus::Completed);
    assert_eq!(restored.queue.files()[1].progress_percent, 100);
    assert_eq!(restored.queue.files()[2].status, FileStatus::Error);
    assert_eq!(restored.queue.files()[2].progress_percent, 0);
    assert_eq!(
        restored.queue.files()[2].conversion_error.as_deref(),
        Some("encoder failed")
    );

    remove_test_source(&idle_path);
    remove_test_source(&completed_path);
    remove_test_source(&error_path);
}

#[test]
fn duplicate_source_paths_remain_distinct_queue_entries() {
    let source_path = create_test_source("duplicate.mp4");
    let mut first = FileItem::from_os_path("first", &source_path);
    first.output_name = "first-output".to_string();
    let mut second = FileItem::from_os_path("second", &source_path);
    second.output_name = "second-output".to_string();
    second.config.video_bitrate = "12000".to_string();
    let mut queue = FileQueue::new();
    queue.add_files([first, second]);

    let restored =
        UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.31.1", "0.32.0", 42)
            .and_then(UpdateSessionSnapshot::restore)
            .expect("duplicate source paths should restore");

    assert_eq!(restored.queue.files().len(), 2);
    assert_eq!(
        restored.queue.files()[0].path,
        restored.queue.files()[1].path
    );
    assert_eq!(restored.queue.files()[0].output_name, "first-output");
    assert_eq!(restored.queue.files()[1].output_name, "second-output");
    assert_eq!(restored.queue.files()[1].config.video_bitrate, "12000");
    remove_test_source(&source_path);
}

#[test]
fn capture_rejects_empty_source_paths() {
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_path("empty", "", 0));

    let error =
        UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.31.1", "0.32.0", 42)
            .expect_err("empty source paths must be rejected");

    assert!(error.to_string().contains("empty source path"));
}

#[test]
fn restore_marks_missing_sources_as_errors_without_probing_them() {
    let mut queue = FileQueue::new();
    let mut file = FileItem::from_path("old", "/missing/frame-source.mp4", 100);
    file.config.audio_bitrate = "256".to_string();
    queue.add_file(file);

    let restored =
        UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.31.1", "0.32.0", 42)
            .and_then(UpdateSessionSnapshot::restore)
            .expect("missing source should become a recoverable queue error");

    let file = &restored.queue.files()[0];
    assert_eq!(file.status, FileStatus::Error);
    assert_eq!(file.size_bytes, 0);
    assert_eq!(file.config.audio_bitrate, "256");
    assert_eq!(file.conversion_error.as_deref(), Some(MISSING_SOURCE_ERROR));
    assert!(restored.probe_targets.is_empty());
}

#[test]
fn store_round_trips_and_consumes_snapshot_exactly_once() {
    let settings_path = test_path("settings.json");
    let store = UpdateSessionStore::from_settings_path(&settings_path);
    let snapshot = UpdateSessionSnapshot::capture(
        &FileQueue::new(),
        ActiveView::Workspace,
        "0.31.1",
        "0.32.0",
        42,
    )
    .expect("empty settled session should be valid");

    store.save(&snapshot).expect("snapshot should save");
    assert_eq!(store.load().expect("snapshot should load"), Some(snapshot));
    store.consume().expect("snapshot should be consumed");
    assert_eq!(
        store.load().expect("consumed snapshot should not load"),
        None
    );

    remove_test_tree(&settings_path);
}

#[test]
fn load_rejects_unsupported_snapshot_versions() {
    let settings_path = test_path("settings.json");
    let store = UpdateSessionStore::from_settings_path(&settings_path);
    let snapshot = UpdateSessionSnapshot::capture(
        &FileQueue::new(),
        ActiveView::Workspace,
        "0.31.1",
        "0.32.0",
        42,
    )
    .expect("snapshot should be valid");
    let mut value = serde_json::to_value(snapshot).expect("snapshot should serialize");
    value["version"] = serde_json::Value::from(99);
    let parent = store
        .pending_path
        .parent()
        .expect("test snapshot path should have a parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(
        &store.pending_path,
        serde_json::to_vec(&value).expect("test JSON should serialize"),
    )
    .expect("test snapshot should be written");

    let error = store
        .load()
        .expect_err("unsupported version must not be restored");

    assert!(
        error
            .to_string()
            .contains("unsupported update session version 99")
    );
    remove_test_tree(&settings_path);
}

#[test]
fn snapshot_json_uses_the_versioned_v1_field_names() {
    let snapshot = UpdateSessionSnapshot::capture(
        &FileQueue::new(),
        ActiveView::Workspace,
        "0.31.1",
        "0.32.0",
        42,
    )
    .expect("snapshot should be valid");

    let value = serde_json::to_value(snapshot).expect("snapshot should serialize");

    assert_eq!(value["version"], 1);
    assert_eq!(value["sourceVersion"], "0.31.1");
    assert_eq!(value["targetVersion"], "0.32.0");
    assert_eq!(value["createdAtUnixSeconds"], 42);
    assert_eq!(value["activeView"], "workspace");
    assert!(value.get("capturedAt").is_none());
}

#[test]
fn restore_falls_back_for_unknown_view_and_invalid_selection() {
    let source_path = create_test_source("fallback.mp4");
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_os_path("old", &source_path));
    let snapshot = UpdateSessionSnapshot::capture(&queue, ActiveView::Logs, "0.31.1", "0.32.0", 42)
        .expect("snapshot should be valid");
    let mut value = serde_json::to_value(snapshot).expect("snapshot should serialize");
    value["activeView"] = serde_json::Value::from("future-view");
    value["selectedFileIndex"] = serde_json::Value::from(99);
    let snapshot = serde_json::from_value::<UpdateSessionSnapshot>(value)
        .expect("unknown view should deserialize to the compatibility fallback");

    let restored = snapshot.restore().expect("snapshot should restore");

    assert_eq!(restored.active_view, ActiveView::Workspace);
    assert_eq!(restored.queue.selected_file_id(), Some("file-1"));
    remove_test_source(&source_path);
}

#[test]
fn capture_drops_stale_errors_from_non_error_files() {
    let source_path = create_test_source("stale-error.mp4");
    let mut file = FileItem::from_os_path("old", &source_path);
    file.conversion_error = Some("stale FFmpeg output".to_string());
    let mut queue = FileQueue::new();
    queue.add_file(file);

    let snapshot =
        UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.31.1", "0.32.0", 42)
            .expect("snapshot should be valid");
    let value = serde_json::to_value(snapshot).expect("snapshot should serialize");

    assert!(value["files"][0]["conversionError"].is_null());
    remove_test_source(&source_path);
}

#[test]
fn corrupted_json_is_reported_and_retained_for_diagnostics() {
    let settings_path = test_path("settings.json");
    let store = UpdateSessionStore::from_settings_path(&settings_path);
    let parent = store
        .pending_path
        .parent()
        .expect("test snapshot path should have a parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(&store.pending_path, b"{not-json")
        .expect("corrupted test snapshot should be written");

    let error = store.load().expect_err("corrupted JSON must be rejected");

    assert!(error.to_string().contains("failed to parse"));
    assert!(store.pending_path.is_file());
    remove_test_tree(&settings_path);
}

#[test]
fn oversized_snapshot_is_rejected_before_json_allocation() {
    let settings_path = test_path("settings.json");
    let store = UpdateSessionStore::from_settings_path(&settings_path);
    let parent = store
        .pending_path
        .parent()
        .expect("test snapshot path should have a parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    let file =
        fs::File::create(&store.pending_path).expect("oversized test snapshot should be created");
    file.set_len(MAX_UPDATE_SESSION_BYTES + 1)
        .expect("oversized test snapshot should be extended");

    let error = store
        .load()
        .expect_err("oversized snapshot must be rejected");

    assert!(error.to_string().contains("snapshot is too large"));
    remove_test_tree(&settings_path);
}

#[test]
fn discard_pending_rolls_back_snapshot_without_affecting_settings_path() {
    let settings_path = test_path("settings.json");
    let store = UpdateSessionStore::from_settings_path(&settings_path);
    let snapshot = UpdateSessionSnapshot::capture(
        &FileQueue::new(),
        ActiveView::Workspace,
        "0.31.1",
        "0.32.0",
        42,
    )
    .expect("snapshot should be valid");
    store.save(&snapshot).expect("snapshot should save");

    store
        .discard_pending()
        .expect("pending snapshot should be discarded");

    assert_eq!(store.load().expect("store should remain readable"), None);
    assert!(!settings_path.exists());
    remove_test_tree(&settings_path);
}

#[test]
fn failed_temp_write_keeps_the_previous_valid_snapshot() {
    let settings_path = test_path("settings.json");
    let store = UpdateSessionStore::from_settings_path(&settings_path);
    let first = UpdateSessionSnapshot::capture(
        &FileQueue::new(),
        ActiveView::Workspace,
        "0.31.1",
        "0.32.0",
        42,
    )
    .expect("first snapshot should be valid");
    store.save(&first).expect("first snapshot should save");
    let temp_path = store
        .pending_path
        .with_file_name("pending-update-session.json.tmp");
    fs::create_dir(&temp_path).expect("blocking temp directory should be created");
    let second =
        UpdateSessionSnapshot::capture(&FileQueue::new(), ActiveView::Logs, "0.31.1", "0.33.0", 84)
            .expect("second snapshot should be valid");

    store
        .save(&second)
        .expect_err("blocked temp write must fail without replacing pending");

    assert_eq!(
        store.load().expect("previous snapshot should remain valid"),
        Some(first)
    );
    remove_test_tree(&settings_path);
}

fn create_test_source(name: &str) -> PathBuf {
    let path = test_path(name);
    let parent = path.parent().expect("test source should have a parent");
    fs::create_dir_all(parent).expect("test directory should be created");
    fs::write(&path, b"frame").expect("test source should be written");
    path
}

fn remove_test_source(path: &Path) {
    fs::remove_file(path).expect("test source should be removed");
    remove_test_tree(path);
}

fn remove_test_tree(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::remove_dir_all(parent).expect("test directory should be removed");
    }
}

fn test_path(file_name: &str) -> PathBuf {
    let sequence = TEST_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should follow the Unix epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("frame-update-session-{timestamp}-{sequence}"))
        .join(file_name)
}
