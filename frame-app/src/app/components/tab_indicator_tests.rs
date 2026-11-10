//! Inspect the real option-tab rail and its painted shared background with a
//! controlled native transition clock, including interrupted transitions.
use crate::app::components::{FrameOption, frame_option_tabs};
use crate::app::model::ActiveView;
use crate::app::motion::{tab_indicator_ease, tab_indicator_motion};
use crate::app::root::FrameRoot;
use crate::appearance::ColorTheme;
use crate::theme;
use gpui::prelude::*;
use gpui::{
    AppContext, Bounds, Context, Entity, Lerp, Modifiers, Pixels, Quad, Subscription,
    TestAppContext, TransitionState, VisualTestContext, Window, div, px,
};
use std::time::Instant;

type IndicatorMotion = Entity<TransitionState<Bounds<Pixels>>>;

struct TabsTestView {
    root: Entity<FrameRoot>,
    selected: usize,
    enabled: bool,
    mode: ColorTheme,
    rem: f32,
    width: f32,
    motion: Option<IndicatorMotion>,
    _subscription: Subscription,
}

impl gpui::Render for TabsTestView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(self.rem));
        let (selected, enabled, mode) = (self.selected, self.enabled, self.mode);
        let (tabs, motion) = self.root.update(cx, |_, cx| {
            let choices = ["On", "Preserve original metadata", "Off"]
                .into_iter()
                .enumerate()
                .map(|(index, label)| {
                    FrameOption::new(
                        format!("indicator-option-{index}"),
                        label,
                        index == selected,
                        enabled,
                        |root, _, cx| {
                            root.active_view = ActiveView::Logs;
                            cx.notify();
                        },
                    )
                })
                .collect();
            let tabs = frame_option_tabs(
                "indicator-options",
                "Metadata",
                choices,
                theme::palette(mode),
                window,
                cx,
            );
            let motion =
                window.use_keyed_state("indicator-options-indicator-motion", cx, |_, _| {
                    panic!("the real option rail must initialize its persistent transition")
                });
            (tabs, motion)
        });
        self.motion = Some(motion);
        div()
            .size_full()
            .p_8()
            .child(div().w(px(self.width)).child(tabs))
    }
}

fn setup(
    app: &mut TestAppContext,
    mode: ColorTheme,
    rem: f32,
) -> (Entity<TabsTestView>, &mut VisualTestContext, Instant) {
    let now = Instant::now();
    app.set_transition_time(now);
    let (view, cx) = app.add_window_view(|_, cx| {
        let root = cx.new(|_| FrameRoot::new());
        let subscription = cx.observe(&root, |_, _, cx| cx.notify());
        TabsTestView {
            root,
            selected: 0,
            enabled: true,
            mode,
            rem,
            width: 600.0 * rem / 16.0,
            motion: None,
            _subscription: subscription,
        }
    });
    draw(cx);
    (view, cx, now)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn change(view: &Entity<TabsTestView>, selected: usize, cx: &mut VisualTestContext) {
    view.update(cx, |view, cx| {
        view.selected = selected;
        cx.notify();
    });
    draw(cx);
}

fn motion(view: &Entity<TabsTestView>, cx: &VisualTestContext) -> IndicatorMotion {
    view.read_with(cx, |view, _| {
        view.motion.clone().expect("indicator transition")
    })
}

fn target(motion: &IndicatorMotion, cx: &VisualTestContext) -> Bounds<Pixels> {
    motion.read_with(cx, |state, _| *state.value())
}

fn active_quad(cx: &mut VisualTestContext, mode: ColorTheme) -> Quad {
    let rail = cx.debug_bounds("indicator-options").unwrap();
    cx.update(|window, _| {
        let rail = rail.scale(window.scale_factor());
        let expected = theme::palette(mode).tab_active;
        let quads: Vec<_> = window
            .painted_quads()
            .into_iter()
            .filter(|quad| {
                rail.contains(&quad.bounds.center())
                    && quad.background.as_solid().is_some_and(|fill| {
                        let fill = gpui::hsla_to_rgba(fill);
                        (fill.red - expected.red).abs() < 0.001
                            && (fill.green - expected.green).abs() < 0.001
                            && (fill.blue - expected.blue).abs() < 0.001
                            && (fill.alpha - expected.alpha).abs() < 0.001
                    })
            })
            .collect();
        assert_eq!(
            quads.len(),
            1,
            "one opaque background, not two fading triggers"
        );
        quads.into_iter().next().unwrap()
    })
}

fn bounds(cx: &mut VisualTestContext) -> [Bounds<Pixels>; 3] {
    [
        "indicator-option-0",
        "indicator-option-1",
        "indicator-option-2",
    ]
    .map(|id| cx.debug_bounds(id).unwrap())
}

fn assert_quad_matches(cx: &mut VisualTestContext, mode: ColorTheme, expected: Bounds<Pixels>) {
    let quad = active_quad(cx, mode);
    cx.update(|window, _| assert_eq!(quad.bounds, expected.scale(window.scale_factor())));
}

fn assert_settled(motion: &IndicatorMotion, now: Instant, cx: &VisualTestContext) {
    let mut state = motion.read_with(cx, |state, _| state.clone());
    let expected = *state.value();
    let sample = state.sample(now);
    assert_eq!(sample.value, expected);
    assert!(
        !sample.is_active,
        "initial and reduced-motion frames must be settled"
    );
}

fn pin_halfway(
    motion: &IndicatorMotion,
    from: Bounds<Pixels>,
    to: Bounds<Pixels>,
    now: Instant,
    cx: &mut VisualTestContext,
) {
    let timing = tab_indicator_motion();
    let started = now
        .checked_sub(timing.forward_pass().duration() / 2)
        .unwrap();
    motion.update(cx, |state, cx| {
        *state = TransitionState::new(from, timing.clone());
        state.set(to, timing, started);
        cx.notify();
    });
}

fn near_bounds(actual: Bounds<Pixels>, expected: Bounds<Pixels>) {
    for (actual, expected) in [
        (actual.origin.x, expected.origin.x),
        (actual.origin.y, expected.origin.y),
        (actual.size.width, expected.size.width),
        (actual.size.height, expected.size.height),
    ] {
        assert!(
            (actual - expected).abs() < px(0.05),
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn option_tab_indicator_starts_settled_and_tracks_unequal_trigger_widths() {
    for mode in ColorTheme::ALL {
        for rem in [16.0, 32.0] {
            let mut app = TestAppContext::single();
            let (view, cx, now) = setup(&mut app, mode, rem);
            let geometry = bounds(cx);
            assert!(geometry[1].size.width > geometry[0].size.width);
            let transition = motion(&view, cx);
            assert_settled(&transition, now, cx);
            assert_quad_matches(cx, mode, geometry[0]);
            cx.update(|_, cx| cx.set_reduce_motion(true));
            for selected in [1, 2, 0] {
                change(&view, selected, cx);
                assert_eq!(
                    bounds(cx),
                    geometry,
                    "the moving background cannot reflow labels"
                );
                assert_quad_matches(cx, mode, geometry[selected]);
                assert_settled(&transition, now, cx);
            }
        }
    }
}

#[test]
fn option_tab_indicator_moves_and_resizes_as_one_surface() {
    for mode in ColorTheme::ALL {
        let mut app = TestAppContext::single();
        let (view, cx, now) = setup(&mut app, mode, 16.0);
        let geometry = bounds(cx);
        let transition = motion(&view, cx);
        let from = target(&transition, cx);
        change(&view, 1, cx);
        let to = target(&transition, cx);
        assert_ne!(from.size.width, to.size.width);
        pin_halfway(&transition, from, to, now, cx);
        draw(cx);
        let quad = active_quad(cx, mode);
        let halfway = geometry[0].lerp(&geometry[1], tab_indicator_ease(0.5));
        cx.update(|window, _| {
            let actual = quad.bounds;
            let expected = halfway.scale(window.scale_factor());
            // GPUI snaps each painted edge to a device pixel independently.
            // Inspect both edges (rather than width, whose rounding errors add).
            for (edge, actual, expected) in [
                ("left", actual.left().0, expected.left().0),
                ("right", actual.right().0, expected.right().0),
            ] {
                assert!(
                    (actual - expected).abs() <= 0.51,
                    "{edge} edge: painted {actual}, expected {expected} before pixel snapping"
                );
            }
        });
        assert_eq!(bounds(cx), geometry);
    }
}

#[test]
fn option_tab_indicator_retargets_from_its_visible_position() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup(&mut app, ColorTheme::Dark, 16.0);
    let transition = motion(&view, cx);
    let original_entity = transition.entity_id();
    let first = target(&transition, cx);
    change(&view, 1, cx);
    let second = target(&transition, cx);
    pin_halfway(&transition, first, second, now, cx);
    draw(cx);
    change(&view, 2, cx);
    let mut state = transition.read_with(cx, |state, _| state.clone());
    near_bounds(
        state.sample(now).value,
        first.lerp(&second, tab_indicator_ease(0.5)),
    );
    assert_eq!(motion(&view, cx).entity_id(), original_entity);
    let third = target(&transition, cx);
    pin_halfway(&transition, second, third, now, cx);
    draw(cx);
    change(&view, 0, cx);
    let mut state = transition.read_with(cx, |state, _| state.clone());
    near_bounds(
        state.sample(now).value,
        second.lerp(&third, tab_indicator_ease(0.5)),
    );
    assert_eq!(*state.value(), first);
    assert_eq!(motion(&view, cx).entity_id(), original_entity);
}

#[test]
fn option_tab_indicator_snaps_to_resized_geometry_without_animating_layout() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup(&mut app, ColorTheme::Dark, 16.0);
    let transition = motion(&view, cx);
    view.update(cx, |view, cx| {
        view.width = 780.0;
        view.rem = 24.0;
        cx.notify();
    });
    draw(cx);
    let selected = bounds(cx)[0];
    assert_quad_matches(cx, ColorTheme::Dark, selected);
    assert_settled(&transition, now, cx);
}

#[test]
fn disabled_option_tabs_keep_the_selected_surface_and_ignore_activation() {
    for mode in ColorTheme::ALL {
        let mut app = TestAppContext::single();
        let (view, cx, _) = setup(&mut app, mode, 16.0);
        let geometry = bounds(cx);
        let root = view.read_with(cx, |view, _| view.root.clone());
        cx.simulate_click(geometry[0].center(), Modifiers::default());
        draw(cx);
        root.update(cx, |root, cx| {
            root.active_view = ActiveView::Workspace;
            cx.notify();
        });
        let initial_focus = cx.update(|window, cx| window.focused(cx));
        assert!(initial_focus.is_some());
        view.update(cx, |view, cx| {
            view.enabled = false;
            cx.notify();
        });
        draw(cx);
        // Opacity is applied to a composited group, not to individual scene
        // quads. Here verify the actual selected surface survives disabling;
        // group opacity itself is not exposed by painted_quads.
        assert_quad_matches(cx, mode, geometry[0]);
        assert_eq!(bounds(cx), geometry);
        cx.simulate_click(geometry[0].center(), Modifiers::default());
        draw(cx);
        assert_eq!(
            root.read_with(cx, |root, _| root.active_view),
            ActiveView::Workspace
        );
        assert_eq!(cx.update(|window, cx| window.focused(cx)), initial_focus);
        assert_quad_matches(cx, mode, geometry[0]);
        view.update(cx, |view, cx| {
            view.enabled = true;
            cx.notify();
        });
        draw(cx);
        assert_quad_matches(cx, mode, geometry[0]);
        cx.simulate_click(geometry[0].center(), Modifiers::default());
        draw(cx);
        assert_eq!(
            root.read_with(cx, |root, _| root.active_view),
            ActiveView::Logs
        );
    }
}
