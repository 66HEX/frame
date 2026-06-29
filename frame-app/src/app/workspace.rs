use super::*;
use super::{
    file_list_panel::file_list_panel, preview_panel::preview_panel, primitives::FrameSurface,
    settings_panel::settings_panel,
};

const EMPTY_SETTINGS_HINT_MAX_WIDTH: f32 = 200.0;
const EMPTY_SETTINGS_PANEL_PADDING: f32 = 32.0;

pub(super) fn workspace_view(
    file_queue: &FileQueue,
    settings: SettingsRenderState<'_>,
    preview_props: PreviewPanelProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(WORKSPACE_COLUMNS)
        .gap(px(WORKSPACE_GAP))
        .size_full()
        .child(
            div()
                .col_span(LEFT_COLUMN_SPAN)
                .grid()
                .grid_rows(LEFT_GRID_ROWS)
                .gap(px(WORKSPACE_GAP))
                .size_full()
                .child(
                    preview_panel(file_queue, settings, preview_props, window, cx)
                        .row_span(PREVIEW_ROW_SPAN),
                )
                .child(file_list_panel(file_queue, window, cx).row_span(FILE_LIST_ROW_SPAN)),
        )
        .child(settings_panel_for_selection(
            file_queue, settings, window, cx,
        ))
}

fn settings_panel_for_selection(
    file_queue: &FileQueue,
    settings: SettingsRenderState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    if file_queue.selected_file().is_some() {
        settings_panel(settings, window, cx).col_span(RIGHT_COLUMN_SPAN)
    } else {
        empty_settings_panel().col_span(RIGHT_COLUMN_SPAN)
    }
}

fn empty_settings_panel() -> gpui::Div {
    div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .p(px(EMPTY_SETTINGS_PANEL_PADDING))
        .text_center()
        .card_surface()
        .child(
            div()
                .max_w(px(EMPTY_SETTINGS_HINT_MAX_WIDTH))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child("Select an item from the queue to access configuration"),
        )
}
