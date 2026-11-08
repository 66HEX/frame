//! Background crop detection only proposes a draft; Apply remains the commit point.

use super::crop::{crop_rect_is_full, preview_crop_controls_enabled};
use crate::app::root::FrameRoot;
use crate::auto_crop::{AutoCropError, AutoCropRequest, DetectedCrop, detect_auto_crop};
use crate::preview::{CropRect, parse_time_to_seconds};
use crate::settings::{ProcessingMode, SourceKind};
use gpui::{AppContext, Context};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::app) enum AutoCropStatus {
    #[default]
    Idle,
    Analyzing,
    Ready,
    NoBars,
    Error(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::app) struct AutoCropRenderState {
    pub(in crate::app) available: bool,
    pub(in crate::app) status: AutoCropStatus,
}

struct AutoCropJob {
    file_id: String,
    request: AutoCropRequest,
    cancelled: Arc<AtomicBool>,
}

#[derive(Default)]
pub(in crate::app) struct AutoCropUiState {
    job: Option<AutoCropJob>,
    status: AutoCropStatus,
}

impl Drop for AutoCropUiState {
    fn drop(&mut self) {
        if let Some(job) = &self.job {
            job.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

impl FrameRoot {
    fn auto_crop_request(&self) -> Option<(String, AutoCropRequest)> {
        if !self.preview_ui.crop_mode || self.update_installation_in_progress() {
            return None;
        }
        let file = self.file_queue.selected_file()?;
        if file.config.processing_mode == ProcessingMode::Copy {
            return None;
        }
        let metadata = self.selected_source_metadata()?;
        if metadata.source_kind() != SourceKind::Video
            || !preview_crop_controls_enabled(
                Some(&metadata),
                &file.config,
                self.file_queue.selected_file_locked(),
            )
        {
            return None;
        }
        let duration = metadata.duration.as_deref()?.parse::<f64>().ok()?;
        if !duration.is_finite() || duration <= 0.0 {
            return None;
        }
        let start_seconds = file
            .config
            .start_time
            .as_deref()
            .map_or(0.0, parse_time_to_seconds);
        let end_seconds = file
            .config
            .end_time
            .as_deref()
            .map_or(duration, parse_time_to_seconds);
        if !start_seconds.is_finite() || !end_seconds.is_finite() {
            return None;
        }
        let start_seconds = start_seconds.max(0.0);
        let end_seconds = end_seconds.min(duration);
        if end_seconds <= start_seconds {
            return None;
        }
        Some((
            file.id.clone(),
            AutoCropRequest {
                path: file.path.clone(),
                video_stream_index: metadata.video_stream_index,
                width: metadata.width?,
                height: metadata.height?,
                start_seconds,
                end_seconds,
            },
        ))
    }

    pub(in crate::app) fn auto_crop_render_state(&self) -> AutoCropRenderState {
        AutoCropRenderState {
            available: self.auto_crop_request().is_some(),
            status: self.preview_ui.auto_crop.status.clone(),
        }
    }

    pub(in crate::app) fn cancel_selected_auto_crop(&mut self) -> bool {
        let state = &mut self.preview_ui.auto_crop;
        let changed = state.status != AutoCropStatus::Idle;
        if let Some(job) = state.job.take() {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        state.status = AutoCropStatus::Idle;
        changed
    }

    pub(in crate::app) fn sync_auto_crop_for_selection(&mut self) {
        let stale = self.preview_ui.auto_crop.job.as_ref().is_some_and(|job| {
            self.auto_crop_request()
                .is_none_or(|(file_id, request)| job.file_id != file_id || job.request != request)
        });
        if stale {
            self.cancel_selected_auto_crop();
        }
    }

    fn begin_auto_crop(&mut self) -> Option<(AutoCropRequest, Arc<AtomicBool>)> {
        let (file_id, request) = self.auto_crop_request()?;
        self.cancel_selected_auto_crop();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.preview_ui.auto_crop.job = Some(AutoCropJob {
            file_id,
            request: request.clone(),
            cancelled: cancelled.clone(),
        });
        self.preview_ui.auto_crop.status = AutoCropStatus::Analyzing;
        Some((request, cancelled))
    }

    pub(in crate::app) fn start_selected_auto_crop(&mut self, cx: &mut Context<Self>) {
        let Some((request, cancelled)) = self.begin_auto_crop() else {
            return;
        };
        cx.notify();
        cx.spawn(async move |this, cx| {
            let worker_cancel = cancelled.clone();
            let result = cx
                .background_spawn(async move { detect_auto_crop(&request, &worker_cancel) })
                .await;
            this.update(cx, |root, cx| {
                if root.complete_auto_crop(&cancelled, result) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn complete_auto_crop(
        &mut self,
        cancelled: &Arc<AtomicBool>,
        result: Result<DetectedCrop, AutoCropError>,
    ) -> bool {
        let Some(job) = self.preview_ui.auto_crop.job.as_ref() else {
            return false;
        };
        if !Arc::ptr_eq(&job.cancelled, cancelled) {
            return false;
        }
        if cancelled.load(Ordering::Relaxed)
            || self
                .auto_crop_request()
                .is_none_or(|(file_id, request)| file_id != job.file_id || request != job.request)
        {
            return self.cancel_selected_auto_crop();
        }
        let job = self
            .preview_ui
            .auto_crop
            .job
            .take()
            .expect("matched active job");
        match result {
            Ok(crop) => {
                let rect = CropRect {
                    x: f64::from(crop.x) / f64::from(job.request.width),
                    y: f64::from(crop.y) / f64::from(job.request.height),
                    width: f64::from(crop.width) / f64::from(job.request.width),
                    height: f64::from(crop.height) / f64::from(job.request.height),
                };
                self.preview_ui.crop_drag = None;
                self.preview_ui.crop_aspect = "free".to_string();
                self.preview_ui.crop_options = super::state::CropOptionsUiState::default();
                self.preview_ui.draft_crop = Some(rect);
                self.preview_ui.auto_crop.status = if crop_rect_is_full(rect) {
                    AutoCropStatus::NoBars
                } else {
                    AutoCropStatus::Ready
                };
            }
            Err(AutoCropError::Cancelled) => {
                self.preview_ui.auto_crop.status = AutoCropStatus::Idle;
            }
            Err(error) => {
                self.preview_ui.auto_crop.status = AutoCropStatus::Error(
                    match error {
                        AutoCropError::NoReliableCrop => {
                            "Could not detect a reliable crop. Adjust the frame manually."
                        }
                        AutoCropError::TimedOut => {
                            "Analysis took too long. Try again or adjust the frame manually."
                        }
                        AutoCropError::Unavailable(_) => {
                            "Auto crop is unavailable. Check your FFmpeg installation."
                        }
                        _ => {
                            "Could not analyze this video. Try again or adjust the frame manually."
                        }
                    }
                    .to_string(),
                );
            }
        }
        true
    }
}

#[cfg(test)]
mod tests;
