use crate::app::root::FrameRoot;
use crate::conversion_runner::ConversionProcessController;
use crate::frame_export::{SourceFrameExportOutcome, SourceFrameExportRequest};
use crate::preview_engine::{PreviewSessionSnapshot, PreviewSessionStatus};
use crate::settings::SourceKind;
use frame_core::error::ConversionError;
use gpui::{AppContext, Context, Window};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::app) enum SourceFrameExportStatus {
    #[default]
    Idle,
    Exporting,
    Cancelling,
    Saved,
    Cancelled,
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::app) struct SourceFrameExportResult {
    pub(in crate::app) source_id: String,
    pub(in crate::app) path: PathBuf,
}

struct SourceFrameExportJob {
    request: SourceFrameExportRequest,
    controller: ConversionProcessController,
}

#[derive(Default)]
pub(in crate::app) struct SourceFrameExportUiState {
    job: Option<SourceFrameExportJob>,
    pub(in crate::app) status: SourceFrameExportStatus,
    pub(in crate::app) result: Option<SourceFrameExportResult>,
    pub(in crate::app) dialog: crate::app::motion::DialogPresence,
}

impl SourceFrameExportUiState {
    fn begin(
        &mut self,
        request: SourceFrameExportRequest,
    ) -> Result<ConversionProcessController, ConversionError> {
        if self.job.is_some() {
            return Err(ConversionError::InvalidInput(
                "A source frame export is already running.".into(),
            ));
        }
        let controller = ConversionProcessController::default();
        controller.begin_queued_attempts(std::slice::from_ref(&request.id))?;
        self.job = Some(SourceFrameExportJob {
            request,
            controller: controller.clone(),
        });
        self.status = SourceFrameExportStatus::Exporting;
        self.dialog = crate::app::motion::DialogPresence::default();
        Ok(controller)
    }

    fn cancel(&mut self) -> Result<bool, ConversionError> {
        let Some(job) = &self.job else {
            return Ok(false);
        };
        if self.status == SourceFrameExportStatus::Cancelling {
            return Ok(false);
        }
        job.controller.cancel_task(&job.request.id)?;
        self.status = SourceFrameExportStatus::Cancelling;
        Ok(true)
    }

    fn complete(
        &mut self,
        id: &str,
        result: Result<SourceFrameExportOutcome, ConversionError>,
    ) -> bool {
        if self.job.as_ref().is_none_or(|job| job.request.id != id) {
            return false;
        }
        let Some(job) = self.job.take() else {
            return false;
        };
        self.status = match result {
            Ok(SourceFrameExportOutcome::Published { path }) => {
                self.result = Some(SourceFrameExportResult {
                    source_id: job.request.source_id,
                    path,
                });
                SourceFrameExportStatus::Saved
            }
            Ok(SourceFrameExportOutcome::Cancelled) => SourceFrameExportStatus::Cancelled,
            Err(error) => SourceFrameExportStatus::Error(error.to_string()),
        };
        if self.status != SourceFrameExportStatus::Cancelled {
            self.dialog.open();
        }
        true
    }
}

impl Drop for SourceFrameExportUiState {
    fn drop(&mut self) {
        if let Some(job) = &self.job {
            let _ = job.controller.cancel_task(&job.request.id);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::app) struct SourceFrameExportIntent {
    pub(in crate::app) source_id: String,
    pub(in crate::app) source_path: PathBuf,
    pub(in crate::app) position_microseconds: u64,
    pub(in crate::app) video_stream_index: u32,
    pub(in crate::app) output_directory: PathBuf,
}

struct SourceFrameContext<'a> {
    file: &'a crate::file_queue::FileItem,
    kind: SourceKind,
    video_stream_index: u32,
    output_directory: &'a Path,
}

fn source_frame_context(
    root: &FrameRoot,
    flatpak: bool,
) -> Result<SourceFrameContext<'_>, &'static str> {
    if root.preview_ui.source_export.job.is_some() {
        return Err("Wait for the current source frame export to finish.");
    }
    if flatpak {
        return Err("Source frame export is unavailable in Flatpak.");
    }
    if root.update_installation_in_progress() {
        return Err("Wait for the update to finish.");
    }
    let file = root
        .file_queue
        .selected_file()
        .ok_or("Choose a video or image.")?;
    let metadata = root
        .source_metadata
        .ready_metadata_for(&file.id)
        .ok_or("Wait for the source details to load.")?;
    let source_kind = metadata.source_kind();
    if !matches!(source_kind, SourceKind::Video | SourceKind::Image) {
        return Err("Choose a video or image.");
    }
    let video_stream_index = metadata
        .video_stream_index
        .ok_or("No video stream is available.")?;
    let output_directory = root
        .default_output_directory
        .as_ref()
        .ok_or("Choose an output folder.")?;
    let preview = &root.preview_ui;
    if preview.crop_mode || preview.overlay.overlay_mode() {
        return Err("Finish editing crop or overlay before exporting a source frame.");
    }
    if preview.pending_runtime_key.is_some() || root.preview_media_snapshot_sync_blocked() {
        return Err("Wait for the preview to reach the selected position.");
    }
    if preview.playback_file_id.as_deref() != Some(file.id.as_str())
        || preview
            .runtime_key
            .as_ref()
            .is_none_or(|key| (&key.file_id, &key.path) != (&file.id, &file.path))
    {
        return Err("Wait for the selected source preview.");
    }
    Ok(SourceFrameContext {
        file,
        kind: source_kind,
        video_stream_index,
        output_directory,
    })
}

fn source_frame_intent(
    root: &FrameRoot,
    flatpak: bool,
    snapshot: Option<&PreviewSessionSnapshot>,
) -> Result<SourceFrameExportIntent, &'static str> {
    let source = source_frame_context(root, flatpak)?;
    let preview = &root.preview_ui;
    let seconds = if source.kind == SourceKind::Image {
        0.0
    } else if preview.playback.is_playing() {
        let snapshot = snapshot.ok_or("Wait for the playback clock.")?;
        if snapshot.file_id != source.file.id || snapshot.status != PreviewSessionStatus::Ready {
            return Err("Wait for the selected source preview.");
        }
        snapshot.playback.position_seconds
    } else {
        preview.playback.current_time()
    };
    let position_microseconds = Duration::try_from_secs_f64(seconds)
        .ok()
        .and_then(|position| u64::try_from(position.as_micros()).ok())
        .ok_or("The source position cannot be exported.")?;
    Ok(SourceFrameExportIntent {
        source_id: source.file.id.clone(),
        source_path: PathBuf::from(&source.file.path),
        position_microseconds,
        video_stream_index: source.video_stream_index,
        output_directory: source.output_directory.to_path_buf(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct SourceFrameExportRenderState {
    pub(in crate::app) visible: bool,
    pub(in crate::app) disabled_reason: Option<&'static str>,
    pub(in crate::app) status: SourceFrameExportStatus,
    pub(in crate::app) running: bool,
    pub(in crate::app) result_name: Option<String>,
}

impl SourceFrameExportRenderState {
    pub(in crate::app) fn control_enabled(&self) -> bool {
        if self.running {
            self.status != SourceFrameExportStatus::Cancelling
        } else {
            self.disabled_reason.is_none()
        }
    }
}

impl FrameRoot {
    pub(in crate::app) const fn source_frame_export_running(&self) -> bool {
        self.preview_ui.source_export.job.is_some()
    }

    pub(in crate::app) fn source_frame_export_render_state(&self) -> SourceFrameExportRenderState {
        let visible = !crate::runtime_environment::is_flatpak()
            && (self.source_frame_export_running()
                || (self.default_output_directory.is_some()
                    && self.file_queue.selected_file().is_some_and(|file| {
                        self.source_metadata
                            .ready_metadata_for(&file.id)
                            .is_some_and(|metadata| {
                                matches!(
                                    metadata.source_kind(),
                                    SourceKind::Video | SourceKind::Image
                                ) && metadata.video_stream_index.is_some()
                            })
                    })));
        SourceFrameExportRenderState {
            visible,
            disabled_reason: source_frame_context(self, crate::runtime_environment::is_flatpak())
                .err(),
            status: self.preview_ui.source_export.status.clone(),
            running: self.source_frame_export_running(),
            result_name: self.preview_ui.source_export.result.as_ref().map(|result| {
                result.path.file_name().map_or_else(
                    || "Source frame".into(),
                    |name| name.to_string_lossy().into_owned(),
                )
            }),
        }
    }

    pub(in crate::app) fn start_source_frame_export(&mut self, cx: &mut Context<Self>) {
        if self.preview_ui.source_export.job.is_some() {
            return;
        }
        let snapshot = self
            .preview_ui
            .playback
            .is_playing()
            .then(|| {
                self.preview_ui
                    .session
                    .as_ref()
                    .map(|session| session.snapshot())
            })
            .flatten();
        let request = source_frame_intent(
            self,
            crate::runtime_environment::is_flatpak(),
            snapshot.as_ref(),
        )
        .map_err(str::to_owned)
        .and_then(|intent| {
            SourceFrameExportRequest::new(
                intent.source_id,
                intent.source_path,
                intent.position_microseconds,
                intent.video_stream_index,
                intent.output_directory,
            )
            .map_err(|error| error.to_string())
        });
        let request = match request {
            Ok(request) => request,
            Err(error) => {
                self.preview_ui.source_export.status = SourceFrameExportStatus::Error(error);
                self.preview_ui.source_export.dialog.open();
                cx.notify();
                return;
            }
        };
        let controller = match self.preview_ui.source_export.begin(request.clone()) {
            Ok(controller) => controller,
            Err(error) => {
                self.preview_ui.source_export.status =
                    SourceFrameExportStatus::Error(error.to_string());
                self.preview_ui.source_export.dialog.open();
                cx.notify();
                return;
            }
        };
        cx.notify();
        #[cfg(test)]
        if test_support::capture_export(cx, &request) {
            return;
        }
        cx.spawn(async move |this, cx| {
            let id = request.id.clone();
            let result = cx
                .background_spawn(async move {
                    crate::frame_export::export_source_frame(&request, &controller)
                })
                .await;
            this.update(cx, |root, cx| {
                if root.preview_ui.source_export.complete(&id, result) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(in crate::app) fn cancel_source_frame_export(&mut self, cx: &mut Context<Self>) {
        match self.preview_ui.source_export.cancel() {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(error) => {
                self.preview_ui.source_export.status =
                    SourceFrameExportStatus::Error(error.to_string());
                cx.notify();
            }
        }
    }

    pub(in crate::app) fn open_source_frame_result(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.access_source_frame_result(false, window, cx);
    }

    pub(in crate::app) fn reveal_source_frame_result(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.access_source_frame_result(true, window, cx);
    }

    fn access_source_frame_result(
        &self,
        reveal: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = &self.preview_ui.source_export.result else {
            return;
        };
        let path = &result.path;
        let (title, detail) = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => {
                #[cfg(test)]
                test_support::capture_result_action(cx, path, reveal);
                #[cfg(not(test))]
                if reveal {
                    cx.reveal_path(path);
                } else {
                    cx.open_with_system(path);
                }
                return;
            }
            Ok(_) => (
                "Source frame is not a file",
                format!(
                    "The saved source frame path is no longer a file:\n{}\nExport the source frame again to create a new result.",
                    path.display()
                ),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                "Source frame not found",
                format!(
                    "The source frame may have been moved or deleted:\n{}\nExport the source frame again to create a new result.",
                    path.display()
                ),
            ),
            Err(error) => (
                "Unable to access source frame",
                format!(
                    "Unable to access {}: {error}\nCheck that the drive is available and you have permission to open the file.",
                    path.display()
                ),
            ),
        };
        let response = window.prompt(
            gpui::PromptLevel::Warning,
            title,
            Some(&detail),
            &[gpui::PromptButton::cancel("Cancel")],
            cx,
        );
        cx.spawn(async move |_, _| {
            let _ = response.await;
        })
        .detach();
    }
}

#[cfg(test)]
pub(in crate::app) mod test_support;

#[cfg(test)]
mod tests;
