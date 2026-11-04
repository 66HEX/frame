use crate::app::preview::actions::request::{
    preview_runtime_dimensions, preview_runtime_hash_changed,
};
use crate::app::preview::state::{
    PREVIEW_FILTER_DEBOUNCE_INTERVAL, PREVIEW_FRAME_TICK_INTERVAL, PreviewRuntimeDimensions,
    PreviewRuntimeKey,
};
use crate::app::root::FrameRoot;
use gpui::Context;
use std::time::Instant;

impl FrameRoot {
    pub(super) fn selected_preview_runtime_dimensions(
        &mut self,
        selected_file_id: &str,
    ) -> PreviewRuntimeDimensions {
        let live_dimensions = preview_runtime_dimensions(
            self.preview_ui.canvas_bounds,
            self.preview_ui.canvas.target_zoom,
        );
        let playing_selected_file = self.preview_ui.playback.is_playing()
            && self.preview_ui.playback_file_id.as_deref() == Some(selected_file_id);
        let active_dimensions = self.preview_ui.active_preview_dimensions;
        match active_dimensions {
            Some(dimensions) if playing_selected_file => return dimensions,
            Some(dimensions)
                if dimensions != live_dimensions && self.preview_dimension_update_deferred() =>
            {
                return dimensions;
            }
            _ => {}
        }

        self.preview_ui.active_preview_dimensions = Some(live_dimensions);
        live_dimensions
    }

    pub(super) fn preview_dimension_update_deferred(&mut self) -> bool {
        self.preview_canvas_transform_animating() || self.preview_dimensions_debounce_active()
    }

    pub(super) fn preview_canvas_transform_animating(&self) -> bool {
        !self.preview_canvas_transform_visually_settled()
    }

    pub(super) fn preview_dimensions_debounce_active(&mut self) -> bool {
        let Some(deadline) = self.preview_ui.preview_dimensions_debounce_until else {
            return false;
        };
        if Instant::now() < deadline {
            return true;
        }
        self.preview_ui.preview_dimensions_debounce_until = None;
        false
    }

    pub(super) fn schedule_preview_frame_tick(&mut self, cx: &Context<Self>) {
        if self.preview_ui.frame_tick_active {
            return;
        }
        self.preview_ui.frame_tick_active = true;

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(PREVIEW_FRAME_TICK_INTERVAL)
                    .await;
                let keep_ticking = this
                    .update(cx, |root, cx| {
                        let canvas_changed = if cx.reduce_motion() {
                            root.settle_preview_canvas_animation()
                        } else {
                            root.tick_preview_canvas_animation()
                        };
                        let preview_dimensions_ready = root.tick_preview_dimensions_debounce();
                        let filter_preview_ready = root.tick_filter_preview_debounce();
                        if root.preview_ui.session.is_none() {
                            if canvas_changed || preview_dimensions_ready || filter_preview_ready {
                                cx.notify();
                                return true;
                            }
                            if root.preview_dimensions_debounce_pending()
                                || root.filter_preview_debounce_pending()
                            {
                                return true;
                            }
                            root.preview_ui.frame_tick_active = false;
                            return false;
                        }

                        if root.refresh_preview_render_image(cx)
                            || root.preview_ui.playback.is_playing()
                            || canvas_changed
                            || preview_dimensions_ready
                            || filter_preview_ready
                        {
                            cx.notify();
                        }
                        true
                    })
                    .unwrap_or(false);

                if !keep_ticking {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn tick_preview_dimensions_debounce(&mut self) -> bool {
        let Some(deadline) = self.preview_ui.preview_dimensions_debounce_until else {
            return false;
        };
        if Instant::now() < deadline {
            return false;
        }
        self.preview_ui.preview_dimensions_debounce_until = None;
        true
    }

    pub(super) fn preview_dimensions_debounce_pending(&self) -> bool {
        self.preview_ui
            .preview_dimensions_debounce_until
            .is_some_and(|deadline| Instant::now() < deadline)
    }

    pub(super) fn should_defer_filter_preview_reconfigure(
        &mut self,
        next_key: &PreviewRuntimeKey,
    ) -> bool {
        self.preview_ui
            .runtime_key
            .clone()
            .is_some_and(|current_key| {
                current_key.can_reconfigure_to(next_key)
                    && preview_runtime_hash_changed(&current_key, next_key)
                    && self.filter_preview_debounce_active()
            })
    }

    pub(in crate::app) fn defer_filter_preview_reconfigure(&mut self, cx: &Context<Self>) {
        self.preview_ui.filter_preview_debounce_until =
            Some(Instant::now() + PREVIEW_FILTER_DEBOUNCE_INTERVAL);
        self.schedule_preview_frame_tick(cx);
    }

    pub(super) fn tick_filter_preview_debounce(&mut self) -> bool {
        let Some(deadline) = self.preview_ui.filter_preview_debounce_until else {
            return false;
        };
        if Instant::now() < deadline {
            return false;
        }
        self.preview_ui.filter_preview_debounce_until = None;
        true
    }

    pub(in crate::app) fn filter_preview_debounce_active(&mut self) -> bool {
        let Some(deadline) = self.preview_ui.filter_preview_debounce_until else {
            return false;
        };
        if Instant::now() < deadline {
            return true;
        }
        self.preview_ui.filter_preview_debounce_until = None;
        false
    }

    pub(super) fn filter_preview_debounce_pending(&self) -> bool {
        self.preview_ui
            .filter_preview_debounce_until
            .is_some_and(|deadline| Instant::now() < deadline)
    }
}
