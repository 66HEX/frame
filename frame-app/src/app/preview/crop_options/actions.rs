use crate::app::preview::crop::{crop_base_dimensions, crop_settings_from_rect, full_crop_rect};
use crate::app::root::FrameRoot;
use crate::preview::{CropRect, PreviewRotation, transform_crop_rect};
use crate::settings::CropSettings;

impl FrameRoot {
    pub(in crate::app) fn set_crop_custom_mode(&mut self, ratio: bool) {
        self.preview_ui.crop_options.is_ratio = ratio;
        self.preview_ui.crop_options.custom_dirty = false;
        self.preview_ui.crop_options.custom_edit_base = None;
        if ratio {
            let (width, height) =
                if let Some((width, height)) = self.preview_ui.crop_aspect.split_once(':') {
                    (width.to_string(), height.to_string())
                } else if let Some(crop) = self.current_draft_crop_settings() {
                    let mut divisor = crop.width;
                    let mut remainder = crop.height;
                    while remainder != 0 {
                        (divisor, remainder) = (remainder, divisor % remainder);
                    }
                    let divisor = divisor.max(1);
                    (
                        (crop.width / divisor).to_string(),
                        (crop.height / divisor).to_string(),
                    )
                } else {
                    ("1".to_string(), "1".to_string())
                };
            self.preview_ui.crop_options.width = width;
            self.preview_ui.crop_options.height = height;
        } else if let Some(crop) = self.current_draft_crop_settings() {
            self.preview_ui.crop_options.width = crop.width.to_string();
            self.preview_ui.crop_options.height = crop.height.to_string();
        }
    }

    pub(super) fn current_draft_crop_settings(&self) -> Option<CropSettings> {
        let config = self.selected_config()?;
        crop_settings_from_rect(
            self.preview_ui.draft_crop?,
            &self.preview_ui.crop_aspect,
            &config.rotation,
            config.flip_horizontal,
            config.flip_vertical,
            self.selected_source_metadata().as_ref(),
        )
    }

    pub(in crate::app) fn custom_crop_values(&self) -> Option<(u32, u32)> {
        if !self.preview_ui.crop_mode
            || self.file_queue.selected_file_locked()
            || self.update_installation_in_progress()
        {
            return None;
        }
        let width = self.preview_ui.crop_options.width.parse::<u32>().ok()?;
        let height = self.preview_ui.crop_options.height.parse::<u32>().ok()?;
        if width == 0 || height == 0 || width > 65_535 || height > 65_535 {
            return None;
        }
        if !self.preview_ui.crop_options.is_ratio {
            let metadata = self.selected_source_metadata()?;
            let dimensions =
                crop_base_dimensions(Some(&metadata), &self.selected_config()?.rotation)?;
            if width > dimensions.width || height > dimensions.height {
                return None;
            }
        }
        Some((width, height))
    }

    pub(super) fn custom_crop_selection(&self) -> Option<(CropRect, String)> {
        let (width, height) = self.custom_crop_values()?;
        let config = self.selected_config()?;
        let metadata = self.selected_source_metadata();
        let dimensions = crop_base_dimensions(metadata.as_ref(), &config.rotation)?;
        let rotation = PreviewRotation::from(config.rotation.as_str());
        let current = transform_crop_rect(
            self.preview_ui
                .crop_options
                .custom_edit_base
                .or(self.preview_ui.draft_crop)
                .unwrap_or_else(full_crop_rect),
            rotation,
            config.flip_horizontal,
            config.flip_vertical,
            false,
        );
        let (rect, aspect) = if self.preview_ui.crop_options.is_ratio {
            (
                crate::preview::adjust_rect_to_ratio(
                    current,
                    f64::from(width) / f64::from(height),
                    f64::from(dimensions.width),
                    f64::from(dimensions.height),
                    false,
                ),
                format!("{width}:{height}"),
            )
        } else {
            let w = f64::from(width) / f64::from(dimensions.width);
            let h = f64::from(height) / f64::from(dimensions.height);
            (
                CropRect {
                    x: (current.x + current.width / 2.0 - w / 2.0).clamp(0.0, 1.0 - w),
                    y: (current.y + current.height / 2.0 - h / 2.0).clamp(0.0, 1.0 - h),
                    width: w,
                    height: h,
                },
                "free".to_string(),
            )
        };
        Some((
            transform_crop_rect(
                rect,
                rotation,
                config.flip_horizontal,
                config.flip_vertical,
                true,
            ),
            aspect,
        ))
    }

    pub(in crate::app) fn apply_custom_crop_values(&mut self) -> bool {
        let Some((rect, aspect)) = self.custom_crop_selection() else {
            return false;
        };
        self.preview_ui.draft_crop = Some(rect);
        self.preview_ui.crop_aspect = aspect;
        self.preview_ui.crop_options.popover = None;
        self.preview_ui.crop_options.custom_dirty = false;
        self.preview_ui.crop_options.custom_edit_base = None;
        true
    }

    pub(in crate::app) fn preview_custom_crop_values(&mut self) {
        if !self.preview_ui.crop_options.custom {
            return;
        }
        // Both fields share the pre-edit rectangle so intermediate values cannot
        // progressively shrink the selection while typing a new ratio.
        if let Some((rect, aspect)) = self.custom_crop_selection() {
            self.preview_ui.draft_crop = Some(rect);
            self.preview_ui.crop_aspect = aspect;
        }
    }

    pub(in crate::app) fn finish_crop_custom_edit(&mut self) {
        if self.preview_ui.crop_options.custom && self.preview_ui.crop_options.custom_dirty {
            // Moving or resizing now starts from the visible selection.
            self.set_crop_custom_mode(self.preview_ui.crop_options.is_ratio);
        }
    }

    pub(in crate::app) fn crop_settings_preview(
        &self,
    ) -> Option<crate::settings::ConversionConfig> {
        if !self.preview_ui.crop_mode {
            return None;
        }
        let mut config = self.selected_config()?.clone();
        let metadata = self.selected_source_metadata()?;
        let rect = self.preview_ui.draft_crop?;
        let aspect = &self.preview_ui.crop_aspect;
        config.crop = crop_settings_from_rect(
            rect,
            aspect,
            &config.rotation,
            config.flip_horizontal,
            config.flip_vertical,
            Some(&metadata),
        );
        if crate::app::preview::crop_rect_is_full(rect) {
            config.crop = None;
        }
        config.resolution = "original".to_string();
        config.custom_width = None;
        config.custom_height = None;
        let (width, height) = frame_core::dimensions::output_dimensions(
            &crate::conversion_runner::core_config_from_gpui(&config),
            metadata.width?,
            metadata.height?,
            metadata.pixel_format.as_deref(),
        )?;
        config.resolution = "custom".to_string();
        config.custom_width = Some(width.to_string());
        config.custom_height = Some(height.to_string());
        Some(config)
    }
}
