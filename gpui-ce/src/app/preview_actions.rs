use super::*;

impl FrameRoot {
    pub(super) fn sync_preview_crop_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        selected_config: &ConversionConfig,
    ) {
        if self.preview_crop_file_id.as_deref() != selected_file_id {
            self.preview_crop_file_id = selected_file_id.map(str::to_string);
            self.preview_crop_mode = false;
            self.preview_draft_crop = None;
            self.preview_crop_drag = None;
        }

        if !self.preview_crop_mode {
            self.preview_crop_aspect = selected_config
                .crop
                .as_ref()
                .and_then(|crop| crop.aspect_ratio.clone())
                .unwrap_or_else(|| "free".to_string());
            self.preview_draft_crop = None;
            self.preview_crop_drag = None;
        }
    }
    pub(super) fn preview_crop_render_state(
        &self,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
    ) -> PreviewCropRenderState {
        PreviewCropRenderState {
            crop_mode: self.preview_crop_mode,
            draft_crop: self.preview_draft_crop,
            applied_crop: crop_rect_from_settings(config.crop.as_ref(), config),
            crop_aspect: self.preview_crop_aspect.clone(),
            has_crop_dimensions: preview_crop_source_dimensions(metadata, &config.rotation)
                .is_some(),
            rotation: config.rotation.clone(),
            flip_horizontal: config.flip_horizontal,
            flip_vertical: config.flip_vertical,
        }
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

        if self.preview_crop_mode {
            self.preview_crop_mode = false;
            self.preview_draft_crop = None;
            self.preview_crop_drag = None;
            return true;
        }

        self.preview_crop_mode = true;
        self.preview_draft_crop = Some(applied_crop.unwrap_or_else(default_crop_rect));
        self.preview_crop_aspect = crop_aspect;
        true
    }
    pub(super) fn select_preview_crop_aspect(&mut self, aspect_id: &str) -> bool {
        if !self.preview_crop_mode || !is_known_crop_aspect(aspect_id) {
            return false;
        }

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let Some(dimensions) = preview_crop_source_dimensions(metadata.as_ref(), &config.rotation)
        else {
            return false;
        };
        let is_side_rotation = is_side_rotation(&config.rotation);

        let previous_aspect = self.preview_crop_aspect.clone();
        let previous_rect = self.preview_draft_crop;
        self.preview_crop_aspect = aspect_id.to_string();
        if let Some(rect) = self.preview_draft_crop {
            self.preview_draft_crop = Some(if let Some(ratio) = aspect_value(aspect_id) {
                clamp_rect(adjust_rect_to_ratio(
                    rect,
                    ratio,
                    f64::from(dimensions.width),
                    f64::from(dimensions.height),
                    is_side_rotation,
                ))
            } else {
                clamp_rect(rect)
            });
        }

        previous_aspect != self.preview_crop_aspect || previous_rect != self.preview_draft_crop
    }
    pub(super) fn reset_preview_crop_selection(&mut self) -> bool {
        if !self.preview_crop_mode {
            return false;
        }

        let previous_rect = self.preview_draft_crop;
        let previous_aspect = self.preview_crop_aspect.clone();
        self.preview_draft_crop = Some(if self.preview_draft_crop.is_some() {
            full_crop_rect()
        } else {
            default_crop_rect()
        });
        self.preview_crop_aspect = "free".to_string();
        previous_rect != self.preview_draft_crop || previous_aspect != self.preview_crop_aspect
    }
    pub(super) fn apply_selected_crop(&mut self) -> bool {
        if !self.preview_crop_mode {
            return false;
        }
        let Some(draft_crop) = self.preview_draft_crop else {
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
                &self.preview_crop_aspect,
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
        self.preview_crop_mode = false;
        self.preview_draft_crop = None;
        self.preview_crop_drag = None;
        if cleared_crop {
            self.preview_crop_aspect = "free".to_string();
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
        if !self.preview_crop_mode {
            return false;
        }
        let Some(current_rect) = self.preview_draft_crop else {
            return false;
        };

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let Some(dimensions) = preview_crop_source_dimensions(metadata.as_ref(), &config.rotation)
        else {
            return false;
        };
        let is_side_rotation = is_side_rotation(&config.rotation);

        let drag_state = match self.preview_crop_drag {
            Some(state) if state.handle == handle => state,
            _ => {
                let state = PreviewCropDragState {
                    handle,
                    start_rect: current_rect,
                    start_point: point,
                };
                self.preview_crop_drag = Some(state);
                state
            }
        };

        let next_rect = crate::preview::apply_visual_crop_drag(crate::preview::VisualCropDrag {
            start_rect: drag_state.start_rect,
            handle,
            start_point: drag_state.start_point,
            current_point: point,
            aspect_id: &self.preview_crop_aspect,
            source_width: f64::from(dimensions.width),
            source_height: f64::from(dimensions.height),
            is_side_rotation,
        });
        let changed = self.preview_draft_crop != Some(next_rect);
        self.preview_draft_crop = Some(next_rect);
        changed
    }
    pub(super) fn end_preview_crop_drag(&mut self) -> bool {
        let had_drag = self.preview_crop_drag.is_some();
        self.preview_crop_drag = None;
        had_drag
    }
    pub(super) fn apply_selected_trim_drag(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
    ) -> bool {
        if target == TimelineDragTarget::Scrub {
            return false;
        }

        let metadata = self.selected_source_metadata();
        let duration_seconds = preview_duration_seconds(metadata.as_ref());
        if duration_seconds <= 0.0 {
            return false;
        }

        let Some(config) = self.selected_config() else {
            return false;
        };
        let metadata_status = if metadata.is_some() {
            PreviewMetadataStatus::Ready
        } else {
            PreviewMetadataStatus::Idle
        };
        let availability = preview_control_availability(PreviewControlInput {
            metadata_status,
            source_media_kind: preview_source_media_kind(metadata.as_ref()),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        });
        if availability.trim_disabled {
            return false;
        }

        let mut playback = preview_playback_state(
            availability.media_kind,
            duration_seconds,
            config.start_time.as_deref(),
            config.end_time.as_deref(),
        );
        if !playback.begin_handle_drag(target) {
            return false;
        }

        let Some(trim) = playback.drag_to_percent(percent).trim else {
            return false;
        };

        self.update_selected_config(|config| {
            apply_trim_times(config, trim.start_time, trim.end_time)
        })
    }
    pub(super) fn resolve_selected_settings_tab(&mut self, metadata: Option<&SourceMetadata>) {
        let next_tab = self
            .selected_config()
            .map_or(SettingsTab::Source, |config| {
                resolve_active_settings_tab(self.settings_active_tab, config, metadata)
            });
        self.settings_active_tab = next_tab;
    }
}
