//! Persistence model for restoring Frame's workspace after an app update.

use crate::ActiveView;
use crate::app_persistence::write_bytes_atomically;
use crate::file_queue::{FileItem, FileQueue, FileStatus};
use crate::settings::ConversionConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

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
mod tests;
