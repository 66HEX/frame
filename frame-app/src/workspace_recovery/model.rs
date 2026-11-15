use super::RecoveryError;
use crate::ActiveView;
use crate::app_info::FRAME_APP_VERSION;
use crate::file_queue::{FileItem, FileQueue, FileStatus};
use crate::settings::ConversionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_FILES: usize = 10_000;
pub(super) const INTERRUPTED: &str = "Interrupted when Frame closed. Start again when ready.";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySnapshot {
    version: u32,
    app_version: String,
    pub(super) session_id: String,
    pub(super) revision: u64,
    captured_at: u64,
    active_view: SavedView,
    selected_file_id: Option<String>,
    default_output_directory: Option<PathBuf>,
    files: Vec<SavedFile>,
}

impl RecoverySnapshot {
    pub fn capture(
        queue: &FileQueue,
        active_view: ActiveView,
        default_output_directory: Option<PathBuf>,
        session_id: impl Into<String>,
        revision: u64,
        captured_at: u64,
    ) -> Result<Self, RecoveryError> {
        let snapshot = Self {
            version: 1,
            app_version: FRAME_APP_VERSION.to_string(),
            session_id: session_id.into(),
            revision,
            captured_at,
            active_view: match active_view {
                ActiveView::Workspace => SavedView::Workspace,
                ActiveView::Logs => SavedView::Logs,
            },
            selected_file_id: queue.selected_file_id().map(str::to_owned),
            default_output_directory,
            files: queue.files().iter().map(SavedFile::from).collect(),
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn restore(self) -> Result<RestoredWorkspace, RecoveryError> {
        self.validate()?;
        let mut queue = FileQueue::new();
        let mut probe_targets = Vec::new();
        let mut interrupted_count = 0;
        let mut missing_source_count = 0;
        let mut next_file_sequence = 0;
        for saved in self.files {
            next_file_sequence = next_file_sequence.max(file_sequence(&saved.id));
            let source_exists = Path::new(&saved.path).is_file();
            let interrupted = saved.status.is_interrupted();
            interrupted_count += usize::from(interrupted);
            missing_source_count += usize::from(!source_exists);
            let mut file = FileItem::from_os_path(saved.id, Path::new(&saved.path));
            file.output_name = saved.output_name;
            file.config = saved.config;
            file.is_selected_for_conversion = saved.selected_for_conversion && source_exists;
            if saved.status == SavedStatus::Completed {
                file.status = FileStatus::Completed;
                file.progress_percent = 100;
                file.output_path = saved.output_path;
            } else if !source_exists {
                file.status = FileStatus::Error;
                file.conversion_error = Some(if interrupted {
                    "Interrupted when Frame closed. Source file is no longer available.".into()
                } else {
                    "Source file is no longer available.".into()
                });
            } else if interrupted {
                file.status = FileStatus::Error;
                file.conversion_error = Some(INTERRUPTED.into());
            } else if saved.status == SavedStatus::Error {
                file.status = FileStatus::Error;
                file.conversion_error = Some(saved.conversion_error.unwrap_or_else(|| {
                    "The previous conversion did not complete successfully.".into()
                }));
            }
            if source_exists {
                probe_targets.push((file.id.clone(), file.path.clone()));
            }
            queue.add_file(file);
        }
        queue.select_file(self.selected_file_id);
        Ok(RestoredWorkspace {
            queue,
            active_view: match self.active_view {
                SavedView::Workspace => ActiveView::Workspace,
                SavedView::Logs => ActiveView::Logs,
            },
            default_output_directory: self.default_output_directory,
            next_file_sequence,
            probe_targets,
            interrupted_count,
            missing_source_count,
        })
    }

    #[must_use]
    pub fn same_workspace_as(&self, other: &Self) -> bool {
        self.active_view == other.active_view
            && self.selected_file_id == other.selected_file_id
            && self.default_output_directory == other.default_output_directory
            && self.files == other.files
    }

    #[must_use]
    pub const fn file_count(&self) -> usize {
        self.files.len()
    }

    pub(super) fn validate(&self) -> Result<(), RecoveryError> {
        if self.version != 1 {
            return Err(RecoveryError::UnsupportedVersion(self.version));
        }
        if self.app_version.trim().is_empty()
            || self.session_id.trim().is_empty()
            || self.revision == 0
        {
            return Err(RecoveryError::Invalid(
                "missing version, session or revision",
            ));
        }
        if self.files.len() > MAX_FILES {
            return Err(RecoveryError::TooLarge);
        }
        let mut ids = HashSet::new();
        for file in &self.files {
            if file.path.trim().is_empty() || file.id.trim().is_empty() || !ids.insert(&file.id) {
                return Err(RecoveryError::Invalid(
                    "empty source path or duplicate file ID",
                ));
            }
            if file_sequence(&file.id) > u64::MAX - 10_000 {
                return Err(RecoveryError::Invalid("file ID sequence exhausted"));
            }
            if (file.status == SavedStatus::Completed) != file.output_path.is_some()
                || file
                    .output_path
                    .as_ref()
                    .is_some_and(|path| path.trim().is_empty())
            {
                return Err(RecoveryError::Invalid("unconfirmed published output"));
            }
        }
        if self
            .selected_file_id
            .as_ref()
            .is_some_and(|id| !ids.contains(id))
        {
            return Err(RecoveryError::Invalid("unknown selected file ID"));
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RestoredWorkspace {
    pub queue: FileQueue,
    pub active_view: ActiveView,
    pub default_output_directory: Option<PathBuf>,
    pub next_file_sequence: u64,
    pub probe_targets: Vec<(String, String)>,
    pub interrupted_count: usize,
    pub missing_source_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum SavedView {
    Workspace,
    Logs,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedFile {
    id: String,
    path: String,
    output_name: String,
    selected_for_conversion: bool,
    status: SavedStatus,
    conversion_error: Option<String>,
    output_path: Option<String>,
    config: ConversionConfig,
}

impl From<&FileItem> for SavedFile {
    fn from(file: &FileItem) -> Self {
        let missing_result = file.status == FileStatus::Completed && file.output_path.is_none();
        Self {
            id: file.id.clone(),
            path: file.path.clone(),
            output_name: file.output_name.clone(),
            selected_for_conversion: file.is_selected_for_conversion && !missing_result,
            status: match file.status {
                FileStatus::Idle => SavedStatus::Idle,
                FileStatus::Queued => SavedStatus::Queued,
                FileStatus::Converting => SavedStatus::Converting,
                FileStatus::Paused => SavedStatus::Paused,
                FileStatus::Cancelling => SavedStatus::Cancelling,
                FileStatus::Completed if missing_result => SavedStatus::Error,
                FileStatus::Completed => SavedStatus::Completed,
                FileStatus::Error => SavedStatus::Error,
            },
            conversion_error: if missing_result {
                Some(
                    "The previous result path was not recorded. Select this file to convert again."
                        .into(),
                )
            } else {
                file.conversion_error.clone()
            },
            output_path: file.completed_output_path().map(str::to_owned),
            config: file.config.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum SavedStatus {
    Idle,
    Queued,
    Converting,
    Paused,
    Cancelling,
    Completed,
    Error,
}

fn file_sequence(id: &str) -> u64 {
    id.strip_prefix("file-")
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

impl SavedStatus {
    const fn is_interrupted(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Converting | Self::Paused | Self::Cancelling
        )
    }
}
