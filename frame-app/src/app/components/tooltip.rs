use crate::app::components::tooltip_state::TooltipUiState;
use crate::app::motion::{INTERACTION_MOTION_DURATION, motion_target, set_motion_target};
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    Context, Entity, FocusHandle, InteractiveElement, IntoElement, StatefulInteractiveElement,
    Subscription, Window, deferred, div,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(0);
const TOOLTIP_HYSTERESIS_WINDOW: Duration = Duration::from_millis(300);
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

    #[expect(
        clippy::too_many_lines,
        reason = "Keep the shared pointer, focus, dismissal and hover corridor lifecycle together; geometry lives in tooltip_position."
    )]
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
            .with_easing(crate::app::motion::css_ease);
        set_motion_target(&motion, motion_target(visible), cx);
        // Mira's zero-delay Radix tooltip enters `instant-open`, which has no
        // enter keyframe. Its `closed` state still uses the 150 ms exit animation.
        if visible {
            motion.jump_to(1.0, cx);
        }
        let progress = *motion.evaluate(window, cx);
        let anchor_bounds = Rc::new(Cell::new(gpui::Bounds::default()));
        let measured_anchor = anchor_bounds.clone();
        let content_bounds = Rc::new(Cell::new(gpui::Bounds::default()));
        let hover_anchor = anchor_bounds.clone();
        let hover_content = content_bounds.clone();
        let content_anchor = anchor_bounds.clone();
        let content_hover = content_bounds.clone();
        let content_id = self.id.clone();
        let pointer_id = self.id.clone();
        let down_id = self.id.clone();
        let click_id = self.id.clone();
        let click_focus = focus_state.clone();
        let on_pointer_move = Rc::new(cx.listener(
            move |root, event: &gpui::MouseMoveEvent, _, cx| {
                if root.tooltip_ui.visible_id.as_deref() == Some(&pointer_id)
                    && root
                        .tooltip_ui
                        .grace_area
                        .as_ref()
                        .is_some_and(|area| !area.contains(event.position))
                {
                    root.end_tooltip_hover(&pointer_id, cx);
                }
            },
        ));
        let scale = window.rem_size().as_f32() / crate::appearance::BASE_REM_PX;

        anchor
            .debug_selector(|| self.id.clone())
            .relative()
            .on_prepaint(move |info, _, _| measured_anchor.set(info.bounds))
            .on_hover(cx.listener(move |root, hovered: &bool, window, cx| {
                if let Some(state) = &hover_focus_state {
                    state.update(cx, |state, cx| {
                        state.dismissed = false;
                        cx.notify();
                    });
                }
                if !*hovered && !visible {
                    root.end_tooltip_hover(&hover_id, cx);
                } else {
                    root.change_tooltip_hover(
                        &hover_id,
                        *hovered,
                        (hover_anchor.get(), hover_content.get()),
                        window.mouse_position(),
                        cx,
                    );
                }
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |root, _, _, cx| {
                    root.end_tooltip_hover(&down_id, cx);
                }),
            )
            .on_click(cx.listener(move |root, _, _, cx| {
                if let Some(state) = &click_focus {
                    state.update(cx, |state, cx| {
                        state.dismissed = true;
                        cx.notify();
                    });
                }
                root.end_tooltip_hover(&click_id, cx);
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
            .when(
                visible || !crate::app::motion::motion_is_hidden(progress),
                |this| {
                    let bubble = div()
                        .id(format!("tooltip-{}", self.id))
                        .debug_selector(|| format!("tooltip-{}", self.id))
                        .on_hover(cx.listener(move |root, hovered: &bool, window, cx| {
                            root.change_tooltip_hover(
                                &content_id,
                                *hovered,
                                (content_hover.get(), content_anchor.get()),
                                window.mouse_position(),
                                cx,
                            );
                        }))
                        .flex()
                        .child(
                            div()
                                .debug_selector(|| "frame-tooltip-bubble".to_string())
                                .relative()
                                .flex_none()
                                .max_w(theme::ui_rem(320.0))
                                .line_height(theme::ui_rem(16.0))
                                .rounded(theme::ui_rem(theme::RADIUS_MD))
                                .bg(color(palette.tooltip_surface))
                                .px_3()
                                .py(theme::ui_rem(6.0))
                                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                                .text_color(color(palette.tooltip_foreground))
                                .opacity(progress)
                                .child(theme::ui_text_owned(self.label)),
                        );
                    let arrow = gpui::svg()
                        .path(crate::assets::TOOLTIP_ARROW)
                        .absolute()
                        .size(theme::ui_rem(10.0 * std::f32::consts::SQRT_2))
                        .text_color(color(palette.tooltip_surface))
                        .opacity(progress);
                    this.child(
                        deferred(super::tooltip_position::PositionedTooltip {
                            content: bubble.into_any_element(),
                            arrow: arrow.into_any_element(),
                            anchor: anchor_bounds,
                            content_bounds,
                            on_pointer_move,
                            placement: self.placement,
                            scale,
                            progress,
                            open: visible,
                        })
                        .priority(TOOLTIP_DEFERRED_PRIORITY),
                    )
                },
            )
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
    }
    .render(anchor, is_visible, None, palette, window, cx)
}

impl FrameRoot {
    fn change_tooltip_hover(
        &mut self,
        id: &str,
        hovered: bool,
        bounds: (gpui::Bounds<gpui::Pixels>, gpui::Bounds<gpui::Pixels>),
        pointer: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        if hovered {
            self.tooltip_ui.grace_area = None;
            self.begin_tooltip_hover(id.to_owned(), cx);
        } else if self.tooltip_ui.visible_id.as_deref() == Some(id) {
            if bounds.1.contains(&pointer) {
                self.tooltip_ui.grace_area = None;
                return;
            }
            let area = super::tooltip_grace::TooltipGraceArea::new(pointer, bounds.0, bounds.1);
            if area.contains(pointer) {
                self.tooltip_ui.grace_area = Some(area);
            } else {
                self.end_tooltip_hover(id, cx);
            }
        } else {
            self.end_tooltip_hover(id, cx);
        }
    }

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
        self.grace_area = None;
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        self.hovered_id = Some(id);
        self.hover_epoch
    }

    fn end_hover(&mut self, now: Instant) {
        self.grace_area = None;
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
