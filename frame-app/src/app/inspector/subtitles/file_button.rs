use super::state::SettingsSubtitleMode;
use crate::app::components::frame_text_button_inner;
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, Window};

/// The same file action in both subtitle modes; only the picker contract differs.
pub(super) fn settings_subtitle_file_button(
    mode: SettingsSubtitleMode,
    enabled: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let (id, label) = match mode {
        SettingsSubtitleMode::Selectable => {
            ("settings-subtitle-add-external", "Add subtitle files")
        }
        SettingsSubtitleMode::BurnIn => ("settings-subtitle-burn-file", "Add subtitle file"),
    };
    frame_text_button_inner(
        id,
        label,
        ButtonVariant::Outline,
        false,
        enabled,
        focus,
        palette,
        window,
        cx,
    )
    .w_full()
    .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        if enabled {
            match mode {
                SettingsSubtitleMode::Selectable => root.prompt_external_subtitle_files(window, cx),
                SettingsSubtitleMode::BurnIn => root.prompt_subtitle_burn_file(window, cx),
            }
        }
    }))
}
