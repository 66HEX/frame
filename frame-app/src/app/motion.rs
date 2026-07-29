use super::{App, Duration, ElementId, Lerp, Rgba, Window, color, ease_in_out, theme};

pub(super) const SURFACE_MOTION_DURATION: Duration = Duration::from_millis(100);
pub(super) const INTERACTION_MOTION_DURATION: Duration = Duration::from_millis(75);
pub(super) const MOTION_DONE_EPSILON: f32 = 0.001;

const SETTINGS_SHEET_SLIDE_DISTANCE: f32 = 24.0;
const SETTINGS_SHEET_EDGE_INSET: f32 = 8.0;
const SUBTITLE_POPOVER_SLIDE_DISTANCE: f32 = 4.0;

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

pub(super) fn subtitle_popover_slide_offset(progress: f32) -> f32 {
    (1.0 - progress.clamp(0.0, 1.0)) * SUBTITLE_POPOVER_SLIDE_DISTANCE
}

pub(super) fn hover_motion(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Transition<f32> {
    window
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_window, _cx| 0.0_f32)
        .with_easing(ease_in_out)
}

pub(super) fn selected_motion(
    key: impl Into<ElementId>,
    selected: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let transition = window
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_window, _cx| 0.0_f32)
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
        .use_keyed_transition(key, cx, INTERACTION_MOTION_DURATION, |_window, _cx| 0.0_f32)
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

    let alpha = from.a.lerp(&to.a, progress);
    if alpha <= f32::EPSILON {
        return Rgba::default();
    }

    let premultiplied = |from_channel: f32, to_channel: f32| {
        (from_channel * from.a).lerp(&(to_channel * to.a), progress) / alpha
    };

    Rgba {
        r: premultiplied(from.r, to.r),
        g: premultiplied(from.g, to.g),
        b: premultiplied(from.b, to.b),
        a: alpha,
    }
}

pub(super) fn mix_scalar(from: f32, to: f32, progress: f32) -> f32 {
    from.lerp(&to, progress.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::ColorTheme;

    #[test]
    fn color_motion_does_not_fade_through_transparent_black_in_either_theme() {
        for color_theme in [ColorTheme::Light, ColorTheme::Dark] {
            let palette = theme::palette(color_theme);
            let midpoint = mix_color(palette.transparent, palette.fill_subtle, 0.5);

            assert!((midpoint.r - palette.fill_subtle.red).abs() <= f32::EPSILON);
            assert!((midpoint.g - palette.fill_subtle.green).abs() <= f32::EPSILON);
            assert!((midpoint.b - palette.fill_subtle.blue).abs() <= f32::EPSILON);
            let expected_alpha = palette.fill_subtle.alpha * 0.5;
            assert!((midpoint.a - expected_alpha).abs() <= f32::EPSILON);
        }
    }
}
