#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStatus {
    Idle,
    Queued,
    Converting,
    Paused,
    Cancelling,
    Completed,
    Error,
}

impl FileStatus {
    #[must_use]
    pub const fn locks_settings(self) -> bool {
        matches!(
            self,
            Self::Converting | Self::Queued | Self::Paused | Self::Cancelling | Self::Completed
        )
    }

    #[must_use]
    pub const fn can_be_cancelled(self) -> bool {
        matches!(self, Self::Converting | Self::Paused | Self::Queued)
    }

    #[must_use]
    pub const fn can_be_removed_from_list(self) -> bool {
        matches!(self, Self::Idle | Self::Completed | Self::Error)
    }

    #[must_use]
    pub const fn is_actionable_for_conversion(self) -> bool {
        matches!(self, Self::Idle | Self::Error)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "空闲",
            Self::Queued => "排队中",
            Self::Converting => "转换中",
            Self::Paused => "已暂停",
            Self::Cancelling => "取消中",
            Self::Completed => "就绪",
            Self::Error => "错误",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStateTone {
    Foreground,
    Muted,
    Blue,
    Amber,
    Red,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RowPrimaryAction {
    #[default]
    None,
    Pause,
    Resume,
    Reconvert,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RowSecondaryAction {
    #[default]
    None,
    Cancel,
    Delete,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RowActionAvailability {
    pub primary: RowPrimaryAction,
    pub secondary: RowSecondaryAction,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BatchSelectionState {
    pub is_checked: bool,
    pub is_indeterminate: bool,
    pub is_enabled: bool,
}
