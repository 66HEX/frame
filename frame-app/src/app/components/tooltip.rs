use crate::app::components::tooltip_state::TooltipUiState;
use crate::app::motion::{INTERACTION_MOTION_DURATION, motion_target, set_motion_target};
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    StatefulInteractiveElement, Subscription, Window, deferred, div,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(400);
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
    pub(in crate::app) placement: TooltipPlacement,
    dismiss_on_click: bool,
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

// Every icon button uses one window-local hover lifecycle, including delay and
// warm switching. Callers do not have to route preview-specific state through UI.
fn tooltip_ui_state(window: &mut Window, cx: &mut App) -> Entity<TooltipUiState> {
    // An unscoped, typed element-state slot keeps virtualized rows and deferred
    // overlays on the same lifecycle as controls built in the root render pass.
    let view = window.current_view();
    window.with_element_state(
        &gpui::GlobalElementId::default(),
        |state: Option<Entity<TooltipUiState>>, _| {
            let state = state.unwrap_or_else(|| {
                let state = cx.new(|_| TooltipUiState::default());
                cx.observe(&state, move |_, cx| cx.notify(view)).detach();
                state
            });
            (state.clone(), state)
        },
    )
}

impl FrameTooltip {
    pub(in crate::app) fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            placement: TooltipPlacement::Above,
            dismiss_on_click: true,
        }
    }

    /// Confirmation controls can update their label while the tooltip stays open.
    pub(in crate::app) const fn keep_open_on_click(mut self) -> Self {
        self.dismiss_on_click = false;
        self
    }

    pub(in crate::app) const fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub(in crate::app) fn for_control(
        self,
        button: gpui::Stateful<gpui::Div>,
        focus: Option<&FocusHandle>,
        enabled: bool,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<FrameRoot>,
    ) -> gpui::Stateful<gpui::Div> {
        let hover_state = tooltip_ui_state(window, cx);
        if !enabled {
            hover_state.update(cx, |state, cx| state.end_tooltip_hover(&self.id, cx));
        }
        let focus_state =
            window.use_keyed_state(format!("tooltip-{}-focus", self.id), cx, |window, cx| {
                let focus = focus.cloned().unwrap_or_else(|| cx.focus_handle());
                TooltipFocusState::new(focus, window, cx)
            });
        let state = focus_state.read(cx);
        let visible = enabled
            && !state.dismissed
            && if window.last_input_was_keyboard() {
                state.focus.is_focused(window)
            } else {
                hover_state.read(cx).visible_id.as_deref() == Some(self.id.as_str())
            };
        let focus = state.focus.clone().tab_stop(enabled);
        let button = button.track_focus(&focus).tab_stop(enabled);
        self.render(button, visible, focus_state, palette, window, cx)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Keep the shared pointer, focus, dismissal and hover corridor lifecycle together; geometry lives in tooltip_position."
    )]
    fn render(
        self,
        anchor: gpui::Stateful<gpui::Div>,
        visible: bool,
        focus_state: Entity<TooltipFocusState>,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<FrameRoot>,
    ) -> gpui::Stateful<gpui::Div> {
        let hover_state = tooltip_ui_state(window, cx);
        let hover_id = self.id.clone();
        let dismiss_id = self.id.clone();
        let hover_focus_state = focus_state.clone();
        let hover_owner: gpui::AnyWeakEntity = focus_state.downgrade().into();
        let content_owner = hover_owner.clone();
        let motion = window
            .use_keyed_transition(
                format!("tooltip-{}-motion", self.id),
                cx,
                INTERACTION_MOTION_DURATION,
                |_window, _cx| 0.0_f32,
            )
            .with_easing(crate::app::motion::css_ease);
        set_motion_target(&motion, motion_target(visible), cx);
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
        let dismiss_on_click = self.dismiss_on_click;
        let pointer_state = hover_state.clone();
        let on_pointer_move = Rc::new(
            move |event: &gpui::MouseMoveEvent, _: &mut Window, cx: &mut App| {
                pointer_state.update(cx, |state, cx| {
                    if state.visible_id.as_deref() == Some(&pointer_id)
                        && state
                            .grace_area
                            .as_ref()
                            .is_some_and(|area| !area.contains(event.position))
                    {
                        state.end_tooltip_hover(&pointer_id, cx);
                    }
                });
            },
        );
        let down_state = hover_state.clone();
        let click_state = hover_state.clone();
        let dismiss_state = hover_state.clone();
        let content_state = hover_state.clone();
        let scale = window.rem_size().as_f32() / crate::appearance::BASE_REM_PX;

        anchor
            .debug_selector(|| self.id.clone())
            .relative()
            .on_prepaint(move |info, _, _| measured_anchor.set(info.bounds))
            .on_hover(move |hovered: &bool, window, cx| {
                hover_focus_state.update(cx, |state, cx| {
                    state.dismissed = false;
                    cx.notify();
                });
                hover_state.update(cx, |state, cx| {
                    if !*hovered && !visible {
                        state.end_tooltip_hover(&hover_id, cx);
                    } else {
                        state.change_tooltip_hover(
                            &hover_id,
                            *hovered,
                            (hover_anchor.get(), hover_content.get()),
                            window.mouse_position(),
                            &hover_owner,
                            cx,
                        );
                    }
                });
            })
            .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| {
                if dismiss_on_click {
                    down_state.update(cx, |state, cx| state.end_tooltip_hover(&down_id, cx));
                }
            })
            .on_click(move |_, _, cx| {
                if !dismiss_on_click {
                    return;
                }
                click_focus.update(cx, |state, cx| {
                    state.dismissed = true;
                    cx.notify();
                });
                click_state.update(cx, |state, cx| state.end_tooltip_hover(&click_id, cx));
            })
            .on_key_down(move |event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && visible {
                    focus_state.update(cx, |state, cx| {
                        state.dismissed = true;
                        cx.notify();
                    });
                    dismiss_state.update(cx, |state, cx| state.end_tooltip_hover(&dismiss_id, cx));
                    // Help is nonmodal: preserve Escape handling in the owning
                    // editor, popover or drawer after dismissing its tooltip.
                }
            })
            .when(
                visible || !crate::app::motion::motion_is_hidden(progress),
                |this| {
                    let bubble = div()
                        .id(format!("tooltip-{}", self.id))
                        .debug_selector(|| format!("tooltip-{}", self.id))
                        .on_hover(move |hovered: &bool, window, cx| {
                            content_state.update(cx, |state, cx| {
                                state.change_tooltip_hover(
                                    &content_id,
                                    *hovered,
                                    (content_hover.get(), content_anchor.get()),
                                    window.mouse_position(),
                                    &content_owner,
                                    cx,
                                );
                            });
                        })
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
                                .shadow(theme::shadows::ShadowLevel::Sm.shadows())
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
                        })
                        .priority(TOOLTIP_DEFERRED_PRIORITY),
                    )
                },
            )
    }
}

impl TooltipUiState {
    fn change_tooltip_hover(
        &mut self,
        id: &str,
        hovered: bool,
        bounds: (gpui::Bounds<gpui::Pixels>, gpui::Bounds<gpui::Pixels>),
        pointer: gpui::Point<gpui::Pixels>,
        owner: &gpui::AnyWeakEntity,
        cx: &mut Context<Self>,
    ) {
        if hovered {
            self.grace_area = None;
            self.begin_tooltip_hover(id.to_owned(), owner, cx);
        } else if self.visible_id.as_deref() == Some(id) {
            if bounds.1.contains(&pointer) {
                self.grace_area = None;
                return;
            }
            let area = super::tooltip_grace::TooltipGraceArea::new(pointer, bounds.0, bounds.1);
            if area.contains(pointer) {
                self.grace_area = Some(area);
            } else {
                self.end_tooltip_hover(id, cx);
            }
        } else {
            self.end_tooltip_hover(id, cx);
        }
    }

    fn begin_tooltip_hover(
        &mut self,
        id: String,
        owner: &gpui::AnyWeakEntity,
        cx: &mut Context<Self>,
    ) {
        let now = Instant::now();
        // Virtualized or changing queue actions may disappear without a leave
        // event. Their stale state must not keep unrelated tooltips warm.
        if self
            .hover_owner
            .as_ref()
            .is_some_and(|owner| !owner.is_upgradable())
        {
            self.end_hover(now);
            self.warm_until = None;
        }
        let show_without_delay = self.visible_id.is_some() || self.is_warm(now);
        let epoch = self.begin_hover(id.clone());
        self.hover_owner = Some(owner.clone());

        if show_without_delay {
            self.visible_id = Some(id);
            cx.notify();
            return;
        }

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOOLTIP_HOVER_DELAY).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |state, cx| {
                    if state.hover_epoch == epoch
                        && state.hovered_id.as_deref() == Some(id.as_str())
                        && state
                            .hover_owner
                            .as_ref()
                            .is_some_and(gpui::AnyWeakEntity::is_upgradable)
                    {
                        state.visible_id = Some(id);
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    fn end_tooltip_hover(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.hovered_id.as_deref() != Some(id) {
            return;
        }

        self.end_hover(Instant::now());
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
        self.hover_owner = None;
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
