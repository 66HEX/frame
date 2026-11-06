//! One painted selection surface per rail, independent of trigger layout and hit targets.
use crate::app::{motion, primitives::color};
use crate::theme;
use gpui::prelude::*;
use gpui::{App, Bounds, Div, Entity, Pixels, Stateful, Transition, Window, canvas};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct IndicatorState {
    selection: Option<String>,
}

type SelectedBounds = Rc<RefCell<Option<(String, Bounds<Pixels>, f32)>>>;

pub(in crate::app) struct FrameTabIndicator {
    selected: SelectedBounds,
    state: Entity<IndicatorState>,
    motion: Transition<Bounds<Pixels>>,
}

impl FrameTabIndicator {
    pub(in crate::app) fn new(key: impl Into<String>, window: &mut Window, cx: &mut App) -> Self {
        let key = key.into();
        Self {
            selected: Rc::default(),
            state: window.use_keyed_state(format!("{key}-indicator-state"), cx, |_, _| {
                IndicatorState::default()
            }),
            motion: window.use_keyed_transition(
                format!("{key}-indicator-motion"),
                cx,
                motion::tab_indicator_motion(),
                |_, _| Bounds::default(),
            ),
        }
    }

    pub(in crate::app) fn track(
        &self,
        trigger: Stateful<Div>,
        selection: Option<String>,
    ) -> Stateful<Div> {
        self.track_with_opacity(trigger, selection, 1.0)
    }

    pub(in crate::app) fn track_with_opacity(
        &self,
        trigger: Stateful<Div>,
        selection: Option<String>,
        opacity: f32,
    ) -> Stateful<Div> {
        let selected = self.selected.clone();
        trigger.when_some(selection, |trigger, selection| {
            trigger.on_prepaint(move |info, _, _| {
                *selected.borrow_mut() = Some((selection.clone(), info.bounds, opacity));
            })
        })
    }

    /// Insert before the triggers. All trigger bounds are available after prepaint,
    /// so the decoration can paint beneath them without a measurement-frame flash.
    pub(in crate::app) fn background(
        &self,
        palette: &'static theme::ThemePalette,
    ) -> impl IntoElement + use<> {
        let selected = self.selected.clone();
        let state = self.state.clone();
        let motion = self.motion.clone();
        canvas(
            |_, _, _| (),
            move |rail, (), window, cx| {
                let Some((selection, bounds, opacity)) = selected.borrow().clone() else {
                    state.update(cx, |state, _| state.selection = None);
                    return;
                };
                // Both canvas and triggers share the rail's scroll offset. Animate
                // in that local space; scrolling must never chase a moving target.
                let target = Bounds::new(bounds.origin - rail.origin, bounds.size);
                let previous =
                    state.update(cx, |state, _| state.selection.replace(selection.clone()));
                if previous.is_none()
                    || cx.reduce_motion()
                    || (previous.as_ref() == Some(&selection) && *motion.read_goal(cx) != target)
                {
                    // Initial paint, reflow and zoom are already at their final
                    // geometry. Only an actual selection change slides the surface.
                    motion.jump_to(target, cx);
                } else {
                    motion.update(cx, |goal, _| *goal = target);
                }
                let mut painted = *motion.evaluate(window, cx);
                painted.origin += rail.origin;
                gpui::Style {
                    background: Some(color(palette.tab_active).into()),
                    corner_radii: gpui::Corners::all(theme::ui_rem(theme::RADIUS_MD))
                        .map(|radius| (*radius).into()),
                    box_shadow: theme::shadows::ShadowLevel::Xs.shadows(),
                    opacity: Some(opacity),
                    ..Default::default()
                }
                .paint(painted, window, cx, |_, _| {});
            },
        )
        .absolute()
        // Div includes absolute children in scroll extents. A zero-size paint
        // anchor cannot widen the rail or introduce artificial overflow.
        .left(theme::ui_rem(theme::metrics::TABS_PADDING))
        .top(theme::ui_rem(theme::metrics::TABS_PADDING))
        .size_0()
    }
}

#[cfg(test)]
#[path = "tab_indicator_tests.rs"]
mod tests;
