use crate::app::inspector::settings_panel;
use crate::app::inspector::state::SettingsRenderState;
use crate::app::layout::{
    FILE_LIST_ROW_SPAN, LEFT_COLUMN_SPAN, LEFT_GRID_ROWS, PREVIEW_ROW_SPAN, RIGHT_COLUMN_SPAN,
    TITLEBAR_HEIGHT, WORKSPACE_COLUMNS, WORKSPACE_GAP,
};
use crate::app::preview::{PreviewPanelProps, preview_panel};
use crate::app::primitives::{ButtonVariant, FrameSurface, action_button, color};
use crate::app::queue::view::file_list_panel;
use crate::app::root::FrameRoot;
use crate::file_queue::FileQueue;
use crate::{assets, theme};
use gpui::prelude::{ParentElement, Styled};
use gpui::{
    ClickEvent, Context, InteractiveElement, StatefulInteractiveElement, UniformListScrollHandle,
    Window, div,
};

const EMPTY_SETTINGS_HINT_MAX_WIDTH: f32 = 200.0;
const EMPTY_SETTINGS_PANEL_PADDING: f32 = 32.0;
const WELCOME_MAX_WIDTH: f32 = 420.0;
const WELCOME_HEADING_SIZE: f32 = 20.0;

pub(super) fn workspace_view(
    file_queue: &FileQueue,
    file_list_scroll_handle: &UniformListScrollHandle,
    settings: &SettingsRenderState<'_>,
    preview_props: PreviewPanelProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    if file_queue.files().is_empty() {
        return welcome_view(settings.palette, window, cx);
    }

    div()
        .grid()
        .grid_cols(WORKSPACE_COLUMNS)
        .gap(theme::ui_rem(WORKSPACE_GAP))
        .size_full()
        .child(
            div()
                .col_span(LEFT_COLUMN_SPAN)
                .grid()
                .grid_rows(LEFT_GRID_ROWS)
                .gap(theme::ui_rem(WORKSPACE_GAP))
                .size_full()
                .child(
                    preview_panel(file_queue, settings, preview_props, window, cx)
                        .row_span(PREVIEW_ROW_SPAN),
                )
                .child(
                    file_list_panel(
                        file_queue,
                        file_list_scroll_handle,
                        settings.palette,
                        window,
                        cx,
                    )
                    .row_span(FILE_LIST_ROW_SPAN),
                ),
        )
        .child(settings_panel_for_selection(
            file_queue, settings, window, cx,
        ))
}

pub(super) fn welcome_view(
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let open_file_focus = crate::app::accessibility::welcome_open_file_focus(window, cx);
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        // Balance the titlebar above the workspace to center in the whole window.
        .pb(theme::ui_rem(TITLEBAR_HEIGHT))
        .child(
            div()
                .max_w(theme::ui_rem(WELCOME_MAX_WIDTH))
                .flex()
                .flex_col()
                .items_center()
                .gap_5()
                .text_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(theme::ui_rem(WELCOME_HEADING_SIZE))
                                .line_height(gpui::relative(1.4))
                                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                                .text_color(color(palette.text_primary))
                                .child(theme::ui_text("Open media to start")),
                        )
                        .child(
                            div()
                                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                                .text_color(color(palette.text_muted))
                                .child(theme::ui_text("Convert video, audio, and images.")),
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
                                Some("Open file"),
                                "Open file",
                                ButtonVariant::Primary,
                                true,
                                palette,
                                window,
                                cx,
                            )
                            .h(theme::ui_rem(theme::metrics::ControlSize::Lg.height()))
                            .px(theme::ui_rem(theme::metrics::ControlSize::Lg.padding()))
                            .track_focus(&open_file_focus)
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
                                Some("Open folder"),
                                "Open folder",
                                ButtonVariant::Secondary,
                                true,
                                palette,
                                window,
                                cx,
                            )
                            .h(theme::ui_rem(theme::metrics::ControlSize::Lg.height()))
                            .px(theme::ui_rem(theme::metrics::ControlSize::Lg.padding()))
                            .on_click(cx.listener(
                                |_root, _: &ClickEvent, window, cx| {
                                    cx.stop_propagation();
                                    FrameRoot::prompt_add_source_folder(window, cx);
                                },
                            )),
                        ),
                )
                .child(
                    div()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .text_color(color(palette.text_muted))
                        .child(theme::ui_text("Or drop files here")),
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
        empty_settings_panel(settings.palette).col_span(RIGHT_COLUMN_SPAN)
    }
}

fn empty_settings_panel(palette: &'static theme::ThemePalette) -> gpui::Div {
    div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .p(theme::ui_rem(EMPTY_SETTINGS_PANEL_PADDING))
        .text_center()
        .card_surface(palette)
        .child(
            div()
                .max_w(theme::ui_rem(EMPTY_SETTINGS_HINT_MAX_WIDTH))
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.text_muted))
                .child(theme::ui_text(
                    "Select an item from the queue to access configuration",
                )),
        )
}
