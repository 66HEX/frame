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
            Self::Idle => "Idle",
            Self::Queued => "Queued",
            Self::Converting => "Converting",
            Self::Paused => "Paused",
            Self::Cancelling => "Cancelling",
            Self::Completed => "Ready",
            Self::Error => "Error",
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
