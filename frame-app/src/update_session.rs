//! Persistence model for restoring Frame's workspace after an app update.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ActiveView,
    app_persistence::write_bytes_atomically,
    file_queue::{FileItem, FileQueue, FileStatus},
    settings::ConversionConfig,
};

const UPDATE_SESSION_VERSION: u32 = 1;
const PENDING_UPDATE_SESSION_FILE_NAME: &str = "pending-update-session.json";
const CONSUMED_UPDATE_SESSION_FILE_NAME: &str = "restored-update-session.json";
const MAX_UPDATE_SESSION_BYTES: u64 = 16 * 1024 * 1024;
const MAX_UPDATE_SESSION_FILES: usize = 10_000;
const MISSING_SOURCE_ERROR: &str = "Source file is no longer available.";
const RESTORED_CONVERSION_ERROR: &str = "The previous conversion did not complete successfully.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSessionStore {
    pending_path: PathBuf,
    consumed_path: PathBuf,
}

impl UpdateSessionStore {
    #[must_use]
    pub fn from_settings_path(settings_path: &Path) -> Self {
        Self {
            pending_path: settings_path.with_file_name(PENDING_UPDATE_SESSION_FILE_NAME),
            consumed_path: settings_path.with_file_name(CONSUMED_UPDATE_SESSION_FILE_NAME),
        }
    }

    pub fn save(&self, snapshot: &UpdateSessionSnapshot) -> Result<(), UpdateSessionError> {
        snapshot.validate()?;
        let json = serde_json::to_vec_pretty(snapshot)?;
        let byte_count = u64::try_from(json.len()).unwrap_or(u64::MAX);
        if byte_count > MAX_UPDATE_SESSION_BYTES {
            return Err(UpdateSessionError::SnapshotTooLarge(byte_count));
        }
        write_bytes_atomically(&self.pending_path, &json)?;
        Ok(())
    }

    pub fn load(&self) -> Result<Option<UpdateSessionSnapshot>, UpdateSessionError> {
        let metadata = match fs::metadata(&self.pending_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if metadata.len() > MAX_UPDATE_SESSION_BYTES {
            return Err(UpdateSessionError::SnapshotTooLarge(metadata.len()));
        }

        let bytes = fs::read(&self.pending_path)?;
        let snapshot: UpdateSessionSnapshot = serde_json::from_slice(&bytes)?;
        snapshot.validate()?;
        Ok(Some(snapshot))
    }

    pub fn consume(&self) -> Result<(), UpdateSessionError> {
        match fs::remove_file(&self.consumed_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        match fs::rename(&self.pending_path, &self.consumed_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }

        match fs::remove_file(&self.consumed_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn discard_pending(&self) -> Result<(), UpdateSessionError> {
        match fs::remove_file(&self.pending_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateSessionSnapshot {
    version: u32,
    source_version: String,
    target_version: String,
    created_at_unix_seconds: u64,
    active_view: PersistedActiveView,
    selected_file_index: Option<usize>,
    files: Vec<PersistedFileItem>,
}

impl UpdateSessionSnapshot {
    pub fn capture(
        queue: &FileQueue,
        active_view: ActiveView,
        source_version: impl Into<String>,
        target_version: impl Into<String>,
        captured_at: u64,
    ) -> Result<Self, UpdateSessionError> {
        let files = queue
            .files()
            .iter()
            .map(PersistedFileItem::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let selected_file_index = queue
            .selected_file_id()
            .and_then(|selected_id| queue.files().iter().position(|file| file.id == selected_id));
        let snapshot = Self {
            version: UPDATE_SESSION_VERSION,
            source_version: source_version.into(),
            target_version: target_version.into(),
            created_at_unix_seconds: captured_at,
            active_view: active_view.into(),
            selected_file_index,
            files,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn restore(self) -> Result<RestoredUpdateSession, UpdateSessionError> {
        self.validate()?;

        let selected_file_index = self.selected_file_index;
        let active_view = self.active_view.into();
        let mut queue = FileQueue::new();
        let mut probe_targets = Vec::new();

        for (index, persisted) in self.files.into_iter().enumerate() {
            let id = format!("file-{}", index + 1);
            let source_exists = persisted.path.is_file();
            let mut file = if source_exists {
                FileItem::from_os_path(id.clone(), &persisted.path)
            } else {
                FileItem::from_path(id.clone(), persisted.path.to_string_lossy(), 0)
            };
            file.output_name = persisted.output_name;
            file.config = persisted.config;
            file.is_selected_for_conversion = persisted.selected_for_conversion;

            if source_exists {
                file.status = persisted.status.into();
                file.progress_percent = if file.status == FileStatus::Completed {
                    100
                } else {
                    0
                };
                file.conversion_error = if file.status == FileStatus::Error {
                    Some(
                        persisted
                            .conversion_error
                            .unwrap_or_else(|| RESTORED_CONVERSION_ERROR.to_string()),
                    )
                } else {
                    None
                };
                probe_targets.push((id, file.path.clone()));
            } else {
                file.status = FileStatus::Error;
                file.progress_percent = 0;
                file.conversion_error = Some(MISSING_SOURCE_ERROR.to_string());
            }

            queue.add_file(file);
        }

        if let Some(index) = selected_file_index.filter(|index| *index < queue.files().len()) {
            let selected_id = format!("file-{}", index + 1);
            queue.select_existing_file(&selected_id);
        }

        Ok(RestoredUpdateSession {
            next_file_sequence: u64::try_from(queue.files().len()).unwrap_or(u64::MAX),
            queue,
            active_view,
            probe_targets,
        })
    }

    fn validate(&self) -> Result<(), UpdateSessionError> {
        if self.version != UPDATE_SESSION_VERSION {
            return Err(UpdateSessionError::UnsupportedVersion {
                actual: self.version,
                supported: UPDATE_SESSION_VERSION,
            });
        }
        if self.source_version.trim().is_empty() || self.target_version.trim().is_empty() {
            return Err(UpdateSessionError::MissingAppVersion);
        }
        if self.files.len() > MAX_UPDATE_SESSION_FILES {
            return Err(UpdateSessionError::TooManyFiles(self.files.len()));
        }
        if self
            .files
            .iter()
            .any(|file| file.path.as_os_str().is_empty())
        {
            return Err(UpdateSessionError::MissingSourcePath);
        }
        if self.files.iter().any(|file| {
            file.status != PersistedFileStatus::Error && file.conversion_error.is_some()
        }) {
            return Err(UpdateSessionError::UnexpectedConversionError);
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RestoredUpdateSession {
    pub queue: FileQueue,
    pub active_view: ActiveView,
    pub next_file_sequence: u64,
    pub probe_targets: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum PersistedActiveView {
    Logs,
    #[default]
    #[serde(other)]
    Workspace,
}

impl From<ActiveView> for PersistedActiveView {
    fn from(value: ActiveView) -> Self {
        match value {
            ActiveView::Workspace => Self::Workspace,
            ActiveView::Logs => Self::Logs,
        }
    }
}

impl From<PersistedActiveView> for ActiveView {
    fn from(value: PersistedActiveView) -> Self {
        match value {
            PersistedActiveView::Workspace => Self::Workspace,
            PersistedActiveView::Logs => Self::Logs,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct PersistedFileItem {
    path: PathBuf,
    output_name: String,
    selected_for_conversion: bool,
    status: PersistedFileStatus,
    conversion_error: Option<String>,
    config: ConversionConfig,
}

impl TryFrom<&FileItem> for PersistedFileItem {
    type Error = UpdateSessionError;

    fn try_from(file: &FileItem) -> Result<Self, Self::Error> {
        let status = PersistedFileStatus::try_from(file.status)?;
        Ok(Self {
            path: PathBuf::from(&file.path),
            output_name: file.output_name.clone(),
            selected_for_conversion: file.is_selected_for_conversion,
            status,
            conversion_error: if file.status == FileStatus::Error {
                file.conversion_error.clone()
            } else {
                None
            },
            config: file.config.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum PersistedFileStatus {
    #[default]
    Idle,
    Completed,
    Error,
}

impl TryFrom<FileStatus> for PersistedFileStatus {
    type Error = UpdateSessionError;

    fn try_from(value: FileStatus) -> Result<Self, UpdateSessionError> {
        match value {
            FileStatus::Idle => Ok(Self::Idle),
            FileStatus::Completed => Ok(Self::Completed),
            FileStatus::Error => Ok(Self::Error),
            status => Err(UpdateSessionError::UnsettledFileStatus(status.label())),
        }
    }
}

impl From<PersistedFileStatus> for FileStatus {
    fn from(value: PersistedFileStatus) -> Self {
        match value {
            PersistedFileStatus::Idle => Self::Idle,
            PersistedFileStatus::Completed => Self::Completed,
            PersistedFileStatus::Error => Self::Error,
        }
    }
}

#[derive(Debug, Error)]
pub enum UpdateSessionError {
    #[error("session persistence is unavailable")]
    PersistenceUnavailable,
    #[error("failed to read or write the pending update session: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse the pending update session: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported update session version {actual}; supported version is {supported}")]
    UnsupportedVersion { actual: u32, supported: u32 },
    #[error("the update session is missing the source or target app version")]
    MissingAppVersion,
    #[error("the update session contains {0} files, which exceeds the safety limit")]
    TooManyFiles(usize),
    #[error("the update session snapshot is too large ({0} bytes)")]
    SnapshotTooLarge(u64),
    #[error("the update session contains an empty source path")]
    MissingSourcePath,
    #[error("the update session contains a conversion error for a non-error file")]
    UnexpectedConversionError,
    #[error("cannot snapshot a file whose conversion state is {0}")]
    UnsettledFileStatus(&'static str),
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

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

        let restored =
            UpdateSessionSnapshot::capture(&queue, ActiveView::Logs, "0.31.1", "0.32.0", 42)
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

            let error = UpdateSessionSnapshot::capture(
                &queue,
                ActiveView::Workspace,
                "0.31.1",
                "0.32.0",
                42,
            )
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
        let snapshot =
            UpdateSessionSnapshot::capture(&queue, ActiveView::Logs, "0.31.1", "0.32.0", 42)
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
        let file = fs::File::create(&store.pending_path)
            .expect("oversized test snapshot should be created");
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
        let second = UpdateSessionSnapshot::capture(
            &FileQueue::new(),
            ActiveView::Logs,
            "0.31.1",
            "0.33.0",
            84,
        )
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
}
