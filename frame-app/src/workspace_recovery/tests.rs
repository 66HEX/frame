use super::*;
use crate::ActiveView;
use crate::file_queue::{FileItem, FileQueue, FileStatus};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct Fixture(pub(super) PathBuf);
impl Fixture {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "frame-recovery-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(super) fn source(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, b"source fixture").unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

pub(super) fn capture(queue: &FileQueue) -> RecoverySnapshot {
    RecoverySnapshot::capture(
        queue,
        ActiveView::Logs,
        Some("/exports".into()),
        "session-a",
        1,
        42,
    )
    .unwrap()
}

#[test]
fn prepared_queue_restores_full_config_selection_and_folder_without_starting() {
    let fixture = Fixture::new();
    let source = fixture.source("first.mp4");
    let mut queue = FileQueue::new();
    let mut file = FileItem::from_os_path("file-17", &source);
    file.config.video_bitrate = "12345".into();
    file.config.selected_audio_tracks = vec![1, 3];
    file.config.audio_track_overrides.insert(
        3,
        frame_core::types::AudioTrackOverride {
            language: Some(String::new()),
            title: Some("Director commentary".into()),
            is_default: Some(true),
            sample_rate: Some(44_100),
        },
    );
    file.output_name = "prepared-cut".into();
    file.is_selected_for_conversion = false;
    queue.add_file(file.clone());
    let bytes = serde_json::to_vec(&capture(&queue)).unwrap();
    let restored = serde_json::from_slice::<RecoverySnapshot>(&bytes)
        .unwrap()
        .restore()
        .unwrap();
    assert_eq!(restored.queue.files(), &[file]);
    assert_eq!(restored.queue.selected_file_id(), Some("file-17"));
    assert_eq!(restored.next_file_sequence, 17);
    assert_eq!(restored.active_view, ActiveView::Logs);
    assert_eq!(restored.default_output_directory, Some("/exports".into()));
}

#[test]
fn every_active_state_restores_as_interrupted_and_never_queued() {
    let fixture = Fixture::new();
    let source = fixture.source("active.mp4");
    for status in [
        FileStatus::Queued,
        FileStatus::Converting,
        FileStatus::Paused,
        FileStatus::Cancelling,
    ] {
        let mut file = FileItem::from_os_path("file-1", &source);
        file.status = status;
        file.progress_percent = 82;
        file.output_path = Some(fixture.0.join("partial.mp4").to_string_lossy().into());
        let mut queue = FileQueue::new();
        queue.add_file(file);
        let restored = capture(&queue).restore().unwrap();
        assert_eq!(restored.interrupted_count, 1);
        let file = &restored.queue.files()[0];
        assert_eq!(file.status, FileStatus::Error);
        assert_eq!(file.progress_percent, 0);
        assert_eq!(
            file.conversion_error.as_deref(),
            Some(super::model::INTERRUPTED)
        );
        assert_eq!(file.output_path, None);
    }
}

#[test]
fn missing_source_keeps_configuration_and_is_not_selected_for_conversion() {
    let fixture = Fixture::new();
    let mut file = FileItem::from_os_path("file-1", &fixture.0.join("missing.mp4"));
    file.config.video_bitrate = "45678".into();
    let mut queue = FileQueue::new();
    queue.add_file(file.clone());
    let restored = capture(&queue).restore().unwrap();
    assert_eq!(restored.missing_source_count, 1);
    assert!(restored.probe_targets.is_empty());
    assert_eq!(restored.queue.files()[0].config, file.config);
    assert_eq!(restored.queue.files()[0].status, FileStatus::Error);
    assert!(!restored.queue.files()[0].is_selected_for_conversion);
}

#[test]
fn completed_result_remains_accessible_after_source_is_removed_and_bytes_are_untouched() {
    let fixture = Fixture::new();
    let output = fixture.source("published.mp4");
    let mut file = FileItem::from_os_path("file-2", &fixture.0.join("missing-source.mp4"));
    file.status = FileStatus::Completed;
    file.output_path = Some(output.to_string_lossy().into());
    let mut queue = FileQueue::new();
    queue.add_file(file);
    let restored = capture(&queue).restore().unwrap();
    assert_eq!(
        restored.queue.files()[0].completed_output_path(),
        output.to_str()
    );
    assert_eq!(fs::read(&output).unwrap(), b"source fixture");
}

#[test]
fn missing_published_output_is_never_replaced_or_guessed() {
    let fixture = Fixture::new();
    let source = fixture.source("source.mp4");
    let missing = fixture.0.join("removed-output.mp4");
    let mut file = FileItem::from_os_path("file-1", &source);
    file.status = FileStatus::Completed;
    file.output_path = Some(missing.to_string_lossy().into());
    let mut queue = FileQueue::new();
    queue.add_file(file);
    let restored = capture(&queue).restore().unwrap();
    assert_eq!(
        restored.queue.files()[0].completed_output_path(),
        missing.to_str()
    );
    assert!(!missing.exists());
}

#[test]
fn malformed_snapshot_fields_are_rejected() {
    let fixture = Fixture::new();
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_os_path(
        "file-1",
        &fixture.source("source.mp4"),
    ));
    let value = serde_json::to_value(capture(&queue)).unwrap();
    for (name, changed) in [
        ("version", serde_json::json!(2)),
        ("sessionId", serde_json::json!("")),
        ("revision", serde_json::json!(0)),
        ("appVersion", serde_json::json!("")),
        ("selectedFileId", serde_json::json!("unknown-id")),
    ] {
        let mut changed_value = value.clone();
        changed_value[name] = changed;
        let snapshot: RecoverySnapshot = serde_json::from_value(changed_value).unwrap();
        assert!(snapshot.restore().is_err(), "accepted {name}");
    }
}

#[test]
fn empty_paths_duplicate_ids_and_unproven_completed_results_are_rejected() {
    let fixture = Fixture::new();
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_os_path(
        "file-1",
        &fixture.source("source.mp4"),
    ));
    let value = serde_json::to_value(capture(&queue)).unwrap();
    for index in 0..4 {
        let mut changed = value.clone();
        match index {
            0 => changed["files"][0]["path"] = serde_json::json!(""),
            1 => changed["files"][0]["id"] = serde_json::json!(""),
            2 => {
                let duplicate = changed["files"][0].clone();
                changed["files"].as_array_mut().unwrap().push(duplicate);
            }
            _ => changed["files"][0]["status"] = serde_json::json!("completed"),
        }
        assert!(
            serde_json::from_value::<RecoverySnapshot>(changed)
                .unwrap()
                .restore()
                .is_err(),
            "accepted {index}"
        );
    }
}

#[test]
fn unknown_status_is_rejected_by_decoder() {
    let fixture = Fixture::new();
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_os_path(
        "file-1",
        &fixture.source("source.mp4"),
    ));
    let mut value = serde_json::to_value(capture(&queue)).unwrap();
    value["files"][0]["status"] = serde_json::json!("future-running");
    assert!(serde_json::from_value::<RecoverySnapshot>(value).is_err());
}

#[test]
fn interrupted_missing_source_still_explains_that_the_conversion_was_interrupted() {
    let fixture = Fixture::new();
    let mut queue = FileQueue::new();
    let mut file = FileItem::from_os_path("file-1", &fixture.0.join("removed.mp4"));
    file.status = FileStatus::Converting;
    queue.add_file(file);
    let restored = capture(&queue).restore().unwrap();
    let error = restored.queue.files()[0]
        .conversion_error
        .as_deref()
        .unwrap();
    assert!(error.contains("Interrupted"));
    assert!(error.contains("Source file is no longer available"));
    assert!(!restored.queue.files()[0].is_selected_for_conversion);
}

#[test]
fn legacy_completed_entry_without_recorded_path_restores_as_unselected_error_without_guessing() {
    let fixture = Fixture::new();
    let source = fixture.source("legacy.mp4");
    let existing_output = fixture.source("legacy_converted.mp4");
    let mut file = FileItem::from_os_path("file-1", &source);
    file.status = FileStatus::Completed;
    file.progress_percent = 100;
    file.config.crf = 19;
    let mut queue = FileQueue::new();
    queue.add_file(file);
    let snapshot = RecoverySnapshot::capture(
        &queue,
        ActiveView::Workspace,
        Some(fixture.0.clone()),
        "after-update",
        1,
        42,
    )
    .expect("legacy Completed/None must not disable recovery of the whole queue");
    let restored = snapshot.restore().unwrap();
    let file = &restored.queue.files()[0];
    assert_eq!(file.status, FileStatus::Error);
    assert_eq!(file.progress_percent, 0);
    assert_eq!(file.config.crf, 19);
    assert!(!file.is_selected_for_conversion);
    assert_eq!(file.output_path, None);
    assert!(
        file.conversion_error
            .as_deref()
            .unwrap()
            .contains("result path was not recorded")
    );
    assert_eq!(fs::read(existing_output).unwrap(), b"source fixture");
}
