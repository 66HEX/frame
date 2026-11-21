use super::actions::source_export::SourceFrameExportRenderState;
use gpui::FocusHandle;

pub(in crate::app) const SOURCE_FRAME_DESCRIPTION: &str =
    "Save a full-resolution PNG from the source, without conversion settings.";

#[derive(Clone, Copy)]
pub(in crate::app) struct SourceFrameExportFocuses<'a> {
    pub(in crate::app) export: &'a FocusHandle,
}

#[derive(Clone, Copy)]
pub(in crate::app) enum SourceFrameToolbarAction {
    Export,
    Cancel,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct SourceFrameExportControl<'a> {
    pub(in crate::app) focus: &'a FocusHandle,
    pub(in crate::app) enabled: bool,
    pub(in crate::app) action: SourceFrameToolbarAction,
}

impl SourceFrameExportRenderState {
    pub(in crate::app) fn toolbar_control<'a>(
        &self,
        focus: &'a FocusHandle,
    ) -> SourceFrameExportControl<'a> {
        SourceFrameExportControl {
            focus,
            enabled: self.control_enabled(),
            action: if self.running {
                SourceFrameToolbarAction::Cancel
            } else {
                SourceFrameToolbarAction::Export
            },
        }
    }
}

mod dialog;

#[cfg(test)]
mod tests;
