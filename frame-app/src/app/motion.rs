use super::{App, Duration, ElementId, Lerp, Rgba, Window, color, ease_in_out, theme};

// Panels, selects and color pickers share one enter/exit timing. Hover and
// selection feedback stays shorter because it happens on every interaction.
pub(super) const SURFACE_MOTION_DURATION: Duration = Duration::from_millis(120);
pub(super) const INTERACTION_MOTION_DURATION: Duration = Duration::from_millis(75);
pub(super) const MOTION_DONE_EPSILON: f32 = 0.001;

const SETTINGS_SHEET_SLIDE_DISTANCE: f32 = 24.0;
const SETTINGS_SHEET_EDGE_INSET: f32 = 8.0;
const POPOVER_SLIDE_DISTANCE: f32 = 4.0;

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

pub(super) fn retarget_hover_motion(
    transition: &gpui::Transition<f32>,
    is_hovered: bool,
    cx: &mut App,
) {
    set_motion_target(transition, motion_target(is_hovered), cx);
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
    (1.0 - progress.clamp(0.0, 1.0)) * POPOVER_SLIDE_DISTANCE
}

pub(super) fn surface_motion(
    key: impl Into<ElementId>,
    is_open: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let transition = window
        .use_keyed_transition(key, cx, SURFACE_MOTION_DURATION, |_, _| 0.0_f32)
        .with_easing(gpui::ease_out_quint());
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
        .with_easing(ease_in_out);
    if cx.reduce_motion() {
        let target = *transition.read_goal(cx);
        transition.jump_to(target, cx);
    }
    transition
}

pub(super) fn selected_motion(
    key: impl Into<ElementId>,
    selected: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let transition = window
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_, _| {
            motion_target(selected)
        })
        .with_easing(ease_in_out);
    set_motion_target(&transition, motion_target(selected), cx);
    *transition.evaluate(window, cx)
}

pub(super) fn contextual_icon_motion(
    key: impl Into<ElementId>,
    active: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let transition = window
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_, _| {
            motion_target(active)
        })
        .with_easing(ease_in_out);
    set_motion_target(&transition, motion_target(active), cx);
    *transition.evaluate(window, cx)
}

pub(super) fn mix_color(from: theme::RgbaToken, to: theme::RgbaToken, progress: f32) -> Rgba {
    mix_rgba(color(from), color(to), progress)
}

pub(super) fn mix_rgba(from: Rgba, to: Rgba, progress: f32) -> Rgba {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return from;
    }
    if progress >= 1.0 {
        return to;
    }

    let alpha = from.alpha.lerp(&to.alpha, progress);
    if alpha <= f32::EPSILON {
        return Rgba::default();
    }

    let premultiplied = |from_channel: f32, to_channel: f32| {
        (from_channel * from.alpha).lerp(&(to_channel * to.alpha), progress) / alpha
    };

    Rgba::new(
        premultiplied(from.red, to.red),
        premultiplied(from.green, to.green),
        premultiplied(from.blue, to.blue),
        alpha,
    )
}

pub(super) fn mix_scalar(from: f32, to: f32, progress: f32) -> f32 {
    from.lerp(&to, progress.clamp(0.0, 1.0))
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "Settled transitions and reduced-motion jumps must return the exact endpoint, without a residual animation tail."
)]
mod tests {
    use super::*;
    use crate::appearance::ColorTheme;
    use gpui::{Context, IntoElement, Render, TestAppContext, div};

    #[derive(Default)]
    struct MotionProbe {
        open: bool,
        selected: bool,
        surface: f32,
        selection: f32,
        icon: f32,
    }

    impl Render for MotionProbe {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.surface = surface_motion("panel", self.open, window, cx);
            self.selection = selected_motion("selection", self.selected, window, cx);
            self.icon = contextual_icon_motion("icon", self.selected, window, cx);
            div()
        }
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
    fn selected_controls_do_not_animate_their_initial_state() {
        let mut app = TestAppContext::single();
        let (probe, cx) = app.add_window_view(|_, _| MotionProbe {
            selected: true,
            ..MotionProbe::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(probe.read_with(cx, |probe, _| probe.selection), 1.0);
        assert_eq!(probe.read_with(cx, |probe, _| probe.icon), 1.0);
    }

    #[test]
    fn reduced_motion_settles_surfaces_and_selection_without_a_tail() {
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
            assert_eq!(probe.read_with(cx, |probe, _| probe.selection), expected);
            assert_eq!(probe.read_with(cx, |probe, _| probe.icon), expected);
        }
    }

    #[test]
    fn color_motion_does_not_fade_through_transparent_black_in_either_theme() {
        for color_theme in [ColorTheme::Light, ColorTheme::Dark] {
            let palette = theme::palette(color_theme);
            let midpoint = mix_color(palette.transparent, palette.fill_subtle, 0.5);

            assert!((midpoint.red - palette.fill_subtle.red).abs() <= f32::EPSILON);
            assert!((midpoint.green - palette.fill_subtle.green).abs() <= f32::EPSILON);
            assert!((midpoint.blue - palette.fill_subtle.blue).abs() <= f32::EPSILON);
            let expected_alpha = palette.fill_subtle.alpha * 0.5;
            assert!((midpoint.alpha - expected_alpha).abs() <= f32::EPSILON);
        }
    }
}
