use crate::app::components::{
    FrameSelectItem, FrameSelectProps, frame_field, frame_label, frame_select_control,
};
use crate::app::inspector::subtitles::popover::subtitle_popover_progress;
use crate::app::inspector::subtitles::view::SettingsSubtitleFontSelectState;
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::settings::{apply_subtitle_font_name, subtitle_font_options};
use gpui::{Context, ParentElement, Window};

pub(super) fn settings_subtitle_font_select(
    state: SettingsSubtitleFontSelectState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let display = state
        .config
        .subtitle_font_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("Default (e.g. Arial)");
    let options = subtitle_font_options(state.config, state.subtitle_fonts, state.disabled);
    let count = options.len();
    let values: Vec<String> = options.iter().map(|option| option.name.clone()).collect();
    let items = options
        .into_iter()
        .enumerate()
        .map(|(index, option)| FrameSelectItem {
            id: format!("subtitle-font-{}", option.name),
            label: option.name.clone(),
            enabled: !option.is_disabled,
            selected: option.is_selected,
            focus: if index == 0 {
                state.focuses.first_option.cloned()
            } else if index + 1 == count {
                state.focuses.last_option.cloned()
            } else {
                None
            },
        })
        .collect();
    let popover = state.popover;
    let open = state.active_popover == Some(popover);
    let progress = subtitle_popover_progress(popover, open, window, cx);
    frame_field()
        .child(frame_label("Font", state.palette))
        .child(frame_select_control(
            FrameSelectProps {
                id: "settings-subtitle-font-select",
                panel_id: "settings-subtitle-font-options",
                list_id: "settings-subtitle-font-options-list",
                scrollbar_id: "settings-subtitle-font-options-scrollbar",
                label: "Subtitle font",
                display,
                items,
                enabled: !state.disabled,
                open,
                rendered: state.rendered_popover == Some(popover),
                progress,
                trigger_variant: if state.palette.is_light() {
                    ButtonVariant::Accent
                } else {
                    ButtonVariant::Outline
                },
                trigger_focus: state.focuses.trigger,
                panel_focus: state.focuses.panel,
                scroll: state.scroll_handle,
                palette: state.palette,
            },
            move |root, open, _, _| {
                if open {
                    root.subtitle_ui.popover = Some(popover);
                    root.subtitle_ui.rendered_popover = Some(popover);
                } else {
                    root.close_subtitle_popover();
                }
            },
            move |root, index, _, _| {
                // Exit animations can retain the old hit tree. Ignore queued stale option clicks.
                if root.subtitle_ui.rendered_popover == Some(popover)
                    && root.subtitle_ui.popover != Some(popover)
                {
                    return;
                }
                root.update_selected_config(|config| {
                    apply_subtitle_font_name(config, &values[index])
                });
            },
            window,
            cx,
        ))
}
