use super::primitives::*;
use super::*;

pub(super) fn file_list_panel(queue: &FileQueue, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .drag_over::<ExternalPaths>(|style, _paths, _window, _cx| {
            style
                .border_1()
                .border_dashed()
                .border_color(color(theme::FRAME_GRAY_600))
                .shadow(drop_target_shadows())
        })
        .child(file_list_header(queue.batch_selection_state(), cx))
        .child(file_list_body(queue, cx))
}

pub(super) fn file_list_header(
    selection: BatchSelectionState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .relative()
        .px_4()
        .child(
            div()
                .flex_1()
                .grid()
                .grid_cols(12)
                .gap(px(WORKSPACE_GAP))
                .items_center()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(
                    div().col_span(1).flex().items_center().child(
                        checkbox_hit_area(
                            selection.is_checked,
                            selection.is_indeterminate,
                            selection.is_enabled,
                        )
                        .id("file-list-header-checkbox")
                        .when(selection.is_enabled, |this| this.cursor_pointer())
                        .on_click(cx.listener(
                            |root, _: &ClickEvent, _window, cx| {
                                if !root.file_queue.files().is_empty() {
                                    root.file_queue.toggle_all_batch_selection();
                                    cx.notify();
                                }
                            },
                        )),
                    ),
                )
                .child(header_label("NAME", 5, false))
                .child(header_label("SIZE", 2, true))
                .child(header_label("TARGET", 2, true))
                .child(header_label("STATE", 2, true)),
        )
        .child(
            div()
                .ml_4()
                .w(px(FILE_LIST_ACTIONS_WIDTH))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .text_right()
                .child("ACTIONS"),
        )
        .child(panel_bottom_separator())
}

pub(super) fn file_list_body(queue: &FileQueue, cx: &mut Context<FrameRoot>) -> impl IntoElement {
    let body = div()
        .id("file-list-body")
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll();
    if queue.files().is_empty() {
        return body.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(theme::TEXT_ROW_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child("DROP FILES OR USE ADD SOURCE"),
        );
    }

    let mut body = body;
    for file in queue.files() {
        body = body.child(file_list_row(
            file,
            queue.selected_file_id() == Some(file.id.as_str()),
            cx,
        ));
    }
    body
}

pub(super) fn file_list_row(
    file: &FileItem,
    is_selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let group_name = format!("file-list-row-{}", file.id);
    let select_id = file.id.clone();

    div()
        .h(px(FILE_ROW_HEIGHT))
        .w_full()
        .id(element_id("file-list-row", &select_id))
        .group(group_name.clone())
        .flex()
        .items_center()
        .relative()
        .px_4()
        .bg(if is_selected {
            color(theme::FRAME_GRAY_100)
        } else {
            color(theme::TRANSPARENT)
        })
        .hover(|style| style.bg(color(theme::FRAME_GRAY_100)).cursor_pointer())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root.file_queue.select_existing_file(&select_id) {
                cx.notify();
            }
        }))
        .child(
            div()
                .flex_1()
                .grid()
                .grid_cols(12)
                .gap(px(WORKSPACE_GAP))
                .items_center()
                .text_size(px(theme::TEXT_ROW_SIZE))
                .child(
                    div()
                        .col_span(1)
                        .flex()
                        .items_center()
                        .child(row_checkbox_control(
                            file.id.clone(),
                            file.is_selected_for_conversion,
                            cx,
                        )),
                )
                .child(row_label(
                    file.name.clone(),
                    5,
                    false,
                    color(theme::FOREGROUND),
                ))
                .child(row_label(
                    format_file_size(file.size_bytes),
                    2,
                    true,
                    color(theme::FRAME_GRAY_600),
                ))
                .child(row_label(
                    file.original_format.clone(),
                    2,
                    true,
                    color(theme::FRAME_GRAY_600),
                ))
                .child(row_label(
                    file.row_state_label(),
                    2,
                    true,
                    state_tone_color(file.row_state_tone()),
                )),
        )
        .child(row_actions_cell(
            file.id.clone(),
            file.row_actions(),
            group_name,
            cx,
        ))
        .child(panel_bottom_separator())
}

pub(super) fn header_label(label: &'static str, span: u16, align_right: bool) -> gpui::Div {
    let cell = div().col_span(span).truncate();
    let cell = if align_right { cell.text_right() } else { cell };
    cell.child(label)
}

pub(super) fn row_label(
    label: String,
    span: u16,
    align_right: bool,
    text_color: Rgba,
) -> gpui::Div {
    let cell = div()
        .col_span(span)
        .truncate()
        .whitespace_nowrap()
        .text_color(text_color);
    let cell = if align_right { cell.text_right() } else { cell };
    cell.child(label)
}

pub(super) fn row_actions_cell(
    file_id: String,
    actions: RowActionAvailability,
    group_name: String,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let mut cell = div()
        .id(element_id("file-row-actions", &file_id))
        .ml_4()
        .w(px(FILE_LIST_ACTIONS_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .opacity(0.0)
        .group_hover(group_name, |style| style.opacity(1.0))
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }));

    if actions.can_pause {
        let id = file_id.clone();
        cell = cell.child(
            row_action_button(
                element_id("file-row-action-pause", &id),
                assets::ICON_PAUSE,
                true,
                RowActionTone::Normal,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.pause_conversion_task(&id) {
                    cx.notify();
                }
            })),
        );
    }
    if actions.can_resume {
        let id = file_id.clone();
        cell = cell.child(
            row_action_button(
                element_id("file-row-action-resume", &id),
                assets::ICON_PLAY,
                true,
                RowActionTone::Normal,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.resume_conversion_task(&id) {
                    cx.notify();
                }
            })),
        );
    }

    if actions.can_delete {
        let id = file_id;
        cell.child(
            row_action_button(
                element_id("file-row-action-delete", &id),
                assets::ICON_TRASH,
                true,
                RowActionTone::Destructive,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.remove_file_from_queue(&id) {
                    cx.notify();
                }
            })),
        )
    } else {
        cell.child(row_action_button(
            element_id("file-row-action-delete-disabled", &file_id),
            assets::ICON_TRASH,
            false,
            RowActionTone::Destructive,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RowActionTone {
    Normal,
    Destructive,
}

pub(super) fn row_action_button(
    id: String,
    icon: &'static str,
    enabled: bool,
    tone: RowActionTone,
) -> gpui::Stateful<gpui::Div> {
    let (background, hover_background, active_background, foreground, hover_foreground, opacity) =
        match (tone, enabled) {
            (RowActionTone::Normal, true) => (
                theme::TRANSPARENT,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_600,
                theme::FOREGROUND,
                1.0,
            ),
            (RowActionTone::Normal, false) => (
                theme::TRANSPARENT,
                theme::TRANSPARENT,
                theme::TRANSPARENT,
                theme::FRAME_GRAY_600,
                theme::FRAME_GRAY_600,
                0.5,
            ),
            (RowActionTone::Destructive, true) => (
                theme::TRANSPARENT,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_RED,
                theme::FRAME_RED,
                1.0,
            ),
            (RowActionTone::Destructive, false) => (
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_GRAY_100,
                theme::FRAME_RED.with_alpha(0.5),
                theme::FRAME_RED.with_alpha(0.5),
                1.0,
            ),
        };

    div()
        .id(id)
        .w(px(FILE_LIST_ACTION_BUTTON_SIZE))
        .h(px(FILE_LIST_ACTION_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(background))
        .text_color(color(foreground))
        .opacity(opacity)
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(hover_background))
                    .text_color(color(hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(active_background)))
        })
        .child(icon_svg(
            icon,
            FILE_LIST_ACTION_ICON_SIZE,
            color(foreground),
        ))
}
pub(super) fn row_checkbox_control(
    file_id: String,
    is_checked: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    checkbox_hit_area(is_checked, false, true)
        .id(element_id("file-row-checkbox", &file_id))
        .cursor_pointer()
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if root.file_queue.toggle_batch_selection(&file_id).is_some() {
                cx.notify();
            }
        }))
}

pub(super) fn checkbox_hit_area(
    is_checked: bool,
    is_indeterminate: bool,
    is_enabled: bool,
) -> gpui::Div {
    div()
        .w(px(theme::MIN_HIT_AREA))
        .h(px(FILE_ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_start()
        .child(checkbox_indicator(is_checked, is_indeterminate, is_enabled))
}

pub(super) fn checkbox_indicator(
    is_checked: bool,
    is_indeterminate: bool,
    is_enabled: bool,
) -> gpui::Div {
    let is_active = is_checked || is_indeterminate;
    let mut indicator = div()
        .w(px(FILE_LIST_CHECKBOX_SIZE))
        .h(px(FILE_LIST_CHECKBOX_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_XS))
        .border_1()
        .border_color(if is_active || is_enabled {
            color(theme::FRAME_GRAY_600)
        } else {
            color(theme::FRAME_GRAY_200)
        })
        .bg(if is_active {
            color(theme::FRAME_GRAY_600)
        } else {
            color(theme::TRANSPARENT)
        });

    if is_indeterminate {
        indicator = indicator.child(
            div()
                .w(px(6.0))
                .h(px(2.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(color(theme::FOREGROUND)),
        );
    } else if is_checked {
        indicator = indicator.child(icon_svg(
            assets::ICON_CHECK,
            FILE_LIST_CHECK_ICON_SIZE,
            color(theme::FOREGROUND),
        ));
    }

    indicator
}

pub(super) fn state_tone_color(tone: FileStateTone) -> Rgba {
    match tone {
        FileStateTone::Foreground => color(theme::FOREGROUND),
        FileStateTone::Muted => color(theme::FRAME_GRAY_600),
        FileStateTone::Amber => color(theme::FRAME_AMBER),
        FileStateTone::Red => color(theme::FRAME_RED),
    }
}
