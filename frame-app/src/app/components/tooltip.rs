use super::super::primitives::card_surface_shadows;
use super::{
    Context, Duration, FluentBuilder, FrameRoot, INTERACTION_MOTION_DURATION, InteractiveElement,
    IntoElement, ParentElement, StatefulInteractiveElement, Styled, TooltipUiState, Window, color,
    deferred, div, ease_in_out, motion_target, set_motion_target, theme,
};
use gpui::{Entity, FocusHandle, Subscription};
use std::time::Instant;

const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(500);
const TOOLTIP_HYSTERESIS_WINDOW: Duration = Duration::from_millis(300);
const TOOLTIP_OFFSET: f32 = 6.0;
const TOOLTIP_ENTER_DISTANCE: f32 = 4.0;
const TOOLTIP_DEFERRED_PRIORITY: usize = 20;

#[derive(Clone, Copy)]
pub(in crate::app) enum TooltipPlacement {
    Above,
    Right,
}

pub(in crate::app) struct FrameTooltip {
    pub(in crate::app) id: String,
    pub(in crate::app) label: String,
    pub(in crate::app) hovered: bool,
    pub(in crate::app) placement: TooltipPlacement,
    pub(in crate::app) anchor_size: f32,
}

struct TooltipFocusState {
    focus: FocusHandle,
    dismissed: bool,
    _subscriptions: [Subscription; 2],
}

impl TooltipFocusState {
    fn new(focus: FocusHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let on_focus = cx.on_focus(&focus, window, |state, _window, cx| {
            state.dismissed = false;
            cx.notify();
        });
        let on_blur = cx.on_blur(&focus, window, |state, _window, cx| {
            state.dismissed = false;
            cx.notify();
        });
        Self {
            focus,
            dismissed: false,
            _subscriptions: [on_focus, on_blur],
        }
    }
}

impl FrameTooltip {
    pub(in crate::app) fn for_control(
        self,
        button: gpui::Stateful<gpui::Div>,
        focus: Option<&FocusHandle>,
        enabled: bool,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<FrameRoot>,
    ) -> gpui::Stateful<gpui::Div> {
        let focus_state =
            window.use_keyed_state(format!("tooltip-{}-focus", self.id), cx, |window, cx| {
                let focus = focus.cloned().unwrap_or_else(|| cx.focus_handle());
                TooltipFocusState::new(focus, window, cx)
            });
        let state = focus_state.read(cx);
        let visible = !state.dismissed
            && if window.last_input_was_keyboard() {
                enabled && state.focus.is_focused(window)
            } else {
                self.hovered
            };
        let focus = state.focus.clone().tab_stop(enabled);
        let button = button.track_focus(&focus).tab_stop(enabled);
        self.render(button, visible, Some(focus_state), palette, window, cx)
    }

    fn render(
        self,
        anchor: gpui::Stateful<gpui::Div>,
        visible: bool,
        focus_state: Option<Entity<TooltipFocusState>>,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<FrameRoot>,
    ) -> gpui::Stateful<gpui::Div> {
        let hover_id = self.id.clone();
        let dismiss_id = self.id.clone();
        let hover_focus_state = focus_state.clone();
        let motion = window
            .use_keyed_transition(
                format!("tooltip-{}-motion", self.id),
                cx,
                INTERACTION_MOTION_DURATION,
                |_window, _cx| 0.0_f32,
            )
            .with_easing(ease_in_out);
        set_motion_target(&motion, motion_target(visible), cx);
        let progress = *motion.evaluate(window, cx);
        let offset =
            (progress - 1.0).mul_add(TOOLTIP_ENTER_DISTANCE, self.anchor_size + TOOLTIP_OFFSET);

        anchor
            .debug_selector(|| self.id.clone())
            .relative()
            .on_hover(cx.listener(move |root, hovered: &bool, _window, cx| {
                if let Some(state) = &hover_focus_state {
                    state.update(cx, |state, cx| {
                        state.dismissed = false;
                        cx.notify();
                    });
                }
                if *hovered {
                    root.begin_tooltip_hover(hover_id.clone(), cx);
                } else {
                    root.end_tooltip_hover(&hover_id, cx);
                }
            }))
            .when_some(focus_state, |this, state| {
                this.on_key_down(cx.listener(
                    move |root, event: &gpui::KeyDownEvent, _window, cx| {
                        if event.keystroke.key == "escape" && visible {
                            state.update(cx, |state, cx| {
                                state.dismissed = true;
                                cx.notify();
                            });
                            root.end_tooltip_hover(&dismiss_id, cx);
                            cx.stop_propagation();
                        }
                    },
                ))
            })
            .when(visible, |this| {
                let bubble = div()
                    .id(format!("tooltip-{}", self.id))
                    .debug_selector(|| format!("tooltip-{}", self.id))
                    .absolute()
                    .flex()
                    .map(|this| match self.placement {
                        TooltipPlacement::Above => this
                            .bottom(theme::ui_rem(offset))
                            .left_0()
                            .right_0()
                            .justify_center(),
                        TooltipPlacement::Right => this
                            .left(theme::ui_rem(offset))
                            .top_0()
                            .h_full()
                            .items_center(),
                    })
                    .child(
                        div()
                            .debug_selector(|| "frame-tooltip-bubble".to_string())
                            .flex_none()
                            .whitespace_nowrap()
                            .rounded(theme::ui_rem(theme::RADIUS_SM))
                            .bg(color(palette.text_primary))
                            .px_2()
                            .py(theme::ui_rem(2.0))
                            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                            .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                            .text_color(color(palette.canvas))
                            .opacity(progress)
                            .shadow(card_surface_shadows(palette))
                            .child(theme::ui_text_owned(self.label)),
                    );
                this.child(deferred(bubble).with_priority(TOOLTIP_DEFERRED_PRIORITY))
            })
    }
}

pub(in crate::app) fn frame_tooltip(
    id: impl Into<String>,
    label: impl Into<String>,
    is_visible: bool,
    child: impl IntoElement,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let anchor = div().id(format!("tooltip-{id}-anchor")).child(child);
    FrameTooltip {
        id,
        label: label.into(),
        hovered: is_visible,
        placement: TooltipPlacement::Above,
        anchor_size: super::super::SETTINGS_TAB_BUTTON_SIZE,
    }
    .render(anchor, is_visible, None, palette, window, cx)
}

impl FrameRoot {
    fn begin_tooltip_hover(&mut self, id: String, cx: &mut Context<Self>) {
        let now = Instant::now();
        let show_without_delay = self.tooltip_ui.is_warm(now);
        let epoch = self.tooltip_ui.begin_hover(id.clone());

        if show_without_delay {
            self.tooltip_ui.visible_id = Some(id);
            cx.notify();
            return;
        }

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOOLTIP_HOVER_DELAY).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |root, cx| {
                    if root.tooltip_ui.hover_epoch == epoch
                        && root.tooltip_ui.hovered_id.as_deref() == Some(id.as_str())
                    {
                        root.tooltip_ui.visible_id = Some(id);
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    fn end_tooltip_hover(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.tooltip_ui.hovered_id.as_deref() != Some(id) {
            return;
        }

        self.tooltip_ui.end_hover(Instant::now());
        cx.notify();
    }
}

impl TooltipUiState {
    fn is_warm(&self, now: Instant) -> bool {
        self.warm_until.is_some_and(|deadline| now <= deadline)
    }

    fn begin_hover(&mut self, id: String) -> u64 {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        self.hovered_id = Some(id);
        self.hover_epoch
    }

    fn end_hover(&mut self, now: Instant) {
        let was_visible = self.hovered_id == self.visible_id;
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        self.hovered_id = None;
        self.visible_id = None;
        self.warm_until = was_visible
            .then(|| now.checked_add(TOOLTIP_HYSTERESIS_WINDOW))
            .flatten();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_hysteresis_is_warm_immediately_after_leaving() {
        let now = Instant::now();
        let mut state = TooltipUiState {
            hovered_id: Some("source".to_string()),
            visible_id: Some("source".to_string()),
            ..TooltipUiState::default()
        };

        state.end_hover(now);

        assert!(state.is_warm(now));
    }

    #[test]
    fn tooltip_hysteresis_expires_after_window() {
        let now = Instant::now();
        let mut state = TooltipUiState {
            hovered_id: Some("source".to_string()),
            visible_id: Some("source".to_string()),
            ..TooltipUiState::default()
        };
        state.end_hover(now);
        let after_window = now
            .checked_add(TOOLTIP_HYSTERESIS_WINDOW + Duration::from_millis(1))
            .unwrap_or(now);

        assert!(!state.is_warm(after_window));
    }

    #[test]
    fn leaving_before_tooltip_appears_does_not_warm_hysteresis() {
        let now = Instant::now();
        let mut state = TooltipUiState::default();
        state.begin_hover("source".to_string());

        state.end_hover(now);

        assert!(!state.is_warm(now));
    }

    #[test]
    fn beginning_new_hover_invalidates_previous_delay_epoch() {
        let mut state = TooltipUiState::default();
        let first_epoch = state.begin_hover("source".to_string());

        let second_epoch = state.begin_hover("output".to_string());

        assert_ne!(first_epoch, second_epoch);
    }
}

#[cfg(test)]
#[path = "tooltip_ui_tests.rs"]
mod ui_tests;
