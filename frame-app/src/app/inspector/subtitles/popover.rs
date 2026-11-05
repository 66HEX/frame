use crate::app::inspector::subtitles::state::SettingsSubtitlePopover;
use crate::app::layout::SETTINGS_CONTROL_HEIGHT;
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::root::FrameRoot;
use gpui::{Context, Window};

pub(super) const SUBTITLE_FIELD_LABEL_STACK_HEIGHT: f32 = 20.0;

pub(super) const SUBTITLE_POPOVER_TRIGGER_GAP: f32 = 8.0;

pub(super) const SUBTITLE_POPOVER_TOP_OFFSET: f32 =
    SUBTITLE_FIELD_LABEL_STACK_HEIGHT + SETTINGS_CONTROL_HEIGHT + SUBTITLE_POPOVER_TRIGGER_GAP;

pub(super) fn subtitle_popover_progress(
    popover: SettingsSubtitlePopover,
    is_open: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> f32 {
    let progress = surface_motion(subtitle_popover_motion_key(popover), is_open, window, cx);

    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, move |root, _window, cx| {
            if root.finish_subtitle_popover_close(popover) {
                cx.notify();
            }
        });
    }

    progress
}

pub(super) const fn subtitle_popover_motion_key(popover: SettingsSubtitlePopover) -> &'static str {
    match popover {
        SettingsSubtitlePopover::FontName => "settings-subtitle-font-popover-motion",
        SettingsSubtitlePopover::FontSize => "settings-subtitle-size-popover-motion",
        SettingsSubtitlePopover::FontColor => "settings-subtitle-font-color-popover-motion",
        SettingsSubtitlePopover::OutlineColor => "settings-subtitle-outline-color-popover-motion",
    }
}
