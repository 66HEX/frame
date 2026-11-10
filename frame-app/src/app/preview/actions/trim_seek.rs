use crate::app::preview::panel::preview_source_media_kind;
use crate::app::preview::state::{TRIM_PREVIEW_SEEK_INTERVAL, TrimPreviewSeekRequest};
use crate::app::root::FrameRoot;
use crate::preview::SourceMediaKind;
use crate::preview_engine::PreviewEngineError;
use gpui::Context;

impl FrameRoot {
    pub(super) fn trim_preview_seek_available(&self) -> bool {
        self.preview_ui.session.is_some()
            && self.selected_source_metadata().is_some_and(|metadata| {
                preview_source_media_kind(&metadata) == SourceMediaKind::Video
            })
    }

    pub(super) fn queue_trim_preview_seek(
        &mut self,
        seconds: f64,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if !seconds.is_finite() || !self.trim_preview_seek_available() {
            return false;
        }

        self.preview_ui.trim_preview_seek.queue(seconds);
        if let Some(cx) = cx {
            self.start_trim_preview_seek_worker(cx);
        }
        true
    }

    pub(super) fn finish_trim_preview_seek(&mut self, cx: Option<&Context<Self>>) -> bool {
        if !self.preview_ui.trim_preview_seek.finish() {
            return false;
        }
        if let Some(cx) = cx {
            self.start_trim_preview_seek_worker(cx);
        }
        true
    }

    pub(super) fn cancel_trim_preview_seek(&mut self) -> bool {
        if !self.preview_ui.trim_preview_seek.is_active() {
            return false;
        }
        self.preview_ui.trim_preview_seek.reset();
        true
    }

    pub(super) fn start_trim_preview_seek_worker(&mut self, cx: &Context<Self>) {
        if self.preview_ui.trim_preview_seek.worker_active || self.preview_ui.session.is_none() {
            return;
        }
        self.preview_ui.trim_preview_seek.worker_active = true;
        self.queue_next_trim_preview_seek(self.preview_ui.trim_preview_seek.generation, cx);
    }

    fn queue_next_trim_preview_seek(&mut self, generation: u64, cx: &Context<Self>) {
        if self.preview_ui.trim_preview_seek.generation != generation {
            return;
        }
        let Some(request) = self.take_next_trim_preview_seek(generation) else {
            self.preview_ui.trim_preview_seek.worker_active = false;
            return;
        };
        // Use the same decoder worker as transport commands. A delayed trim
        // task can no longer acquire the session mutex after a final Seek/Play.
        self.queue_trim_media_command(request, generation, cx);
    }

    pub(super) fn schedule_next_trim_preview_seek(generation: u64, cx: &Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(TRIM_PREVIEW_SEEK_INTERVAL)
                .await;
            this.update(cx, |root, cx| {
                root.queue_next_trim_preview_seek(generation, cx);
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn take_next_trim_preview_seek(
        &mut self,
        generation: u64,
    ) -> Option<TrimPreviewSeekRequest> {
        let trim_preview_seek = &mut self.preview_ui.trim_preview_seek;
        if trim_preview_seek.generation != generation {
            return None;
        }
        trim_preview_seek.take_next()
    }

    pub(super) fn finish_trim_preview_seek_request(
        &mut self,
        generation: u64,
        seconds: f64,
        result: Result<(), PreviewEngineError>,
        cx: &mut Context<Self>,
    ) -> bool {
        let trim_preview_seek = &mut self.preview_ui.trim_preview_seek;
        if trim_preview_seek.generation != generation {
            return true;
        }

        match result {
            Ok(()) => {
                if self.preview_ui.failed_runtime_key.is_none() {
                    self.preview_ui.runtime_error = None;
                }
                self.schedule_preview_frame_tick(cx);
            }
            Err(error) => {
                self.preview_ui.runtime_error = Some(error.to_string());
            }
        }
        cx.notify();

        if self.preview_ui.trim_preview_seek.complete_restore(seconds) {
            return true;
        }

        false
    }
}
