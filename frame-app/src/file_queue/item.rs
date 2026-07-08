use std::path::Path;

use crate::settings::ConversionConfig;

use super::{
    format::{derive_output_name, file_name_from_path, file_size_bytes, original_format_from_name},
    status::{
        FileStateTone, FileStatus, RowActionAvailability, RowPrimaryAction, RowSecondaryAction,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub status: FileStatus,
    pub progress_percent: u8,
    pub original_format: String,
    pub output_name: String,
    pub config: ConversionConfig,
    pub path: String,
    pub is_selected_for_conversion: bool,
    pub conversion_error: Option<String>,
}

impl FileItem {
    #[must_use]
    pub fn from_path(id: impl Into<String>, path: impl Into<String>, size_bytes: u64) -> Self {
        let path = path.into();
        let name = file_name_from_path(&path).to_string();
        Self {
            id: id.into(),
            original_format: original_format_from_name(&name).to_string(),
            output_name: derive_output_name(&name),
            config: ConversionConfig::default(),
            name,
            size_bytes,
            status: FileStatus::Idle,
            progress_percent: 0,
            path,
            is_selected_for_conversion: true,
            conversion_error: None,
        }
    }

    #[must_use]
    pub fn from_os_path(id: impl Into<String>, path: &Path) -> Self {
        Self::from_path(id, path.to_string_lossy(), file_size_bytes(path))
    }

    #[must_use]
    pub const fn locks_settings(&self) -> bool {
        self.status.locks_settings()
    }

    #[must_use]
    pub fn row_state_label(&self) -> String {
        match self.status {
            FileStatus::Converting | FileStatus::Paused | FileStatus::Cancelling => {
                format!("{}%", self.progress_percent)
            }
            FileStatus::Completed => "ready".to_string(),
            FileStatus::Queued => "queued".to_string(),
            FileStatus::Error => "error".to_string(),
            FileStatus::Idle => "idle".to_string(),
        }
    }

    #[must_use]
    pub const fn row_state_tone(&self) -> FileStateTone {
        match self.status {
            FileStatus::Converting => FileStateTone::Amber,
            FileStatus::Completed => FileStateTone::Blue,
            FileStatus::Error => FileStateTone::Red,
            FileStatus::Idle | FileStatus::Queued | FileStatus::Paused | FileStatus::Cancelling => {
                FileStateTone::Muted
            }
        }
    }

    #[must_use]
    pub const fn row_actions(&self) -> RowActionAvailability {
        match self.status {
            FileStatus::Queued => RowActionAvailability {
                primary: RowPrimaryAction::None,
                secondary: RowSecondaryAction::Cancel,
            },
            FileStatus::Converting => RowActionAvailability {
                primary: RowPrimaryAction::Pause,
                secondary: RowSecondaryAction::Cancel,
            },
            FileStatus::Paused => RowActionAvailability {
                primary: RowPrimaryAction::Resume,
                secondary: RowSecondaryAction::Cancel,
            },
            FileStatus::Completed => RowActionAvailability {
                primary: RowPrimaryAction::Reconvert,
                secondary: RowSecondaryAction::Delete,
            },
            FileStatus::Idle | FileStatus::Error => RowActionAvailability {
                primary: RowPrimaryAction::None,
                secondary: RowSecondaryAction::Delete,
            },
            FileStatus::Cancelling => RowActionAvailability {
                primary: RowPrimaryAction::None,
                secondary: RowSecondaryAction::None,
            },
        }
    }
}
