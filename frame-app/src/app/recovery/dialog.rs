use crate::app::accessibility::{
    APP_ROOT_FOCUS_ID, FrameFocusKey, handle_focus_scope_tab_navigation,
};
use crate::app::components::{FrameStyle, frame_text_button_with_focus};
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, KeyDownEvent, Window, div};

#[derive(Clone, Copy)]
enum RecoveryAction {
    Restore,
    Discard,
    Later,
    Continue,
}

impl RecoveryAction {
    const fn id(self) -> &'static str {
        match self {
            Self::Restore => "recovery-restore",
            Self::Discard => "recovery-discard",
            Self::Later => "recovery-later",
            Self::Continue => "recovery-continue",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Restore => "Restore queue",
            Self::Discard => "Discard queue",
            Self::Later => "Later",
            Self::Continue => "Continue",
        }
    }

    fn apply(self, root: &mut FrameRoot, window: &mut Window, cx: &mut Context<FrameRoot>) {
        if root.recovery_loading() {
            return;
        }
        match self {
            Self::Restore => root.restore_workspace(cx),
            Self::Discard => root.discard_workspace(cx),
            Self::Later => root.defer_workspace_recovery(cx),
            Self::Continue => root.dismiss_recovery_notice(cx),
        }
        root.focus_registered_control(APP_ROOT_FOCUS_ID, window, cx);
    }
}

impl FrameRoot {
    pub(in crate::app) fn render_recovery_overlay(
        &mut self,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<gpui::Div>> {
        let count = self.recovery_prompt_count().filter(|count| *count > 0);
        let loading = self.recovery_loading();
        let (title, description, actions): (_, String, &[RecoveryAction]) = if loading {
            (
                "Loading saved queue…",
                "Checking the previous workspace.".into(),
                &[],
            )
        } else if let Some(count) = count {
            let files = if count == 1 {
                "1 prepared file".into()
            } else {
                format!("{count} prepared files")
            };
            (
                "Restore prepared queue?",
                format!(
                    "{files} can be restored. Conversions will not start automatically. Later keeps this queue for the next start; new changes will not be saved for recovery."
                ),
                &[
                    RecoveryAction::Restore,
                    RecoveryAction::Discard,
                    RecoveryAction::Later,
                ],
            )
        } else if let Some(notice) = self.recovery_notice() {
            (
                "Queue recovery unavailable",
                notice.to_owned(),
                &[RecoveryAction::Continue],
            )
        } else {
            return None;
        };
        let scope = self.ensure_focus(FrameFocusKey::Control("recovery-dialog".into()), false, cx);
        let mut footer = div().flex().flex_wrap().justify_end().gap_2();
        let mut first_focus = None;
        for action in actions {
            let focus = self.ensure_focus(FrameFocusKey::Control(action.id().into()), true, cx);
            if first_focus.is_none() {
                first_focus = Some(focus.clone());
            }
            footer = footer.child(recovery_button(*action, &focus, palette, window, cx));
        }
        if !scope.contains_focused(window, cx) {
            first_focus.as_ref().unwrap_or(&scope).focus(window, cx);
        }
        let panel = recovery_panel(title, description, footer, &scope, palette);
        Some(
            div()
                .id("recovery-overlay")
                .absolute()
                .inset_0()
                .p_4()
                .flex()
                .items_center()
                .justify_center()
                .bg(color(palette.overlay_backdrop))
                .occlude()
                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.stop_propagation()))
                .on_key_down(cx.listener(move |root, event: &KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" if !loading => {
                            let action = if count.is_some() {
                                RecoveryAction::Later
                            } else {
                                RecoveryAction::Continue
                            };
                            action.apply(root, window, cx);
                        }
                        "tab" if !loading => {
                            handle_focus_scope_tab_navigation(event, &scope, window, cx);
                        }
                        _ => {}
                    }
                    cx.stop_propagation();
                }))
                .child(panel),
        )
    }
}

fn recovery_button(
    action: RecoveryAction,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = match action {
        RecoveryAction::Restore => ButtonVariant::Primary,
        RecoveryAction::Discard => ButtonVariant::Destructive,
        RecoveryAction::Later | RecoveryAction::Continue => ButtonVariant::Secondary,
    };
    frame_text_button_with_focus(
        action.id(),
        action.label(),
        variant,
        false,
        true,
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

fn recovery_panel(
    title: &'static str,
    description: String,
    footer: gpui::Div,
    scope: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("recovery-dialog")
        .debug_selector(|| "recovery-dialog".to_owned())
        .role(gpui::Role::Dialog)
        .aria_modal(true)
        .aria_label(title)
        .aria_description(description.clone())
        .track_focus(scope)
        .tab_stop(false)
        .w_full()
        .max_w(theme::ui_rem(520.0))
        .max_h(gpui::relative(0.9))
        .overflow_y_scroll()
        .dialog_surface(palette)
        .flex()
        .flex_col()
        .gap_4()
        .p_4()
        .child(
            div()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(palette.text_primary))
                .child(theme::ui_text(title)),
        )
        .child(
            div()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.text_muted))
                .child(theme::ui_text_owned(description)),
        )
        .child(footer)
}
