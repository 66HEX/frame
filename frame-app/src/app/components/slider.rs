use crate::app::accessibility::apply_accessible_slider;
use crate::app::motion::{hover_motion, motion_target, set_motion_target};
use crate::app::primitives::color;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{InteractiveElement, StatefulInteractiveElement, div, relative};

pub(in crate::app) const FRAME_SLIDER_VISUAL_HEIGHT: f32 = super::MIN_POINTER_TARGET;
pub(in crate::app) const FRAME_SLIDER_TRACK_TOP: f32 =
    (FRAME_SLIDER_VISUAL_HEIGHT - theme::RANGE_TRACK_HEIGHT) / 2.0;
pub(in crate::app) const FRAME_SLIDER_HANDLE_WIDTH: f32 = super::MIN_POINTER_TARGET;
pub(in crate::app) const FRAME_SLIDER_HANDLE_HEIGHT: f32 = FRAME_SLIDER_VISUAL_HEIGHT;
pub(in crate::app) const FRAME_SLIDER_HANDLE_TOP: f32 = 0.0;

#[derive(Default)]
struct SliderInteraction {
    hovered: bool,
    pressed: bool,
}

/// Shared recessed rail for values, trim ranges, and download progress.
pub(in crate::app) fn frame_range_track(
    start: f32,
    end: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    range_track(start, end, false, palette)
}

pub(in crate::app) fn frame_progress_track(
    fraction: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    range_track(0.0, fraction, true, palette)
}

fn range_track(
    start: f32,
    end: f32,
    animate: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let start = start.clamp(0.0, 1.0);
    let end = end.clamp(start, 1.0);
    let radius = theme::RANGE_TRACK_HEIGHT / 2.0;

    div()
        .relative()
        .h(theme::ui_rem(theme::RANGE_TRACK_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(radius))
        .bg(color(palette.input_background))
        .child(div().absolute().top_0().bottom_0().left_0().right_0().when(
            end > start || animate,
            |this| {
                this.child(
                    div()
                        .id("range-fill")
                        .when(animate, |this| {
                            this.transitions(|t| t.right(crate::app::motion::interaction_motion()))
                        })
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(start))
                        .right(relative(1.0 - end))
                        .rounded(theme::ui_rem(radius))
                        .bg(color(palette.primary)),
                )
            },
        ))
}

/// Pointer values use the inner rail, so its visible endpoints remain 0 and 100%.
pub(in crate::app) fn frame_slider_content_bounds(
    mut bounds: gpui::Bounds<gpui::Pixels>,
    rem_size: gpui::Pixels,
) -> gpui::Bounds<gpui::Pixels> {
    let inset = theme::ui_rem(theme::metrics::SLIDER_THUMB / 2.0)
        .to_pixels(rem_size)
        .min(bounds.size.width / 2.0);
    bounds.origin.x += inset;
    bounds.size.width -= inset * 2.0;
    bounds
}

pub(in crate::app) fn frame_slider_fraction_from_bounds(
    position: gpui::Point<gpui::Pixels>,
    bounds: gpui::Bounds<gpui::Pixels>,
    rem_size: gpui::Pixels,
) -> f64 {
    let bounds = frame_slider_content_bounds(bounds, rem_size);
    let width = bounds.size.width.as_f32();
    if width <= 0.0 {
        return 0.0;
    }
    f64::from(((position.x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0))
}

#[expect(
    clippy::too_many_lines,
    reason = "Keep the slider thumb, expanded hover target and press/focus ring lifecycle together."
)]
pub(in crate::app) fn frame_slider(
    id: &'static str,
    label: impl Into<String>,
    fraction: f32,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<crate::app::root::FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let clamped_fraction = fraction.clamp(0.0, 1.0);
    let value_percent = f64::from(clamped_fraction) * 100.0;
    let focus = window
        .use_keyed_state(format!("{id}-slider-focus"), cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone()
        .tab_stop(!disabled);
    let thumb = theme::metrics::SLIDER_THUMB;
    let thumb_offset = -thumb * clamped_fraction;
    let interaction = window.use_keyed_state(format!("{id}-slider-interaction"), cx, |_, _| {
        SliderInteraction::default()
    });
    let ring = hover_motion(format!("{id}-slider-ring"), window, cx);
    let state = interaction.read(cx);
    let show_ring = !disabled
        && (state.hovered
            || state.pressed
            || (focus.is_focused(window) && window.last_input_was_keyboard()));
    set_motion_target(&ring, motion_target(show_ring), cx);
    let ring_progress = *ring.evaluate(window, cx);
    let measure_state = interaction.clone();
    let leave_state = interaction.clone();
    let press_state = interaction.clone();
    let slider = div()
        .id(id)
        .track_focus(&focus)
        .debug_selector(move || id.to_string())
        .relative()
        .h(theme::ui_rem(FRAME_SLIDER_VISUAL_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(!disabled, gpui::Styled::cursor_default)
        .on_prepaint(move |info, window, cx| {
            let pointer = window.mouse_position();
            let scale = window.rem_size().as_f32() / crate::appearance::BASE_REM_PX;
            let center = info.bounds.left()
                + gpui::px(thumb * 0.5 * scale)
                + (info.bounds.size.width - gpui::px(thumb * scale)) * clamped_fraction;
            let half_target = gpui::px((thumb / 2.0 + 8.0) * scale);
            let hovered = !disabled
                && (pointer.x - center).abs() <= half_target
                && (pointer.y - info.bounds.center().y).abs() <= half_target;
            measure_state.update(cx, |state, cx| {
                if state.hovered != hovered {
                    state.hovered = hovered;
                    cx.notify();
                }
            });
        })
        .on_mouse_move(|_, window, _| window.refresh())
        .on_hover(move |hovered, _, cx| {
            if !hovered {
                leave_state.update(cx, |state, cx| {
                    state.hovered = false;
                    cx.notify();
                });
            }
        })
        .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| {
            press_state.update(cx, |state, cx| {
                state.pressed = !disabled && state.hovered;
                cx.notify();
            });
        })
        .capture_any_mouse_up(move |_, _, cx| {
            interaction.update(cx, |state, cx| {
                if state.pressed {
                    state.pressed = false;
                    cx.notify();
                }
            });
        })
        .child(
            frame_range_track(0.0, clamped_fraction, palette)
                .absolute()
                .left_0()
                .right_0()
                .top(theme::ui_rem(FRAME_SLIDER_TRACK_TOP)),
        )
        .child(
            div()
                .id(format!("{id}-thumb"))
                .absolute()
                .left(relative(clamped_fraction))
                .ml(theme::ui_rem(thumb_offset))
                .top(theme::ui_rem((FRAME_SLIDER_VISUAL_HEIGHT - thumb) / 2.0))
                .size(theme::ui_rem(thumb))
                .rounded(theme::ui_rem(theme::RADIUS_MD))
                .border_1()
                .border_color(color(palette.transparent))
                .bg(theme::shadows::surface_fill(palette.slider_thumb, palette))
                .shadow(theme::shadows::ShadowLevel::Xs.shadows())
                .ring(gpui::px(theme::metrics::FOCUS_WIDTH * ring_progress))
                .ring_color(color(palette.focus_ring.with_alpha(
                    palette.focus_ring.alpha * theme::metrics::FOCUS_ALPHA * ring_progress,
                ))),
        );

    apply_accessible_slider(
        slider,
        label,
        !disabled,
        value_percent,
        0.0,
        100.0,
        format!("{value_percent:.0}%"),
        palette,
    )
    .focus_visible_style(|style| style.ring(gpui::px(0.0)))
}

pub(in crate::app) fn frame_slider_handle(
    id: &'static str,
    fraction: f32,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let fraction = fraction.clamp(0.0, 1.0);
    let inset_offset = (theme::metrics::SLIDER_THUMB / 2.0) * 2.0_f32.mul_add(-fraction, 1.0);
    div()
        .id(id)
        .absolute()
        .left(relative(fraction))
        .top(theme::ui_rem(FRAME_SLIDER_HANDLE_TOP))
        .ml(theme::ui_rem(
            inset_offset - FRAME_SLIDER_HANDLE_WIDTH / 2.0,
        ))
        .w(theme::ui_rem(FRAME_SLIDER_HANDLE_WIDTH))
        .h(theme::ui_rem(FRAME_SLIDER_HANDLE_HEIGHT))
        .when(enabled, gpui::Styled::cursor_default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::FrameRoot;
    use crate::app::motion::INTERACTION_MOTION_DURATION;
    use gpui::{
        AppContext, Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext,
        Window, point, px,
    };

    struct SliderProbe {
        root: Entity<FrameRoot>,
        progress: f32,
        disabled: bool,
    }

    impl Render for SliderProbe {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.root.update(cx, |_, cx| {
                let slider = frame_slider(
                    "probe",
                    "Quality",
                    0.5,
                    self.disabled,
                    theme::palette(crate::appearance::ColorTheme::Dark),
                    window,
                    cx,
                );
                self.progress = *hover_motion("probe-slider-ring", window, cx).evaluate(window, cx);
                div().p_8().w(px(264.0)).child(slider)
            })
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[test]
    fn slider_ring_animates_hover_focus_and_exit_and_respects_disabled() {
        let mut app = TestAppContext::single();
        let (probe, cx) = app.add_window_view(|_, cx| SliderProbe {
            root: cx.new(|_| FrameRoot::new()),
            progress: 0.0,
            disabled: false,
        });
        cx.update(|window, _| window.activate());
        draw(cx);
        let bounds = cx.debug_bounds("probe").unwrap();
        cx.simulate_mouse_move(bounds.center(), None, gpui::Modifiers::default());
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) < 1.0);
        std::thread::sleep(INTERACTION_MOTION_DURATION);
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) > 0.999);
        cx.simulate_mouse_move(
            point(px(500.0), px(400.0)),
            None,
            gpui::Modifiers::default(),
        );
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) > 0.0);
        std::thread::sleep(INTERACTION_MOTION_DURATION);
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) < 0.001);
        cx.simulate_keystrokes("tab");
        cx.update(Window::focus_next);
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) < 1.0);
        std::thread::sleep(INTERACTION_MOTION_DURATION);
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) > 0.999);
        cx.update(|_, cx| cx.set_reduce_motion(true));
        probe.update(cx, |probe, cx| {
            probe.disabled = true;
            cx.notify();
        });
        draw(cx);
        assert!(probe.read_with(cx, |probe, _| probe.progress) < 0.001);
    }
}
