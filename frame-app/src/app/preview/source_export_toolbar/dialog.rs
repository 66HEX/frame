use crate::app::accessibility::{
    APP_ROOT_FOCUS_ID, FrameFocusKey, handle_focus_scope_tab_navigation,
};
use crate::app::components::{
    frame_dialog_close_button, frame_dialog_footer, frame_dialog_header, frame_dialog_overlay,
    frame_dialog_panel, frame_text_button_with_focus,
};
use crate::app::motion::surface_motion;
use crate::app::preview::actions::source_export::{
    SourceFrameExportRenderState, SourceFrameExportStatus,
};
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, KeyDownEvent, Window};

#[derive(Clone, Copy)]
enum Action {
    Cancel,
    Close,
    Reveal,
    Open,
}

impl Action {
    const fn id(self) -> &'static str {
        match self {
            Self::Cancel => "source-frame-cancel",
            Self::Close => "source-frame-dismiss",
            Self::Reveal => "source-frame-reveal",
            Self::Open => "source-frame-open",
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Cancel => "Cancel export",
            Self::Close => "Close",
            Self::Reveal => "Show in folder",
            Self::Open => "Open image",
        }
    }
    fn apply(self, root: &mut FrameRoot, window: &mut Window, cx: &mut Context<FrameRoot>) {
        match self {
            Self::Cancel => root.cancel_source_frame_export(cx),
            Self::Close => root.close_source_frame_dialog(cx),
            Self::Reveal => root.reveal_source_frame_result(window, cx),
            Self::Open => root.open_source_frame_result(window, cx),
        }
    }
}

impl FrameRoot {
    fn close_source_frame_dialog(&mut self, cx: &mut Context<Self>) {
        if !self.source_frame_export_running() && self.preview_ui.source_export.dialog.close() {
            cx.notify();
        }
    }

    pub(in crate::app) fn render_source_frame_export_overlay(
        &mut self,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<gpui::Div>> {
        if !self.preview_ui.source_export.dialog.is_present() {
            return None;
        }
        let open = self.preview_ui.source_export.dialog.is_open();
        let progress = surface_motion("source-frame-dialog-motion", open, window, cx);
        if self.preview_ui.source_export.dialog.finish_close(progress) {
            if self
                .source_frame_export_render_state()
                .disabled_reason
                .is_some()
                || !self.focus_registered_control("source-frame-export", window, cx)
            {
                self.focus_registered_control(APP_ROOT_FOCUS_ID, window, cx);
            }
            return None;
        }
        let state = self.source_frame_export_render_state();
        let (panel, scope) = self.source_frame_dialog_panel(&state, open, palette, window, cx);
        Some(
            frame_dialog_overlay(
                "source-frame-overlay",
                panel,
                progress,
                open,
                palette,
                window,
            )
            .on_key_down(cx.listener(move |root, event: &KeyDownEvent, window, cx| {
                if open {
                    match event.keystroke.key.as_str() {
                        "escape" => root.close_source_frame_dialog(cx),
                        "tab" => {
                            handle_focus_scope_tab_navigation(event, &scope, window, cx);
                        }
                        _ => {}
                    }
                }
                cx.stop_propagation();
            })),
        )
    }
    fn source_frame_dialog_panel(
        &mut self,
        state: &SourceFrameExportRenderState,
        open: bool,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Stateful<gpui::Div>, FocusHandle) {
        let (title, description) = source_frame_message(state);
        let scope = self.ensure_focus(
            FrameFocusKey::Control("source-frame-dialog".into()),
            false,
            cx,
        );
        let mut footer = frame_dialog_footer(palette, window);
        let actions: &[Action] = if state.running {
            &[Action::Cancel]
        } else if state.status == SourceFrameExportStatus::Saved && state.result_name.is_some() {
            &[Action::Close, Action::Reveal, Action::Open]
        } else {
            &[Action::Close]
        };
        let mut initial_focus = None;
        for action in actions {
            let enabled = open
                && (!matches!(action, Action::Cancel)
                    || state.status != SourceFrameExportStatus::Cancelling);
            let focus = self.ensure_focus(FrameFocusKey::Control(action.id().into()), enabled, cx);
            if enabled && (initial_focus.is_none() || matches!(action, Action::Open)) {
                initial_focus = Some(focus.clone());
            }
            footer = footer.child(action_button(*action, &focus, enabled, palette, window, cx));
        }
        if open
            && (!scope.contains_focused(window, cx)
                || (scope.is_focused(window) && initial_focus.is_some()))
        {
            initial_focus.as_ref().unwrap_or(&scope).focus(window, cx);
        }
        let mut panel = frame_dialog_panel(
            "source-frame-dialog",
            title,
            Some(&description),
            &scope,
            palette,
            window,
        )
        .child(frame_dialog_header(title, Some(&description), palette))
        .child(footer);
        if !state.running {
            let close_focus = self.ensure_focus(
                FrameFocusKey::Control("source-frame-close".into()),
                open,
                cx,
            );
            panel = panel.child(
                frame_dialog_close_button(
                    "source-frame-close",
                    "Close source frame dialog",
                    open,
                    &close_focus,
                    palette,
                    window,
                    cx,
                )
                .on_click(
                    cx.listener(|root, _: &ClickEvent, _, cx| root.close_source_frame_dialog(cx)),
                ),
            );
        }
        (panel, scope)
    }
}

fn source_frame_message(state: &SourceFrameExportRenderState) -> (&'static str, String) {
    match &state.status {
        SourceFrameExportStatus::Idle => ("Source frame", String::new()),
        SourceFrameExportStatus::Exporting => (
            "Exporting source frame…",
            "Saving a PNG from the source, without conversion settings.".into(),
        ),
        SourceFrameExportStatus::Cancelling => (
            "Cancelling export…",
            "Waiting for the export to stop. A file already saved will remain available.".into(),
        ),
        SourceFrameExportStatus::Saved => (
            "Source frame saved",
            state
                .result_name
                .clone()
                .unwrap_or_else(|| "Your PNG is ready.".into()),
        ),
        SourceFrameExportStatus::Cancelled => {
            ("Export cancelled", "No new source frame was saved.".into())
        }
        SourceFrameExportStatus::Error(error) => ("Unable to export source frame", error.clone()),
    }
}

fn action_button(
    action: Action,
    focus: &FocusHandle,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button_with_focus(
        action.id(),
        action.label(),
        if matches!(action, Action::Open) {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Secondary
        },
        false,
        enabled,
        focus,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        action.apply(root, window, cx);
    }))
}
