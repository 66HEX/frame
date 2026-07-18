use super::{
    text::{
        clamp_text_offset, clamp_text_range, next_text_boundary, next_timecode_cursor,
        previous_text_boundary, previous_timecode_cursor, replace_timecode_mask,
        sanitize_hex_draft, sanitize_number_input, sanitize_replacement_text,
        text_range_from_utf16, text_range_to_utf16, timecode_cursor_at_or_after,
    },
    *,
};

impl FrameRoot {
    pub(in crate::app) const fn text_input_runtime(
        &self,
        kind: FrameTextInputKind,
    ) -> &FrameTextInputRuntime {
        self.text_input_ui.runtimes.runtime(kind)
    }

    pub(in crate::app) const fn text_input_runtime_mut(
        &mut self,
        kind: FrameTextInputKind,
    ) -> &mut FrameTextInputRuntime {
        self.text_input_ui.runtimes.runtime_mut(kind)
    }

    pub(in crate::app) const fn text_input_focus_handle(
        &self,
        kind: FrameTextInputKind,
    ) -> Option<&FocusHandle> {
        self.text_input_ui.focuses.focus(kind)
    }

    pub(in crate::app) fn ensure_text_input_focus(
        &mut self,
        kind: FrameTextInputKind,
        cx: &Context<Self>,
    ) -> FocusHandle {
        self.text_input_ui
            .focuses
            .focus_mut(kind)
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    pub(in crate::app) fn focused_text_input_kind(
        &self,
        window: &Window,
    ) -> Option<FrameTextInputKind> {
        FrameTextInputKind::ALL.into_iter().find(|kind| {
            self.text_input_focus_handle(*kind)
                .is_some_and(|focus| focus.is_focused(window))
        })
    }

    pub(in crate::app) fn active_text_input_kind(
        &self,
        window: &Window,
    ) -> Option<FrameTextInputKind> {
        self.focused_text_input_kind(window)
            .or(self.text_input_ui.active)
    }

    pub(in crate::app) fn activate_text_input(
        &mut self,
        kind: FrameTextInputKind,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_ui.active == Some(kind) {
            return;
        }

        if let Some(active) = self.text_input_ui.active {
            self.commit_preview_timecode_input(active, Some(cx));
        }
        self.text_input_ui.active = Some(kind);
        self.start_text_input_cursor(cx);
    }

    pub(in crate::app) fn reconcile_text_input_focus(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(active) = self.text_input_ui.active else {
            return;
        };
        if self.focused_text_input_kind(window) == Some(active) {
            return;
        }

        let changed = self.commit_preview_timecode_input(active, Some(cx));
        self.stop_text_input_cursor();
        if changed {
            cx.notify();
        }
    }

    pub(in crate::app) fn text_input_disabled(&self, kind: FrameTextInputKind) -> bool {
        kind != FrameTextInputKind::MaxConcurrency && self.file_queue.selected_file_locked()
    }

    pub(in crate::app) fn text_input_value(&self, kind: FrameTextInputKind) -> String {
        match kind {
            FrameTextInputKind::MaxConcurrency => self.settings_ui.max_concurrency_draft.clone(),
            FrameTextInputKind::OutputName => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.output_name.clone()),
            FrameTextInputKind::AudioBitrate => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.config.audio_bitrate.clone()),
            FrameTextInputKind::VideoCustomWidth => self
                .file_queue
                .selected_file()
                .and_then(|file| file.config.custom_width.clone())
                .unwrap_or_default(),
            FrameTextInputKind::VideoCustomHeight => self
                .file_queue
                .selected_file()
                .and_then(|file| file.config.custom_height.clone())
                .unwrap_or_default(),
            FrameTextInputKind::VideoBitrate => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.config.video_bitrate.clone()),
            FrameTextInputKind::GifLoop => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.config.gif_loop.to_string()),
            FrameTextInputKind::PreviewStartTime | FrameTextInputKind::PreviewEndTime => self
                .text_input_runtime(kind)
                .timecode_draft
                .as_ref()
                .filter(|draft| self.file_queue.selected_file_id() == Some(draft.file_id.as_str()))
                .map_or_else(
                    || match kind {
                        FrameTextInputKind::PreviewStartTime => {
                            format_time(self.preview_ui.playback.start_value())
                        }
                        FrameTextInputKind::PreviewEndTime => {
                            format_time(self.preview_ui.playback.end_value())
                        }
                        _ => unreachable!("matched preview timecode text input variants"),
                    },
                    |draft| draft.value.clone(),
                ),
            FrameTextInputKind::MetadataTitle
            | FrameTextInputKind::MetadataArtist
            | FrameTextInputKind::MetadataAlbum
            | FrameTextInputKind::MetadataGenre
            | FrameTextInputKind::MetadataDate
            | FrameTextInputKind::MetadataComment => self
                .file_queue
                .selected_file()
                .and_then(|file| {
                    metadata_field_for_text_input(kind).and_then(|field| {
                        metadata_field_value(&file.config, field).map(str::to_string)
                    })
                })
                .unwrap_or_default(),
            FrameTextInputKind::PresetName => self.settings_ui.preset_name_draft.clone(),
            FrameTextInputKind::SubtitleFontColorHex => self.subtitle_ui.font_color_draft.clone(),
            FrameTextInputKind::SubtitleOutlineColorHex => {
                self.subtitle_ui.outline_color_draft.clone()
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Text input routing centralizes per-field normalization so field behavior stays auditable in one place."
    )]
    pub(in crate::app) fn write_text_input_value(
        &mut self,
        kind: FrameTextInputKind,
        candidate: &str,
    ) -> Option<String> {
        match kind {
            FrameTextInputKind::MaxConcurrency => {
                let next = sanitize_number_input(candidate);
                if self.settings_ui.max_concurrency_draft != next {
                    self.settings_ui.max_concurrency_draft.clone_from(&next);
                    self.settings_ui.max_concurrency_error = None;
                }
                Some(next)
            }
            FrameTextInputKind::OutputName => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_output_name(candidate);
                self.file_queue.set_selected_output_name_from_input(&next);
                Some(next)
            }
            FrameTextInputKind::AudioBitrate => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_audio_bitrate(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::VideoCustomWidth => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_custom_width(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::VideoCustomHeight => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_custom_height(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::VideoBitrate => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_video_bitrate(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::GifLoop => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_gif_loop(&mut file.config, &next);
                })?;
                Some(file_gif_loop_value(&self.file_queue))
            }
            FrameTextInputKind::PreviewStartTime | FrameTextInputKind::PreviewEndTime => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let file_id = self.file_queue.selected_file_id()?.to_string();
                let next = candidate.to_string();
                self.text_input_runtime_mut(kind).timecode_draft = Some(FrameTimecodeInputDraft {
                    file_id,
                    value: next.clone(),
                });
                Some(next)
            }
            FrameTextInputKind::MetadataTitle
            | FrameTextInputKind::MetadataArtist
            | FrameTextInputKind::MetadataAlbum
            | FrameTextInputKind::MetadataGenre
            | FrameTextInputKind::MetadataDate
            | FrameTextInputKind::MetadataComment => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let field = metadata_field_for_text_input(kind)?;
                self.file_queue.selected_file_mut().map(|file| {
                    apply_metadata_field(&mut file.config, field, candidate);
                })?;
                Some(candidate.to_string())
            }
            FrameTextInputKind::PresetName => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next: String = candidate.chars().filter(|ch| !ch.is_control()).collect();
                self.settings_ui.preset_name_draft.clone_from(&next);
                self.settings_ui.preset_notice = None;
                Some(next)
            }
            FrameTextInputKind::SubtitleFontColorHex
            | FrameTextInputKind::SubtitleOutlineColorHex => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_hex_draft(candidate);
                let target = match kind {
                    FrameTextInputKind::SubtitleFontColorHex => SettingsSubtitleColorTarget::Font,
                    FrameTextInputKind::SubtitleOutlineColorHex => {
                        SettingsSubtitleColorTarget::Outline
                    }
                    _ => unreachable!("matched subtitle color text input variants"),
                };
                self.set_subtitle_color_draft(target, next.clone());
                if let Some(normalized) = normalized_hex_color(&next) {
                    self.commit_subtitle_color(target, &normalized);
                }
                Some(next)
            }
        }
    }

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

    fn replace_timecode_input_range(
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

    pub(in crate::app) fn text_input_index_for_mouse_position(
        &mut self,
        kind: FrameTextInputKind,
        position: Point<Pixels>,
    ) -> usize {
        let text = self.text_input_value(kind);
        if text.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line_width)) = ({
            let runtime = self.text_input_runtime(kind);
            (
                runtime.last_bounds,
                runtime.last_layout.as_ref().map(ShapedLine::width),
            )
        }) else {
            return text.len();
        };
        let viewport_width = bounds.size.width;
        let mut local_x = position.x - bounds.left();

        {
            let runtime = self.text_input_runtime_mut(kind);
            let mut scroll_x =
                clamp_text_input_scroll_x(runtime.scroll_x, line_width, viewport_width);
            if position.x < bounds.left() {
                scroll_x = clamp_text_input_scroll_x(
                    scroll_x - text_input_drag_scroll_amount(bounds.left() - position.x),
                    line_width,
                    viewport_width,
                );
                local_x = Pixels::ZERO;
            } else if position.x > bounds.right() {
                scroll_x = clamp_text_input_scroll_x(
                    scroll_x + text_input_drag_scroll_amount(position.x - bounds.right()),
                    line_width,
                    viewport_width,
                );
                local_x = viewport_width;
            }
            runtime.scroll_x = scroll_x;
        }

        let runtime = self.text_input_runtime(kind);
        let Some(line) = runtime.last_layout.as_ref() else {
            return text.len();
        };
        clamp_text_offset(&text, line.closest_index_for_x(local_x + runtime.scroll_x))
    }

    pub(in crate::app) fn text_input_mouse_down(
        &mut self,
        kind: FrameTextInputKind,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_disabled(kind) {
            return;
        }
        if let Some(focus) = self.text_input_focus_handle(kind) {
            focus.focus(window, cx);
        }
        self.activate_text_input(kind, cx);
        self.text_input_runtime_mut(kind).is_selecting = true;
        let offset = self.text_input_index_for_mouse_position(kind, event.position);
        if event.modifiers.shift {
            self.select_text_input_to(kind, offset, cx);
        } else {
            self.move_text_input_to(kind, offset, cx);
        }
    }

    pub(in crate::app) fn text_input_mouse_move(
        &mut self,
        kind: FrameTextInputKind,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_runtime(kind).is_selecting {
            let offset = self.text_input_index_for_mouse_position(kind, event.position);
            self.select_text_input_to(kind, offset, cx);
        }
    }

    pub(in crate::app) const fn text_input_mouse_up(
        &mut self,
        kind: FrameTextInputKind,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.text_input_runtime_mut(kind).is_selecting = false;
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

    pub(in crate::app) fn text_input_commit(
        &mut self,
        _: &TextInputCommit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        if self.commit_preview_timecode_input(kind, Some(cx)) {
            self.pause_text_input_cursor(cx);
        }
    }

    pub(in crate::app) fn text_input_cancel(
        &mut self,
        _: &TextInputCancel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.active_text_input_kind(window) else {
            return;
        };
        if self.cancel_preview_timecode_input(kind) {
            self.pause_text_input_cursor(cx);
        }
    }

    pub(in crate::app) fn cancel_preview_timecode_input(
        &mut self,
        kind: FrameTextInputKind,
    ) -> bool {
        kind.is_preview_timecode()
            && self
                .text_input_runtime_mut(kind)
                .timecode_draft
                .take()
                .is_some()
    }

    pub(in crate::app) fn commit_preview_timecode_input(
        &mut self,
        kind: FrameTextInputKind,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if !kind.is_preview_timecode() {
            return false;
        }
        let Some(draft) = self.text_input_runtime_mut(kind).timecode_draft.take() else {
            return false;
        };

        if self.preview_ui.playback_file_id.as_deref() != Some(draft.file_id.as_str())
            || self
                .file_queue
                .file_by_id(&draft.file_id)
                .is_none_or(FileItem::locks_settings)
        {
            return true;
        }

        let seconds = parse_time_to_seconds(&draft.value);
        let duration = self.preview_ui.playback.duration();
        if !seconds.is_finite() || !duration.is_finite() || duration <= 0.0 {
            return true;
        }

        let command = match kind {
            FrameTextInputKind::PreviewStartTime => {
                let maximum = (self.preview_ui.playback.end_value() - TIMECODE_PRECISION_SECONDS)
                    .max(0.0)
                    .min(duration);
                self.preview_ui
                    .playback
                    .set_start_from_input(seconds.clamp(0.0, maximum))
            }
            FrameTextInputKind::PreviewEndTime => {
                let minimum = (self.preview_ui.playback.start_value() + TIMECODE_PRECISION_SECONDS)
                    .min(duration);
                self.preview_ui
                    .playback
                    .set_end_from_input(seconds.clamp(minimum, duration))
            }
            _ => unreachable!("matched preview timecode text input variants"),
        };
        let Some(command) = command else {
            return true;
        };
        let Some(trim) = self.preview_ui.playback.commit_trim_values() else {
            return true;
        };

        if let Some(file) = self.file_queue.file_by_id_mut(&draft.file_id) {
            apply_trim_times(&mut file.config, trim.start_time, trim.end_time);
        }
        if self.file_queue.selected_file_id() == Some(draft.file_id.as_str()) {
            self.apply_preview_media_command(command, true, cx);
        }

        true
    }

    pub(in crate::app) const fn next_text_input_cursor_epoch(&mut self) -> usize {
        self.text_input_ui.cursor_epoch += 1;
        self.text_input_ui.cursor_epoch
    }

    pub(in crate::app) fn start_text_input_cursor(&mut self, cx: &mut Context<Self>) {
        self.text_input_ui.cursor_paused = false;
        self.blink_text_input_cursor(self.text_input_ui.cursor_epoch, cx);
    }

    pub(in crate::app) const fn stop_text_input_cursor(&mut self) {
        self.text_input_ui.active = None;
        self.text_input_ui.cursor_paused = false;
        self.text_input_ui.cursor_visible = false;
        self.next_text_input_cursor_epoch();
    }

    pub(in crate::app) fn pause_text_input_cursor(&mut self, cx: &mut Context<Self>) {
        self.text_input_ui.cursor_paused = true;
        self.text_input_ui.cursor_visible = true;
        cx.notify();

        let epoch = self.next_text_input_cursor_epoch();
        self.text_input_ui.cursor_task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TEXT_INPUT_BLINK_PAUSE).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |root, cx| {
                    root.text_input_ui.cursor_paused = false;
                    root.blink_text_input_cursor(epoch, cx);
                });
            }
        });
    }

    pub(in crate::app) fn blink_text_input_cursor(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if self.text_input_ui.active.is_none() {
            self.text_input_ui.cursor_visible = false;
            return;
        }
        if self.text_input_ui.cursor_paused || epoch != self.text_input_ui.cursor_epoch {
            self.text_input_ui.cursor_visible = true;
            return;
        }

        self.text_input_ui.cursor_visible = !self.text_input_ui.cursor_visible;
        cx.notify();

        let next_epoch = self.next_text_input_cursor_epoch();
        self.text_input_ui.cursor_task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(TEXT_INPUT_BLINK_INTERVAL)
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |root, cx| {
                    root.blink_text_input_cursor(next_epoch, cx);
                });
            }
        });
    }
}

fn file_gif_loop_value(file_queue: &FileQueue) -> String {
    file_queue
        .selected_file()
        .map_or_else(String::new, |file| file.config.gif_loop.to_string())
}

const fn metadata_field_for_text_input(kind: FrameTextInputKind) -> Option<MetadataField> {
    match kind {
        FrameTextInputKind::MetadataTitle => Some(MetadataField::Title),
        FrameTextInputKind::MetadataArtist => Some(MetadataField::Artist),
        FrameTextInputKind::MetadataAlbum => Some(MetadataField::Album),
        FrameTextInputKind::MetadataGenre => Some(MetadataField::Genre),
        FrameTextInputKind::MetadataDate => Some(MetadataField::Date),
        FrameTextInputKind::MetadataComment => Some(MetadataField::Comment),
        _ => None,
    }
}

fn text_input_drag_scroll_amount(distance: Pixels) -> Pixels {
    distance.clamp(px(4.0), px(40.0))
}
