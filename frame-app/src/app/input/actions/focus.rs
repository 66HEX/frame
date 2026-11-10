use crate::app::input::runtime::{FrameTextInputKind, FrameTextInputRuntime};
use crate::app::root::FrameRoot;
use gpui::{Context, FocusHandle, Window};

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
        self.update_installation_in_progress()
            || (self.preview_ui.crop_mode
                && matches!(
                    kind,
                    FrameTextInputKind::VideoCustomWidth | FrameTextInputKind::VideoCustomHeight
                ))
            || (kind != FrameTextInputKind::MaxConcurrency
                && self.file_queue.selected_file_locked())
    }
}
