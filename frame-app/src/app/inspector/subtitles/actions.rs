use crate::app::input::FrameTextInputKind;
use crate::app::inspector::subtitles::color::{
    hex_to_subtitle_hsv, subtitle_hsv_from_hue_bounds, subtitle_hsv_from_sv_bounds,
    subtitle_hsv_to_hex,
};
use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorDrag, SettingsSubtitleColorDragKind, SettingsSubtitleColorTarget,
    SettingsSubtitleHsv, SettingsSubtitleMode, SettingsSubtitlePopover,
};
use crate::app::root::FrameRoot;
use crate::settings::{
    apply_subtitle_font_color, apply_subtitle_outline_color, normalized_hex_color,
};
use gpui::{Bounds, Pixels, Point};

impl FrameRoot {
    pub(in crate::app) fn select_subtitle_mode(&mut self, mode: SettingsSubtitleMode) -> bool {
        if self.subtitle_ui.mode == mode {
            return false;
        }

        self.subtitle_ui.mode = mode;
        if mode == SettingsSubtitleMode::Selectable {
            self.subtitle_ui.popover = None;
            self.subtitle_ui.rendered_popover = None;
        }
        true
    }

    pub(in crate::app) fn toggle_subtitle_popover(&mut self, popover: SettingsSubtitlePopover) {
        if self.subtitle_ui.popover == Some(popover) {
            self.subtitle_ui.popover = None;
        } else {
            self.subtitle_ui.popover = Some(popover);
            self.subtitle_ui.rendered_popover = Some(popover);
        }
    }

    pub(in crate::app) const fn close_subtitle_popover(&mut self) {
        self.subtitle_ui.popover = None;
        if matches!(
            self.text_input_ui.active,
            Some(
                FrameTextInputKind::SubtitleFontColorHex
                    | FrameTextInputKind::SubtitleOutlineColorHex
            )
        ) {
            self.stop_text_input_cursor();
        }
    }

    pub(in crate::app) fn finish_subtitle_popover_close(
        &mut self,
        popover: SettingsSubtitlePopover,
    ) -> bool {
        if self.subtitle_ui.popover.is_some() || self.subtitle_ui.rendered_popover != Some(popover)
        {
            return false;
        }
        self.subtitle_ui.rendered_popover = None;
        true
    }

    pub(in crate::app) fn open_subtitle_color_popover(
        &mut self,
        popover: SettingsSubtitlePopover,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) {
        self.subtitle_ui.popover = Some(popover);
        self.subtitle_ui.rendered_popover = Some(popover);
        self.set_subtitle_color_hsv_draft(target, hex_to_subtitle_hsv(value));
        self.set_subtitle_color_draft(target, value.to_uppercase());
    }

    pub(in crate::app) fn toggle_subtitle_color_popover(
        &mut self,
        popover: SettingsSubtitlePopover,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) {
        if self.subtitle_ui.popover == Some(popover) {
            self.close_subtitle_popover();
        } else {
            self.open_subtitle_color_popover(popover, target, value);
        }
    }

    pub(in crate::app) fn set_subtitle_color_draft(
        &mut self,
        target: SettingsSubtitleColorTarget,
        value: String,
    ) -> bool {
        match target {
            SettingsSubtitleColorTarget::Font => {
                if self.subtitle_ui.font_color_draft == value {
                    return false;
                }
                self.subtitle_ui.font_color_draft = value;
            }
            SettingsSubtitleColorTarget::Outline => {
                if self.subtitle_ui.outline_color_draft == value {
                    return false;
                }
                self.subtitle_ui.outline_color_draft = value;
            }
        }
        true
    }

    pub(in crate::app) fn set_subtitle_color_hsv_draft(
        &mut self,
        target: SettingsSubtitleColorTarget,
        hsv: SettingsSubtitleHsv,
    ) -> bool {
        match target {
            SettingsSubtitleColorTarget::Font => {
                if self.subtitle_ui.font_color_hsv_draft == hsv {
                    return false;
                }
                self.subtitle_ui.font_color_hsv_draft = hsv;
            }
            SettingsSubtitleColorTarget::Outline => {
                if self.subtitle_ui.outline_color_hsv_draft == hsv {
                    return false;
                }
                self.subtitle_ui.outline_color_hsv_draft = hsv;
            }
        }
        true
    }

    pub(super) const fn subtitle_color_hsv_draft(
        &self,
        target: SettingsSubtitleColorTarget,
    ) -> SettingsSubtitleHsv {
        match target {
            SettingsSubtitleColorTarget::Font => self.subtitle_ui.font_color_hsv_draft,
            SettingsSubtitleColorTarget::Outline => self.subtitle_ui.outline_color_hsv_draft,
        }
    }

    pub(in crate::app) fn commit_subtitle_color(
        &mut self,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) -> bool {
        let Some(normalized) = normalized_hex_color(value) else {
            return self.set_subtitle_color_draft(target, value.to_uppercase());
        };
        let draft_changed = self.set_subtitle_color_draft(target, normalized.to_uppercase());
        let hsv_changed =
            self.set_subtitle_color_hsv_draft(target, hex_to_subtitle_hsv(&normalized));
        let config_changed = self.update_selected_config(|config| match target {
            SettingsSubtitleColorTarget::Font => apply_subtitle_font_color(config, &normalized),
            SettingsSubtitleColorTarget::Outline => {
                apply_subtitle_outline_color(config, &normalized)
            }
        });
        draft_changed || hsv_changed || config_changed
    }

    pub(in crate::app) fn commit_subtitle_hsv_color(
        &mut self,
        target: SettingsSubtitleColorTarget,
        hsv: SettingsSubtitleHsv,
    ) -> bool {
        let hex = subtitle_hsv_to_hex(hsv.h, hsv.s, hsv.v);
        let draft_changed = self.set_subtitle_color_draft(target, hex.to_uppercase());
        let hsv_changed = self.set_subtitle_color_hsv_draft(target, hsv);
        let config_changed = self.update_selected_config(|config| match target {
            SettingsSubtitleColorTarget::Font => apply_subtitle_font_color(config, &hex),
            SettingsSubtitleColorTarget::Outline => apply_subtitle_outline_color(config, &hex),
        });
        draft_changed || hsv_changed || config_changed
    }

    pub(in crate::app) const fn set_subtitle_color_picker_bounds(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        bounds: Bounds<Pixels>,
    ) {
        match (target, kind) {
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::SaturationValue) => {
                self.subtitle_ui.color_picker_bounds.font_sv = Some(bounds);
            }
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.font_hue = Some(bounds);
            }
            (
                SettingsSubtitleColorTarget::Outline,
                SettingsSubtitleColorDragKind::SaturationValue,
            ) => {
                self.subtitle_ui.color_picker_bounds.outline_sv = Some(bounds);
            }
            (SettingsSubtitleColorTarget::Outline, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.outline_hue = Some(bounds);
            }
        }
    }

    pub(super) const fn subtitle_color_picker_bounds(
        &self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
    ) -> Option<Bounds<Pixels>> {
        match (target, kind) {
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::SaturationValue) => {
                self.subtitle_ui.color_picker_bounds.font_sv
            }
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.font_hue
            }
            (
                SettingsSubtitleColorTarget::Outline,
                SettingsSubtitleColorDragKind::SaturationValue,
            ) => self.subtitle_ui.color_picker_bounds.outline_sv,
            (SettingsSubtitleColorTarget::Outline, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.outline_hue
            }
        }
    }

    pub(in crate::app) fn commit_subtitle_color_at_position(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        position: Point<Pixels>,
    ) -> bool {
        let base_hsv = self.subtitle_color_hsv_draft(target);
        self.commit_subtitle_color_with_base_hsv(target, kind, position, base_hsv)
    }

    pub(in crate::app) fn commit_subtitle_color_drag_at_position(
        &mut self,
        drag: SettingsSubtitleColorDrag,
        position: Point<Pixels>,
    ) -> bool {
        self.commit_subtitle_color_with_base_hsv(drag.target, drag.kind, position, drag.base_hsv)
    }

    pub(super) fn commit_subtitle_color_with_base_hsv(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        position: Point<Pixels>,
        base_hsv: SettingsSubtitleHsv,
    ) -> bool {
        let Some(bounds) = self.subtitle_color_picker_bounds(target, kind) else {
            return false;
        };
        let hsv = match kind {
            SettingsSubtitleColorDragKind::SaturationValue => {
                subtitle_hsv_from_sv_bounds(position, bounds, base_hsv)
            }
            SettingsSubtitleColorDragKind::Hue => {
                subtitle_hsv_from_hue_bounds(position, bounds, base_hsv)
            }
        };
        self.commit_subtitle_hsv_color(target, hsv)
    }
}
