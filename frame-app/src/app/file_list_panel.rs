use super::{
    BatchSelectionState, ClickEvent, Context, ExternalPaths, FILE_LIST_ACTION_BUTTON_SIZE,
    FILE_LIST_ACTION_ICON_SIZE, FILE_LIST_ACTIONS_WIDTH, FILE_ROW_HEIGHT, FileItem, FileQueue,
    FileStateTone, FluentBuilder, FrameRoot, InteractiveElement, IntoElement, MouseButton,
    PANEL_HEADER_HEIGHT, ParentElement, Rgba, RowActionAvailability, RowPrimaryAction,
    RowSecondaryAction, StatefulInteractiveElement, Styled, WORKSPACE_GAP, Window, assets, div,
    format_file_size, px, theme,
};
use super::{
    accessibility::apply_accessible_checkbox,
    components::{
        FrameIconButtonSize, FrameIconButtonVariant, frame_checkbox_indicator, frame_icon_button,
    },
    primitives::{
        FrameSurface, button_mouse_down, color, drop_target_shadows, element_id,
        panel_bottom_separator,
    },
};

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
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let selection_enabled = selection.is_enabled;
    let header_checkbox = div()
        .id("file-list-header-checkbox-hit-area")
        .w(px(theme::MIN_HIT_AREA))
        .h(px(FILE_ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_start()
        .when(selection_enabled, gpui::Styled::cursor_pointer)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(selection_enabled, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if selection_enabled
                && !root.file_queue.files().is_empty()
                && root.toggle_all_file_batch_selection().is_some()
            {
                cx.notify();
            }
        }))
        .child(apply_accessible_checkbox(
            frame_checkbox_indicator(
                selection.is_checked,
                selection.is_indeterminate,
                !selection_enabled,
            )
            .id("file-list-header-checkbox")
            .on_key_down(cx.listener(
                move |root, event: &gpui::KeyDownEvent, _window, cx| {
                    if !matches!(event.keystroke.key.as_str(), "space" | "enter") {
                        return;
                    }
                    cx.stop_propagation();
                    if selection_enabled
                        && !root.file_queue.files().is_empty()
                        && root.toggle_all_file_batch_selection().is_some()
                    {
                        cx.notify();
                    }
                },
            )),
            "Select all files for conversion",
            selection_enabled,
            selection.is_checked,
            selection.is_indeterminate,
        ));

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
                    div()
                        .col_span(1)
                        .flex()
                        .items_center()
                        .child(header_checkbox),
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
        .role(gpui::Role::List)
        .aria_label("File queue")
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
    let row_accessible_label = format!(
        "{}, {}, {}, {}",
        file.name,
        format_file_size(file.size_bytes),
        file.original_format,
        file.row_state_label()
    );

    div()
        .h(px(FILE_ROW_HEIGHT))
        .w_full()
        .id(element_id("file-list-row", &select_id))
        .role(gpui::Role::ListItem)
        .aria_label(row_accessible_label)
        .aria_selected(is_selected)
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
            if root.select_workspace_file(&select_id) {
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
                            file.id.as_str(),
                            file.name.as_str(),
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
        .focusable()
        .tab_stop(false)
        .contains_focus(|style| style.opacity(1.0))
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }));

    if let Some(primary) = row_primary_action_button(file_id.clone(), actions.primary, window, cx) {
        cell = cell.child(primary);
    }
    if let Some(secondary) = row_secondary_action_button(file_id, actions.secondary, window, cx) {
        cell = cell.child(secondary);
    }

    cell
}

fn row_primary_action_button(
    file_id: String,
    action: RowPrimaryAction,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Stateful<gpui::Div>> {
    let (id_prefix, icon, label) = match action {
        RowPrimaryAction::None => return None,
        RowPrimaryAction::Pause => (
            "file-row-action-pause",
            assets::ICON_PAUSE,
            "Pause conversion",
        ),
        RowPrimaryAction::Resume => (
            "file-row-action-resume",
            assets::ICON_PLAY,
            "Resume conversion",
        ),
        RowPrimaryAction::Reconvert => (
            "file-row-action-reconvert",
            assets::ICON_REFRESH,
            "Convert again",
        ),
    };
    let id = file_id;
    Some(
        row_action_button(
            element_id(id_prefix, &id),
            icon,
            label,
            true,
            RowActionTone::Normal,
            window,
            cx,
        )
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            let changed = match action {
                RowPrimaryAction::None => false,
                RowPrimaryAction::Pause => root.pause_conversion_task(&id),
                RowPrimaryAction::Resume => root.resume_conversion_task(&id),
                RowPrimaryAction::Reconvert => root.prepare_file_for_reconversion(&id),
            };
            if changed {
                cx.notify();
            }
        })),
    )
}

fn row_secondary_action_button(
    file_id: String,
    action: RowSecondaryAction,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Stateful<gpui::Div>> {
    let (id_prefix, icon, label) = match action {
        RowSecondaryAction::None => return None,
        RowSecondaryAction::Cancel => (
            "file-row-action-cancel",
            assets::ICON_SQUARE,
            "Cancel conversion",
        ),
        RowSecondaryAction::Delete => ("file-row-action-delete", assets::ICON_TRASH, "Remove file"),
    };
    let id = file_id;
    Some(
        row_action_button(
            element_id(id_prefix, &id),
            icon,
            label,
            true,
            RowActionTone::Destructive,
            window,
            cx,
        )
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            let changed = match action {
                RowSecondaryAction::None => false,
                RowSecondaryAction::Cancel => root.cancel_conversion_task(&id),
                RowSecondaryAction::Delete => root.remove_file_from_queue(&id),
            };
            if changed {
                cx.notify();
            }
        })),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RowActionTone {
    Normal,
    Destructive,
}

pub(super) fn row_action_button(
    id: String,
    icon: &'static str,
    label: &'static str,
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
        label,
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
    file_id: &str,
    file_name: &str,
    is_checked: bool,
    cx: &Context<FrameRoot>,
) -> impl IntoElement {
    let label = format!("Select {file_name} for conversion");
    let click_id = file_id.to_string();
    let key_id = file_id.to_string();

    div()
        .id(element_id("file-row-checkbox-hit-area", file_id))
        .w(px(theme::MIN_HIT_AREA))
        .h(px(FILE_ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_start()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            let mut changed = root.select_workspace_file(&click_id);
            changed |= root.toggle_file_batch_selection(&click_id).is_some();
            if changed {
                cx.notify();
            }
        }))
        .child(apply_accessible_checkbox(
            frame_checkbox_indicator(is_checked, false, false)
                .id(element_id("file-row-checkbox", file_id))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                        if !matches!(event.keystroke.key.as_str(), "space" | "enter") {
                            return;
                        }
                        cx.stop_propagation();
                        let mut changed = root.select_workspace_file(&key_id);
                        changed |= root.toggle_file_batch_selection(&key_id).is_some();
                        if changed {
                            cx.notify();
                        }
                    }),
                ),
            label,
            true,
            is_checked,
            false,
        ))
}

pub(super) const fn state_tone_color(tone: FileStateTone) -> Rgba {
    match tone {
        FileStateTone::Foreground => color(theme::FOREGROUND),
        FileStateTone::Muted => color(theme::FRAME_GRAY_600),
        FileStateTone::Blue => color(theme::FRAME_BLUE),
        FileStateTone::Amber => color(theme::FRAME_AMBER),
        FileStateTone::Red => color(theme::FRAME_RED),
    }
}
