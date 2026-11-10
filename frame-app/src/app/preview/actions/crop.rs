use crate::app::preview::crop::{
    crop_aspect_id, crop_base_dimensions, crop_rect_from_settings, crop_rect_is_full,
    crop_settings_from_rect, default_crop_rect, full_crop_rect, is_known_crop_aspect,
    next_rotation, preview_crop_controls_enabled, preview_crop_source_dimensions,
    preview_transform_controls_enabled,
};
use crate::app::preview::crop_overlay::FlipAxis;
use crate::app::preview::panel::PreviewCropRenderState;
use crate::app::preview::state::PreviewCropDragState;
use crate::app::root::FrameRoot;
use crate::preview::{
    ASPECT_OPTIONS, DragHandle, Point as PreviewPoint, PreviewRotation, adjust_rect_to_ratio,
    aspect_value, clamp_rect, transform_crop_rect,
};
use crate::settings::{ConversionConfig, SourceMetadata};

impl FrameRoot {
    pub(in crate::app) fn sync_preview_crop_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        selected_config: &ConversionConfig,
    ) {
        if self.preview_ui.crop_file_id.as_deref() != selected_file_id {
            self.cancel_selected_auto_crop();
            self.preview_ui.crop_file_id = selected_file_id.map(str::to_string);
            self.preview_ui.crop_mode = false;
            self.preview_ui.crop_options.popover = None;
            self.preview_ui.crop_options.custom = false;
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
        }

        if !self.preview_ui.crop_mode {
            self.cancel_selected_auto_crop();
            self.preview_ui.crop_aspect = selected_config
                .crop
                .as_ref()
                .and_then(|crop| crop.aspect_ratio.clone())
                .unwrap_or_else(|| "free".to_string());
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
        }
        self.sync_auto_crop_for_selection();
    }

    pub(in crate::app) fn preview_crop_render_state(
        &self,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
    ) -> PreviewCropRenderState {
        PreviewCropRenderState {
            request_epoch: self.preview_ui.runtime_request_generation,
            frame_ready: self.preview_edit_frame_ready(),
            auto_crop: self.auto_crop_render_state(),
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

    pub(in crate::app) fn toggle_selected_crop_mode(&mut self) -> bool {
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

        self.cancel_selected_auto_crop();

        if self.preview_ui.crop_mode {
            self.preview_ui.crop_mode = false;
            self.preview_ui.crop_options.popover = None;
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
            return true;
        }

        self.preview_ui.crop_options.popover = None;
        self.preview_ui.crop_mode = true;
        self.preview_ui.draft_crop = Some(applied_crop.unwrap_or_else(full_crop_rect));
        self.preview_ui.crop_aspect = crop_aspect;
        self.preview_ui.crop_options.custom |= !ASPECT_OPTIONS
            .iter()
            .any(|option| option.id == self.preview_ui.crop_aspect);
        if self.preview_ui.crop_options.custom {
            self.set_crop_custom_mode(self.preview_ui.crop_options.is_ratio);
        }
        true
    }

    pub(in crate::app) fn select_preview_crop_aspect(&mut self, aspect_id: &str) -> bool {
        if !self.preview_ui.crop_mode || !is_known_crop_aspect(aspect_id) {
            return false;
        }
        let cancelled = self.cancel_selected_auto_crop();

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

        cancelled
            || previous_aspect != self.preview_ui.crop_aspect
            || previous_rect != self.preview_ui.draft_crop
    }

    pub(in crate::app) fn reset_preview_crop_selection(&mut self) -> bool {
        if !self.preview_ui.crop_mode {
            return false;
        }
        let cancelled = self.cancel_selected_auto_crop();
        let was_custom = self.preview_ui.crop_options.custom;
        self.preview_ui.crop_options.custom = false;
        self.preview_ui.crop_options.custom_dirty = false;
        self.preview_ui.crop_options.custom_edit_base = None;
        self.preview_ui.crop_options.popover = None;

        let previous_rect = self.preview_ui.draft_crop;
        let previous_aspect = self.preview_ui.crop_aspect.clone();
        self.preview_ui.draft_crop = Some(if self.preview_ui.draft_crop.is_some() {
            full_crop_rect()
        } else {
            default_crop_rect()
        });
        self.preview_ui.crop_aspect = "free".to_string();
        cancelled
            || was_custom
            || previous_rect != self.preview_ui.draft_crop
            || previous_aspect != self.preview_ui.crop_aspect
    }

    pub(in crate::app) fn apply_selected_crop(&mut self) -> bool {
        if !self.preview_ui.crop_mode
            || self.file_queue.selected_file_locked()
            || self.update_installation_in_progress()
        {
            return false;
        }
        if self.preview_ui.crop_options.custom
            && self.preview_ui.crop_options.custom_dirty
            && !self.apply_custom_crop_values()
        {
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
        self.cancel_selected_auto_crop();
        let cleared_crop = next_crop.is_none();
        let crop_preview = self.crop_settings_preview();

        let _config_changed = self.update_selected_config(|config| {
            let mut changed = config.crop != next_crop;
            config.crop = next_crop;
            if let Some(preview) = crop_preview {
                changed |= config.resolution != preview.resolution
                    || config.custom_width != preview.custom_width
                    || config.custom_height != preview.custom_height;
                config.resolution = preview.resolution;
                config.custom_width = preview.custom_width;
                config.custom_height = preview.custom_height;
            }
            changed
        });
        self.preview_ui.crop_mode = false;
        self.preview_ui.crop_options.popover = None;
        self.preview_ui.draft_crop = None;
        self.preview_ui.crop_drag = None;
        if cleared_crop {
            self.preview_ui.crop_aspect = "free".to_string();
        }
        true
    }

    pub(in crate::app) fn rotate_selected_preview(&mut self) -> bool {
        self.cancel_selected_auto_crop();
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

    pub(in crate::app) fn toggle_selected_flip(&mut self, axis: FlipAxis) -> bool {
        self.cancel_selected_auto_crop();
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

    pub(in crate::app) fn apply_preview_crop_drag(
        &mut self,
        handle: DragHandle,
        point: PreviewPoint,
    ) -> bool {
        if !self.preview_ui.crop_mode || !self.preview_edit_frame_ready() {
            return false;
        }
        let cancelled = self.cancel_selected_auto_crop();
        self.finish_crop_custom_edit();
        let Some(current_rect) = self.preview_ui.draft_crop else {
            return cancelled;
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
        changed || cancelled
    }

    pub(in crate::app) const fn end_preview_crop_drag(&mut self) -> bool {
        let had_drag = self.preview_ui.crop_drag.is_some();
        self.preview_ui.crop_drag = None;
        had_drag
    }

    pub(in crate::app) fn adjust_preview_crop_from_keyboard_with_step(
        &mut self,
        handle: DragHandle,
        key: &str,
        large_step: bool,
    ) -> bool {
        if !self.preview_ui.crop_mode || !self.preview_edit_frame_ready() {
            return false;
        }
        let Some(current_rect) = self.preview_ui.draft_crop else {
            return false;
        };
        let Some(delta) = preview_crop_keyboard_delta(handle, key, large_step) else {
            return false;
        };
        let cancelled = self.cancel_selected_auto_crop();
        self.finish_crop_custom_edit();

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
        changed || cancelled
    }
}

pub(super) const PREVIEW_KEYBOARD_NUDGE: f64 = 0.01;

pub(super) const PREVIEW_KEYBOARD_LARGE_NUDGE: f64 = 0.05;

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
