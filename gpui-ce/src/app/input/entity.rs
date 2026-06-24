use super::{text::*, *};

impl EntityInputHandler for FrameRoot {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let kind = self.active_text_input?;
        let text = self.text_input_value(kind);
        let range = text_range_from_utf16(&text, &range_utf16);
        actual_range.replace(text_range_to_utf16(&text, &range));
        Some(text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let kind = self.active_text_input?;
        let text = self.text_input_value(kind);
        let runtime = self.text_input_runtime(kind);
        Some(UTF16Selection {
            range: text_range_to_utf16(&text, &runtime.selected_range),
            reversed: runtime.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let kind = self.active_text_input?;
        let text = self.text_input_value(kind);
        self.text_input_runtime(kind)
            .marked_range
            .as_ref()
            .map(|range| text_range_to_utf16(&text, range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(kind) = self.active_text_input {
            self.text_input_runtime_mut(kind).marked_range = None;
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input else {
            return;
        };
        if self.replace_text_input_range(kind, range_utf16, text, None, false) {
            self.pause_text_input_cursor(cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input else {
            return;
        };
        if self.replace_text_input_range(
            kind,
            range_utf16,
            new_text,
            new_selected_range_utf16,
            true,
        ) {
            self.pause_text_input_cursor(cx);
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let kind = self.active_text_input?;
        let text = self.text_input_value(kind);
        let range = text_range_from_utf16(&text, &range_utf16);
        let line = self.text_input_runtime(kind).last_layout.as_ref()?;
        let text_top = bounds.top() + px((SETTINGS_CONTROL_HEIGHT - TEXT_INPUT_CARET_HEIGHT) / 2.0);
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(range.start), text_top),
            point(
                bounds.left() + line.x_for_index(range.end),
                text_top + px(TEXT_INPUT_CARET_HEIGHT),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let kind = self.active_text_input?;
        let text = self.text_input_value(kind);
        let runtime = self.text_input_runtime(kind);
        let bounds = runtime.last_bounds.as_ref()?;
        let line = runtime.last_layout.as_ref()?;
        let offset = clamp_text_offset(&text, line.closest_index_for_x(point.x - bounds.left()));
        Some(text_offset_to_utf16(&text, offset))
    }

    fn accepts_text_input(&self, _window: &mut Window, _cx: &mut Context<Self>) -> bool {
        self.active_text_input
            .is_some_and(|kind| !self.text_input_disabled(kind))
    }
}
