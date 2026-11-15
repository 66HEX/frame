use super::*;
use gpui::{App, Global};
use std::cell::RefCell;

#[derive(Default)]
pub(in crate::app) struct SourceFrameRequests {
    pub(in crate::app) source_directory: Option<PathBuf>,
    pub(in crate::app) exports: RefCell<Vec<SourceFrameExportRequest>>,
    pub(in crate::app) opened: RefCell<Vec<PathBuf>>,
}

impl Global for SourceFrameRequests {}

impl Drop for SourceFrameRequests {
    fn drop(&mut self) {
        if let Some(path) = &self.source_directory {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

pub(in crate::app) fn capture_export(cx: &App, request: &SourceFrameExportRequest) -> bool {
    let Some(requests) = cx.try_global::<SourceFrameRequests>() else {
        return false;
    };
    requests.exports.borrow_mut().push(request.clone());
    true
}

pub(in crate::app) fn capture_open(cx: &App, path: &Path) {
    if let Some(requests) = cx.try_global::<SourceFrameRequests>() {
        requests.opened.borrow_mut().push(path.to_path_buf());
    }
}

pub(in crate::app) fn begin(
    root: &mut FrameRoot,
    request: SourceFrameExportRequest,
) -> ConversionProcessController {
    root.preview_ui.source_export.begin(request).unwrap()
}

pub(in crate::app) fn complete(
    root: &mut FrameRoot,
    id: &str,
    outcome: SourceFrameExportOutcome,
) -> bool {
    root.preview_ui.source_export.complete(id, Ok(outcome))
}
