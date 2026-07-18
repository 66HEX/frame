use super::accessibility::focus_visible_ring;
use super::components::{
    FRAME_ICON_BUTTON_SM_SIZE, FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant,
    frame_icon_button, frame_icon_swap_button, frame_vertical_uniform_scrollbar,
};
use super::primitives::{
    FrameSurface, apply_button_motion, button_motion, card_surface_shadows, color, element_id,
    panel_bottom_separator,
};
use super::{
    ActiveLogFile, ClickEvent, Context, ConversionEventState, FileQueue, FluentBuilder, FrameRoot,
    InteractiveElement, IntoElement, LOG_LINE_HEIGHT, LOG_LINE_NUMBER_WIDTH,
    LOG_SCROLL_BUTTON_OFFSET, LOG_SCROLL_BUTTON_PADDING, LOG_SCROLL_BUTTON_SIZE,
    LOG_SCROLL_ICON_SIZE, Lerp, LogLine, PANEL_HEADER_HEIGHT, ParentElement, ScrollStrategy,
    ScrollWheelEvent, StatefulInteractiveElement, Styled, UniformListScrollHandle, Window, assets,
    div, px, theme, uniform_list,
};
use crate::numeric::usize_to_f32;

pub(super) fn logs_view(
    queue: &FileQueue,
    conversion_events: &ConversionEventState,
    scroll_handle: &UniformListScrollHandle,
    follow_tail: bool,
    copied_log_file_id: Option<&str>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let active_files = conversion_events.active_log_files(queue);
    let selected_id = conversion_events.selected_log_file_id();
    let selected_line_count = selected_id.map_or(0, |id| conversion_events.logs_for(id).len());
    let selected_logs_copied =
        selected_id.is_some_and(|id| copied_log_file_id.is_some_and(|copied| copied == id));

    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .child(logs_tab_strip(
            &active_files,
            selected_id,
            selected_line_count,
            selected_logs_copied,
            window,
            cx,
        ))
        .child(logs_body(
            conversion_events,
            selected_id,
            !active_files.is_empty(),
            scroll_handle,
            follow_tail,
            window,
            cx,
        ))
}

pub(super) fn logs_tab_strip(
    active_files: &[ActiveLogFile],
    selected_id: Option<&str>,
    selected_line_count: usize,
    selected_logs_copied: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut tabs = div()
        .id("logs-tab-list")
        .role(gpui::Role::TabList)
        .aria_label("Log files")
        .h_full()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap_6()
        .overflow_hidden();

    let tab_ids = active_files
        .iter()
        .map(|file| file.id.clone())
        .collect::<Vec<_>>();
    for file in active_files {
        tabs = tabs.child(log_tab_button(
            file,
            selected_id.is_some_and(|id| id == file.id),
            &tab_ids,
            window,
            cx,
        ));
    }

    if active_files.is_empty() {
        tabs = tabs.child(
            div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(theme::ui_text("No active processes")),
        );
    }

    let header = div()
        .h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .child(tabs)
        .when_some(selected_id, |this, selected_id| {
            this.child(logs_copy_button(
                selected_id,
                selected_line_count > 0,
                selected_logs_copied,
                window,
                cx,
            ))
        });

    header.child(panel_bottom_separator())
}

pub(super) fn log_tab_button(
    file: &ActiveLogFile,
    selected: bool,
    active_file_ids: &[String],
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let file_id = file.id.clone();
    let key_file_id = file.id.clone();
    let keyboard_file_ids = active_file_ids.to_vec();
    let motion = button_motion(element_id("logs-tab-hover", &file.id), window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    let foreground = if selected {
        color(theme::FOREGROUND)
    } else {
        color(theme::FRAME_GRAY_600).lerp(&color(theme::FOREGROUND), hover_progress)
    };

    let button = div()
        .id(element_id("logs-tab", &file.id))
        .role(gpui::Role::Tab)
        .aria_label(file.name.clone())
        .aria_selected(selected)
        .focusable()
        .tab_stop(true)
        .focus_visible(focus_visible_ring)
        .flex_none()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .hover(gpui::Styled::cursor_pointer)
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root.select_log_file_for_logs_view(&file_id) {
                cx.notify();
            }
            cx.stop_propagation();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_id) = log_tab_id_for_key(
                    key_file_id.as_str(),
                    &keyboard_file_ids,
                    event.keystroke.key.as_str(),
                ) else {
                    return;
                };
                if root.select_log_file_for_logs_view(next_id) {
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        )
        .child(file.name.clone());

    apply_button_motion(button, motion, true)
}

pub(super) fn logs_body(
    conversion_events: &ConversionEventState,
    selected_id: Option<&str>,
    has_active_files: bool,
    scroll_handle: &UniformListScrollHandle,
    follow_tail: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let body = div()
        .id("logs-body")
        .relative()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden();

    if !has_active_files {
        return body.child(logs_empty_state("Select a task to view console output"));
    }

    let Some(selected_id) = selected_id else {
        return body.child(logs_empty_state("Select a task to view console output"));
    };

    let line_count = conversion_events.logs_for(selected_id).len();
    if line_count == 0 {
        return body.child(logs_empty_state("Process started, waiting for output..."));
    }

    let body = body.child(log_lines_list(selected_id, line_count, scroll_handle, cx));
    if follow_tail {
        body
    } else {
        body.child(log_scroll_to_bottom_button(window, cx))
    }
}

pub(super) fn log_lines_list(
    selected_id: &str,
    line_count: usize,
    scroll_handle: &UniformListScrollHandle,
    cx: &Context<FrameRoot>,
) -> impl IntoElement {
    let selected_id = selected_id.to_string();
    let list_id = element_id("logs-line-list", &selected_id);
    let key_scroll_handle = scroll_handle.clone();
    let processor_selected_id = selected_id.clone();

    let list = uniform_list(
        list_id,
        line_count,
        cx.processor(move |root, range, _window, _cx| {
            root.conversion_events
                .log_line_window_for(&processor_selected_id, range)
                .into_iter()
                .map(log_line_row)
                .collect()
        }),
    )
    .track_scroll(scroll_handle)
    .on_scroll_wheel(cx.listener(|_root, _event: &ScrollWheelEvent, window, cx| {
        cx.defer_in(window, |root, _window, cx| {
            if root.sync_logs_follow_tail_after_user_scroll() {
                cx.notify();
            }
        });
        cx.notify();
    }))
    .size_full()
    .p(px(2.0))
    .text_color(color(theme::FOREGROUND))
    .line_height(px(LOG_LINE_HEIGHT));

    div()
        .relative()
        .id(element_id("logs-scroll-area", &selected_id))
        .role(gpui::Role::Log)
        .aria_label("Conversion log output")
        .focusable()
        .tab_stop(true)
        .focus_visible(focus_visible_ring)
        .size_full()
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(target) = log_scroll_target_for_key(
                    root.logs_keyboard_scroll_top,
                    line_count,
                    event.keystroke.key.as_str(),
                ) else {
                    return;
                };
                key_scroll_handle.scroll_to_item_strict(target, ScrollStrategy::Top);
                root.logs_keyboard_scroll_top = target;
                root.logs_follow_tail = target + 1 >= line_count;
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .child(list)
        .child(frame_vertical_uniform_scrollbar(
            "logs-line-list-scrollbar",
            scroll_handle,
            usize_to_f32(line_count) * LOG_LINE_HEIGHT,
        ))
}

fn log_tab_id_for_key<'a>(
    current_id: &str,
    active_file_ids: &'a [String],
    key: &str,
) -> Option<&'a str> {
    let current_index = active_file_ids.iter().position(|id| id == current_id)?;
    match key {
        "left" => Some(if current_index == 0 {
            active_file_ids.last()?.as_str()
        } else {
            active_file_ids[current_index - 1].as_str()
        }),
        "right" => Some(active_file_ids[(current_index + 1) % active_file_ids.len()].as_str()),
        "home" => active_file_ids.first().map(String::as_str),
        "end" => active_file_ids.last().map(String::as_str),
        _ => None,
    }
}

fn log_scroll_target_for_key(current_top: usize, line_count: usize, key: &str) -> Option<usize> {
    if line_count == 0 {
        return None;
    }
    match key {
        "pageup" => Some(current_top.saturating_sub(10)),
        "pagedown" => Some((current_top + 10).min(line_count - 1)),
        "home" => Some(0),
        "end" => Some(line_count - 1),
        _ => None,
    }
}

#[cfg(test)]
mod keyboard_tests {
    use super::*;

    fn log_ids() -> Vec<String> {
        vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ]
    }

    #[test]
    fn log_tab_id_for_key_wraps_left_from_first_tab() {
        let ids = log_ids();

        assert_eq!(log_tab_id_for_key("first", &ids, "left"), Some("third"));
    }

    #[test]
    fn log_tab_id_for_key_wraps_right_from_last_tab() {
        let ids = log_ids();

        assert_eq!(log_tab_id_for_key("third", &ids, "right"), Some("first"));
    }

    #[test]
    fn log_scroll_target_for_key_moves_page_down_by_ten_lines() {
        assert_eq!(log_scroll_target_for_key(3, 30, "pagedown"), Some(13));
    }

    #[test]
    fn log_scroll_target_for_key_clamps_page_up_to_start() {
        assert_eq!(log_scroll_target_for_key(3, 30, "pageup"), Some(0));
    }

    #[test]
    fn log_scroll_target_for_key_moves_end_to_last_line() {
        assert_eq!(log_scroll_target_for_key(3, 30, "end"), Some(29));
    }
}

pub(super) fn log_line_row(line: LogLine) -> impl IntoElement {
    let tone = log_line_tone(&line.text);
    let row_group = format!("logs-line-{}", line.index);

    div()
        .id(row_group.clone())
        .group(row_group.clone())
        .h(px(LOG_LINE_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .rounded(px(theme::RADIUS_XS))
        .overflow_hidden()
        .px_1()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .line_height(px(LOG_LINE_HEIGHT))
        .hover(|style| style.bg(color(theme::FRAME_GRAY_100)))
        .child(
            div()
                .flex_none()
                .w(px(LOG_LINE_NUMBER_WIDTH))
                .mr(px(12.0))
                .text_right()
                .text_color(color(theme::FRAME_GRAY_400))
                .font_features(assets::frame_tabular_number_font_features())
                .group_hover(row_group.clone(), |style| {
                    style.text_color(color(theme::FRAME_GRAY_600))
                })
                .child(line.index.to_string()),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_color(color(log_line_tone_color(tone)))
                .group_hover(row_group, move |style| {
                    style.text_color(color(log_line_hover_tone_color(tone)))
                })
                .child(line.text),
        )
}

pub(super) fn log_scroll_to_bottom_button(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    div()
        .absolute()
        .right(px(LOG_SCROLL_BUTTON_OFFSET))
        .bottom(px(LOG_SCROLL_BUTTON_OFFSET))
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(LOG_SCROLL_BUTTON_PADDING))
        .shadow(card_surface_shadows())
        .child(
            frame_icon_button(
                "logs-scroll-to-bottom",
                assets::ICON_ARROW_DOWN,
                "Scroll logs to bottom",
                FrameIconButtonVariant::Ghost,
                true,
                FrameIconButtonSize {
                    button: LOG_SCROLL_BUTTON_SIZE,
                    icon: LOG_SCROLL_ICON_SIZE,
                },
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.scroll_selected_log_to_bottom() {
                    cx.notify();
                }
            })),
        )
}

pub(super) fn logs_copy_button(
    file_id: &str,
    enabled: bool,
    copied: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let file_id = file_id.to_string();
    frame_icon_swap_button(
        "logs-copy",
        assets::ICON_COPY,
        assets::ICON_CHECK,
        copied,
        if copied { "Logs copied" } else { "Copy logs" },
        FrameIconButtonVariant::Ghost,
        enabled,
        FrameIconButtonSize {
            button: FRAME_ICON_BUTTON_SM_SIZE,
            icon: FRAME_ICON_SM_SIZE,
        },
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if enabled && root.copy_log_lines_to_clipboard(&file_id, cx) {
            cx.notify();
        }
    }))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LogLineTone {
    Default,
    Warning,
    Error,
}

#[must_use]
pub(super) fn log_line_tone(text: &str) -> LogLineTone {
    let trimmed = text.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    if lower.contains("[error]")
        || lower.contains(" error")
        || lower.starts_with("error")
        || lower.contains("failed")
        || lower.contains("invalid")
        || lower.contains("panic")
    {
        return LogLineTone::Error;
    }

    if lower.contains("[warning]")
        || lower.contains(" warning")
        || lower.starts_with("warning")
        || lower.contains("deprecated")
    {
        return LogLineTone::Warning;
    }

    LogLineTone::Default
}

#[must_use]
pub(super) const fn log_line_tone_color(tone: LogLineTone) -> theme::RgbaToken {
    match tone {
        LogLineTone::Default => theme::FOREGROUND,
        LogLineTone::Warning => theme::FRAME_AMBER,
        LogLineTone::Error => theme::FRAME_RED,
    }
}

#[must_use]
pub(super) const fn log_line_hover_tone_color(tone: LogLineTone) -> theme::RgbaToken {
    match tone {
        LogLineTone::Default => theme::FOREGROUND,
        LogLineTone::Warning => theme::FRAME_AMBER,
        LogLineTone::Error => theme::FRAME_RED,
    }
}

pub(super) fn logs_empty_state(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(theme::ui_text(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_line_tone_marks_frame_error_lines() {
        assert_eq!(log_line_tone("[ERROR] ffmpeg failed"), LogLineTone::Error);
    }

    #[test]
    fn log_line_tone_marks_ffmpeg_warnings() {
        assert_eq!(
            log_line_tone("  warning: deprecated pixel format"),
            LogLineTone::Warning
        );
    }

    #[test]
    fn log_line_tone_keeps_ffmpeg_preamble_as_default_text() {
        assert_eq!(log_line_tone("ffmpeg version 7.1"), LogLineTone::Default);
    }
}
