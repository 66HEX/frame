use crate::app::preview::actions::crop::{PREVIEW_KEYBOARD_LARGE_NUDGE, PREVIEW_KEYBOARD_NUDGE};
use crate::app::preview::panel::{
    PreviewMediaRenderState, PreviewOverlayRenderState, preview_source_media_kind,
};
use crate::app::preview::state::PreviewOverlayImageDimensions;
use crate::app::preview::timeline_slider_percent_from_bounds;
use crate::app::root::FrameRoot;
use crate::file_filters::is_supported_overlay_image_path;
use crate::native_dialogs::{overlay_image_dialog, pick_overlay_image_file};
use crate::preview::{
    MetadataStatus as PreviewMetadataStatus, OverlayDragHandle, OverlayDragPoint,
    OverlayModeChange, OverlaySizeDirection, Point as PreviewPoint, PreviewControlInput,
    PreviewOverlay, PreviewOverlayState, preview_control_availability,
};
use crate::preview_engine::MIN_PREVIEW_DIMENSION;
use crate::settings::{ConversionConfig, OverlaySettings};
use gpui::prelude::*;
use gpui::{Bounds, Context, Pixels, Point, Window};
use std::path::PathBuf;

impl FrameRoot {
    pub(in crate::app) fn sync_preview_overlay_for_selection(
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

    pub(in crate::app) fn preview_overlay_render_state(&self) -> PreviewOverlayRenderState {
        PreviewOverlayRenderState {
            overlay_mode: self.preview_ui.overlay.overlay_mode(),
            has_overlay: self.preview_ui.overlay.has_overlay(),
            overlay: self.preview_ui.overlay.render_overlay().cloned(),
            image_dimensions: self.preview_ui.overlay_image_dimensions,
        }
    }

    pub(in crate::app) fn trigger_selected_overlay(
        &mut self,
        window: &Window,
        cx: &Context<Self>,
    ) -> bool {
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

    pub(in crate::app) fn prompt_selected_overlay_image(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) {
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
                root.cancel_selected_auto_crop();
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

    pub(in crate::app) fn set_selected_overlay_mode(&mut self, value: bool) -> bool {
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

    pub(in crate::app) fn remove_selected_overlay(&mut self) -> bool {
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

    pub(in crate::app) fn nudge_selected_overlay_size(
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

    pub(in crate::app) fn set_selected_overlay_opacity(&mut self, value: f64) -> bool {
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

    pub(in crate::app) fn commit_preview_overlay_opacity_at_position(
        &mut self,
        position: Point<Pixels>,
    ) -> bool {
        let Some(bounds) = self.preview_ui.overlay_opacity_slider_bounds else {
            return false;
        };
        let opacity = timeline_slider_percent_from_bounds(position, bounds);
        self.set_selected_overlay_opacity(opacity)
    }

    pub(in crate::app) fn apply_preview_overlay_drag(
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

    pub(in crate::app) fn end_preview_overlay_drag(&mut self) -> bool {
        let was_dragging = self.preview_ui.overlay.is_dragging();
        self.preview_ui.overlay.end_overlay_drag();
        was_dragging
    }

    pub(in crate::app) fn adjust_preview_overlay_from_keyboard_with_step(
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

    pub(super) fn preview_overlay_height_ratio(
        &self,
        media: Option<PreviewMediaRenderState>,
    ) -> f64 {
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

    pub(super) fn apply_preview_overlay_mode_change(&mut self, change: OverlayModeChange) -> bool {
        if change.should_deactivate_crop {
            self.cancel_selected_auto_crop();
            self.preview_ui.crop_mode = false;
            self.preview_ui.crop_options.popover = None;
            self.preview_ui.draft_crop = None;
            self.preview_ui.crop_drag = None;
        }
        change.changed || change.should_deactivate_crop
    }

    pub(super) fn commit_preview_overlay(&mut self, overlay: Option<PreviewOverlay>) -> bool {
        let next_overlay = overlay.map(|overlay| overlay_settings_from_preview(&overlay));
        self.update_selected_config(|config| {
            let changed = config.overlay != next_overlay;
            config.overlay = next_overlay;
            changed
        })
    }

    pub(super) fn selected_preview_overlay_controls_enabled(&self) -> bool {
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

    pub(super) fn sync_preview_overlay_dimensions(&mut self, cx: &Context<Self>) {
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

    pub(super) fn clear_preview_overlay_dimensions(&mut self) {
        self.preview_ui.overlay_dimensions_key = None;
        self.preview_ui.pending_overlay_dimensions_key = None;
        self.preview_ui.overlay_image_dimensions = None;
    }
}

pub(super) fn preview_overlay_from_settings(settings: &OverlaySettings) -> PreviewOverlay {
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

pub(super) fn overlay_settings_from_preview(overlay: &PreviewOverlay) -> OverlaySettings {
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

pub(super) fn load_preview_overlay_image_dimensions(
    path: PathBuf,
) -> Option<PreviewOverlayImageDimensions> {
    let (width, height) = image::image_dimensions(path).ok()?;
    if width == 0 || height == 0 {
        return None;
    }

    Some(PreviewOverlayImageDimensions { width, height })
}

pub(super) const fn valid_preview_dimensions(
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

pub(super) fn preview_overlay_keyboard_start_point(
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
