//! Roving focus for Mira `ToggleGroup` compositions. Arrows move focus; Enter/Space activate.
use crate::app::root::FrameRoot;
use gpui::{App, Context, Entity, FocusHandle, InteractiveElement, Window};
use std::collections::HashMap;

#[derive(Default)]
struct ChoiceState {
    handles: HashMap<String, FocusHandle>,
    items: Vec<(String, bool, bool)>,
    active: Option<String>,
    has_focused: bool,
    saw_selected: bool,
}

pub(in crate::app) struct FrameChoiceFocus(Entity<ChoiceState>, bool);

impl FrameChoiceFocus {
    pub(in crate::app) fn new(id: &'static str, window: &mut Window, cx: &mut App) -> Self {
        let state =
            window.use_keyed_state(format!("{id}-choices"), cx, |_, _| ChoiceState::default());
        state.update(cx, |state, _| {
            let focused = state
                .handles
                .iter()
                .find(|(_, focus)| focus.is_focused(window))
                .map(|(id, _)| id.clone());
            state.has_focused = focused.is_some();
            state.active = focused
                .or_else(|| {
                    state
                        .items
                        .iter()
                        .find(|(_, enabled, selected)| *enabled && *selected)
                        .map(|(id, _, _)| id.clone())
                })
                .or_else(|| state.active.clone());
            state.items.clear();
            state.saw_selected = false;
        });
        Self(state, false)
    }

    pub(in crate::app) fn vertical(id: &'static str, window: &mut Window, cx: &mut App) -> Self {
        let mut group = Self::new(id, window, cx);
        group.1 = true;
        group
    }

    pub(in crate::app) fn register(
        &self,
        id: &str,
        selected: bool,
        enabled: bool,
        cx: &mut App,
    ) -> FocusHandle {
        self.0.update(cx, |state, cx| {
            let focus = state
                .handles
                .entry(id.into())
                .or_insert_with(|| cx.focus_handle())
                .clone();
            if !enabled && state.active.as_deref() == Some(id) {
                state.active = None;
                cx.notify();
            }
            let select_entry = enabled && selected && !state.saw_selected;
            if enabled && selected {
                state.saw_selected = true;
            }
            if !state.has_focused
                && (select_entry || state.active.is_none())
                && state.active.as_deref() != Some(id)
            {
                state.active = Some(id.into());
                // On first paint the selected option can follow the first enabled option.
                // Redraw once so only that selected entry remains in the tab sequence.
                cx.notify();
            }
            state.items.push((id.into(), enabled, selected));
            focus
        })
    }

    pub(in crate::app) fn apply(
        &self,
        button: gpui::Stateful<gpui::Div>,
        id: String,
        enabled: bool,
        focus: &FocusHandle,
        cx: &Context<FrameRoot>,
    ) -> gpui::Stateful<gpui::Div> {
        let vertical = self.1;
        let active = self.0.read(cx).active.as_ref() == Some(&id);
        let pointer_state = self.0.clone();
        let key_state = self.0.clone();
        let pointer_id = id.clone();
        let pointer_focus = focus.clone();
        button
            // Explicit handles own their tab-stop flag in GPUI; the element's
            // flag only configures an implicitly created handle.
            .track_focus(&focus.clone().tab_stop(enabled && active))
            .tab_stop(enabled && active)
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                if enabled {
                    pointer_focus.focus(window, cx);
                    pointer_state.update(cx, |state, cx| {
                        state.active = Some(pointer_id.clone());
                        cx.notify();
                    });
                }
            })
            .on_key_down(
                cx.listener(move |_, event: &gpui::KeyDownEvent, window, cx| {
                    let key = event.keystroke.key.as_str();
                    if !enabled
                        || !(matches!(key, "home" | "end")
                            || if vertical {
                                matches!(key, "up" | "down")
                            } else {
                                matches!(key, "left" | "right")
                            })
                    {
                        return;
                    }
                    key_state.update(cx, |state, cx| {
                        let enabled: Vec<_> = state
                            .items
                            .iter()
                            .filter(|(_, enabled, _)| *enabled)
                            .map(|(id, _, _)| id.clone())
                            .collect();
                        let Some(index) = enabled.iter().position(|candidate| *candidate == id)
                        else {
                            return;
                        };
                        let next = match key {
                            "left" | "up" => (index + enabled.len() - 1) % enabled.len(),
                            "right" | "down" => (index + 1) % enabled.len(),
                            "home" => 0,
                            _ => enabled.len() - 1,
                        };
                        state.handles[&enabled[next]].focus(window, cx);
                        state.active = Some(enabled[next].clone());
                        cx.notify();
                    });
                    cx.stop_propagation();
                }),
            )
    }
}
