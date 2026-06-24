use super::primitives::*;
use super::*;

pub(super) fn logs_view(
    queue: &FileQueue,
    conversion_events: &ConversionEventState,
    scroll_handle: &UniformListScrollHandle,
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
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root
                .conversion_events
                .select_log_file(&root.file_queue, &file_id)
            {
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

    body.child(log_lines_list(selected_id, line_count, scroll_handle, cx))
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
                .iter()
                .map(log_line_row)
                .collect()
        }),
    )
    .track_scroll(scroll_handle)
    .size_full()
    .p(px(2.0))
    .text_color(color(theme::FOREGROUND))
    .line_height(px(LOG_LINE_HEIGHT))
}

pub(super) fn log_line_row(line: &LogLine) -> gpui::Div {
    div()
        .min_h(px(LOG_LINE_HEIGHT))
        .w_full()
        .flex()
        .items_start()
        .rounded(px(theme::RADIUS_XS))
        .px_1()
        .py(px(2.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .hover(|style| style.bg(color(theme::FRAME_GRAY_100)))
        .child(
            div()
                .flex_none()
                .w(px(LOG_LINE_NUMBER_WIDTH))
                .mr(px(12.0))
                .pt(px(0.5))
                .text_right()
                .text_color(color(theme::FRAME_GRAY_400))
                .child(line.index.to_string()),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(line.text.clone()),
        )
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
