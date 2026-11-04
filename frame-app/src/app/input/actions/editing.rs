use crate::app::input::runtime::FrameTextInputKind;
use crate::app::input::text::{
    clamp_text_offset, clamp_text_range, next_text_boundary, next_timecode_cursor,
    previous_text_boundary, previous_timecode_cursor, replace_timecode_mask,
    sanitize_replacement_text, text_range_from_utf16, text_range_to_utf16,
    timecode_cursor_at_or_after,
};
use crate::app::root::FrameRoot;
use crate::app::{
    TextInputBackspace, TextInputCopy, TextInputCut, TextInputDelete, TextInputEnd, TextInputHome,
    TextInputLeft, TextInputPaste, TextInputRight, TextInputSelectAll, TextInputSelectLeft,
    TextInputSelectRight,
};
use gpui::{ClipboardItem, Context, Window};
use std::ops::Range;

impl FrameRoot {
    pub(in crate::app) fn clamped_text_input_selection(
        &mut self,
        kind: FrameTextInputKind,
        text: &str,
    ) -> Range<usize> {
        let runtime = self.text_input_runtime_mut(kind);
        runtime.selected_range = clamp_text_range(text, &runtime.selected_range);
        runtime.selected_range.clone()
    }

    pub(in crate::app) fn text_input_cursor_offset(
        &mut self,
        kind: FrameTextInputKind,
        text: &str,
    ) -> usize {
        self.clamped_text_input_selection(kind, text);
        let runtime = self.text_input_runtime(kind);
        if runtime.selection_reversed {
            runtime.selected_range.start
        } else {
            runtime.selected_range.end
        }
    }

    pub(in crate::app) fn move_text_input_to(
        &mut self,
        kind: FrameTextInputKind,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        let text = self.text_input_value(kind);
        let offset = if kind.is_preview_timecode() {
            timecode_cursor_at_or_after(&text, offset)
        } else {
            clamp_text_offset(&text, offset)
        };
        let runtime = self.text_input_runtime_mut(kind);
        runtime.selected_range = offset..offset;
        runtime.selection_reversed = false;
        runtime.marked_range = None;
        self.text_input_ui.active = Some(kind);
        self.pause_text_input_cursor(cx);
    }

    pub(in crate::app) fn select_text_input_to(
        &mut self,
        kind: FrameTextInputKind,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        let text = self.text_input_value(kind);
        let offset = if kind.is_preview_timecode() {
            timecode_cursor_at_or_after(&text, offset)
        } else {
            clamp_text_offset(&text, offset)
        };
        let runtime = self.text_input_runtime_mut(kind);
        if runtime.selection_reversed {
            runtime.selected_range.start = offset;
        } else {
            runtime.selected_range.end = offset;
        }
        if runtime.selected_range.end < runtime.selected_range.start {
            runtime.selection_reversed = !runtime.selection_reversed;
            runtime.selected_range = runtime.selected_range.end..runtime.selected_range.start;
        }
        runtime.selected_range = clamp_text_range(&text, &runtime.selected_range);
        runtime.marked_range = None;
        self.text_input_ui.active = Some(kind);
        self.pause_text_input_cursor(cx);
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Platform input events hand optional UTF-16 ranges to this edit boundary by value."
    )]
    pub(in crate::app) fn replace_text_input_range(
        &mut self,
        kind: FrameTextInputKind,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        mark_inserted_text: bool,
    ) -> bool {
        if self.text_input_disabled(kind) {
            return false;
        }

        if kind.is_preview_timecode() {
            return self.replace_timecode_input_range(kind, range_utf16.as_ref(), new_text);
        }

        let current = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &current);
        let marked_range = self.text_input_runtime(kind).marked_range.clone();
        let range = range_utf16
            .as_ref()
            .map(|range| text_range_from_utf16(&current, range))
            .or(marked_range)
            .unwrap_or(selected_range);
        let range = clamp_text_range(&current, &range);
        let replacement = sanitize_replacement_text(kind, new_text);

        if replacement.is_empty() && !new_text.is_empty() && range.is_empty() {
            return false;
        }

        let candidate = format!(
            "{}{}{}",
            &current[..range.start],
            replacement,
            &current[range.end..]
        );
        let Some(actual) = self.write_text_input_value(kind, &candidate) else {
            return false;
        };

        let selection_start = new_selected_range_utf16
            .as_ref()
            .map_or(replacement.len(), |range| {
                text_range_from_utf16(&replacement, range).start
            });
        let selection_end = new_selected_range_utf16
            .as_ref()
            .map_or(replacement.len(), |range| {
                text_range_from_utf16(&replacement, range).end
            });
        let next_range = clamp_text_range(
            &actual,
            &((range.start + selection_start)..(range.start + selection_end)),
        );
        let next_marked_range = if mark_inserted_text && !replacement.is_empty() {
            Some(clamp_text_range(
                &actual,
                &(range.start..(range.start + replacement.len())),
            ))
        } else {
            None
        };

        let runtime = self.text_input_runtime_mut(kind);
        runtime.selected_range = next_range;
        runtime.selection_reversed = false;
        runtime.marked_range = next_marked_range;
        self.text_input_ui.active = Some(kind);
        true
    }

    pub(super) fn replace_timecode_input_range(
        &mut self,
        kind: FrameTextInputKind,
        range_utf16: Option<&Range<usize>>,
        new_text: &str,
    ) -> bool {
        let current = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &current);
        let marked_range = self.text_input_runtime(kind).marked_range.clone();
        let range = range_utf16
            .map(|range| text_range_from_utf16(&current, range))
            .or(marked_range)
            .unwrap_or(selected_range);
        let range = clamp_text_range(&current, &range);
        let replacement = sanitize_replacement_text(kind, new_text);

        if replacement.is_empty() && !new_text.is_empty() {
            return false;
        }

        let Some(edit) = replace_timecode_mask(&current, &range, &replacement) else {
            return false;
        };
        let Some(actual) = self.write_text_input_value(kind, &edit.value) else {
            return false;
        };
        let cursor = timecode_cursor_at_or_after(&actual, edit.cursor);
        let runtime = self.text_input_runtime_mut(kind);
        runtime.selected_range = cursor..cursor;
        runtime.selection_reversed = false;
        runtime.marked_range = None;
        self.text_input_ui.active = Some(kind);
        true
    }

    pub(in crate::app) fn text_input_backspace(
        &mut self,
        _: &TextInputBackspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &text);
        let range = if kind.is_preview_timecode() && selected_range.is_empty() {
            let cursor = self.text_input_cursor_offset(kind, &text);
            if cursor == 0 {
                return;
            }
            let previous = previous_timecode_cursor(&text, cursor);
            previous..previous.saturating_add(1)
        } else if selected_range.is_empty() {
            let cursor = self.text_input_cursor_offset(kind, &text);
            previous_text_boundary(&text, cursor)..cursor
        } else {
            selected_range
        };
        let range_utf16 = text_range_to_utf16(&text, &range);
        if self.replace_text_input_range(kind, Some(range_utf16), "", None, false) {
            self.pause_text_input_cursor(cx);
        }
    }

    pub(in crate::app) fn text_input_delete(
        &mut self,
        _: &TextInputDelete,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &text);
        let range = if kind.is_preview_timecode() && selected_range.is_empty() {
            let cursor =
                timecode_cursor_at_or_after(&text, self.text_input_cursor_offset(kind, &text));
            if cursor == text.len() {
                return;
            }
            cursor..cursor.saturating_add(1)
        } else if selected_range.is_empty() {
            let cursor = self.text_input_cursor_offset(kind, &text);
            cursor..next_text_boundary(&text, cursor)
        } else {
            selected_range
        };
        let range_utf16 = text_range_to_utf16(&text, &range);
        if self.replace_text_input_range(kind, Some(range_utf16), "", None, false) {
            self.pause_text_input_cursor(cx);
        }
    }

    pub(in crate::app) fn text_input_left(
        &mut self,
        _: &TextInputLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &text);
        let next = if kind.is_preview_timecode() && selected_range.is_empty() {
            previous_timecode_cursor(&text, self.text_input_cursor_offset(kind, &text))
        } else if selected_range.is_empty() {
            previous_text_boundary(&text, self.text_input_cursor_offset(kind, &text))
        } else {
            selected_range.start
        };
        self.move_text_input_to(kind, next, cx);
    }

    pub(in crate::app) fn text_input_right(
        &mut self,
        _: &TextInputRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &text);
        let next = if kind.is_preview_timecode() && selected_range.is_empty() {
            next_timecode_cursor(&text, self.text_input_cursor_offset(kind, &text))
        } else if selected_range.is_empty() {
            next_text_boundary(&text, self.text_input_cursor_offset(kind, &text))
        } else {
            selected_range.end
        };
        self.move_text_input_to(kind, next, cx);
    }

    pub(in crate::app) fn text_input_select_left(
        &mut self,
        _: &TextInputSelectLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let cursor = self.text_input_cursor_offset(kind, &text);
        let next = if kind.is_preview_timecode() {
            previous_timecode_cursor(&text, cursor)
        } else {
            previous_text_boundary(&text, cursor)
        };
        self.select_text_input_to(kind, next, cx);
    }

    pub(in crate::app) fn text_input_select_right(
        &mut self,
        _: &TextInputSelectRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let cursor = self.text_input_cursor_offset(kind, &text);
        let next = if kind.is_preview_timecode() {
            next_timecode_cursor(&text, cursor)
        } else {
            next_text_boundary(&text, cursor)
        };
        self.select_text_input_to(kind, next, cx);
    }

    pub(in crate::app) fn text_input_home(
        &mut self,
        _: &TextInputHome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(kind) = self.active_text_input_kind(window) {
            self.move_text_input_to(kind, 0, cx);
        }
    }

    pub(in crate::app) fn text_input_end(
        &mut self,
        _: &TextInputEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(kind) = self.active_text_input_kind(window) {
            let text = self.text_input_value(kind);
            self.move_text_input_to(kind, text.len(), cx);
        }
    }

    pub(in crate::app) fn text_input_select_all(
        &mut self,
        _: &TextInputSelectAll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let runtime = self.text_input_runtime_mut(kind);
        runtime.selected_range = 0..text.len();
        runtime.selection_reversed = false;
        runtime.marked_range = None;
        self.pause_text_input_cursor(cx);
    }

    pub(in crate::app) fn text_input_copy(
        &mut self,
        _: &TextInputCopy,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &text);
        if !selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text[selected_range].to_string()));
        }
    }

    pub(in crate::app) fn text_input_cut(
        &mut self,
        _: &TextInputCut,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.text_input_copy(&TextInputCopy, window, cx);
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let text = self.text_input_value(kind);
        let selected_range = self.clamped_text_input_selection(kind, &text);
        if selected_range.is_empty() {
            return;
        }
        let range_utf16 = text_range_to_utf16(&text, &selected_range);
        if self.replace_text_input_range(kind, Some(range_utf16), "", None, false) {
            self.pause_text_input_cursor(cx);
        }
    }

    pub(in crate::app) fn text_input_paste(
        &mut self,
        _: &TextInputPaste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        let Some(text) = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .map(|text| text.replace('\n', " "))
        else {
            return;
        };
        if self.replace_text_input_range(kind, None, &text, None, false) {
            self.pause_text_input_cursor(cx);
        }
    }
}
