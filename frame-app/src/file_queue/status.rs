#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStatus {
    Idle,
    Queued,
    Converting,
    Paused,
    Completed,
    Error,
}

impl FileStatus {
    #[must_use]
    pub const fn locks_settings(self) -> bool {
        matches!(self, Self::Converting | Self::Queued | Self::Completed)
    }

    #[must_use]
    pub const fn can_be_cancelled_before_removal(self) -> bool {
        matches!(self, Self::Converting | Self::Paused | Self::Queued)
    }

    #[must_use]
    pub const fn can_be_removed_from_list(self) -> bool {
        !matches!(self, Self::Converting)
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
pub struct RowActionAvailability {
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_delete: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BatchSelectionState {
    pub is_checked: bool,
    pub is_indeterminate: bool,
    pub is_enabled: bool,
}
