use crate::app::preview::actions::request::preview_runtime_request;
use crate::app::preview::state::{PreviewRuntimeKey, PreviewRuntimeRequest};
use crate::app::root::FrameRoot;
use crate::preview_engine::{PreviewEngineError, PreviewRenderPresentation, PreviewSession};
use crate::source_metadata::SourceMetadataEntry;
use gpui::prelude::*;
use gpui::{Context, RenderImage};
use std::sync::Arc;

impl FrameRoot {
    pub(in crate::app) fn selected_preview_runtime_request(
        &mut self,
        metadata_entry: &SourceMetadataEntry,
    ) -> Option<PreviewRuntimeRequest> {
        let selected_file_id = self.file_queue.selected_file()?.id.clone();
        let preview_dimensions = self.selected_preview_runtime_dimensions(&selected_file_id);
        let selected_file = self.file_queue.selected_file()?;
        preview_runtime_request(
            selected_file,
            metadata_entry,
            !self.preview_ui.crop_mode,
            !self.preview_ui.overlay.overlay_mode(),
            preview_dimensions,
        )
    }

    pub(in crate::app) fn sync_preview_runtime_for_selection(
        &mut self,
        request: Option<PreviewRuntimeRequest>,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = request else {
            self.clear_preview_runtime_for_selection(cx);
            self.preview_ui.render_presentation = PreviewRenderPresentation::default();
            self.preview_ui.rendered_presentation = PreviewRenderPresentation::default();
            return;
        };

        let presentation_changed = self.preview_ui.render_presentation != request.presentation;
        self.preview_ui.render_presentation = request.presentation;

        let next_key = Some(request.key.clone());
        if self.preview_ui.runtime_key == next_key
            || self.preview_ui.pending_runtime_key == next_key
        {
            if presentation_changed {
                self.preview_ui.rendered_presentation = request.presentation;
                self.apply_preview_canvas_auto_fit();
                cx.notify();
            }
            return;
        }

        // Keep one rebuild per source in flight. On completion the next render
        // requests the latest preset, skipping intermediate selections.
        if self
            .preview_ui
            .pending_runtime_key
            .as_ref()
            .is_some_and(|pending| pending.can_reconfigure_to(&request.key))
        {
            return;
        }

        if self.should_defer_filter_preview_reconfigure(&request.key) {
            return;
        }

        let session = self.preview_ui.session.clone().filter(|_| {
            self.preview_ui
                .runtime_key
                .as_ref()
                .is_some_and(|current| current.can_reconfigure_to(&request.key))
        });
        if session.is_none() {
            self.clear_preview_runtime_for_selection(cx);
        }

        let key = request.key.clone();
        self.preview_ui.runtime_request_generation =
            self.preview_ui.runtime_request_generation.wrapping_add(1);
        let generation = self.preview_ui.runtime_request_generation;
        self.preview_ui.pending_runtime_key = Some(key.clone());
        cx.spawn(async move |this, cx| {
            let config = request.config;
            let result = cx
                .background_spawn(async move {
                    if let Some(session) = session {
                        session.reconfigure(config)?;
                        Ok(session)
                    } else {
                        PreviewSession::start(config).map(Arc::new)
                    }
                })
                .await;

            // Retain ownership outside the UI callback: a rejected result (or
            // a closed window) must also release its processes in the background.
            let accepted = this
                .update(cx, |root, cx| {
                    root.finish_preview_runtime_request(generation, &key, &result, cx)
                })
                .unwrap_or(false);
            if !accepted && let Ok(session) = result {
                cx.background_spawn(async move {
                    if let Err(error) = session.stop() {
                        eprintln!("Failed to stop the stale FFmpeg preview runtime: {error}");
                    }
                })
                .detach();
            }
        })
        .detach();
    }

    pub(super) fn finish_preview_runtime_request(
        &mut self,
        generation: u64,
        key: &PreviewRuntimeKey,
        result: &Result<Arc<PreviewSession>, PreviewEngineError>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.preview_ui.runtime_request_generation != generation
            || self.preview_ui.pending_runtime_key.as_ref() != Some(key)
        {
            return false;
        }

        self.preview_ui.pending_runtime_key = None;
        match result {
            Ok(session) => {
                self.preview_ui.runtime_key = Some(key.clone());
                self.preview_ui.session = Some(Arc::clone(session));
                self.preview_ui.runtime_error = None;
                self.refresh_preview_render_image(cx);
                self.schedule_preview_frame_tick(cx);
            }
            Err(error) => {
                self.preview_ui.runtime_error = Some(error.to_string());
            }
        }
        cx.notify();
        true
    }

    pub(in crate::app) fn preview_render_image(&self) -> Option<Arc<RenderImage>> {
        self.preview_ui.render_image.clone()
    }

    pub(in crate::app) fn preview_runtime_error(&self) -> Option<String> {
        self.preview_ui.runtime_error.clone()
    }

    pub(in crate::app) fn clear_preview_runtime(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<(), PreviewEngineError>> {
        let session = self.preview_ui.session.take();
        if let Some(image) = self.preview_ui.render_image.take() {
            cx.drop_image(image, None);
        }
        self.preview_ui.active_preview_dimensions = None;
        self.preview_ui.preview_dimensions_debounce_until = None;
        self.preview_ui.runtime_key = None;
        self.preview_ui.pending_runtime_key = None;
        self.preview_ui.runtime_request_generation =
            self.preview_ui.runtime_request_generation.wrapping_add(1);
        self.preview_ui.media_command_generation =
            self.preview_ui.media_command_generation.wrapping_add(1);
        self.preview_ui.trim_preview_seek.reset();
        self.preview_ui.render_generation = 0;
        self.preview_ui.runtime_error = None;
        session.map_or_else(
            || gpui::Task::ready(Ok(())),
            |session| cx.background_spawn(async move { session.stop() }),
        )
    }

    pub(super) fn clear_preview_runtime_for_selection(&mut self, cx: &mut Context<Self>) {
        let stop = self.clear_preview_runtime(cx);
        cx.background_spawn(async move {
            if let Err(error) = stop.await {
                eprintln!("Failed to stop the FFmpeg preview runtime: {error}");
            }
        })
        .detach();
    }

    pub(super) fn refresh_preview_render_image(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = &self.preview_ui.session else {
            return false;
        };
        let Some(latest) = session.latest_frame() else {
            return false;
        };
        if latest.generation == self.preview_ui.render_generation {
            return false;
        }

        let image = latest.frame.render_image();
        if let Some(previous) = self.preview_ui.render_image.replace(image) {
            let drop_previous = self
                .preview_ui
                .render_image
                .as_ref()
                .is_none_or(|current| current.id != previous.id);
            if drop_previous {
                cx.drop_image(previous, None);
            }
        }
        session.mark_frame_presented(latest.generation);
        self.preview_ui.render_generation = latest.generation;
        self.preview_ui.rendered_presentation = self.preview_ui.render_presentation;
        self.preview_ui.runtime_error = None;
        self.apply_preview_canvas_auto_fit();
        true
    }
}
