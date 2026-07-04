use super::{
    ClickEvent, Context, FILE_LIST_ROW_SPAN, FileQueue, FrameRoot, LEFT_COLUMN_SPAN,
    LEFT_GRID_ROWS, PREVIEW_ROW_SPAN, ParentElement, PreviewPanelProps, RIGHT_COLUMN_SPAN,
    SettingsRenderState, StatefulInteractiveElement, Styled, WORKSPACE_COLUMNS, WORKSPACE_GAP,
    Window, assets, color, div, px, svg, theme,
};
use super::{
    file_list_panel::file_list_panel,
    preview_panel::preview_panel,
    primitives::{ButtonVariant, FrameSurface, action_button},
    settings_panel::settings_panel,
};

const EMPTY_SETTINGS_HINT_MAX_WIDTH: f32 = 200.0;
const EMPTY_SETTINGS_PANEL_PADDING: f32 = 32.0;
const WELCOME_LOGO_SIZE: f32 = 56.0;
const WELCOME_MAX_WIDTH: f32 = 420.0;

pub(super) fn workspace_view(
    file_queue: &FileQueue,
    settings: &SettingsRenderState<'_>,
    preview_props: PreviewPanelProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    if file_queue.files().is_empty() {
        return welcome_view(window, cx);
    }

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

pub(super) fn welcome_view(window: &mut Window, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .max_w(px(WELCOME_MAX_WIDTH))
                .flex()
                .flex_col()
                .items_center()
                .gap_5()
                .text_center()
                .child(
                    svg()
                        .path(assets::ICON_FRAME)
                        .w(px(WELCOME_LOGO_SIZE))
                        .h(px(WELCOME_LOGO_SIZE))
                        .text_color(color(theme::FRAME_GRAY_600)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(theme::TEXT_ROW_SIZE))
                                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                                .text_color(color(theme::FOREGROUND))
                                .child(theme::ui_text("Frame")),
                        )
                        .child(
                            div()
                                .text_size(px(theme::TEXT_LABEL_SIZE))
                                .text_color(color(theme::FRAME_GRAY_600))
                                .child(theme::ui_text("Add media to start a conversion queue.")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            action_button(
                                "welcome-open-file",
                                assets::ICON_FILE_IMPORT,
                                Some("Open File"),
                                "Open file",
                                ButtonVariant::Default,
                                true,
                                window,
                                cx,
                            )
                            .on_click(cx.listener(
                                |_root, _: &ClickEvent, window, cx| {
                                    cx.stop_propagation();
                                    FrameRoot::prompt_add_source(window, cx);
                                },
                            )),
                        )
                        .child(
                            action_button(
                                "welcome-open-folder",
                                assets::ICON_FOLDER_IMPORT,
                                Some("Open Folder"),
                                "Open folder",
                                ButtonVariant::Secondary,
                                true,
                                window,
                                cx,
                            )
                            .on_click(cx.listener(
                                |_root, _: &ClickEvent, window, cx| {
                                    cx.stop_propagation();
                                    FrameRoot::prompt_add_source_folder(window, cx);
                                },
                            )),
                        ),
                ),
        )
}

fn settings_panel_for_selection(
    file_queue: &FileQueue,
    settings: &SettingsRenderState<'_>,
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
                .child(theme::ui_text(
                    "Select an item from the queue to access configuration",
                )),
        )
}
