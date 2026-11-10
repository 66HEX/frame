use frame_updater::{UpdateInfo, UpdatePackage};
use gpui::ScrollHandle;

pub(in crate::app) const UPDATE_INSTALL_WAIT_MESSAGE: &str =
    "Finish or cancel active conversions before installing the update.";

#[derive(Clone, Debug)]
pub(in crate::app) struct UpdateUiState {
    pub(in crate::app) status: UpdateStatus,
    pub(in crate::app) dialog_open: bool,
    pub(in crate::app) dialog_present: bool,
    pub(in crate::app) dialog_info: Option<Box<UpdateInfo>>,
    pub(in crate::app) release_notes_scroll_handle: ScrollHandle,
    pub(in crate::app) status_dismiss_epoch: u64,
}

impl Default for UpdateUiState {
    fn default() -> Self {
        Self {
            status: UpdateStatus::default(),
            dialog_open: false,
            dialog_present: false,
            dialog_info: None,
            release_notes_scroll_handle: ScrollHandle::new(),
            status_dismiss_epoch: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(in crate::app) enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Box<UpdateInfo>),
    Downloading {
        version: String,
        progress_percent: Option<u8>,
        received_bytes: u64,
        total_bytes: Option<u64>,
    },
    ReadyToInstall(Box<UpdatePackage>),
    Installing,
    Disabled(String),
    Error(String),
}

impl UpdateStatus {
    pub(in crate::app) const fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading { .. } | Self::Installing
        )
    }
}
