use super::*;
use super::{components::*, primitives::*};

pub(super) fn file_list_panel(
    queue: &FileQueue,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
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
        .child(file_list_body(queue, window, cx))
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
                        frame_checkbox_hit_area(
                            selection.is_checked,
                            selection.is_indeterminate,
                            selection.is_enabled,
                            FILE_ROW_HEIGHT,
                        )
                        .id("file-list-header-checkbox")
                        .when(selection.is_enabled, |this| this.cursor_pointer())
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            button_mouse_down(selection.is_enabled, window, cx);
                        })
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
                .child(header_label("Name", 5, false))
                .child(header_label("Size", 2, true))
                .child(header_label("Target", 2, true))
                .child(header_label("State", 2, true)),
        )
        .child(
            div()
                .ml_4()
                .w(px(FILE_LIST_ACTIONS_WIDTH))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(theme::FRAME_GRAY_600))
                .text_right()
                .child(theme::ui_text("Actions")),
        )
        .child(panel_bottom_separator())
}

pub(super) fn file_list_body(
    queue: &FileQueue,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
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
                .text_size(px(theme::TEXT_UI_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(theme::ui_text("Drop files or use Add Source")),
        );
    }

    let mut body = body;
    for file in queue.files() {
        body = body.child(file_list_row(
            file,
            queue.selected_file_id() == Some(file.id.as_str()),
            window,
            cx,
        ));
    }
    body
}

pub(super) fn file_list_row(
    file: &FileItem,
    is_selected: bool,
    window: &mut Window,
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
            window,
            cx,
        ))
        .child(panel_bottom_separator())
}

pub(super) fn header_label(label: &'static str, span: u16, align_right: bool) -> gpui::Div {
    let cell = div()
        .col_span(span)
        .truncate()
        .font_weight(theme::TEXT_WEIGHT_MEDIUM);
    let cell = if align_right { cell.text_right() } else { cell };
    cell.child(theme::ui_text(label))
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
    window: &mut Window,
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
                window,
                cx,
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
                window,
                cx,
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
                window,
                cx,
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
            window,
            cx,
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = match tone {
        RowActionTone::Normal => FrameIconButtonVariant::Ghost,
        RowActionTone::Destructive => FrameIconButtonVariant::DestructiveGhost,
    };

    frame_icon_button(
        id,
        icon,
        variant,
        enabled,
        FrameIconButtonSize {
            button: FILE_LIST_ACTION_BUTTON_SIZE,
            icon: FILE_LIST_ACTION_ICON_SIZE,
        },
        window,
        cx,
    )
}
pub(super) fn row_checkbox_control(
    file_id: String,
    is_checked: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    frame_checkbox_hit_area(is_checked, false, true, FILE_ROW_HEIGHT)
        .id(element_id("file-row-checkbox", &file_id))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if root.file_queue.toggle_batch_selection(&file_id).is_some() {
                cx.notify();
            }
        }))
}

pub(super) fn state_tone_color(tone: FileStateTone) -> Rgba {
    match tone {
        FileStateTone::Foreground => color(theme::FOREGROUND),
        FileStateTone::Muted => color(theme::FRAME_GRAY_600),
        FileStateTone::Amber => color(theme::FRAME_AMBER),
        FileStateTone::Red => color(theme::FRAME_RED),
    }
}
