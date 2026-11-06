//! Exercise the real widgets through pointer/keyboard input and inspect painted
//! scene quads, including device scaling. Helpers alone cannot catch a builder
//! overwriting another builder's hover, focus or transition styles.
use super::{TooltipPlacement, frame_list_item};
use crate::app::accessibility::handle_tab_navigation;
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::motion::css_transition_ease;
use crate::app::preview::preview_tool_button;
use crate::app::root::FrameRoot;
use crate::appearance::ColorTheme;
use crate::theme;
use gpui::prelude::*;
use gpui::{
    AppContext, Context, Entity, FocusHandle, Modifiers, MouseButton, Quad, Rgba, Subscription,
    TestAppContext, VisualTestContext, Window, div, point, px,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Control {
    Button,
    Toggle,
    PreviewToggle,
    Input,
    Item,
}

struct MotionView {
    root: Entity<FrameRoot>,
    focus: FocusHandle,
    control: Control,
    selected: bool,
    theme: ColorTheme,
    rem: f32,
    _subscription: Subscription,
}

impl gpui::Render for MotionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let (control, selected, mode) = (self.control, self.selected, self.theme);
        window.set_rem_size(px(self.rem));
        self.root.update(cx, |root, cx| {
            let palette = theme::palette(mode);
            let widget = match control {
                Control::Button | Control::PreviewToggle => preview_tool_button(
                    "motion-control",
                    crate::assets::ICON_MINUS,
                    "Preview control",
                    matches!(control, Control::PreviewToggle).then_some(selected),
                    !root.is_processing,
                    TooltipPlacement::Above,
                    palette,
                    window,
                    cx,
                ),
                Control::Toggle => super::frame_toggle_button_base(
                    "motion-control",
                    "Toggle",
                    selected,
                    true,
                    palette,
                    window,
                    cx,
                ),
                Control::Input => frame_text_input(
                    FrameTextInputSpec {
                        id: "motion-control",
                        value: "",
                        placeholder: "Preset label",
                        disabled: false,
                        focus: None,
                        kind: FrameTextInputKind::PresetName,
                    },
                    palette,
                    window,
                    cx,
                ),
                Control::Item => frame_list_item(
                    "motion-control",
                    "Preset",
                    selected,
                    true,
                    palette,
                    window,
                    cx,
                )
                .child("Preset"),
            }
            .debug_selector(|| "motion-control".to_owned());
            div()
                .size_full()
                .p_8()
                .track_focus(&focus)
                .on_key_down(|event, window, cx| {
                    handle_tab_navigation(event, window, cx);
                })
                .child(div().w(px(240.0)).child(widget))
        })
    }
}

fn setup(
    cx: &mut TestAppContext,
    control: Control,
    theme: ColorTheme,
    rem: f32,
) -> (Entity<MotionView>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let root = cx.new(|_| FrameRoot::new());
        let subscription = cx.observe(&root, |_, _, cx| cx.notify());
        MotionView {
            root,
            focus,
            control,
            selected: false,
            theme,
            rem,
            _subscription: subscription,
        }
    });
    cx.update(|window, _| window.activate());
    draw(cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
}

fn advance(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    draw(cx);
}

fn quads(cx: &mut VisualTestContext) -> (Vec<Quad>, f32, f32) {
    let bounds = cx.debug_bounds("motion-control").unwrap();
    cx.update(|window, _| {
        let scale = window.scale_factor();
        let bounds = bounds.scale(scale);
        let quads = window
            .painted_quads()
            .into_iter()
            .filter(|q| {
                q.bounds.center() == bounds.center()
                    && q.bounds.size.width <= bounds.size.width + gpui::ScaledPixels(8.0 * scale)
            })
            .collect();
        (quads, bounds.size.width.0, scale)
    })
}

fn ring(cx: &mut VisualTestContext) -> (f32, Rgba) {
    let (quads, width, scale) = quads(cx);
    quads.iter().find(|q| q.bounds.size.width.0 > width).map_or(
        (0.0, Rgba::new(0.0, 0.0, 0.0, 0.0)),
        |q| {
            (
                q.border_widths.top.0 / scale,
                gpui::hsla_to_rgba(q.border_color.as_solid().unwrap()),
            )
        },
    )
}

fn fill(cx: &mut VisualTestContext) -> Rgba {
    quads(cx)
        .0
        .iter()
        .map(|q| gpui::hsla_to_rgba(q.background.as_solid().unwrap()))
        .max_by(|a, b| a.alpha.total_cmp(&b.alpha))
        .unwrap_or(Rgba::new(0.0, 0.0, 0.0, 0.0))
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, painted {actual}"
    );
}

#[test]
fn actual_controls_keep_mira_ring_widths_at_both_ui_scales_and_in_both_themes() {
    for theme in ColorTheme::ALL {
        for rem in [16.0, 32.0] {
            for (control, width, alpha, animated) in [
                (Control::Button, 2.0, 0.3, true),
                (Control::PreviewToggle, 2.0, 0.3, true),
                (Control::Toggle, 3.0, 0.5, true),
                (Control::Input, 2.0, 0.3, false),
                (Control::Item, 3.0, 0.5, false),
            ] {
                let mut app = TestAppContext::single();
                let (_, cx) = setup(&mut app, control, theme, rem);
                cx.simulate_keystrokes("tab");
                draw(cx);
                if !animated {
                    near(ring(cx).0, width);
                    near(ring(cx).1.alpha, alpha);
                }
                for frame in 1_u8..=10 {
                    advance(cx, 15);
                    let (painted_width, color) = ring(cx);
                    let progress = if animated {
                        css_transition_ease(f32::from(frame) / 10.0)
                    } else {
                        1.0
                    };
                    // Ring animation must retain subpixel widths, including the first frame.
                    near(painted_width, width * progress);
                    if painted_width > 0.0 {
                        near(color.alpha, alpha * progress);
                        near(color.red, theme::palette(theme).focus_ring.red);
                    }
                }
                near(ring(cx).0, width);
                cx.update(gpui::Window::blur);
                draw(cx);
                if animated {
                    advance(cx, 75);
                    assert!(ring(cx).0 > 0.0);
                    advance(cx, 75);
                }
                near(ring(cx).0, 0.0);
            }
        }
    }
}

#[test]
fn preview_hover_and_pointer_release_follow_the_same_css_transition() {
    for theme in ColorTheme::ALL {
        let mut app = TestAppContext::single();
        let (_, cx) = setup(&mut app, Control::Button, theme, 16.0);
        let bounds = cx.debug_bounds("motion-control").unwrap();
        let target = theme::palette(theme).ghost_hover;
        cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
        draw(cx);
        advance(cx, 75);
        let mid = fill(cx);
        near(mid.alpha, target.alpha * css_transition_ease(0.5));
        near(mid.red, target.red); // No fade through black or white.
        // A press during hover must not snap the background to the endpoint.
        cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::default());
        draw(cx);
        near(fill(cx).alpha, mid.alpha);
        advance(cx, 75);
        near(fill(cx).alpha, target.alpha);
        cx.simulate_mouse_move(
            point(px(2.0), px(2.0)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        draw(cx);
        advance(cx, 75);
        near(
            fill(cx).alpha,
            target.alpha * (1.0 - css_transition_ease(0.5)),
        );
        cx.simulate_mouse_up(
            point(px(2.0), px(2.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        draw(cx);
        advance(cx, 75);
        near(fill(cx).alpha, 0.0);
    }
}

#[test]
fn button_pressed_offset_still_animates_after_separating_drag_geometry() {
    let mut app = TestAppContext::single();
    let (_, cx) = setup(&mut app, Control::Button, ColorTheme::Dark, 16.0);
    let bounds = cx.debug_bounds("motion-control").unwrap();
    cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::default());
    draw(cx);
    assert_eq!(
        cx.debug_bounds("motion-control").unwrap().top(),
        bounds.top()
    );
    advance(cx, 150);
    assert_eq!(
        cx.debug_bounds("motion-control").unwrap().top(),
        bounds.top() + px(1.0)
    );
    cx.simulate_mouse_up(bounds.center(), MouseButton::Left, Modifiers::default());
    draw(cx);
    advance(cx, 150);
    assert_eq!(
        cx.debug_bounds("motion-control").unwrap().top(),
        bounds.top()
    );
}

#[test]
fn preview_toggle_selection_animates_without_changing_its_focus_contract() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app, Control::PreviewToggle, ColorTheme::Dark, 16.0);
    cx.simulate_keystrokes("tab");
    draw(cx);
    advance(cx, 150);
    for selected in [true, false] {
        view.update(cx, |view, cx| {
            view.selected = selected;
            cx.notify();
        });
        draw(cx);
        advance(cx, 75);
        assert!(fill(cx).alpha > 0.0 && fill(cx).alpha < 1.0);
        near(ring(cx).0, 2.0);
        advance(cx, 75);
        near(fill(cx).alpha, if selected { 1.0 } else { 0.0 });
        near(ring(cx).0, 2.0);
    }
}

#[test]
fn reduced_motion_settles_real_button_and_toggle_styles_immediately() {
    for control in [Control::Button, Control::PreviewToggle, Control::Toggle] {
        let mut app = TestAppContext::single();
        let (_, cx) = setup(&mut app, control, ColorTheme::Dark, 16.0);
        cx.update(|_, cx| cx.set_reduce_motion(true));
        let bounds = cx.debug_bounds("motion-control").unwrap();
        cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
        draw(cx);
        near(
            fill(cx).alpha,
            if matches!(control, Control::Toggle) {
                theme::palette(ColorTheme::Dark).fill_selected.alpha
            } else {
                theme::palette(ColorTheme::Dark).ghost_hover.alpha
            },
        );
        cx.simulate_keystrokes("tab");
        draw(cx);
        near(
            ring(cx).0,
            if matches!(control, Control::Toggle) {
                3.0
            } else {
                2.0
            },
        );
    }
}

#[test]
fn preview_button_press_moves_one_css_pixel_and_toggle_stays_in_place() {
    for rem in [16.0, 32.0] {
        for control in [Control::Button, Control::PreviewToggle, Control::Toggle] {
            let mut app = TestAppContext::single();
            let (_, cx) = setup(&mut app, control, ColorTheme::Dark, rem);
            let initial = cx.debug_bounds("motion-control").unwrap();
            cx.simulate_mouse_move(initial.center(), None, Modifiers::default());
            draw(cx);
            cx.simulate_mouse_down(initial.center(), MouseButton::Left, Modifiers::default());
            draw(cx);
            // Taffy rounds relative layout offsets to logical pixels. Check that
            // pressing does not jump immediately, then check the exact endpoint.
            advance(cx, 30);
            near(
                (cx.debug_bounds("motion-control").unwrap().top() - initial.top()).as_f32(),
                0.0,
            );
            advance(cx, 120);
            let expected = if matches!(control, Control::Toggle) {
                0.0
            } else {
                1.0
            };
            near(
                (cx.debug_bounds("motion-control").unwrap().top() - initial.top()).as_f32(),
                expected,
            );
            cx.simulate_mouse_up(initial.center(), MouseButton::Left, Modifiers::default());
            draw(cx);
            advance(cx, 150);
            near(
                (cx.debug_bounds("motion-control").unwrap().top() - initial.top()).as_f32(),
                0.0,
            );
        }
    }
}
