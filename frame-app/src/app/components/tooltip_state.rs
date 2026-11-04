use std::time::Instant;

#[derive(Default)]
pub(in crate::app) struct TooltipUiState {
    pub(in crate::app) hovered_id: Option<String>,
    pub(in crate::app) visible_id: Option<String>,
    pub(in crate::app) warm_until: Option<Instant>,
    pub(in crate::app) hover_epoch: u64,
}
