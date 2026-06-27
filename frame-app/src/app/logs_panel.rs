use super::components::{FrameIconButtonVariant, frame_icon_button};
use super::primitives::*;
use super::*;

pub(super) fn logs_view(
    queue: &FileQueue,
    conversion_events: &ConversionEventState,
    scroll_handle: &UniformListScrollHandle,
    follow_tail: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let active_files = conversion_events.active_log_files(queue);
    let selected_id = conversion_events.selected_log_file_id();

    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .child(logs_tab_strip(&active_files, selected_id, cx))
        .child(logs_body(
            conversion_events,
            selected_id,
            !active_files.is_empty(),
            scroll_handle,
            follow_tail,
            cx,
        ))
}

pub(super) fn logs_tab_strip(
    active_files: &[ActiveLogFile],
    selected_id: Option<&str>,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut tabs = div()
        .size_full()
        .flex()
        .items_center()
        .gap_6()
        .overflow_hidden()
        .px_4();

    for file in active_files {
        tabs = tabs.child(log_tab_button(
            file,
            selected_id.is_some_and(|id| id == file.id),
            cx,
        ));
    }

    if active_files.is_empty() {
        tabs = tabs.child(
            div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child("No active processes"),
        );
    }

    div()
        .h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .relative()
        .child(tabs)
        .child(panel_bottom_separator())
}

pub(super) fn log_tab_button(
    file: &ActiveLogFile,
    selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let file_id = file.id.clone();

    div()
        .id(element_id("logs-tab", &file.id))
        .flex_none()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(if selected {
            color(theme::FOREGROUND)
        } else {
            color(theme::FRAME_GRAY_600)
        })
        .hover(|style| style.text_color(color(theme::FOREGROUND)).cursor_pointer())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root.select_log_file_for_logs_view(&file_id) {
                cx.notify();
            }
            cx.stop_propagation();
        }))
        .child(file.name.clone())
}

pub(super) fn logs_body(
    conversion_events: &ConversionEventState,
    selected_id: Option<&str>,
    has_active_files: bool,
    scroll_handle: &UniformListScrollHandle,
    follow_tail: bool,
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
        body.child(log_scroll_to_bottom_button(cx))
    }
}

pub(super) fn log_lines_list(
    selected_id: &str,
    line_count: usize,
    scroll_handle: &UniformListScrollHandle,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let selected_id = selected_id.to_string();
    let list_id = element_id("logs-line-list", &selected_id);

    uniform_list(
        list_id,
        line_count,
        cx.processor(move |root, range, _window, _cx| {
            root.conversion_events
                .log_line_window_for(&selected_id, range)
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
    }))
    .size_full()
    .p(px(2.0))
    .text_color(color(theme::FOREGROUND))
    .line_height(px(LOG_LINE_HEIGHT))
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

pub(super) fn log_scroll_to_bottom_button(cx: &mut Context<FrameRoot>) -> impl IntoElement {
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
                FrameIconButtonVariant::Ghost,
                true,
                LOG_SCROLL_BUTTON_SIZE,
                LOG_SCROLL_ICON_SIZE,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.scroll_selected_log_to_bottom() {
                    cx.notify();
                }
            })),
        )
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
        .child(message)
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
