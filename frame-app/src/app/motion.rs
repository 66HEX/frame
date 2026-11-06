use gpui::{App, ElementId, Window};
use std::time::Duration;

// Mira uses 100 ms enter/exit keyframes and 150 ms CSS transitions.
pub(super) const SURFACE_MOTION_DURATION: Duration = Duration::from_millis(100);
pub(super) const INTERACTION_MOTION_DURATION: Duration = Duration::from_millis(150);
pub(super) const MOTION_DONE_EPSILON: f32 = 0.001;

const SETTINGS_SHEET_SLIDE_DISTANCE: f32 = 384.0 * 0.1;
const SETTINGS_SHEET_EDGE_INSET: f32 = 0.0;
const POPOVER_SLIDE_DISTANCE: f32 = 8.0;

/// CSS `ease`, the default timing function of tw-animate-css enter/exit keyframes.
pub(super) fn css_ease(progress: f32) -> f32 {
    cubic_bezier(progress, 0.25, 0.1, 0.25, 1.0)
}

/// Tailwind's default transition timing, also used by Mira's sheet.
pub(super) fn css_transition_ease(progress: f32) -> f32 {
    cubic_bezier(progress, 0.4, 0.0, 0.2, 1.0)
}

fn cubic_bezier(progress: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if progress <= 0.0 || progress >= 1.0 {
        return progress.clamp(0.0, 1.0);
    }
    let sample = |t: f32, a: f32, b: f32| {
        (t * t).mul_add(
            t,
            (3.0 * (1.0 - t).powi(2) * t).mul_add(a, 3.0 * (1.0 - t) * t * t * b),
        )
    };
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..24 {
        let t = f32::midpoint(low, high);
        if sample(t, x1, x2) < progress {
            low = t;
        } else {
            high = t;
        }
    }
    sample(f32::midpoint(low, high), y1, y2)
}

pub(super) fn interaction_motion() -> gpui::Motion {
    gpui::Motion::new(INTERACTION_MOTION_DURATION).with_easing(css_transition_ease)
}

/// A quick departure and soft landing for the shared tab surface, without bounce.
pub(super) fn tab_indicator_ease(progress: f32) -> f32 {
    cubic_bezier(progress, 0.2, 0.0, 0.0, 1.0)
}

pub(super) fn tab_indicator_motion() -> gpui::Motion {
    gpui::Motion::new(Duration::from_millis(180)).with_easing(tab_indicator_ease)
}

pub(super) fn control_style_transitions(t: gpui::StyleTransitions) -> gpui::StyleTransitions {
    let motion = interaction_motion();
    t.ring(motion.clone())
        .bg(motion.clone())
        .text_color(motion.clone())
        .border_color(motion.clone())
        .opacity(motion)
}

pub(super) const fn motion_target(is_open: bool) -> f32 {
    if is_open { 1.0 } else { 0.0 }
}

pub(super) fn set_motion_target(transition: &gpui::Transition<f32>, target: f32, cx: &mut App) {
    if motion_target_changed(*transition.read_goal(cx), target) {
        transition.update(cx, |progress, cx| {
            *progress = target;
            cx.notify();
        });
    }
    if cx.reduce_motion() {
        // Preserve the notification above for event-driven hover changes too.
        transition.jump_to(target, cx);
    }
}

fn motion_target_changed(current: f32, target: f32) -> bool {
    (current - target).abs() > f32::EPSILON
}

pub(super) fn motion_is_hidden(progress: f32) -> bool {
    progress <= MOTION_DONE_EPSILON
}

pub(super) fn settings_sheet_slide_offset(progress: f32) -> f32 {
    (1.0 - progress.clamp(0.0, 1.0)) * SETTINGS_SHEET_SLIDE_DISTANCE
}

pub(super) fn settings_sheet_right_inset(progress: f32) -> f32 {
    SETTINGS_SHEET_EDGE_INSET - settings_sheet_slide_offset(progress)
}

pub(super) fn popover_slide_offset(progress: f32) -> f32 {
    -(1.0 - progress.clamp(0.0, 1.0)) * POPOVER_SLIDE_DISTANCE
}

pub(super) fn surface_motion(
    key: impl Into<ElementId>,
    is_open: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let transition = window
        .use_keyed_transition(key, cx, SURFACE_MOTION_DURATION, |_, _| 0.0_f32)
        .with_easing(css_ease);
    set_motion_target(&transition, motion_target(is_open), cx);
    *transition.evaluate(window, cx)
}

pub(super) fn sheet_motion(is_open: bool, window: &mut Window, cx: &mut App) -> f32 {
    let transition = window
        .use_keyed_transition(
            "app-settings-sheet-motion",
            cx,
            Duration::from_millis(200),
            |_, _| 0.0_f32,
        )
        .with_easing(css_transition_ease);
    set_motion_target(&transition, motion_target(is_open), cx);
    *transition.evaluate(window, cx)
}

pub(super) fn hover_motion(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Transition<f32> {
    let transition = window
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_window, _cx| 0.0_f32)
        .with_easing(css_transition_ease);
    if cx.reduce_motion() {
        let target = *transition.read_goal(cx);
        transition.jump_to(target, cx);
    }
    transition
}

pub(super) fn contextual_icon_motion(
    key: impl Into<ElementId>,
    active: bool,
    initially_active: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let transition = window
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_, _| {
            motion_target(initially_active)
        })
        .with_easing(css_transition_ease);
    set_motion_target(&transition, motion_target(active), cx);
    *transition.evaluate(window, cx)
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "Settled transitions and reduced-motion jumps must return the exact endpoint, without a residual animation tail."
)]
mod tests {
    use super::*;
    use gpui::{Context, IntoElement, Render, TestAppContext, div};

    #[derive(Default)]
    struct MotionProbe {
        open: bool,
        selected: bool,
        surface: f32,
        icon: f32,
    }

    impl Render for MotionProbe {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.surface = surface_motion("panel", self.open, window, cx);
            self.icon = contextual_icon_motion("icon", self.selected, self.selected, window, cx);
            div()
        }
    }

    #[test]
    fn timing_curves_match_css_samples_and_do_not_overshoot() {
        for (input, ease, transition) in [
            (0.25, 0.408_510_6, 0.236_587_4),
            (0.5, 0.802_403_4, 0.775_561_3),
            (0.75, 0.960_459, 0.959_367_7),
        ] {
            assert!((css_ease(input) - ease).abs() < 0.00001);
            assert!((css_transition_ease(input) - transition).abs() < 0.00001);
        }
        assert_eq!(css_ease(0.0), 0.0);
        assert_eq!(css_ease(1.0), 1.0);
        assert_eq!(css_transition_ease(0.0), 0.0);
        assert_eq!(css_transition_ease(1.0), 1.0);
    }

    #[test]
    fn surface_motion_animates_both_directions_and_can_be_reopened() {
        let mut app = TestAppContext::single();
        let (probe, cx) = app.add_window_view(|_, _| MotionProbe::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(probe.read_with(cx, |probe, _| probe.surface), 0.0);
        probe.update(cx, |probe, cx| {
            probe.open = true;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(probe.read_with(cx, |probe, _| probe.surface) < 1.0);
        std::thread::sleep(SURFACE_MOTION_DURATION);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(probe.read_with(cx, |probe, _| probe.surface), 1.0);

        probe.update(cx, |probe, cx| {
            probe.open = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(probe.read_with(cx, |probe, _| probe.surface) > 0.0);
        // Reverse an exit before completion: it must not restart at zero.
        probe.update(cx, |probe, cx| {
            probe.open = true;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(probe.read_with(cx, |probe, _| probe.surface) > 0.0);
        std::thread::sleep(SURFACE_MOTION_DURATION);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(probe.read_with(cx, |probe, _| probe.surface), 1.0);
        probe.update(cx, |probe, cx| {
            probe.open = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        std::thread::sleep(SURFACE_MOTION_DURATION);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(probe.read_with(cx, |probe, _| probe.surface), 0.0);
    }

    #[test]
    fn contextual_icons_do_not_animate_their_initial_state() {
        let mut app = TestAppContext::single();
        let (probe, cx) = app.add_window_view(|_, _| MotionProbe {
            selected: true,
            ..MotionProbe::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(probe.read_with(cx, |probe, _| probe.icon), 1.0);
    }

    #[test]
    fn reduced_motion_settles_surfaces_and_icons_without_a_tail() {
        let mut app = TestAppContext::single();
        let (probe, cx) = app.add_window_view(|_, cx| {
            cx.set_reduce_motion(true);
            MotionProbe::default()
        });
        for open in [true, false, true] {
            probe.update(cx, |probe, cx| {
                probe.open = open;
                probe.selected = open;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let expected = motion_target(open);
            assert_eq!(probe.read_with(cx, |probe, _| probe.surface), expected);
            assert_eq!(probe.read_with(cx, |probe, _| probe.icon), expected);
        }
    }
}
