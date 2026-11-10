#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::app) enum PopoverState {
    #[default]
    Hidden,
    Closing,
    Open,
}

impl PopoverState {
    pub(in crate::app) const fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }

    pub(in crate::app) const fn is_rendered(self) -> bool {
        !matches!(self, Self::Hidden)
    }
}
