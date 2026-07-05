use super::*;
use crate::app::preview_panel::preview_presented_frame;
use crate::conversion_runner::core_config_from_gpui;
use crate::numeric::rounded_f64_to_u64;
use crate::preview_engine::PreviewEngineError;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

impl FrameRoot {
    pub(super) fn selected_preview_runtime_request(
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

    fn selected_preview_runtime_dimensions(
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

    fn preview_dimension_update_deferred(&mut self) -> bool {
        self.preview_canvas_transform_animating() || self.preview_dimensions_debounce_active()
    }

    fn preview_canvas_transform_animating(&self) -> bool {
        !self.preview_canvas_transform_visually_settled()
    }

    fn preview_dimensions_debounce_active(&mut self) -> bool {
        let Some(deadline) = self.preview_ui.preview_dimensions_debounce_until else {
            return false;
        };
        if Instant::now() < deadline {
            return true;
        }
        self.preview_ui.preview_dimensions_debounce_until = None;
        false
    }

    pub(super) fn sync_preview_runtime_for_selection(
        &mut self,
        request: Option<PreviewRuntimeRequest>,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = request else {
            self.clear_preview_runtime(cx);
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

        if self.preview_ui.pending_runtime_key.is_some() {
            return;
        }

        if let (Some(session), Some(current_key)) = (
            self.preview_ui.session.clone(),
            self.preview_ui.runtime_key.as_ref(),
        ) && current_key.can_reconfigure_to(&request.key)
        {
            let key = request.key.clone();
            self.preview_ui.pending_runtime_key = Some(key.clone());
            cx.spawn(async move |this, cx| {
                let config = request.config;
                let result = cx
                    .background_spawn({
                        let session = Arc::clone(&session);
                        async move { session.reconfigure(config) }
                    })
                    .await;

                this.update(cx, move |root, cx| {
                    if root.preview_ui.pending_runtime_key.as_ref() != Some(&key) {
                        return;
                    }

                    root.preview_ui.pending_runtime_key = None;
                    match result {
                        Ok(()) => {
                            root.preview_ui.runtime_key = Some(key);
                            root.preview_ui.session = Some(session);
                            root.preview_ui.runtime_error = None;
                            root.refresh_preview_render_image(cx);
                            root.schedule_preview_frame_tick(cx);
                        }
                        Err(error) => {
                            root.preview_ui.runtime_error = Some(error.to_string());
                        }
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
            return;
        }

        self.clear_preview_runtime(cx);

        let key = request.key.clone();
        self.preview_ui.pending_runtime_key = Some(key.clone());
        cx.spawn(async move |this, cx| {
            let config = request.config;
            let result = cx
                .background_spawn(async move { PreviewSession::start(config).map(Arc::new) })
                .await;

            this.update(cx, move |root, cx| {
                if root.preview_ui.pending_runtime_key.as_ref() != Some(&key) {
                    if let Ok(session) = result {
                        session.stop();
                    }
                    return;
                }

                root.preview_ui.pending_runtime_key = None;
                match result {
                    Ok(session) => {
                        root.preview_ui.runtime_key = Some(key);
                        root.preview_ui.session = Some(session);
                        root.preview_ui.runtime_error = None;
                        root.refresh_preview_render_image(cx);
                        root.schedule_preview_frame_tick(cx);
                    }
                    Err(error) => {
                        root.preview_ui.runtime_error = Some(error.to_string());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn preview_render_image(&self) -> Option<Arc<RenderImage>> {
        self.preview_ui.render_image.clone()
    }

    pub(super) fn preview_runtime_error(&self) -> Option<String> {
        self.preview_ui.runtime_error.clone()
    }

    pub(super) fn sync_preview_canvas_for_selection(&mut self, selected_file_id: Option<&str>) {
        if self.preview_ui.canvas_file_id.as_deref() == selected_file_id {
            return;
        }

        self.preview_ui.canvas_file_id = selected_file_id.map(str::to_string);
        self.preview_ui.canvas = PreviewCanvasState::default();
        self.preview_ui.canvas_pan_drag = None;
    }

    pub(super) fn sync_preview_canvas_auto_fit(&mut self) -> bool {
        self.apply_preview_canvas_auto_fit()
    }

    pub(super) fn preview_canvas_render_state(&self) -> PreviewCanvasRenderState {
        let (viewport_width, viewport_height) =
            self.preview_ui.canvas_bounds.map_or((0.0, 0.0), |bounds| {
                (
                    f64::from(bounds.size.width.as_f32()),
                    f64::from(bounds.size.height.as_f32()),
                )
            });
        PreviewCanvasRenderState {
            zoom: self.preview_ui.canvas.current_zoom,
            pan_x: self.preview_ui.canvas.current_pan_x,
            pan_y: self.preview_ui.canvas.current_pan_y,
            viewport_width,
            viewport_height,
        }
    }

    pub(super) fn zoom_preview_canvas(
        &mut self,
        direction: PreviewCanvasZoomDirection,
        cx: &Context<Self>,
    ) -> bool {
        let multiplier = match direction {
            PreviewCanvasZoomDirection::In => PREVIEW_CANVAS_ZOOM_STEP,
            PreviewCanvasZoomDirection::Out => 1.0 / PREVIEW_CANVAS_ZOOM_STEP,
        };
        let current_zoom = self.preview_ui.canvas.target_zoom;
        let next_zoom = clamp_preview_canvas_zoom(current_zoom * multiplier);
        if (next_zoom - current_zoom).abs() <= f64::EPSILON {
            return false;
        }

        let zoom_ratio = if current_zoom > f64::EPSILON {
            next_zoom / current_zoom
        } else {
            1.0
        };
        let target_pan_x = self.preview_ui.canvas.target_pan_x * zoom_ratio;
        let target_pan_y = self.preview_ui.canvas.target_pan_y * zoom_ratio;
        let (target_pan_x, target_pan_y) =
            self.clamp_preview_canvas_pan_for_state(target_pan_x, target_pan_y, next_zoom);

        self.preview_ui.canvas.target_zoom = next_zoom;
        self.preview_ui.canvas.target_pan_x = target_pan_x;
        self.preview_ui.canvas.target_pan_y = target_pan_y;
        self.preview_ui.canvas.auto_fit_pending = false;
        self.schedule_preview_frame_tick(cx);
        true
    }

    pub(super) fn zoom_preview_canvas_at_position(
        &mut self,
        position: Point<Pixels>,
        multiplier: f64,
        cx: &Context<Self>,
    ) -> bool {
        if !multiplier.is_finite() || multiplier <= 0.0 {
            return false;
        }
        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return false;
        };
        let viewport_width = f64::from(bounds.size.width.as_f32());
        let viewport_height = f64::from(bounds.size.height.as_f32());
        if viewport_width <= 0.0 || viewport_height <= 0.0 {
            return false;
        }
        if position.x < bounds.origin.x
            || position.x > bounds.origin.x + bounds.size.width
            || position.y < bounds.origin.y
            || position.y > bounds.origin.y + bounds.size.height
        {
            return false;
        }

        let current_zoom = self.preview_ui.canvas.target_zoom;
        let next_zoom = clamp_preview_canvas_zoom(current_zoom * multiplier);
        if (next_zoom - current_zoom).abs() <= f64::EPSILON {
            return false;
        }

        let ratio = if current_zoom > f64::EPSILON {
            next_zoom / current_zoom
        } else {
            1.0
        };
        let pointer_x = f64::from((position.x - bounds.origin.x).as_f32()) - (viewport_width / 2.0);
        let pointer_y =
            f64::from((position.y - bounds.origin.y).as_f32()) - (viewport_height / 2.0);
        let target_pan_x =
            (pointer_x - self.preview_ui.canvas.target_pan_x).mul_add(-ratio, pointer_x);
        let target_pan_y =
            (pointer_y - self.preview_ui.canvas.target_pan_y).mul_add(-ratio, pointer_y);
        let (target_pan_x, target_pan_y) =
            self.clamp_preview_canvas_pan_for_state(target_pan_x, target_pan_y, next_zoom);

        self.preview_ui.canvas.auto_fit_pending = false;
        self.preview_ui.canvas.target_zoom = next_zoom;
        self.preview_ui.canvas.target_pan_x = target_pan_x;
        self.preview_ui.canvas.target_pan_y = target_pan_y;
        self.schedule_preview_frame_tick(cx);
        true
    }

    pub(super) fn zoom_preview_canvas_from_wheel(
        &mut self,
        position: Point<Pixels>,
        delta_y: f64,
        cx: &Context<Self>,
    ) -> bool {
        let Some(multiplier) = preview_canvas_wheel_zoom_multiplier(delta_y) else {
            return false;
        };
        self.zoom_preview_canvas_at_position(position, multiplier, cx)
    }

    pub(super) fn apply_preview_canvas_pan_drag(
        &mut self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        cx: &Context<Self>,
    ) -> bool {
        if bounds.size.width.as_f32() <= 0.0 || bounds.size.height.as_f32() <= 0.0 {
            return false;
        }

        let drag_state = if let Some(state) = self.preview_ui.canvas_pan_drag {
            state
        } else {
            let state = PreviewCanvasPanDragState {
                start_position: position,
                start_pan_x: self.preview_ui.canvas.current_pan_x,
                start_pan_y: self.preview_ui.canvas.current_pan_y,
            };
            self.preview_ui.canvas_pan_drag = Some(state);
            state
        };

        let delta_x = f64::from((position.x - drag_state.start_position.x).as_f32());
        let delta_y = f64::from((position.y - drag_state.start_position.y).as_f32());
        let (next_pan_x, next_pan_y) = self.clamp_preview_canvas_pan_for_state(
            drag_state.start_pan_x + delta_x,
            drag_state.start_pan_y + delta_y,
            self.preview_ui.canvas.target_zoom,
        );
        let changed = (next_pan_x - self.preview_ui.canvas.target_pan_x).abs() > f64::EPSILON
            || (next_pan_y - self.preview_ui.canvas.target_pan_y).abs() > f64::EPSILON;

        self.preview_ui.canvas.target_pan_x = next_pan_x;
        self.preview_ui.canvas.target_pan_y = next_pan_y;
        if changed {
            self.preview_ui.canvas.auto_fit_pending = false;
            self.schedule_preview_frame_tick(cx);
        }
        changed
    }

    pub(super) const fn end_preview_canvas_pan_drag(&mut self) -> bool {
        let had_drag = self.preview_ui.canvas_pan_drag.is_some();
        self.preview_ui.canvas_pan_drag = None;
        had_drag
    }

    pub(super) fn adjust_preview_canvas_pan_from_keyboard(
        &mut self,
        key: &str,
        cx: &Context<Self>,
    ) -> bool {
        let Some(delta) = preview_canvas_keyboard_pan_delta(key) else {
            return false;
        };
        let (next_pan_x, next_pan_y) = self.clamp_preview_canvas_pan_for_state(
            self.preview_ui.canvas.target_pan_x + delta.x,
            self.preview_ui.canvas.target_pan_y + delta.y,
            self.preview_ui.canvas.target_zoom,
        );
        let changed = (next_pan_x - self.preview_ui.canvas.target_pan_x).abs() > f64::EPSILON
            || (next_pan_y - self.preview_ui.canvas.target_pan_y).abs() > f64::EPSILON;
        self.preview_ui.canvas.target_pan_x = next_pan_x;
        self.preview_ui.canvas.target_pan_y = next_pan_y;
        if changed {
            self.preview_ui.canvas.auto_fit_pending = false;
            self.schedule_preview_frame_tick(cx);
        }
        true
    }

    pub(in crate::app) fn set_preview_canvas_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
        cx: &Context<Self>,
    ) -> bool {
        let previous_bounds = self.preview_ui.canvas_bounds;
        let bounds_changed = previous_bounds != Some(bounds);
        self.preview_ui.canvas_bounds = Some(bounds);
        if bounds_changed {
            self.preview_ui.preview_dimensions_debounce_until =
                Some(Instant::now() + PREVIEW_DIMENSION_DEBOUNCE_INTERVAL);
            self.schedule_preview_frame_tick(cx);
        }
        let auto_fit_changed = self.apply_preview_canvas_auto_fit();
        bounds_changed || auto_fit_changed
    }

    fn clamp_preview_canvas_pan_for_state(&self, pan_x: f64, pan_y: f64, zoom: f64) -> (f64, f64) {
        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return (0.0, 0.0);
        };
        let Some((media_width, media_height)) = self.preview_canvas_media_dimensions() else {
            return (0.0, 0.0);
        };
        let Some((max_x, max_y)) = preview_canvas_pan_limits(
            f64::from(bounds.size.width.as_f32()),
            f64::from(bounds.size.height.as_f32()),
            media_width,
            media_height,
            zoom,
        ) else {
            return (0.0, 0.0);
        };

        (pan_x.clamp(-max_x, max_x), pan_y.clamp(-max_y, max_y))
    }

    fn preview_canvas_media_dimensions(&self) -> Option<(f64, f64)> {
        let frame = preview_presented_frame(
            self.preview_ui.render_image.as_ref()?,
            self.preview_ui.render_presentation,
        )?;
        let width = f64::from(frame.visible_width);
        let height = f64::from(frame.visible_height);
        (width > 0.0 && height > 0.0).then_some((width, height))
    }

    fn apply_preview_canvas_auto_fit(&mut self) -> bool {
        if !self.preview_ui.canvas.auto_fit_pending {
            return false;
        }
        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return false;
        };
        let Some((media_width, media_height)) = self.preview_canvas_media_dimensions() else {
            return false;
        };
        let Some(zoom) = preview_canvas_initial_zoom(
            f64::from(bounds.size.width.as_f32()),
            f64::from(bounds.size.height.as_f32()),
            media_width,
            media_height,
        ) else {
            return false;
        };

        self.preview_ui.canvas.current_zoom = zoom;
        self.preview_ui.canvas.target_zoom = zoom;
        self.preview_ui.canvas.current_pan_x = 0.0;
        self.preview_ui.canvas.current_pan_y = 0.0;
        self.preview_ui.canvas.target_pan_x = 0.0;
        self.preview_ui.canvas.target_pan_y = 0.0;
        self.preview_ui.canvas.auto_fit_pending = false;
        true
    }

    pub(super) fn tick_preview_canvas_animation(&mut self) -> bool {
        if self.preview_canvas_transform_visually_settled() {
            return self.settle_preview_canvas_animation();
        }

        let next_zoom = lerp_preview_canvas_value(
            self.preview_ui.canvas.current_zoom,
            self.preview_ui.canvas.target_zoom,
        );
        let next_pan_x = lerp_preview_canvas_value(
            self.preview_ui.canvas.current_pan_x,
            self.preview_ui.canvas.target_pan_x,
        );
        let next_pan_y = lerp_preview_canvas_value(
            self.preview_ui.canvas.current_pan_y,
            self.preview_ui.canvas.target_pan_y,
        );

        self.preview_ui.canvas.current_zoom = next_zoom;
        self.preview_ui.canvas.current_pan_x = next_pan_x;
        self.preview_ui.canvas.current_pan_y = next_pan_y;

        true
    }

    fn preview_canvas_transform_visually_settled(&self) -> bool {
        let fallback = || {
            preview_canvas_transform_settled(
                self.preview_ui.canvas.current_zoom,
                self.preview_ui.canvas.target_zoom,
                self.preview_ui.canvas.current_pan_x,
                self.preview_ui.canvas.target_pan_x,
                self.preview_ui.canvas.current_pan_y,
                self.preview_ui.canvas.target_pan_y,
            )
        };

        let Some(bounds) = self.preview_ui.canvas_bounds else {
            return fallback();
        };
        let Some((media_width, media_height)) = self.preview_canvas_media_dimensions() else {
            return fallback();
        };

        preview_canvas_transform_visual_delta(
            f64::from(bounds.size.width.as_f32()),
            f64::from(bounds.size.height.as_f32()),
            media_width,
            media_height,
            self.preview_ui.canvas.current_zoom,
            self.preview_ui.canvas.target_zoom,
            self.preview_ui.canvas.current_pan_x,
            self.preview_ui.canvas.target_pan_x,
            self.preview_ui.canvas.current_pan_y,
            self.preview_ui.canvas.target_pan_y,
        )
        .is_some_and(|delta| delta <= PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON)
    }

    fn settle_preview_canvas_animation(&mut self) -> bool {
        let changed = (self.preview_ui.canvas.current_zoom - self.preview_ui.canvas.target_zoom)
            .abs()
            > f64::EPSILON
            || (self.preview_ui.canvas.current_pan_x - self.preview_ui.canvas.target_pan_x).abs()
                > f64::EPSILON
            || (self.preview_ui.canvas.current_pan_y - self.preview_ui.canvas.target_pan_y).abs()
                > f64::EPSILON;

        self.preview_ui.canvas.current_zoom = self.preview_ui.canvas.target_zoom;
        self.preview_ui.canvas.current_pan_x = self.preview_ui.canvas.target_pan_x;
        self.preview_ui.canvas.current_pan_y = self.preview_ui.canvas.target_pan_y;

        changed
    }

    pub(super) fn sync_preview_playback_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
        cx: &Context<Self>,
    ) {
        let media_kind = preview_control_availability(PreviewControlInput {
            metadata_status: if metadata.is_some() {
                PreviewMetadataStatus::Ready
            } else {
                PreviewMetadataStatus::Idle
            },
            source_media_kind: metadata.map(preview_source_media_kind),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        })
        .media_kind;
        let duration_seconds = preview_duration_seconds(metadata);

        if self.preview_ui.playback_file_id.as_deref() != selected_file_id {
            self.preview_ui.trim_preview_seek.reset();
            self.preview_ui.playback_file_id = selected_file_id.map(str::to_string);
            self.preview_ui.playback = preview_playback_state(
                media_kind,
                duration_seconds,
                config.start_time.as_deref(),
                config.end_time.as_deref(),
            );
            return;
        }

        self.preview_ui
            .playback
            .set_is_image(media_kind == PreviewMediaKind::Image);
        self.preview_ui
            .playback
            .sync_initial_values(config.start_time.as_deref(), config.end_time.as_deref());

        if let Some(session) = &self.preview_ui.session {
            let snapshot = session.snapshot();
            if !self.preview_media_snapshot_sync_blocked() {
                self.preview_ui.playback.sync_media(MediaSnapshot {
                    current_time: snapshot.playback.position_seconds,
                    duration: snapshot.playback.duration_seconds,
                    paused: !snapshot.playback.playing,
                });
            }
        } else if media_kind == PreviewMediaKind::Unknown {
            self.preview_ui.playback.clear_media();
        }

        let command = self
            .preview_ui
            .playback
            .handle_time_update(self.preview_ui.playback.current_time());
        self.apply_preview_media_command(command, true, Some(cx));
    }

    pub(super) fn preview_playback_state(&self) -> PreviewPlaybackState {
        self.preview_ui.playback.clone()
    }

    pub(super) const fn preview_media_snapshot_sync_blocked(&self) -> bool {
        self.preview_ui.playback.dragging().is_some()
            || self.preview_ui.trim_preview_seek.is_active()
    }

    fn clear_preview_runtime(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.preview_ui.session.take() {
            session.stop();
        }
        if let Some(image) = self.preview_ui.render_image.take() {
            cx.drop_image(image, None);
        }
        self.preview_ui.active_preview_dimensions = None;
        self.preview_ui.preview_dimensions_debounce_until = None;
        self.preview_ui.runtime_key = None;
        self.preview_ui.pending_runtime_key = None;
        self.preview_ui.trim_preview_seek.reset();
        self.preview_ui.render_generation = 0;
        self.preview_ui.runtime_error = None;
    }

    fn refresh_preview_render_image(&mut self, cx: &mut Context<Self>) -> bool {
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

    fn schedule_preview_frame_tick(&mut self, cx: &Context<Self>) {
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
                        let canvas_changed = root.tick_preview_canvas_animation();
                        let preview_dimensions_ready = root.tick_preview_dimensions_debounce();
                        if root.preview_ui.session.is_none() {
                            if canvas_changed || preview_dimensions_ready {
                                cx.notify();
                                return true;
                            }
                            if root.preview_dimensions_debounce_pending() {
                                return true;
                            }
                            root.preview_ui.frame_tick_active = false;
                            return false;
                        }

                        if root.refresh_preview_render_image(cx)
                            || root.preview_ui.playback.is_playing()
                            || canvas_changed
                            || preview_dimensions_ready
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

    fn tick_preview_dimensions_debounce(&mut self) -> bool {
        let Some(deadline) = self.preview_ui.preview_dimensions_debounce_until else {
            return false;
        };
        if Instant::now() < deadline {
            return false;
        }
        self.preview_ui.preview_dimensions_debounce_until = None;
        true
    }

    fn preview_dimensions_debounce_pending(&self) -> bool {
        self.preview_ui
            .preview_dimensions_debounce_until
            .is_some_and(|deadline| Instant::now() < deadline)
    }

    pub(super) fn sync_preview_crop_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        selected_config: &ConversionConfig,
    ) {
        if self.preview_ui.crop_file_id.as_deref() != selected_file_id {
            self.preview_ui.crop_file_id = selected_file_id.map(str::to_string);
            self.preview_ui.crop_mode = false;
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
        }

        if !self.preview_ui.crop_mode {
            self.preview_ui.crop_aspect = selected_config
                .crop
                .as_ref()
                .and_then(|crop| crop.aspect_ratio.clone())
                .unwrap_or_else(|| "free".to_string());
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
        }
    }
    pub(super) fn preview_crop_render_state(
        &self,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
    ) -> PreviewCropRenderState {
        PreviewCropRenderState {
            crop_mode: self.preview_ui.crop_mode,
            draft_crop: self.preview_ui.draft_crop,
            applied_crop: crop_rect_from_settings(config.crop.as_ref(), config),
            crop_aspect: self.preview_ui.crop_aspect.clone(),
            has_crop_dimensions: preview_crop_source_dimensions(metadata, &config.rotation)
                .is_some(),
            rotation: config.rotation.clone(),
            flip_horizontal: config.flip_horizontal,
            flip_vertical: config.flip_vertical,
        }
    }

    pub(super) fn sync_preview_overlay_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        selected_config: &ConversionConfig,
        cx: &Context<Self>,
    ) {
        if self.preview_ui.overlay_file_id.as_deref() != selected_file_id {
            self.preview_ui.overlay_file_id = selected_file_id.map(str::to_string);
            self.preview_ui.overlay = PreviewOverlayState::new();
            self.clear_preview_overlay_dimensions();
        }

        let initial_overlay = selected_config
            .overlay
            .as_ref()
            .map(preview_overlay_from_settings);
        self.preview_ui
            .overlay
            .sync_initial_overlay(initial_overlay.as_ref());
        self.sync_preview_overlay_dimensions(cx);
    }

    pub(super) fn preview_overlay_render_state(&self) -> PreviewOverlayRenderState {
        PreviewOverlayRenderState {
            overlay_mode: self.preview_ui.overlay.overlay_mode(),
            has_overlay: self.preview_ui.overlay.has_overlay(),
            overlay: self.preview_ui.overlay.render_overlay().cloned(),
            image_dimensions: self.preview_ui.overlay_image_dimensions,
        }
    }

    pub(super) fn trigger_selected_overlay(&mut self, window: &Window, cx: &Context<Self>) -> bool {
        if !self.selected_preview_overlay_controls_enabled() {
            return false;
        }

        if self.preview_ui.overlay.overlay().is_none() {
            self.prompt_selected_overlay_image(window, cx);
            return false;
        }

        if self.preview_ui.overlay.overlay_mode() {
            return self.set_selected_overlay_mode(false);
        }

        let change = self
            .preview_ui
            .overlay
            .toggle_overlay_mode(self.file_queue.selected_file_locked());
        self.apply_preview_overlay_mode_change(change)
    }

    pub(super) fn prompt_selected_overlay_image(&self, window: &Window, cx: &Context<Self>) {
        if !self.selected_preview_overlay_controls_enabled() {
            return;
        }

        let dialog = overlay_image_dialog(window);
        cx.spawn(async move |this, cx| {
            let Some(path) = pick_overlay_image_file(dialog).await else {
                return;
            };
            if !is_supported_overlay_image_path(&path) {
                return;
            }
            let dimensions = cx
                .background_spawn({
                    let path = path.clone();
                    async move { load_preview_overlay_image_dimensions(path) }
                })
                .await;
            let path = path.to_string_lossy().to_string();

            this.update(cx, move |root, cx| {
                if !root.selected_preview_overlay_controls_enabled() {
                    return;
                }

                let Some(overlay) = root
                    .preview_ui
                    .overlay
                    .set_overlay_from_path(path.clone(), root.file_queue.selected_file_locked())
                else {
                    return;
                };
                root.preview_ui.crop_mode = false;
                root.preview_ui.draft_crop = None;
                root.preview_ui.crop_drag = None;
                root.preview_ui.overlay_dimensions_key = Some(path.clone());
                root.preview_ui.pending_overlay_dimensions_key = None;
                root.preview_ui.overlay_image_dimensions = dimensions;

                let _ = overlay;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn set_selected_overlay_mode(&mut self, value: bool) -> bool {
        if !value {
            let was_editing = self.preview_ui.overlay.overlay_mode();
            let Some(next_overlay) = self.preview_ui.overlay.apply_overlay_edit() else {
                return false;
            };
            let committed = self.commit_preview_overlay(next_overlay);
            return was_editing || committed;
        }

        let change = self
            .preview_ui
            .overlay
            .set_overlay_mode(value, self.file_queue.selected_file_locked());
        self.apply_preview_overlay_mode_change(change)
    }

    pub(super) fn remove_selected_overlay(&mut self) -> bool {
        let Some(next_overlay) = self
            .preview_ui
            .overlay
            .remove_overlay(self.file_queue.selected_file_locked())
        else {
            return false;
        };

        self.clear_preview_overlay_dimensions();
        self.commit_preview_overlay(next_overlay)
    }

    pub(super) fn nudge_selected_overlay_size(
        &mut self,
        direction: OverlaySizeDirection,
        media: Option<PreviewMediaRenderState>,
    ) -> bool {
        let height_ratio = self.preview_overlay_height_ratio(media);
        let Some(overlay) = self.preview_ui.overlay.nudge_size(
            direction,
            Some(height_ratio),
            self.file_queue.selected_file_locked(),
        ) else {
            return false;
        };

        let _ = overlay;
        true
    }

    pub(super) fn set_selected_overlay_opacity(&mut self, value: f64) -> bool {
        let Some(overlay) = self
            .preview_ui
            .overlay
            .set_opacity(value, self.file_queue.selected_file_locked())
        else {
            return false;
        };

        let _ = overlay;
        true
    }

    pub(in crate::app) const fn set_preview_overlay_opacity_slider_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
    ) {
        self.preview_ui.overlay_opacity_slider_bounds = Some(bounds);
    }

    pub(super) fn commit_preview_overlay_opacity_at_position(
        &mut self,
        position: Point<Pixels>,
    ) -> bool {
        let Some(bounds) = self.preview_ui.overlay_opacity_slider_bounds else {
            return false;
        };
        let opacity = timeline_slider_percent_from_bounds(position, bounds);
        self.set_selected_overlay_opacity(opacity)
    }

    pub(super) fn apply_preview_overlay_drag(
        &mut self,
        handle: OverlayDragHandle,
        point: OverlayDragPoint,
    ) -> bool {
        if !self.selected_preview_overlay_controls_enabled() {
            return false;
        }

        if !self.preview_ui.overlay.is_dragging()
            && !self.preview_ui.overlay.begin_overlay_drag(
                handle,
                point,
                self.file_queue.selected_file_locked(),
            )
        {
            return false;
        }

        let Some(overlay) = self.preview_ui.overlay.update_overlay_drag(point) else {
            return false;
        };
        let _ = overlay;
        true
    }

    pub(super) fn end_preview_overlay_drag(&mut self) -> bool {
        let was_dragging = self.preview_ui.overlay.is_dragging();
        self.preview_ui.overlay.end_overlay_drag();
        was_dragging
    }

    pub(super) fn adjust_preview_overlay_from_keyboard_with_step(
        &mut self,
        handle: OverlayDragHandle,
        key: &str,
        media: Option<PreviewMediaRenderState>,
        large_step: bool,
    ) -> bool {
        if !self.selected_preview_overlay_controls_enabled() {
            return false;
        }
        let Some(delta) = preview_overlay_keyboard_delta(key, large_step) else {
            return false;
        };
        let Some(overlay) = self.preview_ui.overlay.overlay().cloned() else {
            return false;
        };

        let height = overlay.width * self.preview_overlay_height_ratio(media);
        let start_point = preview_overlay_keyboard_start_point(handle, &overlay, height);
        let current_point = OverlayDragPoint {
            x: start_point.x + delta.x,
            y: start_point.y + delta.y,
            width: start_point.width,
            height: start_point.height,
        };

        let mut changed = self.apply_preview_overlay_drag(handle, start_point);
        changed |= self.apply_preview_overlay_drag(handle, current_point);
        changed | self.end_preview_overlay_drag()
    }

    fn preview_overlay_height_ratio(&self, media: Option<PreviewMediaRenderState>) -> f64 {
        let overlay_ratio = self
            .preview_ui
            .overlay_image_dimensions
            .map_or(1.0, PreviewOverlayImageDimensions::height_over_width);
        let media_ratio = media.map_or(1.0, |media| {
            if media.height == 0 {
                1.0
            } else {
                f64::from(media.width) / f64::from(media.height)
            }
        });
        overlay_ratio * media_ratio
    }

    const fn apply_preview_overlay_mode_change(&mut self, change: OverlayModeChange) -> bool {
        if change.should_deactivate_crop {
            self.preview_ui.crop_mode = false;
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
        }
        change.changed || change.should_deactivate_crop
    }

    fn commit_preview_overlay(&mut self, overlay: Option<PreviewOverlay>) -> bool {
        let next_overlay = overlay.map(|overlay| overlay_settings_from_preview(&overlay));
        self.update_selected_config(|config| {
            let changed = config.overlay != next_overlay;
            config.overlay = next_overlay;
            changed
        })
    }

    fn selected_preview_overlay_controls_enabled(&self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let availability = preview_control_availability(PreviewControlInput {
            metadata_status: if metadata.is_some() {
                PreviewMetadataStatus::Ready
            } else {
                PreviewMetadataStatus::Idle
            },
            source_media_kind: metadata.as_ref().map(preview_source_media_kind),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        });

        availability.overlay_available && !self.file_queue.selected_file_locked()
    }

    fn sync_preview_overlay_dimensions(&mut self, cx: &Context<Self>) {
        let Some(path) = self
            .preview_ui
            .overlay
            .overlay()
            .map(|overlay| overlay.path.clone())
        else {
            self.clear_preview_overlay_dimensions();
            return;
        };

        if self.preview_ui.overlay_dimensions_key.as_deref() == Some(path.as_str())
            || self.preview_ui.pending_overlay_dimensions_key.as_deref() == Some(path.as_str())
        {
            return;
        }

        self.preview_ui.overlay_dimensions_key = None;
        self.preview_ui.overlay_image_dimensions = None;
        self.preview_ui.pending_overlay_dimensions_key = Some(path.clone());
        cx.spawn(async move |this, cx| {
            let path_for_loader = PathBuf::from(&path);
            let dimensions = cx
                .background_spawn(
                    async move { load_preview_overlay_image_dimensions(path_for_loader) },
                )
                .await;

            this.update(cx, move |root, cx| {
                if root.preview_ui.pending_overlay_dimensions_key.as_deref() != Some(path.as_str())
                {
                    return;
                }

                root.preview_ui.pending_overlay_dimensions_key = None;
                root.preview_ui.overlay_dimensions_key = Some(path);
                root.preview_ui.overlay_image_dimensions = dimensions;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn clear_preview_overlay_dimensions(&mut self) {
        self.preview_ui.overlay_dimensions_key = None;
        self.preview_ui.pending_overlay_dimensions_key = None;
        self.preview_ui.overlay_image_dimensions = None;
    }

    pub(super) fn toggle_selected_crop_mode(&mut self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if !preview_crop_controls_enabled(
            metadata.as_ref(),
            config,
            self.file_queue.selected_file_locked(),
        ) {
            return false;
        }

        let applied_crop = crop_rect_from_settings(config.crop.as_ref(), config);
        let crop_aspect = config
            .crop
            .as_ref()
            .and_then(|crop| crop.aspect_ratio.clone())
            .unwrap_or_else(|| "free".to_string());

        if self.preview_ui.crop_mode {
            self.preview_ui.crop_mode = false;
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
            return true;
        }

        self.preview_ui.crop_mode = true;
        self.preview_ui.draft_crop = Some(applied_crop.unwrap_or_else(default_crop_rect));
        self.preview_ui.crop_aspect = crop_aspect;
        true
    }
    pub(super) fn select_preview_crop_aspect(&mut self, aspect_id: &str) -> bool {
        if !self.preview_ui.crop_mode || !is_known_crop_aspect(aspect_id) {
            return false;
        }

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let rotation = config.rotation.clone();
        let flip_horizontal = config.flip_horizontal;
        let flip_vertical = config.flip_vertical;
        let Some(dimensions) = crop_base_dimensions(metadata.as_ref(), &rotation) else {
            return false;
        };
        let preview_rotation = PreviewRotation::from(rotation.as_str());

        let previous_aspect = self.preview_ui.crop_aspect.clone();
        let previous_rect = self.preview_ui.draft_crop;
        self.preview_ui.crop_aspect = aspect_id.to_string();
        if let Some(rect) = self.preview_ui.draft_crop {
            self.preview_ui.draft_crop = Some(aspect_value(aspect_id).map_or_else(
                || clamp_rect(rect),
                |ratio| {
                    let visual_rect = clamp_rect(transform_crop_rect(
                        rect,
                        preview_rotation,
                        flip_horizontal,
                        flip_vertical,
                        false,
                    ));
                    let adjusted_visual_rect = clamp_rect(adjust_rect_to_ratio(
                        visual_rect,
                        ratio,
                        f64::from(dimensions.width),
                        f64::from(dimensions.height),
                        false,
                    ));
                    clamp_rect(transform_crop_rect(
                        adjusted_visual_rect,
                        preview_rotation,
                        flip_horizontal,
                        flip_vertical,
                        true,
                    ))
                },
            ));
        }

        previous_aspect != self.preview_ui.crop_aspect
            || previous_rect != self.preview_ui.draft_crop
    }
    pub(super) fn reset_preview_crop_selection(&mut self) -> bool {
        if !self.preview_ui.crop_mode {
            return false;
        }

        let previous_rect = self.preview_ui.draft_crop;
        let previous_aspect = self.preview_ui.crop_aspect.clone();
        self.preview_ui.draft_crop = Some(if self.preview_ui.draft_crop.is_some() {
            full_crop_rect()
        } else {
            default_crop_rect()
        });
        self.preview_ui.crop_aspect = "free".to_string();
        previous_rect != self.preview_ui.draft_crop
            || previous_aspect != self.preview_ui.crop_aspect
    }
    pub(super) fn apply_selected_crop(&mut self) -> bool {
        if !self.preview_ui.crop_mode {
            return false;
        }
        let Some(draft_crop) = self.preview_ui.draft_crop else {
            return false;
        };

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if preview_crop_source_dimensions(metadata.as_ref(), &config.rotation).is_none() {
            return false;
        }

        let next_crop = if crop_rect_is_full(draft_crop) {
            None
        } else {
            crop_settings_from_rect(
                draft_crop,
                &self.preview_ui.crop_aspect,
                &config.rotation,
                config.flip_horizontal,
                config.flip_vertical,
                metadata.as_ref(),
            )
        };
        let cleared_crop = next_crop.is_none();

        let _config_changed = self.update_selected_config(|config| {
            let changed = config.crop != next_crop;
            config.crop = next_crop;
            changed
        });
        self.preview_ui.crop_mode = false;
        self.preview_ui.draft_crop = None;
        self.preview_ui.crop_drag = None;
        if cleared_crop {
            self.preview_ui.crop_aspect = "free".to_string();
        }
        true
    }
    pub(super) fn rotate_selected_preview(&mut self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if !preview_transform_controls_enabled(
            metadata.as_ref(),
            config,
            self.file_queue.selected_file_locked(),
        ) {
            return false;
        }

        let next_rotation = next_rotation(&config.rotation);
        let applied_crop = crop_rect_from_settings(config.crop.as_ref(), config);
        let aspect_id = crop_aspect_id(config.crop.as_ref()).to_string();
        let flip_horizontal = config.flip_horizontal;
        let flip_vertical = config.flip_vertical;
        let next_crop = applied_crop.and_then(|rect| {
            crop_settings_from_rect(
                rect,
                &aspect_id,
                &next_rotation,
                flip_horizontal,
                flip_vertical,
                metadata.as_ref(),
            )
        });

        self.update_selected_config(|config| {
            let changed = config.rotation != next_rotation
                || (applied_crop.is_some() && config.crop != next_crop);
            config.rotation = next_rotation;
            if applied_crop.is_some() {
                config.crop = next_crop;
            }
            changed
        })
    }
    pub(super) fn toggle_selected_flip(&mut self, axis: FlipAxis) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if !preview_transform_controls_enabled(
            metadata.as_ref(),
            config,
            self.file_queue.selected_file_locked(),
        ) {
            return false;
        }

        let next_flip_horizontal = if axis == FlipAxis::Horizontal {
            !config.flip_horizontal
        } else {
            config.flip_horizontal
        };
        let next_flip_vertical = if axis == FlipAxis::Vertical {
            !config.flip_vertical
        } else {
            config.flip_vertical
        };
        let applied_crop = crop_rect_from_settings(config.crop.as_ref(), config);
        let aspect_id = crop_aspect_id(config.crop.as_ref()).to_string();
        let rotation = config.rotation.clone();
        let next_crop = applied_crop.and_then(|rect| {
            crop_settings_from_rect(
                rect,
                &aspect_id,
                &rotation,
                next_flip_horizontal,
                next_flip_vertical,
                metadata.as_ref(),
            )
        });

        self.update_selected_config(|config| {
            let changed = config.flip_horizontal != next_flip_horizontal
                || config.flip_vertical != next_flip_vertical
                || (applied_crop.is_some() && config.crop != next_crop);
            config.flip_horizontal = next_flip_horizontal;
            config.flip_vertical = next_flip_vertical;
            if applied_crop.is_some() {
                config.crop = next_crop;
            }
            changed
        })
    }
    pub(super) fn apply_preview_crop_drag(
        &mut self,
        handle: DragHandle,
        point: PreviewPoint,
    ) -> bool {
        if !self.preview_ui.crop_mode {
            return false;
        }
        let Some(current_rect) = self.preview_ui.draft_crop else {
            return false;
        };

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let rotation = config.rotation.clone();
        let flip_horizontal = config.flip_horizontal;
        let flip_vertical = config.flip_vertical;
        let Some(dimensions) = crop_base_dimensions(metadata.as_ref(), &rotation) else {
            return false;
        };
        let preview_rotation = PreviewRotation::from(rotation.as_str());

        let drag_state = match self.preview_ui.crop_drag {
            Some(state) if state.handle == handle => state,
            _ => {
                let state = PreviewCropDragState {
                    handle,
                    start_rect: current_rect,
                    start_point: point,
                };
                self.preview_ui.crop_drag = Some(state);
                state
            }
        };

        let visual_start_rect = clamp_rect(transform_crop_rect(
            drag_state.start_rect,
            preview_rotation,
            flip_horizontal,
            flip_vertical,
            false,
        ));
        let visual_next_rect =
            crate::preview::apply_visual_crop_drag(crate::preview::VisualCropDrag {
                start_rect: visual_start_rect,
                handle,
                start_point: drag_state.start_point,
                current_point: point,
                aspect_id: &self.preview_ui.crop_aspect,
                source_width: f64::from(dimensions.width),
                source_height: f64::from(dimensions.height),
                is_side_rotation: false,
            });
        let next_rect = if visual_next_rect == visual_start_rect {
            drag_state.start_rect
        } else {
            clamp_rect(transform_crop_rect(
                visual_next_rect,
                preview_rotation,
                flip_horizontal,
                flip_vertical,
                true,
            ))
        };
        let changed = self.preview_ui.draft_crop != Some(next_rect);
        self.preview_ui.draft_crop = Some(next_rect);
        changed
    }
    pub(super) const fn end_preview_crop_drag(&mut self) -> bool {
        let had_drag = self.preview_ui.crop_drag.is_some();
        self.preview_ui.crop_drag = None;
        had_drag
    }

    pub(super) fn adjust_preview_crop_from_keyboard_with_step(
        &mut self,
        handle: DragHandle,
        key: &str,
        large_step: bool,
    ) -> bool {
        if !self.preview_ui.crop_mode {
            return false;
        }
        let Some(current_rect) = self.preview_ui.draft_crop else {
            return false;
        };
        let Some(delta) = preview_crop_keyboard_delta(handle, key, large_step) else {
            return false;
        };

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let rotation = config.rotation.clone();
        let flip_horizontal = config.flip_horizontal;
        let flip_vertical = config.flip_vertical;
        let Some(dimensions) = crop_base_dimensions(metadata.as_ref(), &rotation) else {
            return false;
        };
        let preview_rotation = PreviewRotation::from(rotation.as_str());
        let visual_rect = clamp_rect(transform_crop_rect(
            current_rect,
            preview_rotation,
            flip_horizontal,
            flip_vertical,
            false,
        ));
        let visual_next_rect =
            crate::preview::apply_visual_crop_drag(crate::preview::VisualCropDrag {
                start_rect: visual_rect,
                handle,
                start_point: PreviewPoint { x: 0.0, y: 0.0 },
                current_point: delta,
                aspect_id: &self.preview_ui.crop_aspect,
                source_width: f64::from(dimensions.width),
                source_height: f64::from(dimensions.height),
                is_side_rotation: false,
            });
        let next_rect = if visual_next_rect == visual_rect {
            current_rect
        } else {
            clamp_rect(transform_crop_rect(
                visual_next_rect,
                preview_rotation,
                flip_horizontal,
                flip_vertical,
                true,
            ))
        };
        let changed = self.preview_ui.draft_crop != Some(next_rect);
        self.preview_ui.draft_crop = Some(next_rect);
        changed
    }

    pub(super) fn apply_preview_timeline_drag(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
    ) -> bool {
        self.apply_preview_timeline_drag_internal(target, percent, None)
    }

    pub(super) fn apply_preview_timeline_drag_with_context(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
        cx: &Context<Self>,
    ) -> bool {
        self.apply_preview_timeline_drag_internal(target, percent, Some(cx))
    }

    fn apply_preview_timeline_drag_internal(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        if self.preview_ui.playback.dragging().is_none() {
            if target == TimelineDragTarget::Scrub {
                let _ = self.preview_ui.playback.seek_to_percent(percent);
                return true;
            }

            if !self.preview_ui.playback.begin_handle_drag(target) {
                return false;
            }
            let was_playing = self.preview_ui.playback.is_playing();
            if self.trim_preview_seek_available() {
                self.preview_ui
                    .trim_preview_seek
                    .begin_drag(self.preview_ui.playback.current_time());
                if was_playing {
                    self.preview_ui.trim_preview_seek.pause_before_next_seek();
                }
            }
            if was_playing {
                self.apply_preview_media_command(PlaybackMediaCommand::pause(), true, cx);
            }
        }

        let update = self.preview_ui.playback.drag_to_percent(percent);
        self.apply_preview_command_to_local_state(update.command);
        if let Some(preview_seek_to) = update.preview_seek_to {
            self.queue_trim_preview_seek(preview_seek_to, cx);
        }
        true
    }

    pub(in crate::app) const fn set_preview_timeline_track_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
    ) {
        self.preview_ui.timeline_track_bounds = Some(bounds);
    }

    #[cfg(test)]
    pub(super) fn commit_preview_timeline_seek_at_position(
        &mut self,
        position: Point<Pixels>,
    ) -> bool {
        self.commit_preview_timeline_seek_at_position_internal(position, None)
    }

    pub(super) fn commit_preview_timeline_seek_at_position_with_context(
        &mut self,
        position: Point<Pixels>,
        cx: &Context<Self>,
    ) -> bool {
        self.commit_preview_timeline_seek_at_position_internal(position, Some(cx))
    }

    fn commit_preview_timeline_seek_at_position_internal(
        &mut self,
        position: Point<Pixels>,
        cx: Option<&Context<Self>>,
    ) -> bool {
        let Some(bounds) = self.preview_ui.timeline_track_bounds else {
            return false;
        };
        if !self.preview_timeline_enabled() {
            return false;
        }

        let percent = timeline_slider_percent_from_bounds(position, bounds);
        let command = self.preview_ui.playback.seek_once_to_percent(percent);
        self.apply_preview_media_command(command, true, cx)
    }

    #[cfg(test)]
    pub(super) fn end_preview_timeline_drag(&mut self) -> bool {
        self.end_preview_timeline_drag_internal(None)
    }

    pub(super) fn end_preview_timeline_drag_with_context(&mut self, cx: &Context<Self>) -> bool {
        self.end_preview_timeline_drag_internal(Some(cx))
    }

    fn end_preview_timeline_drag_internal(&mut self, cx: Option<&Context<Self>>) -> bool {
        if self.preview_ui.playback.dragging().is_none() {
            return false;
        }

        let was_trim_drag = self
            .preview_ui
            .playback
            .dragging()
            .is_some_and(|target| target != TimelineDragTarget::Scrub);
        let end = self.preview_ui.playback.end_drag();
        let mut changed = self.apply_preview_media_command(end.command, true, cx);
        if let Some(trim) = end.trim {
            changed |= self.update_selected_config(|config| {
                apply_trim_times(config, trim.start_time, trim.end_time)
            });
        }
        if was_trim_drag {
            changed |= self.finish_trim_preview_seek(cx);
        }
        changed
    }

    pub(super) fn adjust_preview_timeline_from_keyboard_with_context(
        &mut self,
        target: TimelineDragTarget,
        key: &str,
        cx: &Context<Self>,
    ) -> bool {
        self.adjust_preview_timeline_from_keyboard_internal(target, key, Some(cx))
    }

    fn adjust_preview_timeline_from_keyboard_internal(
        &mut self,
        target: TimelineDragTarget,
        key: &str,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        let duration = self.preview_ui.playback.duration();
        let current_time = match target {
            TimelineDragTarget::Start => self.preview_ui.playback.start_value(),
            TimelineDragTarget::End => self.preview_ui.playback.end_value(),
            TimelineDragTarget::Scrub => self.preview_ui.playback.current_time(),
        };
        let Some(next_time) = timeline_keyboard_time_for_key(current_time, duration, key) else {
            return false;
        };
        let percent = next_time / duration;

        if target == TimelineDragTarget::Scrub {
            let command = self.preview_ui.playback.seek_once_to_percent(percent);
            return self.apply_preview_media_command(command, true, cx);
        }

        self.apply_preview_timeline_drag(target, percent)
            | self.end_preview_timeline_drag_internal(cx)
    }

    pub(super) fn toggle_preview_playback_with_context(&mut self, cx: &Context<Self>) -> bool {
        self.toggle_preview_playback_internal(Some(cx))
    }

    fn toggle_preview_playback_internal(&mut self, cx: Option<&Context<Self>>) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        let command = self.preview_ui.playback.toggle_play();
        self.apply_preview_media_command(command, true, cx)
    }

    fn trim_preview_seek_available(&self) -> bool {
        self.preview_ui.session.is_some()
            && self.selected_source_metadata().is_some_and(|metadata| {
                preview_source_media_kind(&metadata) == SourceMediaKind::Video
            })
    }

    fn queue_trim_preview_seek(&mut self, seconds: f64, cx: Option<&Context<Self>>) -> bool {
        if !seconds.is_finite() || !self.trim_preview_seek_available() {
            return false;
        }

        self.preview_ui.trim_preview_seek.queue(seconds);
        if let Some(cx) = cx {
            self.start_trim_preview_seek_worker(cx);
        }
        true
    }

    fn finish_trim_preview_seek(&mut self, cx: Option<&Context<Self>>) -> bool {
        if !self.preview_ui.trim_preview_seek.finish() {
            return false;
        }
        if let Some(cx) = cx {
            self.start_trim_preview_seek_worker(cx);
        }
        true
    }

    fn start_trim_preview_seek_worker(&mut self, cx: &Context<Self>) {
        if self.preview_ui.trim_preview_seek.worker_active {
            return;
        }
        let Some(session) = self.preview_ui.session.clone() else {
            return;
        };

        self.preview_ui.trim_preview_seek.worker_active = true;
        let generation = self.preview_ui.trim_preview_seek.generation;
        cx.spawn(async move |this, cx| {
            loop {
                let request = this
                    .update(cx, |root, _cx| root.take_next_trim_preview_seek(generation))
                    .ok()
                    .flatten();
                let Some(request) = request else {
                    let should_continue = this
                        .update(cx, move |root, _cx| {
                            if root.preview_ui.trim_preview_seek.generation != generation {
                                return false;
                            }
                            if root.preview_ui.trim_preview_seek.pending_seconds.is_some() {
                                return true;
                            }
                            root.preview_ui.trim_preview_seek.worker_active = false;
                            false
                        })
                        .unwrap_or(false);
                    if should_continue {
                        continue;
                    }
                    break;
                };

                let result = cx
                    .background_spawn({
                        let session = Arc::clone(&session);
                        async move {
                            if request.pause_first {
                                session.command(PreviewCommand::Pause)?;
                            }
                            session.command(PreviewCommand::SeekPrecise(request.seconds))
                        }
                    })
                    .await;

                let should_stop = this
                    .update(cx, move |root, cx| {
                        root.finish_trim_preview_seek_request(
                            generation,
                            request.seconds,
                            result,
                            cx,
                        )
                    })
                    .unwrap_or(true);
                if should_stop {
                    break;
                }

                cx.background_executor()
                    .timer(TRIM_PREVIEW_SEEK_INTERVAL)
                    .await;
            }
        })
        .detach();
    }

    fn take_next_trim_preview_seek(&mut self, generation: u64) -> Option<TrimPreviewSeekRequest> {
        let trim_preview_seek = &mut self.preview_ui.trim_preview_seek;
        if trim_preview_seek.generation != generation {
            return None;
        }
        trim_preview_seek.take_next()
    }

    fn finish_trim_preview_seek_request(
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
                self.preview_ui.runtime_error = None;
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

    fn apply_preview_media_command(
        &mut self,
        command: PlaybackMediaCommand,
        precise_seek: bool,
        cx: Option<&Context<Self>>,
    ) -> bool {
        let Some(session) = self.preview_ui.session.clone() else {
            return self.apply_preview_command_to_local_state(command);
        };

        let commands = preview_commands_from_playback(command, precise_seek);
        if !commands.is_empty() {
            if let Some(cx) = cx {
                self.preview_ui.media_command_generation =
                    self.preview_ui.media_command_generation.saturating_add(1);
                let command_generation = self.preview_ui.media_command_generation;
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_spawn(async move {
                            for command in commands {
                                session.command(command)?;
                            }
                            Ok::<(), PreviewEngineError>(())
                        })
                        .await;

                    this.update(cx, move |root, cx| {
                        if root.preview_ui.media_command_generation != command_generation {
                            return;
                        }
                        match result {
                            Ok(()) => {
                                root.preview_ui.runtime_error = None;
                                root.schedule_preview_frame_tick(cx);
                            }
                            Err(error) => {
                                root.preview_ui.runtime_error = Some(error.to_string());
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            } else {
                for command in commands {
                    if let Err(error) = session.command(command) {
                        self.preview_ui.runtime_error = Some(error.to_string());
                        return false;
                    }
                }
                self.preview_ui.runtime_error = None;
            }
        }

        self.apply_preview_command_to_local_state(command)
    }

    const fn apply_preview_command_to_local_state(
        &mut self,
        command: PlaybackMediaCommand,
    ) -> bool {
        if command.pause {
            self.preview_ui.playback.handle_pause();
        }
        if command.play {
            self.preview_ui.playback.handle_play();
        }
        command.pause || command.play || command.seek_to.is_some()
    }

    fn preview_timeline_enabled(&self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let availability = preview_control_availability(PreviewControlInput {
            metadata_status: if metadata.is_some() {
                PreviewMetadataStatus::Ready
            } else {
                PreviewMetadataStatus::Idle
            },
            source_media_kind: metadata.as_ref().map(preview_source_media_kind),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        });

        !availability.trim_disabled && self.preview_ui.playback.duration() > 0.0
    }
    pub(super) fn resolve_selected_settings_tab(&mut self, metadata: Option<&SourceMetadata>) {
        let next_tab = self
            .selected_config()
            .map_or(SettingsTab::Source, |config| {
                resolve_active_settings_tab(self.settings_ui.active_tab, config, metadata)
            });
        self.settings_ui.active_tab = next_tab;
    }
}

fn preview_commands_from_playback(
    command: PlaybackMediaCommand,
    precise_seek: bool,
) -> Vec<PreviewCommand> {
    let mut commands = Vec::with_capacity(3);
    if command.pause {
        commands.push(PreviewCommand::Pause);
    }
    if let Some(seconds) = command.seek_to {
        commands.push(if precise_seek {
            PreviewCommand::SeekPrecise(seconds)
        } else {
            PreviewCommand::SeekFast(seconds)
        });
    }
    if command.play {
        commands.push(PreviewCommand::Play);
    }
    commands
}

fn preview_runtime_request(
    selected_file: &FileItem,
    metadata_entry: &SourceMetadataEntry,
    include_applied_crop: bool,
    include_committed_overlay: bool,
    preview_dimensions: PreviewRuntimeDimensions,
) -> Option<PreviewRuntimeRequest> {
    if metadata_entry.status != MetadataStatus::Ready {
        return None;
    }

    let metadata = metadata_entry.metadata.as_ref()?;
    let source_kind = engine_source_kind(metadata);
    let duration_seconds = preview_duration_seconds(Some(metadata));
    let (source_width, source_height) = valid_preview_dimensions(metadata.width, metadata.height);
    let mut preview_config = selected_file.config.clone();
    if !include_applied_crop {
        preview_config.crop = None;
    }
    if !include_committed_overlay {
        preview_config.overlay = None;
    }
    let core_config = core_config_from_gpui(&preview_config);
    let presentation = PreviewRenderPresentation::default();
    let has_audio = preview_has_audio(metadata, source_kind);
    let selected_audio_track = preview_config.selected_audio_tracks.first().copied();
    let key = PreviewRuntimeKey {
        file_id: selected_file.id.clone(),
        path: selected_file.path.clone(),
        source_kind,
        source_width,
        source_height,
        duration_millis: rounded_f64_to_u64(duration_seconds * 1000.0),
        preview_dimensions,
        visual_hash: preview_visual_hash(&preview_config),
        audio_hash: preview_audio_hash(&preview_config, has_audio, selected_audio_track),
    };
    let config = PreviewSessionConfig {
        file_id: key.file_id.clone(),
        path: PathBuf::from(&selected_file.path),
        source_kind,
        source_width,
        source_height,
        has_audio,
        selected_audio_track,
        duration_seconds,
        max_width: preview_dimensions.max_width,
        max_height: preview_dimensions.max_height,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: core_config,
    };

    Some(PreviewRuntimeRequest {
        key,
        config,
        presentation,
    })
}

pub(super) fn preview_runtime_dimensions(
    canvas_bounds: Option<Bounds<Pixels>>,
    target_zoom: f64,
) -> PreviewRuntimeDimensions {
    let Some(bounds) = canvas_bounds else {
        return default_preview_runtime_dimensions();
    };
    let canvas_width = f64::from(bounds.size.width.as_f32());
    let canvas_height = f64::from(bounds.size.height.as_f32());
    if canvas_width <= 0.0 || canvas_height <= 0.0 {
        return default_preview_runtime_dimensions();
    }

    let zoom = if target_zoom.is_finite() {
        target_zoom.max(1.0)
    } else {
        1.0
    };
    PreviewRuntimeDimensions {
        max_width: quantized_preview_dimension(canvas_width * zoom * PREVIEW_ADAPTIVE_SCALE)
            .clamp(PREVIEW_MIN_ADAPTIVE_WIDTH, DEFAULT_PREVIEW_MAX_WIDTH),
        max_height: quantized_preview_dimension(canvas_height * zoom * PREVIEW_ADAPTIVE_SCALE)
            .clamp(PREVIEW_MIN_ADAPTIVE_HEIGHT, DEFAULT_PREVIEW_MAX_HEIGHT),
    }
}

const fn default_preview_runtime_dimensions() -> PreviewRuntimeDimensions {
    PreviewRuntimeDimensions {
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
    }
}

fn quantized_preview_dimension(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let quantum = f64::from(PREVIEW_DIMENSION_QUANTUM);
    let quantized = (value / quantum).ceil() * quantum;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "preview dimensions are finite and clamped before use"
    )]
    {
        quantized.min(f64::from(u32::MAX)) as u32
    }
}

fn preview_visual_hash(config: &ConversionConfig) -> u64 {
    let mut state = DefaultHasher::new();
    config.rotation.hash(&mut state);
    config.flip_horizontal.hash(&mut state);
    config.flip_vertical.hash(&mut state);
    config.resolution.hash(&mut state);
    config.custom_width.hash(&mut state);
    config.custom_height.hash(&mut state);
    config.scaling_algorithm.hash(&mut state);
    config.fps.hash(&mut state);
    hash_crop(config.crop.as_ref(), &mut state);
    config.subtitle_burn_path.hash(&mut state);
    config.subtitle_font_name.hash(&mut state);
    config.subtitle_font_size.hash(&mut state);
    config.subtitle_font_color.hash(&mut state);
    config.subtitle_outline_color.hash(&mut state);
    config.subtitle_position.hash(&mut state);
    hash_overlay(config.overlay.as_ref(), &mut state);
    config.gif_colors.hash(&mut state);
    config.gif_dither.hash(&mut state);
    state.finish()
}

fn preview_audio_hash(
    config: &ConversionConfig,
    has_audio: bool,
    selected_audio_track: Option<u32>,
) -> u64 {
    let mut state = DefaultHasher::new();
    has_audio.hash(&mut state);
    selected_audio_track.hash(&mut state);
    config.audio_volume.hash(&mut state);
    config.audio_normalize.hash(&mut state);
    state.finish()
}

fn hash_crop(crop: Option<&CropSettings>, state: &mut DefaultHasher) {
    let Some(crop) = crop else {
        false.hash(state);
        return;
    };

    crop.enabled.hash(state);
    crop.x.hash(state);
    crop.y.hash(state);
    crop.width.hash(state);
    crop.height.hash(state);
    crop.source_width.hash(state);
    crop.source_height.hash(state);
    crop.aspect_ratio.hash(state);
}

fn hash_overlay(overlay: Option<&OverlaySettings>, state: &mut DefaultHasher) {
    let Some(overlay) = overlay else {
        false.hash(state);
        return;
    };

    overlay.enabled.hash(state);
    overlay.path.hash(state);
    overlay.x.to_bits().hash(state);
    overlay.y.to_bits().hash(state);
    overlay.width.to_bits().hash(state);
    overlay.opacity.to_bits().hash(state);
    overlay.anchor.hash(state);
}

fn engine_source_kind(metadata: &SourceMetadata) -> EnginePreviewSourceKind {
    match metadata.source_kind() {
        SourceKind::Video => EnginePreviewSourceKind::Video,
        SourceKind::Audio => EnginePreviewSourceKind::Audio,
        SourceKind::Image => EnginePreviewSourceKind::Image,
    }
}

fn preview_has_audio(metadata: &SourceMetadata, source_kind: EnginePreviewSourceKind) -> bool {
    source_kind == EnginePreviewSourceKind::Audio
        || !metadata.audio_tracks.is_empty()
        || metadata
            .audio_codec
            .as_deref()
            .is_some_and(|codec| !codec.trim().is_empty())
}

fn preview_overlay_from_settings(settings: &OverlaySettings) -> PreviewOverlay {
    PreviewOverlay {
        enabled: settings.enabled,
        path: settings.path.clone(),
        x: settings.x,
        y: settings.y,
        width: settings.width,
        opacity: settings.opacity,
        anchor: settings.anchor.clone(),
    }
}

fn overlay_settings_from_preview(overlay: &PreviewOverlay) -> OverlaySettings {
    OverlaySettings {
        enabled: overlay.enabled,
        path: overlay.path.clone(),
        x: overlay.x,
        y: overlay.y,
        width: overlay.width,
        opacity: overlay.opacity,
        anchor: overlay.anchor.clone(),
    }
}

fn load_preview_overlay_image_dimensions(path: PathBuf) -> Option<PreviewOverlayImageDimensions> {
    let (width, height) = image::image_dimensions(path).ok()?;
    if width == 0 || height == 0 {
        return None;
    }

    Some(PreviewOverlayImageDimensions { width, height })
}

const fn valid_preview_dimensions(
    width: Option<u32>,
    height: Option<u32>,
) -> (Option<u32>, Option<u32>) {
    match (width, height) {
        (Some(width), Some(height))
            if width >= MIN_PREVIEW_DIMENSION && height >= MIN_PREVIEW_DIMENSION =>
        {
            (Some(width), Some(height))
        }
        _ => (None, None),
    }
}

pub(in crate::app) const fn clamp_preview_canvas_zoom(value: f64) -> f64 {
    value.clamp(PREVIEW_CANVAS_MIN_ZOOM, PREVIEW_CANVAS_MAX_ZOOM)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewCanvasLayoutMetrics {
    pub(in crate::app) width: f64,
    pub(in crate::app) height: f64,
    pub(in crate::app) left: f64,
    pub(in crate::app) top: f64,
}

pub(in crate::app) fn preview_canvas_layout_metrics(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
) -> Option<PreviewCanvasLayoutMetrics> {
    if viewport_width <= 0.0 || viewport_height <= 0.0 || media_width <= 0.0 || media_height <= 0.0
    {
        return None;
    }

    let fit_scale = (viewport_width / media_width).min(viewport_height / media_height);
    if !fit_scale.is_finite() || fit_scale <= 0.0 {
        return None;
    }

    let width = media_width * fit_scale * zoom;
    let height = media_height * fit_scale * zoom;
    Some(PreviewCanvasLayoutMetrics {
        width,
        height,
        left: (viewport_width / 2.0) + pan_x - (width / 2.0),
        top: (viewport_height / 2.0) + pan_y - (height / 2.0),
    })
}

pub(in crate::app) fn preview_canvas_initial_zoom(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
) -> Option<f64> {
    if viewport_width <= 0.0 || viewport_height <= 0.0 || media_width <= 0.0 || media_height <= 0.0
    {
        return None;
    }

    let width_scale = viewport_width / media_width;
    let height_scale = viewport_height / media_height;
    let contain_scale = width_scale.min(height_scale);
    if !contain_scale.is_finite() || contain_scale <= 0.0 {
        return None;
    }

    Some(clamp_preview_canvas_zoom(
        PREVIEW_CANVAS_INITIAL_CONTAIN_SCALE,
    ))
}

pub(in crate::app) fn preview_canvas_pan_limits(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
    zoom: f64,
) -> Option<(f64, f64)> {
    let metrics = preview_canvas_layout_metrics(
        viewport_width,
        viewport_height,
        media_width,
        media_height,
        zoom,
        0.0,
        0.0,
    )?;
    Some((
        (viewport_width * PREVIEW_CANVAS_MAX_PAN).max(metrics.width) / 2.0,
        (viewport_height * PREVIEW_CANVAS_MAX_PAN).max(metrics.height) / 2.0,
    ))
}

const PREVIEW_KEYBOARD_NUDGE: f64 = 0.01;
const PREVIEW_KEYBOARD_LARGE_NUDGE: f64 = 0.05;

pub(in crate::app) fn preview_crop_keyboard_delta(
    handle: DragHandle,
    key: &str,
    large_step: bool,
) -> Option<PreviewPoint> {
    let step = if large_step {
        PREVIEW_KEYBOARD_LARGE_NUDGE
    } else {
        PREVIEW_KEYBOARD_NUDGE
    };
    let horizontal = matches!(
        handle,
        DragHandle::Move
            | DragHandle::East
            | DragHandle::West
            | DragHandle::NorthEast
            | DragHandle::NorthWest
            | DragHandle::SouthEast
            | DragHandle::SouthWest
    );
    let vertical = matches!(
        handle,
        DragHandle::Move
            | DragHandle::North
            | DragHandle::South
            | DragHandle::NorthEast
            | DragHandle::NorthWest
            | DragHandle::SouthEast
            | DragHandle::SouthWest
    );

    match key {
        "left" if horizontal => Some(PreviewPoint { x: -step, y: 0.0 }),
        "right" if horizontal => Some(PreviewPoint { x: step, y: 0.0 }),
        "up" if vertical => Some(PreviewPoint { x: 0.0, y: -step }),
        "down" if vertical => Some(PreviewPoint { x: 0.0, y: step }),
        _ => None,
    }
}

pub(in crate::app) fn preview_overlay_keyboard_delta(
    key: &str,
    large_step: bool,
) -> Option<PreviewPoint> {
    let step = if large_step {
        PREVIEW_KEYBOARD_LARGE_NUDGE
    } else {
        PREVIEW_KEYBOARD_NUDGE
    };
    match key {
        "left" => Some(PreviewPoint { x: -step, y: 0.0 }),
        "right" => Some(PreviewPoint { x: step, y: 0.0 }),
        "up" => Some(PreviewPoint { x: 0.0, y: -step }),
        "down" => Some(PreviewPoint { x: 0.0, y: step }),
        _ => None,
    }
}

pub(in crate::app) fn preview_canvas_keyboard_pan_delta(key: &str) -> Option<PreviewPoint> {
    const PAN_STEP: f64 = 24.0;
    match key {
        "left" => Some(PreviewPoint {
            x: -PAN_STEP,
            y: 0.0,
        }),
        "right" => Some(PreviewPoint {
            x: PAN_STEP,
            y: 0.0,
        }),
        "up" => Some(PreviewPoint {
            x: 0.0,
            y: -PAN_STEP,
        }),
        "down" => Some(PreviewPoint {
            x: 0.0,
            y: PAN_STEP,
        }),
        _ => None,
    }
}

fn preview_overlay_keyboard_start_point(
    handle: OverlayDragHandle,
    overlay: &PreviewOverlay,
    height: f64,
) -> OverlayDragPoint {
    let left = overlay.x - overlay.width / 2.0;
    let right = overlay.x + overlay.width / 2.0;
    let top = overlay.y - height / 2.0;
    let bottom = overlay.y + height / 2.0;
    let (x, y) = match handle {
        OverlayDragHandle::Move => (overlay.x, overlay.y),
        OverlayDragHandle::NorthWest => (left, top),
        OverlayDragHandle::NorthEast => (right, top),
        OverlayDragHandle::SouthEast => (right, bottom),
        OverlayDragHandle::SouthWest => (left, bottom),
    };

    OverlayDragPoint {
        x,
        y,
        width: Some(overlay.width),
        height: Some(height),
    }
}

pub(in crate::app) fn lerp_preview_canvas_value(current: f64, target: f64) -> f64 {
    (target - current).mul_add(PREVIEW_CANVAS_LERP_FACTOR, current)
}

pub(in crate::app) fn preview_canvas_wheel_zoom_multiplier(delta_y: f64) -> Option<f64> {
    if !delta_y.is_finite() {
        return None;
    }

    let magnitude = delta_y.abs();
    if magnitude <= PREVIEW_CANVAS_WHEEL_DEADZONE {
        return None;
    }

    let steps =
        (magnitude - PREVIEW_CANVAS_WHEEL_DEADZONE).min(PREVIEW_CANVAS_WHEEL_MAX_STEPS_PER_EVENT);
    let exponent = if delta_y < 0.0 { steps } else { -steps };
    let multiplier = PREVIEW_CANVAS_WHEEL_ZOOM_STEP.powf(exponent);
    (multiplier.is_finite() && multiplier > 0.0).then_some(multiplier)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Canvas transform metrics are scalar layout values kept explicit for focused tests."
)]
pub(in crate::app) fn preview_canvas_transform_visual_delta(
    viewport_width: f64,
    viewport_height: f64,
    media_width: f64,
    media_height: f64,
    current_zoom: f64,
    target_zoom: f64,
    current_pan_x: f64,
    target_pan_x: f64,
    current_pan_y: f64,
    target_pan_y: f64,
) -> Option<f64> {
    let current = preview_canvas_layout_metrics(
        viewport_width,
        viewport_height,
        media_width,
        media_height,
        current_zoom,
        current_pan_x,
        current_pan_y,
    )?;
    let target = preview_canvas_layout_metrics(
        viewport_width,
        viewport_height,
        media_width,
        media_height,
        target_zoom,
        target_pan_x,
        target_pan_y,
    )?;

    Some(
        (target.left - current.left)
            .abs()
            .max((target.top - current.top).abs())
            .max((target.width - current.width).abs())
            .max((target.height - current.height).abs()),
    )
}

pub(in crate::app) fn preview_canvas_transform_settled(
    current_zoom: f64,
    target_zoom: f64,
    current_pan_x: f64,
    target_pan_x: f64,
    current_pan_y: f64,
    target_pan_y: f64,
) -> bool {
    (target_zoom - current_zoom).abs() <= PREVIEW_CANVAS_ZOOM_SNAP_EPSILON
        && (target_pan_x - current_pan_x).abs() <= PREVIEW_CANVAS_PAN_SNAP_EPSILON
        && (target_pan_y - current_pan_y).abs() <= PREVIEW_CANVAS_PAN_SNAP_EPSILON
}
