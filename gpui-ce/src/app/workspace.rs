use super::*;
use super::{
    file_list_panel::file_list_panel, preview_panel::preview_panel, settings_panel::settings_panel,
};

pub(super) fn workspace_view(
    file_queue: &FileQueue,
    settings: SettingsRenderState<'_>,
    preview_crop: PreviewCropRenderState,
    window: &Window,
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
                    preview_panel(file_queue, settings, preview_crop, cx)
                        .row_span(PREVIEW_ROW_SPAN),
                )
                .child(file_list_panel(file_queue, cx).row_span(FILE_LIST_ROW_SPAN)),
        )
        .child(settings_panel(settings, window, cx).col_span(RIGHT_COLUMN_SPAN))
}
