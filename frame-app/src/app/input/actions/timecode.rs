use crate::app::input::constants::TIMECODE_PRECISION_SECONDS;
use crate::app::input::runtime::FrameTextInputKind;
use crate::app::root::FrameRoot;
use crate::app::{TextInputCancel, TextInputCommit};
use crate::file_queue::FileItem;
use crate::preview::parse_time_to_seconds;
use crate::settings::apply_trim_times;
use gpui::{Context, Window};

impl FrameRoot {
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
}
