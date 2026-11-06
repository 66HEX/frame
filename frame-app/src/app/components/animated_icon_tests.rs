//! Exercise the shared transition through real controls. GPUI's headless scene
//! exposes quads but not SVG layers, so inspect the same keyed glyph entities
//! used by the renderer and measure the controls' native layout bounds.
use super::IconLayers;
use crate::app::components::{
    FrameIconButtonSize, FrameIconButtonVariant, frame_checkbox_indicator, frame_collapsible,
    frame_icon_button_with_motion_key,
};
use crate::app::logs::view::logs_copy_button;
use crate::app::motion::{css_transition_ease, interaction_motion};
use crate::app::root::FrameRoot;
use crate::appearance::ColorTheme;
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{
    AppContext, Context, Entity, Rgba, Subscription, TestAppContext, TransitionState,
    VisualTestContext, Window, div, px,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum Control {
    Copy,
    Checkbox,
    QueueAction,
    Disclosure,
}

impl Control {
    const fn motion_key(self) -> &'static str {
        match self {
            Self::Copy => "logs-copy",
            Self::Checkbox => "animated-checkbox",
            Self::QueueAction => "file-action-primary-test",
            Self::Disclosure => "animated-disclosure-trigger",
        }
    }
}

#[derive(Clone)]
struct GlyphSnapshot {
    path: &'static str,
    color: Option<Rgba>,
    motion: Entity<TransitionState<f32>>,
    state: TransitionState<f32>,
}

struct IconTestView {
    root: Entity<FrameRoot>,
    control: Control,
    state: u8,
    theme: ColorTheme,
    rem: f32,
    render_started: Instant,
    glyphs: Vec<GlyphSnapshot>,
    _subscription: Subscription,
}

impl gpui::Render for IconTestView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(self.rem));
        let (control, state, mode) = (self.control, self.state, self.theme);
        self.render_started = Instant::now();
        let (widget, glyphs) = self.root.update(cx, |_, cx| {
            let palette = theme::palette(mode);
            let widget = match control {
                Control::Copy => {
                    logs_copy_button("test-file", true, state != 0, palette, window, cx)
                        .into_any_element()
                }
                Control::Checkbox => frame_checkbox_indicator(
                    control.motion_key(),
                    state == 1,
                    state == 2,
                    false,
                    palette,
                    window,
                    cx,
                )
                .into_any_element(),
                Control::QueueAction => {
                    let (id, icon, label) = match state {
                        1 => ("file-pause-test", assets::ICON_PAUSE, "Pause"),
                        2 => ("file-reconvert-test", assets::ICON_REFRESH, "Reconvert"),
                        _ => ("file-resume-test", assets::ICON_PLAY, "Resume"),
                    };
                    frame_icon_button_with_motion_key(
                        id,
                        control.motion_key(),
                        icon,
                        label,
                        FrameIconButtonVariant::Ghost,
                        true,
                        FrameIconButtonSize {
                            button: 24.0,
                            icon: 14.0,
                        },
                        palette,
                        window,
                        cx,
                    )
                    .into_any_element()
                }
                Control::Disclosure => frame_collapsible(
                    "animated-disclosure",
                    "Output",
                    state != 0,
                    None,
                    |_, _, _| {},
                    palette,
                    window,
                    cx,
                )
                .into_any_element(),
            };
            (widget, glyph_snapshots(control.motion_key(), window, cx))
        });
        self.glyphs = glyphs;
        div()
            .size_full()
            .p_8()
            .flex()
            .items_start()
            .gap_4()
            .child(
                div()
                    .id("animated-test-slot")
                    .debug_selector(|| "animated-test-slot".to_owned())
                    .flex_none()
                    .flex()
                    .when(matches!(control, Control::Disclosure), |slot| {
                        slot.w(theme::ui_rem(240.0))
                    })
                    .child(widget),
            )
            .child(
                div()
                    .id("animated-test-neighbor")
                    .debug_selector(|| "animated-test-neighbor".to_owned())
                    .size(theme::ui_rem(20.0)),
            )
    }
}

fn glyph_snapshots(key: &str, window: &mut Window, cx: &mut gpui::App) -> Vec<GlyphSnapshot> {
    let layers: Entity<IconLayers> =
        window.use_keyed_state(format!("{key}-icon-layers"), cx, |_, _| {
            panic!("the real control must create its shared icon layers")
        });
    layers
        .read(cx)
        .0
        .clone()
        .into_iter()
        .map(|layer| {
            let motion: Entity<TransitionState<f32>> =
                window.use_keyed_state(format!("{key}-icon-motion-{}", layer.path), cx, |_, _| {
                    panic!("every rendered glyph must own a persistent transition")
                });
            GlyphSnapshot {
                path: layer.path,
                color: layer.color,
                state: motion.read(cx).clone(),
                motion,
            }
        })
        .collect()
}

fn setup(
    app: &mut TestAppContext,
    control: Control,
    state: u8,
    theme: ColorTheme,
    rem: f32,
) -> (Entity<IconTestView>, &mut VisualTestContext) {
    let (view, cx) = app.add_window_view(|_, cx| {
        let root = cx.new(|_| FrameRoot::new());
        let subscription = cx.observe(&root, |_, _, cx| cx.notify());
        IconTestView {
            root,
            control,
            state,
            theme,
            rem,
            render_started: Instant::now(),
            glyphs: Vec::new(),
            _subscription: subscription,
        }
    });
    draw(cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn change(view: &Entity<IconTestView>, state: u8, cx: &mut VisualTestContext) {
    view.update(cx, |view, cx| {
        view.state = state;
        cx.notify();
    });
    draw(cx);
}

fn glyph(view: &Entity<IconTestView>, path: &str, cx: &VisualTestContext) -> GlyphSnapshot {
    view.read_with(cx, |view, _| {
        view.glyphs
            .iter()
            .find(|glyph| glyph.path == path)
            .expect("persistent rendered glyph")
            .clone()
    })
}

fn geometry(
    cx: &mut VisualTestContext,
) -> (gpui::Bounds<gpui::Pixels>, gpui::Bounds<gpui::Pixels>) {
    (
        cx.debug_bounds("animated-test-slot").unwrap(),
        cx.debug_bounds("animated-test-neighbor").unwrap(),
    )
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

fn assert_settled(mut glyph: GlyphSnapshot, target: f32) {
    near(*glyph.state.value(), target);
    let sample = glyph.state.sample(Instant::now());
    near(sample.value, target);
    assert!(!sample.is_active);
}

fn pin_halfway(
    motion: &Entity<TransitionState<f32>>,
    from: f32,
    to: f32,
    cx: &mut VisualTestContext,
) {
    // Native transitions use Instant rather than the TestExecutor clock. Keep a
    // real partial frame stable even when a busy CI worker delays the next draw.
    let duration = Duration::from_hours(1);
    let started = Instant::now().checked_sub(duration / 2).unwrap();
    motion.update(cx, |state, cx| {
        let timing = gpui::Motion::new(duration).with_easing(css_transition_ease);
        *state = TransitionState::new(from, timing.clone());
        state.set(to, timing, started);
        cx.notify();
    });
}

#[test]
fn animated_icons_start_settled_and_preserve_control_geometry() {
    for mode in ColorTheme::ALL {
        for rem in [16.0, 32.0] {
            for (control, initial_icon) in [
                (Control::Copy, assets::ICON_COPY),
                (Control::QueueAction, assets::ICON_PLAY),
                (Control::Disclosure, assets::ICON_PLUS),
            ] {
                let mut app = TestAppContext::single();
                let (view, cx) = setup(&mut app, control, 0, mode, rem);
                assert_settled(glyph(&view, initial_icon, cx), 1.0);
                let before = geometry(cx);
                change(&view, 1, cx);
                assert_eq!(geometry(cx), before);
                change(&view, 0, cx);
                assert_eq!(geometry(cx), before);
            }
        }
    }
}

#[test]
fn logs_copy_retains_both_glyphs_and_green_feedback_during_crossfade() {
    assert_eq!(
        interaction_motion().forward_pass().duration(),
        Duration::from_millis(150)
    );
    for mode in ColorTheme::ALL {
        let mut app = TestAppContext::single();
        let (view, cx) = setup(&mut app, Control::Copy, 0, mode, 16.0);
        let original = glyph(&view, assets::ICON_COPY, cx).motion.entity_id();
        change(&view, 1, cx);
        let copy = glyph(&view, assets::ICON_COPY, cx);
        let check = glyph(&view, assets::ICON_CHECK, cx);
        assert_eq!(copy.motion.entity_id(), original);
        near(*copy.state.value(), 0.0);
        near(*check.state.value(), 1.0);
        assert_eq!(
            check.color,
            Some(crate::app::primitives::color(theme::palette(mode).success))
        );
        pin_halfway(&copy.motion, 1.0, 0.0, cx);
        pin_halfway(&check.motion, 0.0, 1.0, cx);
        draw(cx);
        let at = view.read_with(cx, |view, _| view.render_started);
        near(
            glyph(&view, assets::ICON_COPY, cx).state.sample(at).value,
            1.0 - css_transition_ease(0.5),
        );
        near(
            glyph(&view, assets::ICON_CHECK, cx).state.sample(at).value,
            css_transition_ease(0.5),
        );
        assert_eq!(view.read_with(cx, |view, _| view.glyphs.len()), 2);
        change(&view, 0, cx);
        assert_eq!(glyph(&view, assets::ICON_CHECK, cx).color, check.color);
    }
}

#[test]
fn changing_queue_action_ids_keep_three_glyphs_and_reverse_from_current_values() {
    let mut app = TestAppContext::single();
    let (view, cx) = setup(&mut app, Control::QueueAction, 0, ColorTheme::Dark, 16.0);
    let initial_geometry = geometry(cx);
    let play_id = glyph(&view, assets::ICON_PLAY, cx).motion.entity_id();
    change(&view, 1, cx);
    pin_halfway(&glyph(&view, assets::ICON_PLAY, cx).motion, 1.0, 0.0, cx);
    pin_halfway(&glyph(&view, assets::ICON_PAUSE, cx).motion, 0.0, 1.0, cx);
    change(&view, 2, cx);
    let at = view.read_with(cx, |view, _| view.render_started);
    near(
        glyph(&view, assets::ICON_PAUSE, cx).state.sample(at).value,
        css_transition_ease(0.5),
    );
    pin_halfway(&glyph(&view, assets::ICON_PLAY, cx).motion, 1.0, 0.0, cx);
    pin_halfway(&glyph(&view, assets::ICON_REFRESH, cx).motion, 0.0, 1.0, cx);
    change(&view, 0, cx);
    let at = view.read_with(cx, |view, _| view.render_started);
    let play = glyph(&view, assets::ICON_PLAY, cx);
    assert_eq!(play.motion.entity_id(), play_id);
    near(
        play.state.clone().sample(at).value,
        1.0 - css_transition_ease(0.5),
    );
    near(
        glyph(&view, assets::ICON_REFRESH, cx)
            .state
            .sample(at)
            .value,
        css_transition_ease(0.5),
    );
    near(*play.state.value(), 1.0);
    near(*glyph(&view, assets::ICON_REFRESH, cx).state.value(), 0.0);
    assert_eq!(view.read_with(cx, |view, _| view.glyphs.len()), 3);
    assert_eq!(geometry(cx), initial_geometry);
}

#[test]
fn checkbox_glyph_exits_in_place_and_reduced_motion_settles_every_state() {
    for mode in ColorTheme::ALL {
        let mut app = TestAppContext::single();
        let (view, cx) = setup(&mut app, Control::Checkbox, 0, mode, 16.0);
        let initial_geometry = geometry(cx);
        assert!(view.read_with(cx, |view, _| view.glyphs.is_empty()));
        change(&view, 1, cx);
        let check = glyph(&view, assets::ICON_CHECK, cx);
        pin_halfway(&check.motion, 0.0, 1.0, cx);
        change(&view, 0, cx);
        let at = view.read_with(cx, |view, _| view.render_started);
        let exiting = glyph(&view, assets::ICON_CHECK, cx);
        near(*exiting.state.value(), 0.0);
        near(
            exiting.state.clone().sample(at).value,
            css_transition_ease(0.5),
        );
        assert_eq!(exiting.motion.entity_id(), check.motion.entity_id());
        assert_eq!(
            exiting.color, None,
            "exit inherits the unchecked surface's text color"
        );
        assert_eq!(geometry(cx), initial_geometry);
        cx.update(|_, cx| cx.set_reduce_motion(true));
        for state in [0, 1, 2, 0] {
            change(&view, state, cx);
            assert_settled(
                glyph(&view, assets::ICON_CHECK, cx),
                if state == 0 { 0.0 } else { 1.0 },
            );
            assert_eq!(geometry(cx), initial_geometry);
        }
    }
}
