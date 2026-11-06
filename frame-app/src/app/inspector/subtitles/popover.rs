use crate::app::inspector::subtitles::state::SettingsSubtitlePopover;
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::root::FrameRoot;
use gpui::{Context, Window};

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
