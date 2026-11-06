use crate::app::accessibility::apply_accessible_checkbox;
use crate::app::components::{
    FrameIconButtonSize, FrameIconButtonVariant, frame_checkbox_indicator,
    frame_icon_button_with_motion_key, frame_vertical_uniform_scrollbar,
};
use crate::app::components::{frame_empty, frame_panel_header};
use crate::app::layout::{FILE_ROW_HEIGHT, WORKSPACE_GAP};
use crate::app::primitives::{
    FrameSurface, button_mouse_down, color, element_id, panel_bottom_separator,
};
use crate::app::queue::state::{
    FILE_LIST_ACTION_BUTTON_SIZE, FILE_LIST_ACTION_ICON_SIZE, FILE_LIST_ACTIONS_WIDTH,
};
use crate::app::root::FrameRoot;
use crate::file_queue::{
    BatchSelectionState, FileItem, FileQueue, FileStateTone, RowActionAvailability,
    RowPrimaryAction, RowSecondaryAction, format_file_size,
};
use crate::numeric::usize_to_f32;
use crate::{assets, theme};
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    ClickEvent, Context, ExternalPaths, InteractiveElement, IntoElement, MouseButton, Rgba,
    StatefulInteractiveElement, UniformListScrollHandle, Window, div, uniform_list,
};

pub(in crate::app) fn file_list_panel(
    queue: &FileQueue,
    scroll_handle: &UniformListScrollHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface(palette)
        .on_drag_over::<ExternalPaths>(|style, _paths, _window, _cx| {
            style
                .bg(color(palette.surface_elevated))
                .shadow(theme::shadows::FLOATING.shadows())
        })
        .child(file_list_header(
            queue.batch_selection_state(),
            palette,
            window,
            cx,
        ))
        .child(file_list_body(queue, scroll_handle, palette, window, cx))
}

pub(in crate::app) fn file_list_header(
    selection: BatchSelectionState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let selection_enabled = selection.is_enabled;
    let header_checkbox = div()
        .id("file-list-header-checkbox-hit-area")
        .w(theme::ui_rem(theme::MIN_HIT_AREA))
        .h(theme::ui_rem(FILE_ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_start()
        .when(selection_enabled, gpui::Styled::cursor_default)
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
                "file-list-header-checkbox",
                selection.is_checked,
                selection.is_indeterminate,
                !selection_enabled,
                palette,
                window,
                cx,
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
            palette,
        ));

    frame_panel_header()
        .pr(theme::ui_rem(crate::app::components::FRAME_SCROLLBAR_WIDTH))
        .child(
            div()
                .flex_1()
                .grid()
                .grid_cols(12)
                .gap(theme::ui_rem(WORKSPACE_GAP))
                .items_center()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.text_muted))
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
                .w(theme::ui_rem(FILE_LIST_ACTIONS_WIDTH))
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(palette.text_muted))
                .text_right()
                .child(theme::ui_text("Actions")),
        )
        .child(panel_bottom_separator(palette))
}

fn queue_navigation_state(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Entity<(gpui::FocusHandle, bool)> {
    window.use_keyed_state("file-list-navigation", cx, |_, cx| {
        (cx.focus_handle().tab_stop(true), false)
    })
}

#[derive(Clone)]
pub(in crate::app) struct QueueReturnFocus {
    list: gpui::FocusHandle,
    welcome: gpui::FocusHandle,
}

#[expect(
    clippy::too_many_lines,
    reason = "The virtual list keeps keyboard selection and visible-row rendering together."
)]
pub(in crate::app) fn file_list_body(
    queue: &FileQueue,
    scroll_handle: &UniformListScrollHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let navigation = queue_navigation_state(window, cx);
    let (focus, entered) = navigation.update(cx, |(focus, was_focused), _| {
        let focused = focus.is_focused(window);
        let entered = focused && !*was_focused;
        *was_focused = focused;
        (focus.clone(), entered)
    });
    if entered
        && let Some(index) = queue
            .files()
            .iter()
            .position(|file| Some(file.id.as_str()) == queue.selected_file_id())
    {
        scroll_handle.scroll_to_item(index, gpui::ScrollStrategy::Nearest);
    }
    let keyboard_focus = focus.is_focused(window) && window.last_input_was_keyboard();
    let return_focus = QueueReturnFocus {
        list: focus.clone(),
        welcome: crate::app::accessibility::welcome_open_file_focus(window, cx),
    };
    let key_focus = focus.clone();
    let key_scroll = scroll_handle.clone();
    let body = div()
        .id("file-list-body")
        .debug_selector(|| "file-list-body".to_string())
        .role(gpui::Role::List)
        .aria_label("File queue")
        .aria_description(
            "Use Up and Down to select a file; Space toggles its conversion selection.",
        )
        .track_focus(&focus)
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let files = root.file_queue.files();
                if files.is_empty() {
                    return;
                }
                let current = files
                    .iter()
                    .position(|file| Some(file.id.as_str()) == root.file_queue.selected_file_id())
                    .unwrap_or(0);
                let next = match event.keystroke.key.as_str() {
                    "up" => current.saturating_sub(1),
                    "down" => (current + 1).min(files.len() - 1),
                    "home" => 0,
                    "end" => files.len() - 1,
                    "space" if key_focus.is_focused(window) => {
                        let id = files[current].id.clone();
                        root.toggle_file_batch_selection(&id);
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    _ => return,
                };
                let id = files[next].id.clone();
                root.select_workspace_file(&id);
                key_scroll.scroll_to_item(next, gpui::ScrollStrategy::Nearest);
                key_focus.focus(window, cx);
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .relative()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden();
    if queue.files().is_empty() {
        return body.child(frame_empty(
            theme::ui_text("Drop files or use Add Source"),
            palette,
        ));
    }

    let file_count = queue.files().len();
    let list = uniform_list(
        "file-list-rows",
        file_count,
        cx.processor(move |root, range: std::ops::Range<usize>, window, cx| {
            let selected_id = root.file_queue.selected_file_id();
            range
                .filter_map(|index| root.file_queue.files().get(index))
                .map(|file| {
                    file_list_row(
                        file,
                        selected_id == Some(file.id.as_str()),
                        &return_focus,
                        palette,
                        window,
                        cx,
                    )
                    .when(
                        keyboard_focus && selected_id == Some(file.id.as_str()),
                        |row| {
                            row.inset_ring(gpui::px(theme::metrics::FOCUS_WIDTH))
                                .inset_ring_color(color(
                                    palette.focus_ring.with_opacity(theme::metrics::FOCUS_ALPHA),
                                ))
                                .aria_active_descendant()
                        },
                    )
                })
                .collect()
        }),
    )
    .track_scroll(scroll_handle)
    .size_full();

    body.child(list).child(frame_vertical_uniform_scrollbar(
        "file-list-scrollbar",
        scroll_handle,
        usize_to_f32(file_count) * FILE_ROW_HEIGHT,
        palette,
    ))
}

pub(in crate::app) fn file_list_row(
    file: &FileItem,
    is_selected: bool,
    return_focus: &QueueReturnFocus,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
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
        .h(theme::ui_rem(FILE_ROW_HEIGHT))
        .w_full()
        .id(element_id("file-list-row", &select_id))
        .debug_selector({
            let id = select_id.clone();
            move || format!("file-list-row-{id}")
        })
        .role(gpui::Role::ListItem)
        .aria_label(row_accessible_label)
        .aria_selected(is_selected)
        .group(group_name.clone())
        .flex()
        .items_center()
        .relative()
        .px_4()
        .pr(theme::ui_rem(crate::app::components::FRAME_SCROLLBAR_WIDTH))
        .bg(if is_selected {
            color(palette.fill_subtle)
        } else {
            color(palette.transparent)
        })
        .hover_style(|style| style.bg(color(palette.fill_subtle)).cursor_default())
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
                .gap(theme::ui_rem(WORKSPACE_GAP))
                .items_center()
                .text_size(theme::ui_rem(theme::TEXT_ROW_BASE_SIZE))
                .child(
                    div()
                        .col_span(1)
                        .flex()
                        .items_center()
                        .child(row_checkbox_control(
                            file.id.as_str(),
                            file.name.as_str(),
                            file.is_selected_for_conversion,
                            palette,
                            window,
                            cx,
                        )),
                )
                .child(row_label(
                    file.name.clone(),
                    5,
                    false,
                    color(palette.text_primary),
                ))
                .child(row_label(
                    format_file_size(file.size_bytes),
                    2,
                    true,
                    color(palette.text_muted),
                ))
                .child(row_label(
                    file.original_format.clone(),
                    2,
                    true,
                    color(palette.text_muted),
                ))
                .child(row_label(
                    file.row_state_label(),
                    2,
                    true,
                    state_tone_color(file.row_state_tone(), palette),
                )),
        )
        .child(row_actions_cell(
            file.id.clone(),
            file.row_actions(),
            group_name,
            return_focus,
            palette,
            window,
            cx,
        ))
        .child(panel_bottom_separator(palette))
}

pub(in crate::app) fn header_label(label: &'static str, span: u16, align_right: bool) -> gpui::Div {
    let cell = div()
        .col_span(span)
        .truncate()
        .font_weight(theme::TEXT_WEIGHT_MEDIUM);
    let cell = if align_right { cell.text_right() } else { cell };
    cell.child(theme::ui_text(label))
}

pub(in crate::app) fn row_label(
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

pub(in crate::app) fn row_actions_cell(
    file_id: String,
    actions: RowActionAvailability,
    group_name: String,
    return_focus: &QueueReturnFocus,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let mut cell = div()
        .id(element_id("file-row-actions", &file_id))
        .ml_4()
        .w(theme::ui_rem(FILE_LIST_ACTIONS_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .opacity(0.0)
        .group_hover_style(group_name, |style| style.opacity(1.0))
        .focusable()
        .tab_stop(false)
        .contains_focus(|style| style.opacity(1.0))
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }));

    if let Some(primary) =
        row_primary_action_button(file_id.clone(), actions.primary, palette, window, cx)
    {
        cell = cell.child(primary);
    }
    if let Some(secondary) = row_secondary_action_button(
        file_id,
        actions.secondary,
        return_focus,
        palette,
        window,
        cx,
    ) {
        cell = cell.child(secondary);
    }

    cell
}

fn row_primary_action_button(
    file_id: String,
    action: RowPrimaryAction,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Stateful<gpui::Div>> {
    // Pause, Resume and Convert again occupy one keyboard position even when
    // a conversion event changes their label or removes the primary action.
    let focus = row_action_focus(&file_id, "primary", window, cx);
    let (id_prefix, icon, label) = match action {
        RowPrimaryAction::None => {
            if focus.is_focused(window) {
                row_action_focus(&file_id, "secondary", window, cx).focus(window, cx);
            }
            return None;
        }
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
            element_id("file-row-action-primary", &id),
            icon,
            label,
            true,
            RowActionTone::Normal,
            Some(&focus),
            palette,
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
    return_focus: &QueueReturnFocus,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Stateful<gpui::Div>> {
    let focus = row_action_focus(&file_id, "secondary", window, cx);
    let queue_focus = return_focus.list.clone();
    let welcome_focus = return_focus.welcome.clone();
    let (id_prefix, icon, label) = match action {
        RowSecondaryAction::None => {
            if focus.is_focused(window) {
                queue_focus.focus(window, cx);
            }
            return None;
        }
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
            element_id("file-row-action-secondary", &id),
            icon,
            label,
            true,
            RowActionTone::Destructive,
            Some(&focus),
            palette,
            window,
            cx,
        )
        .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            let changed = match action {
                RowSecondaryAction::None => false,
                RowSecondaryAction::Cancel => root.cancel_conversion_task(&id),
                RowSecondaryAction::Delete => root.remove_file_from_queue(&id),
            };
            if changed {
                if action == RowSecondaryAction::Delete && focus.is_focused(window) {
                    if root.file_queue.files().is_empty() {
                        welcome_focus.focus(window, cx);
                    } else {
                        queue_focus.focus(window, cx);
                    }
                }
                cx.notify();
            }
        })),
    )
}

fn row_action_focus(
    file_id: &str,
    slot: &str,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::FocusHandle {
    window
        .use_keyed_state(format!("file-row-{slot}-focus-{file_id}"), cx, |_, cx| {
            cx.focus_handle().tab_stop(true)
        })
        .read(cx)
        .clone()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum RowActionTone {
    Normal,
    Destructive,
}

#[expect(
    clippy::too_many_arguments,
    reason = "Row actions keep their semantic tone, state, palette, and render context explicit."
)]
pub(in crate::app) fn row_action_button(
    id: String,
    motion_key: String,
    icon: &'static str,
    label: &'static str,
    enabled: bool,
    tone: RowActionTone,
    focus: Option<&gpui::FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = match tone {
        RowActionTone::Normal => FrameIconButtonVariant::Ghost,
        RowActionTone::Destructive => FrameIconButtonVariant::DestructiveGhost,
    };

    frame_icon_button_with_motion_key(
        id,
        motion_key,
        icon,
        label,
        variant,
        enabled,
        FrameIconButtonSize {
            button: FILE_LIST_ACTION_BUTTON_SIZE,
            icon: FILE_LIST_ACTION_ICON_SIZE,
        },
        focus,
        palette,
        window,
        cx,
    )
}
pub(in crate::app) fn row_checkbox_control(
    file_id: &str,
    file_name: &str,
    is_checked: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let label = format!("Select {file_name} for conversion");
    let click_id = file_id.to_string();
    let key_id = file_id.to_string();

    div()
        .id(element_id("file-row-checkbox-hit-area", file_id))
        .w(theme::ui_rem(theme::MIN_HIT_AREA))
        .h(theme::ui_rem(FILE_ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_start()
        .cursor_default()
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
            frame_checkbox_indicator(
                format!("file-row-checkbox-{file_id}"),
                is_checked,
                false,
                false,
                palette,
                window,
                cx,
            )
            .id(element_id("file-row-checkbox", file_id))
            .on_key_down(cx.listener(
                move |root, event: &gpui::KeyDownEvent, _window, cx| {
                    if !matches!(event.keystroke.key.as_str(), "space" | "enter") {
                        return;
                    }
                    cx.stop_propagation();
                    let mut changed = root.select_workspace_file(&key_id);
                    changed |= root.toggle_file_batch_selection(&key_id).is_some();
                    if changed {
                        cx.notify();
                    }
                },
            )),
            label,
            true,
            is_checked,
            false,
            palette,
        ))
}

pub(in crate::app) const fn state_tone_color(
    tone: FileStateTone,
    palette: &theme::ThemePalette,
) -> Rgba {
    match tone {
        FileStateTone::Foreground => color(palette.text_primary),
        FileStateTone::Muted => color(palette.text_muted),
        FileStateTone::Blue => color(palette.accent),
        FileStateTone::Amber => color(palette.warning),
        FileStateTone::Red => color(palette.danger),
    }
}
