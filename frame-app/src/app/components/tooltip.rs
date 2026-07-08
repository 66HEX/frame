use super::super::primitives::card_surface_shadows;
use super::{
    Context, Duration, FluentBuilder, FrameRoot, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TooltipUiState, Window, color, deferred, div,
    ease_out_quint, motion_target, px, set_motion_target, theme,
};
use std::time::Instant;

const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(500);
const TOOLTIP_HYSTERESIS_WINDOW: Duration = Duration::from_millis(300);
const TOOLTIP_MOTION_DURATION: Duration = Duration::from_millis(140);
const TOOLTIP_OFFSET: f32 = 6.0;
const TOOLTIP_ENTER_DISTANCE: f32 = 4.0;
const TOOLTIP_DEFERRED_PRIORITY: usize = 20;

pub(in crate::app) fn frame_tooltip(
    id: impl Into<String>,
    label: impl Into<String>,
    is_visible: bool,
    child: impl IntoElement,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let hover_id = id.clone();
    let motion = window
        .use_keyed_transition(
            format!("tooltip-{id}-motion"),
            cx,
            TOOLTIP_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_out_quint());
    set_motion_target(&motion, motion_target(is_visible), cx);
    let progress = *motion.evaluate(window, cx);

    div()
        .id(format!("tooltip-{id}-anchor"))
        .relative()
        .on_hover(cx.listener(move |root, hovered: &bool, _window, cx| {
            if *hovered {
                root.begin_tooltip_hover(hover_id.clone(), cx);
            } else {
                root.end_tooltip_hover(&hover_id, cx);
            }
        }))
        .child(child)
        .when(is_visible, |this| {
            this.child(
                deferred(
                    div()
                        .id(format!("tooltip-{id}"))
                        .absolute()
                        .bottom(px((progress - 1.0).mul_add(
                            TOOLTIP_ENTER_DISTANCE,
                            super::super::SETTINGS_TAB_BUTTON_SIZE + TOOLTIP_OFFSET,
                        )))
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .rounded(px(theme::RADIUS_SM))
                                .bg(color(theme::DROPDOWN))
                                .px_2()
                                .py(px(2.0))
                                .text_size(px(theme::TEXT_LABEL_SIZE))
                                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                                .text_color(color(theme::FOREGROUND))
                                .opacity(progress)
                                .shadow(card_surface_shadows())
                                .child(theme::ui_text_owned(label)),
                        ),
                )
                .with_priority(TOOLTIP_DEFERRED_PRIORITY),
            )
        })
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
