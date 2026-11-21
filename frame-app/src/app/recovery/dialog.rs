use crate::app::accessibility::{
    APP_ROOT_FOCUS_ID, FrameFocusKey, handle_focus_scope_tab_navigation,
};
use crate::app::components::{
    frame_dialog_close_button, frame_dialog_footer, frame_dialog_header, frame_dialog_overlay,
    frame_dialog_panel, frame_text_button_with_focus,
};
use crate::app::motion::{DialogPresence, surface_motion};
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, KeyDownEvent, Window};

#[derive(Clone, Copy)]
enum RecoveryAction {
    Cancel,
    Restore,
    Discard,
    Later,
    Continue,
}

impl RecoveryAction {
    const fn id(self) -> &'static str {
        match self {
            Self::Cancel => "recovery-cancel",
            Self::Restore => "recovery-restore",
            Self::Discard => "recovery-discard",
            Self::Later => "recovery-later",
            Self::Continue => "recovery-continue",
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Cancel => "Cancel",
            Self::Restore => "Restore queue",
            Self::Discard => "Discard queue",
            Self::Later => "Later",
            Self::Continue => "Continue",
        }
    }
    fn apply(self, root: &mut FrameRoot, cx: &mut Context<FrameRoot>) {
        if root.recovery_loading() {
            return;
        }
        match self {
            Self::Cancel if root.recovery_notice().is_some() => root.dismiss_recovery_notice(cx),
            Self::Cancel | Self::Later => root.defer_workspace_recovery(cx),
            Self::Restore => root.restore_workspace(cx),
            Self::Discard => root.discard_workspace(cx),
            Self::Continue => root.dismiss_recovery_notice(cx),
        }
    }
}

#[derive(Clone)]
struct RecoveryView {
    title: &'static str,
    description: String,
    actions: &'static [RecoveryAction],
    close_action: Option<RecoveryAction>,
    escape_action: Option<RecoveryAction>,
}

#[derive(Default)]
struct RecoveryPresentation {
    presence: DialogPresence,
    view: Option<RecoveryView>,
}

impl FrameRoot {
    fn recovery_dialog_view(&self) -> Option<RecoveryView> {
        if self.recovery_loading() {
            Some(RecoveryView {
                title: "Loading saved queue…",
                description: "Checking the previous workspace.".into(),
                actions: &[RecoveryAction::Cancel],
                close_action: None,
                escape_action: None,
            })
        } else if let Some(count) = self.recovery_prompt_count().filter(|count| *count > 0) {
            let files = if count == 1 {
                "1 prepared file".into()
            } else {
                format!("{count} prepared files")
            };
            Some(RecoveryView {
                title: "Restore prepared queue?",
                description: format!(
                    "{files} can be restored. Conversions will not start automatically. Later keeps this queue for the next start; new changes will not be saved for recovery."
                ),
                actions: &[
                    RecoveryAction::Cancel,
                    RecoveryAction::Later,
                    RecoveryAction::Restore,
                ],
                close_action: Some(RecoveryAction::Discard),
                escape_action: Some(RecoveryAction::Later),
            })
        } else {
            self.recovery_notice().map(|notice| RecoveryView {
                title: "Queue recovery unavailable",
                description: notice.into(),
                actions: &[RecoveryAction::Cancel, RecoveryAction::Continue],
                close_action: Some(RecoveryAction::Continue),
                escape_action: Some(RecoveryAction::Continue),
            })
        }
    }

    pub(in crate::app) fn render_recovery_overlay(
        &mut self,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<gpui::Div>> {
        let current = self.recovery_dialog_view();
        let presentation = window.use_keyed_state("recovery-dialog-presence", cx, |_, _| {
            RecoveryPresentation::default()
        });
        let open = presentation.update(cx, |state, _| {
            if let Some(view) = current {
                state.view = Some(view);
                state.presence.open();
            } else {
                state.presence.close();
            }
            state.presence.is_open()
        });
        if !presentation.read(cx).presence.is_present() {
            return None;
        }
        let progress = surface_motion("recovery-dialog-motion", open, window, cx);
        if presentation.update(cx, |state, _| state.presence.finish_close(progress)) {
            self.focus_registered_control(APP_ROOT_FOCUS_ID, window, cx);
            return None;
        }
        let view = presentation.read(cx).view.clone()?;
        let (panel, scope) = self.recovery_dialog_panel(&view, open, palette, window, cx);
        Some(
            frame_dialog_overlay("recovery-overlay", panel, progress, open, palette, window)
                .on_key_down(cx.listener(move |root, event: &KeyDownEvent, window, cx| {
                    if open {
                        match event.keystroke.key.as_str() {
                            "escape" => {
                                if let Some(action) = view.escape_action {
                                    action.apply(root, cx);
                                }
                            }
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
    fn recovery_dialog_panel(
        &mut self,
        view: &RecoveryView,
        open: bool,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Stateful<gpui::Div>, FocusHandle) {
        let scope = self.ensure_focus(FrameFocusKey::Control("recovery-dialog".into()), false, cx);
        let mut footer = frame_dialog_footer(palette, window);
        let mut initial_focus = None;
        let enabled = open && !self.recovery_loading();
        for action in view.actions {
            let focus = self.ensure_focus(FrameFocusKey::Control(action.id().into()), enabled, cx);
            if enabled
                && (initial_focus.is_none()
                    || matches!(action, RecoveryAction::Restore | RecoveryAction::Continue))
            {
                initial_focus = Some(focus.clone());
            }
            footer = footer.child(recovery_button(
                *action, &focus, enabled, palette, window, cx,
            ));
        }
        if open
            && (!scope.contains_focused(window, cx)
                || (scope.is_focused(window) && initial_focus.is_some()))
        {
            initial_focus.as_ref().unwrap_or(&scope).focus(window, cx);
        }
        let mut panel = frame_dialog_panel(
            "recovery-dialog",
            view.title,
            Some(&view.description),
            &scope,
            palette,
            window,
        )
        .child(frame_dialog_header(
            view.title,
            Some(&view.description),
            palette,
        ))
        .child(footer);
        if let Some(action) = view.close_action {
            let close =
                self.ensure_focus(FrameFocusKey::Control("recovery-close".into()), open, cx);
            panel = panel.child(
                frame_dialog_close_button(
                    "recovery-close",
                    if matches!(action, RecoveryAction::Discard) {
                        "Discard queue"
                    } else {
                        "Close recovery dialog"
                    },
                    open,
                    &close,
                    palette,
                    window,
                    cx,
                )
                .on_click(cx.listener(move |root, _: &ClickEvent, _, cx| action.apply(root, cx))),
            );
        }
        (panel, scope)
    }
}

fn recovery_button(
    action: RecoveryAction,
    focus: &FocusHandle,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = match action {
        RecoveryAction::Cancel | RecoveryAction::Discard => ButtonVariant::Ghost,
        RecoveryAction::Later => ButtonVariant::Secondary,
        RecoveryAction::Restore | RecoveryAction::Continue => ButtonVariant::Primary,
    };
    frame_text_button_with_focus(
        action.id(),
        action.label(),
        variant,
        false,
        enabled,
        focus,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _, cx| {
        cx.stop_propagation();
        if enabled {
            action.apply(root, cx);
        }
    }))
}
