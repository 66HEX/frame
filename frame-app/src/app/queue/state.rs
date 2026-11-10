pub(in crate::app) const FILE_LIST_ACTIONS_WIDTH: f32 = 64.0;

pub(in crate::app) const FILE_LIST_ACTION_BUTTON_SIZE: f32 = 24.0;

pub(in crate::app) const FILE_LIST_ACTION_ICON_SIZE: f32 = 12.0;

pub(in crate::app) const ROOT_DROP_GROUP: &str = "frame-root-drop-target";

#[derive(Default)]
pub(in crate::app) struct DragDropUiState {
    pub(in crate::app) is_open: bool,
    pub(in crate::app) is_present: bool,
}
