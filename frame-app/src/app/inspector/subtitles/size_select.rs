use crate::app::components::{
    FRAME_SELECT_MAX_HEIGHT, FrameSelectFocusTarget, apply_frame_select_popover_focus_trap,
    focus_frame_select_initial_target, focus_frame_select_target, frame_select_content_height,
    frame_select_last_focus, frame_select_option, frame_select_option_focus,
    frame_select_option_with_focus, frame_select_options_list, frame_select_popover,
    frame_select_scrollbar, frame_select_target_index, frame_select_trigger,
    frame_select_trigger_with_focus,
};
use crate::app::inspector::shared::settings_field_label;
use crate::app::inspector::subtitles::popover::{
    SUBTITLE_POPOVER_TOP_OFFSET, defer_focus_optional, focus_optional, subtitle_popover_progress,
};
use crate::app::inspector::subtitles::state::{
    SettingsSubtitlePopover, SettingsSubtitleSelectFocuses,
};
use crate::app::inspector::subtitles::view::SettingsSubtitleFontSizeSelectState;
use crate::app::motion::popover_slide_offset;
use crate::app::root::FrameRoot;
use crate::settings::{
    SubtitleFontSizeOption, apply_subtitle_font_size, subtitle_font_size_options,
};
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, ScrollHandle, Window, deferred, div};

#[expect(
    clippy::too_many_lines,
    reason = "Subtitle font size select keeps trigger, keyboard handling, popover, and scrollbar together for one GPUI control."
)]
pub(super) fn settings_subtitle_font_size_select(
    state: SettingsSubtitleFontSizeSelectState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let display = state
        .config
        .subtitle_font_size
        .as_deref()
        .filter(|size| !size.is_empty())
        .unwrap_or("Default");
    let options = subtitle_font_size_options(state.config, state.disabled);
    let enabled = !state.disabled;
    let popover = state.popover;
    let trigger = if let Some(focus) = state.focuses.trigger {
        frame_select_trigger_with_focus(
            "settings-subtitle-font-size-select",
            "Subtitle font size",
            display,
            enabled,
            state.active_popover == Some(popover),
            focus,
            state.palette,
            window,
            cx,
        )
    } else {
        frame_select_trigger(
            "settings-subtitle-font-size-select",
            "Subtitle font size",
            display,
            enabled,
            state.active_popover == Some(popover),
            state.palette,
            window,
            cx,
        )
    };
    let key_first_option_focus = state.focuses.first_option.cloned();
    let key_last_option_focus = frame_select_last_focus(
        options.len(),
        state.focuses.first_option,
        state.focuses.last_option,
    )
    .cloned();
    let key_scroll_handle = state.scroll_handle.clone();
    let option_count = options.len();

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("Size", state.palette))
        .child(
            trigger
                .on_click(cx.listener(move |root, event: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if event.is_keyboard() || !enabled {
                        return;
                    }
                    root.toggle_subtitle_popover(popover);
                    cx.notify();
                }))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                        if !enabled {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "down" | "up" if root.subtitle_ui.popover == Some(popover) => {
                                cx.stop_propagation();
                                focus_frame_select_initial_target(
                                    event.keystroke.key.as_str(),
                                    option_count,
                                    key_first_option_focus.as_ref(),
                                    key_last_option_focus.as_ref(),
                                    &key_scroll_handle,
                                    window,
                                    cx,
                                );
                            }
                            "down" | "up" | "home" | "end" => {
                                cx.stop_propagation();
                                root.subtitle_ui.popover = Some(popover);
                                root.subtitle_ui.rendered_popover = Some(popover);
                                focus_frame_select_initial_target(
                                    event.keystroke.key.as_str(),
                                    option_count,
                                    key_first_option_focus.as_ref(),
                                    key_last_option_focus.as_ref(),
                                    &key_scroll_handle,
                                    window,
                                    cx,
                                );
                                cx.notify();
                            }
                            "enter" | "space" if root.subtitle_ui.popover == Some(popover) => {
                                cx.stop_propagation();
                                root.close_subtitle_popover();
                                cx.notify();
                            }
                            "enter" | "space" => {
                                cx.stop_propagation();
                                root.subtitle_ui.popover = Some(popover);
                                root.subtitle_ui.rendered_popover = Some(popover);
                                key_scroll_handle.scroll_to_item(0);
                                defer_focus_optional(key_first_option_focus.clone(), window, cx);
                                cx.notify();
                            }
                            "escape" => {
                                cx.stop_propagation();
                                root.close_subtitle_popover();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }),
                ),
        );

    if state.rendered_popover == Some(popover) {
        let progress =
            subtitle_popover_progress(popover, state.active_popover == Some(popover), window, cx);
        let content_height = frame_select_content_height(options.len());
        let mut list = frame_select_options_list(
            "settings-subtitle-font-size-options-list",
            state.scroll_handle,
            content_height > FRAME_SELECT_MAX_HEIGHT,
        );

        let option_count = options.len();
        let option_keyboard_options = options.to_vec();
        for (index, option) in options.into_iter().enumerate() {
            let size = option.size;
            let is_enabled = !option.is_disabled;
            let option_focus = frame_select_option_focus(
                index,
                option_count,
                state.focuses.first_option,
                state.focuses.last_option,
            );
            list = list.child(settings_subtitle_size_option(
                option,
                is_enabled,
                state.active_popover == Some(popover),
                size,
                option_keyboard_options.clone(),
                index,
                option_count,
                option_focus,
                state.focuses,
                state.scroll_handle,
                state.palette,
                cx,
            ));
        }

        let mut popover = frame_select_popover(
            "settings-subtitle-font-size-options",
            SUBTITLE_POPOVER_TOP_OFFSET + popover_slide_offset(progress),
            progress,
            list,
            state.palette,
        );
        popover = apply_frame_select_popover_focus_trap(
            popover,
            state.focuses.panel,
            state.focuses.first_option,
            frame_select_last_focus(
                option_count,
                state.focuses.first_option,
                state.focuses.last_option,
            ),
            cx,
        );

        if content_height > FRAME_SELECT_MAX_HEIGHT {
            popover = popover.child(frame_select_scrollbar(
                "settings-subtitle-font-size-options-scrollbar",
                state.scroll_handle.clone(),
                content_height,
                state.palette,
            ));
        }

        field = field.child(deferred(popover).priority(10));
    }

    field
}

#[expect(
    clippy::too_many_arguments,
    clippy::option_if_let_else,
    reason = "Size options mirror the font option builder and preserve the explicit focus wiring."
)]
pub(super) fn settings_subtitle_size_option(
    option: SubtitleFontSizeOption,
    is_enabled: bool,
    is_open: bool,
    size: &'static str,
    keyboard_options: Vec<SubtitleFontSizeOption>,
    index: usize,
    option_count: usize,
    focus: Option<&FocusHandle>,
    focuses: SettingsSubtitleSelectFocuses<'_>,
    scroll_handle: &ScrollHandle,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let trigger_focus_for_click = focuses.trigger.cloned();
    let trigger_focus_for_key = focuses.trigger.cloned();
    let first_focus_for_key = focuses.first_option.cloned();
    let last_focus_for_key =
        frame_select_last_focus(option_count, focuses.first_option, focuses.last_option).cloned();
    let scroll_handle_for_key = scroll_handle.clone();
    let option = if let Some(focus) = focus {
        frame_select_option_with_focus(
            format!("subtitle-size-{size}"),
            option.size,
            option.is_selected,
            is_enabled,
            focus,
            palette,
        )
    } else {
        frame_select_option(
            format!("subtitle-size-{size}"),
            option.size,
            option.is_selected,
            is_enabled,
            palette,
        )
    };
    option
        .tab_stop(is_enabled && is_open)
        .aria_disabled(!is_enabled || !is_open)
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if event.is_keyboard() {
                return;
            }
            if !is_enabled || root.subtitle_ui.popover != Some(SettingsSubtitlePopover::FontSize) {
                return;
            }
            root.update_selected_config(|config| apply_subtitle_font_size(config, size));
            root.close_subtitle_popover();
            focus_optional(trigger_focus_for_click.as_ref(), window, cx);
            cx.notify();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                if !is_enabled
                    || root.subtitle_ui.popover != Some(SettingsSubtitlePopover::FontSize)
                {
                    return;
                }
                let key = event.keystroke.key.as_str();
                match key {
                    "enter" | "space" if is_enabled => {
                        cx.stop_propagation();
                        root.update_selected_config(|config| {
                            apply_subtitle_font_size(config, size)
                        });
                        root.close_subtitle_popover();
                        defer_focus_optional(trigger_focus_for_key.clone(), window, cx);
                        cx.notify();
                    }
                    "up" | "down" | "home" | "end" if is_enabled => {
                        let target_index = frame_select_target_index(
                            keyboard_options.len(),
                            Some(index),
                            key,
                            |index| !keyboard_options[index].is_disabled,
                        );
                        if let Some(target_index) = target_index {
                            focus_frame_select_target(
                                key,
                                FrameSelectFocusTarget {
                                    current_index: index,
                                    target_index,
                                    first_focus: first_focus_for_key.as_ref(),
                                    last_focus: last_focus_for_key.as_ref(),
                                    scroll_handle: &scroll_handle_for_key,
                                },
                                window,
                                cx,
                            );
                        }
                        cx.stop_propagation();
                    }
                    "escape" => {
                        cx.stop_propagation();
                        root.close_subtitle_popover();
                        focus_optional(trigger_focus_for_key.as_ref(), window, cx);
                        cx.notify();
                    }
                    _ => {}
                }
            }),
        )
}
