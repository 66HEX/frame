use gpui::FocusHandle;

pub(in crate::app) const SOURCE_FRAME_DESCRIPTION: &str =
    "Save a full-resolution PNG from the source, without conversion settings.";

#[derive(Clone, Copy)]
pub(in crate::app) struct SourceFrameExportFocuses<'a> {
    pub(in crate::app) export: &'a FocusHandle,
}

mod dialog;

#[cfg(test)]
mod tests;
