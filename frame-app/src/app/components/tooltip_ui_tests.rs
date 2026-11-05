use super::*;
use crate::VisualFixture;
use crate::app::FrameRoot;
use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext, point, px};

struct TooltipTestView {
    root: Entity<FrameRoot>,
    focus: FocusHandle,
    edge_placement: Option<TooltipPlacement>,
    _subscription: Subscription,
}

impl gpui::Render for TooltipTestView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let edge_placement = self.edge_placement;
        self.root.update(cx, |root, cx| {
            window.set_rem_size(px(
                crate::appearance::BASE_REM_PX * root.appearance.ui_scale.factor()
            ));
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
                    root.tooltip_ui.visible_id.as_deref(),
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
            _subscription: subscription,
        }
    });
    cx.update(|window, _| window.activate());
    cx.run_until_parked();
    let root = view.read_with(cx, |view, _| view.root.clone());
    (root, cx)
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
                    root.tooltip_ui.visible_id = Some("preview-tool-rotate".into());
                    root
                });
                let subscription = cx.observe(&root, |_, _, cx| cx.notify());
                TooltipTestView {
                    root,
                    focus,
                    edge_placement: Some(placement),
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
    std::thread::sleep(INTERACTION_MOTION_DURATION);
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
    std::thread::sleep(INTERACTION_MOTION_DURATION);
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_none());
    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    cx.simulate_click(point(px(600.0), px(50.0)), Modifiers::default());
    draw(cx);
    std::thread::sleep(INTERACTION_MOTION_DURATION);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}

#[test]
fn mira_tooltip_opens_immediately_and_closes_when_the_pointer_leaves() {
    let mut app = TestAppContext::single();
    let (_, cx) = preview_window(&mut app);
    draw(cx);
    let rotate = cx.debug_bounds("preview-tool-rotate").unwrap();
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY / 2);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_some());
    cx.simulate_mouse_move(point(px(600.0), px(50.0)), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    std::thread::sleep(INTERACTION_MOTION_DURATION);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.simulate_mouse_move(rotate.center(), None, Modifiers::default());
    cx.executor().advance_clock(TOOLTIP_HOVER_DELAY);
    draw(cx);
    assert!(cx.debug_bounds("tooltip-preview-tool-rotate").is_some());
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
    std::thread::sleep(INTERACTION_MOTION_DURATION);
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
    std::thread::sleep(INTERACTION_MOTION_DURATION);
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
    std::thread::sleep(INTERACTION_MOTION_DURATION);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}
