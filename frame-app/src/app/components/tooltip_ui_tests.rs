use super::*;
use crate::VisualFixture;
use crate::app::FrameRoot;
use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext, point, px};
use std::cell::RefCell;

struct TooltipTestView {
    root: Entity<FrameRoot>,
    focus: FocusHandle,
    edge_placement: Option<TooltipPlacement>,
    rotate_motion: Rc<RefCell<Option<Entity<gpui::TransitionState<f32>>>>>,
    _subscription: Subscription,
}

impl gpui::Render for TooltipTestView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let edge_placement = self.edge_placement;
        let rotate_motion = self.rotate_motion.clone();
        self.root.update(cx, |root, cx| {
            window.set_rem_size(px(
                crate::appearance::BASE_REM_PX * root.appearance.ui_scale.factor()
            ));
            if edge_placement.is_some() {
                tooltip_ui_state(window, cx).update(cx, |state, _| {
                    state.visible_id = Some("preview-tool-rotate".into());
                });
            }
            let enabled = !root.is_processing;
            let palette = crate::theme::palette(crate::appearance::ColorTheme::Dark);
            let mut controls = if edge_placement.is_some() {
                div().absolute().top_0().right_0()
            } else {
                div().flex().flex_col().gap_8().px_32().py_8()
            };
            for (id, label, placement) in [
                (
                    "preview-tool-rotate",
                    "Rotate preview",
                    TooltipPlacement::Right,
                ),
                (
                    "preview-tool-flip-horizontal",
                    "Flip horizontally",
                    TooltipPlacement::Right,
                ),
                (
                    "preview-tool-flip-vertical",
                    "Flip vertically",
                    TooltipPlacement::Right,
                ),
                ("preview-tool-crop", "Crop", TooltipPlacement::Right),
                (
                    "preview-tool-overlay",
                    "Overlay image",
                    TooltipPlacement::Right,
                ),
                ("preview-zoom-out", "Zoom out", TooltipPlacement::Above),
                ("preview-zoom-in", "Zoom in", TooltipPlacement::Above),
                (
                    "preview-playback-toggle",
                    "Play preview",
                    TooltipPlacement::Above,
                ),
            ] {
                if edge_placement.is_some() && id != "preview-tool-rotate" {
                    continue;
                }
                let button = crate::app::preview::preview_tool_button(
                    id,
                    crate::assets::ICON_PLUS,
                    label,
                    if id.starts_with("preview-tool-") && id != "preview-tool-rotate" {
                        Some(
                            id == "preview-tool-flip-horizontal"
                                && root
                                    .file_queue
                                    .selected_file()
                                    .unwrap()
                                    .config
                                    .flip_horizontal,
                        )
                    } else {
                        None
                    },
                    enabled,
                    edge_placement.unwrap_or(placement),
                    palette,
                    window,
                    cx,
                )
                .on_click(cx.listener(move |root, _: &gpui::ClickEvent, _, cx| {
                    if id == "preview-tool-flip-horizontal" {
                        root.toggle_selected_flip(crate::app::FlipAxis::Horizontal);
                        cx.notify();
                    }
                }));
                if id == "preview-tool-rotate" {
                    *rotate_motion.borrow_mut() = Some(window.use_keyed_state(
                        format!("tooltip-{id}-motion"),
                        cx,
                        |_, _| gpui::TransitionState::new(0.0, INTERACTION_MOTION_DURATION),
                    ));
                }
                controls = controls.child(div().flex().child(button));
            }
            div()
                .size_full()
                .child(controls)
                .track_focus(&focus)
                .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                    crate::app::accessibility::handle_tab_navigation(event, window, cx);
                }))
        })
    }
}

fn preview_window(cx: &mut TestAppContext) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (view, cx) = preview_view(cx);
    let root = view.read_with(cx, |view, _| view.root.clone());
    (root, cx)
}

fn preview_view(cx: &mut TestAppContext) -> (Entity<TooltipTestView>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let root = cx.new(|_| {
            let mut root = FrameRoot::new();
            root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
            root
        });
        let subscription = cx.observe(&root, |_, _, cx| cx.notify());
        TooltipTestView {
            root,
            focus,
            edge_placement: None,
            rotate_motion: Rc::default(),
            _subscription: subscription,
        }
    });
    cx.update(|window, cx| {
        window.activate();
        cx.set_reduce_motion(true);
    });
    cx.run_until_parked();
    (view, cx)
}

#[test]
fn tooltip_flips_at_window_edges_and_keeps_the_bubble_inside_the_viewport() {
    for placement in [TooltipPlacement::Above, TooltipPlacement::Right] {
        for scale in [
            crate::appearance::ScalePreset::Percent100,
            crate::appearance::ScalePreset::Percent200,
        ] {
            let mut app = TestAppContext::single();
            let (_, cx) = app.add_window_view(|window, cx| {
                let focus = cx.focus_handle();
                focus.focus(window, cx);
                let root = cx.new(|_| {
                    let mut root = FrameRoot::new();
                    root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
                    root.appearance.ui_scale = scale;
                    root
                });
                let subscription = cx.observe(&root, |_, _, cx| cx.notify());
                TooltipTestView {
                    root,
                    focus,
                    edge_placement: Some(placement),
                    rotate_motion: Rc::default(),
                    _subscription: subscription,
                }
            });
            cx.update(|window, cx| {
                window.activate();
                cx.set_reduce_motion(true);
            });
            draw(cx);
            let button = cx.debug_bounds("preview-tool-rotate").unwrap();
            let bubble = cx.debug_bounds("frame-tooltip-bubble").unwrap();
            let viewport = cx.update(|window, _| window.viewport_size());
            assert!(bubble.left() >= px(0.0) && bubble.right() <= viewport.width);
            assert!(bubble.top() >= px(0.0) && bubble.bottom() <= viewport.height);
            match placement {
                TooltipPlacement::Above => assert!(bubble.top() > button.bottom()),
                TooltipPlacement::Right => assert!(
                    bubble.right() < button.left(),
                    "button={button:?}, bubble={bubble:?}, viewport={viewport:?}"
                ),
            }
        }
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
}

fn tab_to(cx: &mut VisualTestContext, tooltip_id: &'static str) {
    for _ in 0..40 {
        cx.simulate_keystrokes("tab");
        draw(cx);
        if cx.debug_bounds(tooltip_id).is_some() {
            return;
        }
    }
    panic!("tooltip was not reachable by Tab: {tooltip_id}");
}

#[test]
fn tooltip_keyboard_focus_escape_and_reentry_preserve_button_activation() {
    let mut app = TestAppContext::single();
    let (root, cx) = preview_window(&mut app);
    draw(cx);
    tab_to(cx, "tooltip-preview-tool-flip-horizontal");
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_some());

    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    assert!(!root.read_with(cx, |root, _| {
        root.file_queue
            .selected_file()
            .unwrap()
            .config
            .flip_horizontal
    }));

    press(cx, "enter");
    draw(cx);
    assert!(root.read_with(cx, |root, _| {
        root.file_queue
            .selected_file()
            .unwrap()
            .config
            .flip_horizontal
    }));
    press(cx, "space");
    draw(cx);
    assert!(!root.read_with(cx, |root, _| {
        root.file_queue
            .selected_file()
            .unwrap()
            .config
            .flip_horizontal
    }));

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_some());
}

#[test]
fn tooltip_zoom_and_playback_are_centered_over_their_controls() {
    let mut app = TestAppContext::single();
    let (root, cx) = preview_window(&mut app);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    draw(cx);
    for scale in [
        crate::appearance::ScalePreset::Percent100,
        crate::appearance::ScalePreset::Percent200,
    ] {
        root.update(cx, |root, cx| {
            root.appearance.ui_scale = scale;
            cx.notify();
        });
        draw(cx);
        for (id, tooltip_id) in [
            ("preview-zoom-out", "tooltip-preview-zoom-out"),
            ("preview-zoom-in", "tooltip-preview-zoom-in"),
            ("preview-playback-toggle", "tooltip-preview-playback-toggle"),
        ] {
            tab_to(cx, tooltip_id);
            let button = cx.debug_bounds(id).expect("button bounds");
            let tooltip = cx
                .debug_bounds("frame-tooltip-bubble")
                .expect("tooltip bounds");
            assert!(
                (button.center().x - tooltip.center().x).abs() <= px(1.0),
                "{id}: {button:?} / {tooltip:?}"
            );
            assert!(tooltip.bottom() < button.top());
        }
    }
}

#[test]
fn tooltip_switching_from_hover_to_keyboard_does_not_leave_two_bubbles() {
    let mut app = TestAppContext::single();
    let (_, cx) = preview_window(&mut app);
    draw(cx);
    let rotate = cx.debug_bounds("preview-tool-rotate").unwrap();
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    cx.run_until_parked();
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_some());
    tab_to(cx, "tooltip-preview-tool-flip-horizontal");
    let button = cx.debug_bounds("preview-tool-flip-horizontal").unwrap();
    let tooltip = cx.debug_bounds("frame-tooltip-bubble").unwrap();
    assert!((button.center().y - tooltip.center().y).abs() <= px(1.0));
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_none());
    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    cx.simulate_click(point(px(600.0), px(50.0)), Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}

#[test]
fn tooltip_hover_delay_cancels_early_passes_and_clicks_but_keeps_warm_switching() {
    let mut app = TestAppContext::single();
    let (_, cx) = preview_window(&mut app);
    draw(cx);
    let rotate = cx.debug_bounds("preview-tool-rotate").unwrap();
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY / 2);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());

    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY / 2);
    cx.simulate_click(rotate.center(), Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());

    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(
        TOOLTIP_HOVER_DELAY
            .checked_sub(Duration::from_millis(1))
            .unwrap(),
    );
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.executor().advance_clock(Duration::from_millis(1));
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_some());

    let flip = cx.debug_bounds("preview-tool-flip-horizontal").unwrap();
    cx.simulate_mouse_move(flip.center(), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_none());
    assert!(
        cx.debug_bounds("tooltip-preview-tool-flip-horizontal")
            .is_some()
    );
    // A tooltip that remains open is still warm after the last-close deadline.
    // Move backwards through paint order to exercise enter-before-leave callbacks.
    cx.executor()
        .advance_clock(TOOLTIP_HYSTERESIS_WINDOW + Duration::from_millis(1));
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_some());
    assert!(
        cx.debug_bounds("tooltip-preview-tool-flip-horizontal")
            .is_none()
    );
}

fn pin_tooltip_mid_transition(
    motion: &Entity<gpui::TransitionState<f32>>,
    from: f32,
    to: f32,
    cx: &mut VisualTestContext,
) {
    // GPUI Transition uses std::time::Instant, not the executor clock. Sampling
    // halfway through a long run makes this frame independent of test scheduling.
    let duration = Duration::from_hours(1);
    let started = Instant::now().checked_sub(duration / 2).unwrap();
    let timing = gpui::Motion::new(duration).with_easing(crate::app::motion::css_ease);
    motion.update(cx, |state, cx| {
        *state = gpui::TransitionState::new(from, timing.clone());
        state.set(to, timing, started);
        cx.notify();
    });
    draw(cx);
}

fn painted_tooltip_bounds(cx: &mut VisualTestContext) -> gpui::Bounds<gpui::ScaledPixels> {
    let bounds = cx
        .debug_bounds("frame-tooltip-bubble")
        .expect("tooltip bubble");
    cx.update(|window, _| {
        let bounds = bounds.scale(window.scale_factor());
        // debug_bounds records layout before the .95–1 paint transform. Find the
        // bubble within that slot, then measure the geometry actually submitted.
        window
            .painted_quads()
            .into_iter()
            .find(|quad| {
                (quad.bounds.center().x.0 - bounds.center().x.0).abs() < bounds.size.width.0 * 0.05
                    && (quad.bounds.center().y.0 - bounds.center().y.0).abs()
                        < bounds.size.height.0 * 0.05
                    && quad.bounds.size.width.0 >= bounds.size.width.0 * 0.94
                    && quad.bounds.size.width.0 <= bounds.size.width.0 * 1.01
                    && quad.bounds.size.height.0 >= bounds.size.height.0 * 0.94
                    && quad.bounds.size.height.0 <= bounds.size.height.0 * 1.01
            })
            .expect("painted tooltip bubble")
            .bounds
    })
}

#[test]
fn tooltip_paints_partial_entry_and_exit_instead_of_jumping_to_visible() {
    let mut app = TestAppContext::single();
    let (view, cx) = preview_view(&mut app);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    draw(cx);
    let rotate = cx.debug_bounds("preview-tool-rotate").unwrap();
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    let motion = view.read_with(cx, |view, _| {
        view.rotate_motion.borrow().as_ref().unwrap().clone()
    });
    assert!(motion.read_with(cx, |state, _| (*state.value() - 1.0).abs() < f32::EPSILON));

    pin_tooltip_mid_transition(&motion, 0.0, 1.0, cx);
    let entering = painted_tooltip_bounds(cx);
    motion.update(cx, |state, cx| {
        state.jump_to(1.0);
        cx.notify();
    });
    draw(cx);
    let settled = painted_tooltip_bounds(cx);
    assert!(entering.size.width < settled.size.width && entering.left() < settled.left());
    assert!(entering.size.width.0 > settled.size.width.0 * 0.95);

    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    draw(cx);
    assert!(motion.read_with(cx, |state, _| state.value().abs() < f32::EPSILON));
    pin_tooltip_mid_transition(&motion, 1.0, 0.0, cx);
    let exiting = painted_tooltip_bounds(cx);
    assert!(exiting.size.width < settled.size.width && exiting.left() < settled.left());
    assert!(exiting.size.width.0 > settled.size.width.0 * 0.95);
    motion.update(cx, |state, cx| {
        state.jump_to(0.0);
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}

#[test]
fn tooltip_stays_hidden_if_its_control_is_disabled_during_the_hover_delay() {
    let mut app = TestAppContext::single();
    let (root, cx) = preview_window(&mut app);
    draw(cx);
    let rotate = cx.debug_bounds("preview-tool-rotate").unwrap();
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY / 2);
    root.update(cx, |root, cx| {
        root.is_processing = true;
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_none());
    root.update(cx, |root, cx| {
        root.is_processing = false;
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_none());
}

#[test]
fn disabled_mira_buttons_do_not_activate_or_open_tooltips() {
    let mut app = TestAppContext::single();
    let (root, cx) = preview_window(&mut app);
    root.update(cx, |root, cx| {
        root.is_processing = true;
        cx.notify();
    });
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    let flip = cx.debug_bounds("preview-tool-flip-horizontal").unwrap();
    cx.simulate_mouse_move(flip.center(), None, Modifiers::default());
    cx.simulate_click(flip.center(), Modifiers::default());
    cx.simulate_mouse_move(flip.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    assert!(
        cx.debug_bounds("tooltip-preview-tool-flip-horizontal")
            .is_none()
    );
    assert!(!root.read_with(cx, |root, _| {
        root.file_queue
            .selected_file()
            .unwrap()
            .config
            .flip_horizontal
    }));
}

#[test]
fn tooltip_content_remains_hoverable_across_the_gap_and_click_dismisses_it() {
    let mut app = TestAppContext::single();
    let (_, cx) = preview_window(&mut app);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    draw(cx);
    let button = cx.debug_bounds("preview-tool-rotate").unwrap();
    cx.simulate_mouse_move(button.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    let bubble = cx.debug_bounds("frame-tooltip-bubble").unwrap();
    let gap = point((button.right() + bubble.left()) / 2.0, button.center().y);
    for position in [gap, bubble.center(), gap, button.center()] {
        cx.simulate_mouse_move(position, None, Modifiers::default());
        draw(cx);
        assert!(
            cx.debug_bounds("frame-tooltip-bubble").is_some(),
            "tooltip disappeared at {position:?}"
        );
    }
    cx.simulate_click(button.center(), Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    cx.simulate_mouse_move(button.center(), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_some());
    cx.simulate_mouse_move(bubble.center(), None, Modifiers::default());
    draw(cx);
    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}
