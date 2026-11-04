use crate::app::inspector;
use crate::settings::{DEFAULT_SUBTITLE_FONT_COLOR, DEFAULT_SUBTITLE_OUTLINE_COLOR};
use gpui::{Bounds, FocusHandle, Pixels, ScrollHandle};

pub(in crate::app) struct SubtitleUiState {
    pub(in crate::app) mode: SettingsSubtitleMode,
    pub(in crate::app) external_track_index: Option<usize>,
    pub(in crate::app) popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) font_select_scroll_handle: ScrollHandle,
    pub(in crate::app) font_size_select_scroll_handle: ScrollHandle,
    pub(in crate::app) font_color_draft: String,
    pub(in crate::app) outline_color_draft: String,
    pub(in crate::app) font_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) outline_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) color_picker_bounds: SettingsSubtitleColorPickerBounds,
}

impl Default for SubtitleUiState {
    fn default() -> Self {
        Self {
            mode: SettingsSubtitleMode::Selectable,
            external_track_index: None,
            popover: None,
            rendered_popover: None,
            font_select_scroll_handle: ScrollHandle::new(),
            font_size_select_scroll_handle: ScrollHandle::new(),
            font_color_draft: DEFAULT_SUBTITLE_FONT_COLOR.to_uppercase(),
            outline_color_draft: DEFAULT_SUBTITLE_OUTLINE_COLOR.to_uppercase(),
            font_color_hsv_draft: inspector::hex_to_subtitle_hsv(DEFAULT_SUBTITLE_FONT_COLOR),
            outline_color_hsv_draft: inspector::hex_to_subtitle_hsv(DEFAULT_SUBTITLE_OUTLINE_COLOR),
            color_picker_bounds: SettingsSubtitleColorPickerBounds::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitlePopover {
    FontName,
    FontSize,
    FontColor,
    OutlineColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitleColorTarget {
    Font,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct SettingsSubtitleHsv {
    pub(in crate::app) h: f64,
    pub(in crate::app) s: f64,
    pub(in crate::app) v: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitleColorDragKind {
    SaturationValue,
    Hue,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitleMode {
    #[default]
    Selectable,
    BurnIn,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct SettingsSubtitleColorDrag {
    pub(in crate::app) target: SettingsSubtitleColorTarget,
    pub(in crate::app) kind: SettingsSubtitleColorDragKind,
    pub(in crate::app) base_hsv: SettingsSubtitleHsv,
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::app) struct SettingsSubtitleColorPickerBounds {
    pub(in crate::app) font_sv: Option<Bounds<Pixels>>,
    pub(in crate::app) font_hue: Option<Bounds<Pixels>>,
    pub(in crate::app) outline_sv: Option<Bounds<Pixels>>,
    pub(in crate::app) outline_hue: Option<Bounds<Pixels>>,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsSubtitleColorInputFocuses<'a> {
    pub(in crate::app) font: Option<&'a FocusHandle>,
    pub(in crate::app) outline: Option<&'a FocusHandle>,
}

#[derive(Clone, Copy, Default)]
pub(in crate::app) struct SettingsSubtitleFocuses<'a> {
    pub(in crate::app) add_external_files: Option<&'a FocusHandle>,
    pub(in crate::app) burn_file: Option<&'a FocusHandle>,
    pub(in crate::app) font_select: SettingsSubtitleSelectFocuses<'a>,
    pub(in crate::app) font_size_select: SettingsSubtitleSelectFocuses<'a>,
    pub(in crate::app) font_color: SettingsSubtitleColorPopoverFocuses<'a>,
    pub(in crate::app) outline_color: SettingsSubtitleColorPopoverFocuses<'a>,
}

#[derive(Clone, Copy, Default)]
pub(in crate::app) struct SettingsSubtitleSelectFocuses<'a> {
    pub(in crate::app) trigger: Option<&'a FocusHandle>,
    pub(in crate::app) panel: Option<&'a FocusHandle>,
    pub(in crate::app) first_option: Option<&'a FocusHandle>,
    pub(in crate::app) last_option: Option<&'a FocusHandle>,
}

#[derive(Clone, Copy, Default)]
pub(in crate::app) struct SettingsSubtitleColorPopoverFocuses<'a> {
    pub(in crate::app) trigger: Option<&'a FocusHandle>,
    pub(in crate::app) panel: Option<&'a FocusHandle>,
    pub(in crate::app) sv: Option<&'a FocusHandle>,
    pub(in crate::app) hue: Option<&'a FocusHandle>,
}
