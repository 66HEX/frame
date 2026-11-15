//! One application-owned Mira renderer for every GPUI confirmation and notice.
use crate::app::FrameRoot;
use crate::app::components::{
    frame_button_base, frame_dialog_close_button, frame_dialog_footer, frame_dialog_header,
    frame_dialog_overlay, frame_dialog_panel,
};
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::primitives::ButtonVariant;
use crate::theme;
use gpui::prelude::*;
use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, PromptButton, PromptHandle, PromptLevel,
    PromptResponse, Render, RenderablePromptHandle, Window,
};

pub(super) fn render_prompt(
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    actions: &[PromptButton],
    handle: PromptHandle,
    window: &mut Window,
    cx: &mut App,
) -> RenderablePromptHandle {
    let view = cx.new(|cx| FramePrompt {
        level,
        message: message.to_owned(),
        detail: detail.map(str::to_owned),
        actions: actions.to_vec(),
        focuses: actions.iter().map(|_| cx.focus_handle()).collect(),
        scope: cx.focus_handle(),
        close_focus: cx.focus_handle(),
        response: None,
        completed: false,
    });
    handle.with_view(view, window, cx)
}

struct FramePrompt {
    level: PromptLevel,
    message: String,
    detail: Option<String>,
    actions: Vec<PromptButton>,
    focuses: Vec<FocusHandle>,
    scope: FocusHandle,
    close_focus: FocusHandle,
    response: Option<usize>,
    completed: bool,
}

impl FramePrompt {
    fn cancel_index(&self) -> Option<usize> {
        self.actions.iter().position(PromptButton::is_cancel)
    }

    fn respond(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.response.is_none() && !self.completed && index < self.actions.len() {
            self.response = Some(index);
            cx.notify();
        }
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.cancel_index() {
            self.respond(index, cx);
        }
    }
}

impl EventEmitter<PromptResponse> for FramePrompt {}
impl Focusable for FramePrompt {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.cancel_index()
            .and_then(|i| self.focuses.get(i))
            .or_else(|| self.focuses.first())
            .unwrap_or(&self.scope)
            .clone()
    }
}

impl Render for FramePrompt {
    #[expect(
        clippy::too_many_lines,
        reason = "One render keeps the prompt response lifecycle, accessible actions and shared Mira shell together."
    )]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = theme::palette(
            window
                .root::<FrameRoot>()
                .flatten()
                .map(|root| root.read(cx).appearance.color_theme)
                .unwrap_or_default(),
        );
        let open = self.response.is_none() && !self.completed;
        let progress = surface_motion(("frame-prompt-motion", cx.entity_id()), open, window, cx);
        if !open
            && motion_is_hidden(progress)
            && let Some(index) = self.response.take()
        {
            self.completed = true;
            // GPUI removes the prompt and restores its original focus on this event.
            // Keep the response pending until the rendered exit has settled.
            cx.defer_in(window, move |_, _, cx| cx.emit(PromptResponse(index)));
        }
        let primary = self
            .actions
            .iter()
            .position(|action| matches!(action, PromptButton::Ok(_)))
            .or_else(|| self.actions.iter().position(|action| !action.is_cancel()));
        let mut order: Vec<_> = (0..self.actions.len()).collect();
        order.sort_by_key(|index| {
            if self.actions[*index].is_cancel() {
                0
            } else if Some(*index) == primary {
                2
            } else {
                1
            }
        });
        let mut footer = frame_dialog_footer(palette, window);
        for index in order {
            let label = self.actions[index].label().to_string();
            footer = footer.child(
                frame_button_base(
                    format!("frame-prompt-action-{index}"),
                    &label,
                    if self.actions[index].is_cancel() {
                        ButtonVariant::Ghost
                    } else if Some(index) == primary {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Outline
                    },
                    false,
                    open,
                    theme::metrics::ControlSize::Default,
                    palette,
                    window,
                    cx,
                )
                .track_focus(&self.focuses[index])
                .child(label)
                .on_click(cx.listener(move |prompt, _, _, cx| {
                    cx.stop_propagation();
                    prompt.respond(index, cx);
                })),
            );
        }
        let mut panel = frame_dialog_panel(
            "frame-prompt-panel",
            &self.message,
            self.detail.as_deref(),
            &self.scope,
            palette,
            window,
        )
        .role(match self.level {
            PromptLevel::Info => gpui::Role::Dialog,
            _ => gpui::Role::AlertDialog,
        })
        .max_h(gpui::relative(0.9))
        .overflow_y_scroll()
        .child(frame_dialog_header(
            &self.message,
            self.detail.as_deref(),
            palette,
        ))
        .child(footer);
        if self.cancel_index().is_some() {
            panel = panel.child(
                frame_dialog_close_button(
                    "frame-prompt-close",
                    "Close dialog",
                    open,
                    &self.close_focus,
                    palette,
                    window,
                    cx,
                )
                .on_click(cx.listener(|prompt, _, _, cx| {
                    cx.stop_propagation();
                    prompt.cancel(cx);
                })),
            );
        }
        frame_dialog_overlay("frame-prompt", panel, progress, open, palette, window)
            .on_click(cx.listener(|prompt, _, _, cx| {
                cx.stop_propagation();
                prompt.cancel(cx);
            }))
            .on_key_down(
                cx.listener(|prompt, event: &gpui::KeyDownEvent, window, cx| {
                    if prompt.response.is_some() || prompt.completed {
                        cx.stop_propagation();
                        return;
                    }
                    match event.keystroke.key.as_str() {
                        "escape" => {
                            prompt.cancel(cx);
                            cx.stop_propagation();
                        }
                        "tab" => {
                            crate::app::accessibility::handle_focus_scope_tab_navigation(
                                event,
                                &prompt.scope,
                                window,
                                cx,
                            );
                        }
                        "enter" | "space" => {
                            if prompt.close_focus.is_focused(window) {
                                prompt.cancel(cx);
                            } else if let Some(index) = prompt
                                .focuses
                                .iter()
                                .position(|focus| focus.is_focused(window))
                            {
                                prompt.respond(index, cx);
                            }
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }),
            )
    }
}

#[cfg(test)]
pub(super) mod tests;
