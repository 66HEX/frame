use super::*;
use crate::app::preview_panel::preview_presented_frame;
use crate::conversion_runner::core_config_from_gpui;
use crate::numeric::rounded_f64_to_u64;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

impl FrameRoot {
    pub(super) fn selected_preview_runtime_request(
        &self,
        metadata_entry: &SourceMetadataEntry,
    ) -> Option<PreviewRuntimeRequest> {
        let selected_file = self.file_queue.selected_file()?;
        preview_runtime_request(selected_file, metadata_entry, !self.preview_ui.crop_mode)
    }

    pub(super) fn sync_preview_runtime_for_selection(
        &mut self,
        request: Option<PreviewRuntimeRequest>,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = request else {
            self.clear_preview_runtime();
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

        self.clear_preview_runtime();

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
                        root.refresh_preview_render_image();
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
        if !delta_y.is_finite() || delta_y.abs() <= f64::EPSILON {
            return false;
        }
        let multiplier = if delta_y < 0.0 {
            PREVIEW_CANVAS_WHEEL_ZOOM_STEP
        } else {
            1.0 / PREVIEW_CANVAS_WHEEL_ZOOM_STEP
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
                start_pan_x: self.preview_ui.canvas.target_pan_x,
                start_pan_y: self.preview_ui.canvas.target_pan_y,
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

    pub(in crate::app) fn set_preview_canvas_bounds(&mut self, bounds: Bounds<Pixels>) -> bool {
        let previous_bounds = self.preview_ui.canvas_bounds;
        self.preview_ui.canvas_bounds = Some(bounds);
        previous_bounds != Some(bounds) && self.apply_preview_canvas_auto_fit()
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
        if preview_canvas_transform_settled(
            self.preview_ui.canvas.current_zoom,
            self.preview_ui.canvas.target_zoom,
            self.preview_ui.canvas.current_pan_x,
            self.preview_ui.canvas.target_pan_x,
            self.preview_ui.canvas.current_pan_y,
            self.preview_ui.canvas.target_pan_y,
        ) {
            return false;
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

    pub(super) fn sync_preview_playback_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
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
            if self.preview_ui.playback.dragging().is_none() {
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
        self.apply_preview_media_command(command, true);
    }

    pub(super) fn preview_playback_state(&self) -> PreviewPlaybackState {
        self.preview_ui.playback.clone()
    }

    fn clear_preview_runtime(&mut self) {
        if let Some(session) = self.preview_ui.session.take() {
            session.stop();
        }
        self.preview_ui.runtime_key = None;
        self.preview_ui.pending_runtime_key = None;
        self.preview_ui.render_generation = 0;
        self.preview_ui.render_image = None;
        self.preview_ui.runtime_error = None;
    }

    fn refresh_preview_render_image(&mut self) -> bool {
        let Some(session) = &self.preview_ui.session else {
            return false;
        };
        let Some(latest) = session.latest_frame() else {
            return false;
        };
        if latest.generation == self.preview_ui.render_generation {
            return false;
        }

        match render_image_from_frame(&latest.frame) {
            Ok(image) => {
                self.preview_ui.render_generation = latest.generation;
                self.preview_ui.rendered_presentation = self.preview_ui.render_presentation;
                self.preview_ui.render_image = Some(image);
                self.preview_ui.runtime_error = None;
                self.apply_preview_canvas_auto_fit();
                true
            }
            Err(error) => {
                self.preview_ui.runtime_error = Some(error.to_string());
                false
            }
        }
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
                        if root.preview_ui.session.is_none() {
                            if canvas_changed {
                                cx.notify();
                                return true;
                            }
                            root.preview_ui.frame_tick_active = false;
                            return false;
                        }

                        if root.refresh_preview_render_image()
                            || root.preview_ui.playback.is_playing()
                            || canvas_changed
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
            overlay: self.preview_ui.overlay.overlay().cloned(),
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

                if root.commit_preview_overlay(Some(overlay)) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn set_selected_overlay_mode(&mut self, value: bool) -> bool {
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

        self.commit_preview_overlay(Some(overlay))
    }

    pub(super) fn set_selected_overlay_opacity(&mut self, value: f64) -> bool {
        let Some(overlay) = self
            .preview_ui
            .overlay
            .set_opacity(value, self.file_queue.selected_file_locked())
        else {
            return false;
        };

        self.commit_preview_overlay(Some(overlay))
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
        self.commit_preview_overlay(Some(overlay))
    }

    pub(super) fn end_preview_overlay_drag(&mut self) -> bool {
        let was_dragging = self.preview_ui.overlay.is_dragging();
        self.preview_ui.overlay.end_overlay_drag();
        was_dragging
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

        let changed = self.update_selected_config(|config| {
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
        changed
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
    pub(super) fn apply_preview_timeline_drag(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
    ) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        if self.preview_ui.playback.dragging().is_none() {
            if target == TimelineDragTarget::Scrub {
                let command = self.preview_ui.playback.seek_to_percent(percent);
                self.apply_preview_media_command(command, false);
                return true;
            }

            if !self.preview_ui.playback.begin_handle_drag(target) {
                return false;
            }
            if self.preview_ui.playback.is_playing() {
                self.apply_preview_media_command(PlaybackMediaCommand::pause(), true);
            }
        }

        let update = self.preview_ui.playback.drag_to_percent(percent);
        self.apply_preview_media_command(update.command, false);

        if let Some(trim) = update.trim {
            return self.update_selected_config(|config| {
                apply_trim_times(config, trim.start_time, trim.end_time)
            });
        }

        true
    }

    pub(in crate::app) const fn set_preview_timeline_track_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
    ) {
        self.preview_ui.timeline_track_bounds = Some(bounds);
    }

    pub(super) fn commit_preview_timeline_seek_at_position(
        &mut self,
        position: Point<Pixels>,
    ) -> bool {
        let Some(bounds) = self.preview_ui.timeline_track_bounds else {
            return false;
        };
        if !self.preview_timeline_enabled() {
            return false;
        }

        let percent = timeline_slider_percent_from_bounds(position, bounds);
        let command = self.preview_ui.playback.seek_once_to_percent(percent);
        self.apply_preview_media_command(command, true)
    }

    pub(super) fn end_preview_timeline_drag(&mut self) -> bool {
        if self.preview_ui.playback.dragging().is_none() {
            return false;
        }

        let end = self.preview_ui.playback.end_drag();
        let mut changed = self.apply_preview_media_command(end.command, true);
        if let Some(trim) = end.trim {
            changed |= self.update_selected_config(|config| {
                apply_trim_times(config, trim.start_time, trim.end_time)
            });
        }
        changed
    }

    pub(super) fn toggle_preview_playback(&mut self) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        let command = self.preview_ui.playback.toggle_play();
        self.apply_preview_media_command(command, true)
    }

    fn apply_preview_media_command(
        &mut self,
        command: PlaybackMediaCommand,
        precise_seek: bool,
    ) -> bool {
        let Some(session) = self.preview_ui.session.clone() else {
            return self.apply_preview_command_to_local_state(command);
        };

        if command.pause
            && let Err(error) = session.command(PreviewCommand::Pause)
        {
            self.preview_ui.runtime_error = Some(error.to_string());
            return false;
        }

        if let Some(seconds) = command.seek_to {
            let preview_command = if precise_seek {
                PreviewCommand::SeekPrecise(seconds)
            } else {
                PreviewCommand::SeekFast(seconds)
            };
            if let Err(error) = session.command(preview_command) {
                self.preview_ui.runtime_error = Some(error.to_string());
                return false;
            }
        }

        if command.play
            && let Err(error) = session.command(PreviewCommand::Play)
        {
            self.preview_ui.runtime_error = Some(error.to_string());
            return false;
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

fn preview_runtime_request(
    selected_file: &FileItem,
    metadata_entry: &SourceMetadataEntry,
    include_applied_crop: bool,
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
    let core_config = core_config_from_gpui(&preview_config);
    let presentation = PreviewRenderPresentation::default();
    let key = PreviewRuntimeKey {
        file_id: selected_file.id.clone(),
        path: selected_file.path.clone(),
        source_kind,
        source_width,
        source_height,
        duration_millis: rounded_f64_to_u64(duration_seconds * 1000.0),
        visual_hash: preview_visual_hash(&preview_config),
    };
    let config = PreviewSessionConfig {
        file_id: key.file_id.clone(),
        path: PathBuf::from(&selected_file.path),
        source_kind,
        source_width,
        source_height,
        duration_seconds,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: core_config,
    };

    Some(PreviewRuntimeRequest {
        key,
        config,
        presentation,
    })
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
    config.start_time.hash(&mut state);
    config.end_time.hash(&mut state);
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

pub(in crate::app) fn lerp_preview_canvas_value(current: f64, target: f64) -> f64 {
    (target - current).mul_add(PREVIEW_CANVAS_LERP_FACTOR, current)
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
