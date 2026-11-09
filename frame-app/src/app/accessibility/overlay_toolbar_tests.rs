use super::*;

const CONTROLS: [&str; 4] = [
    "preview-overlay-replace",
    "preview-overlay-opacity-slider",
    "preview-overlay-remove",
    "preview-overlay-done",
];

fn bounds(cx: &mut VisualTestContext, id: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("missing {id}"))
}

fn disjoint(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    a.right() <= b.left() || b.right() <= a.left() || a.bottom() <= b.top() || b.bottom() <= a.top()
}

fn assert_toolbar_geometry(cx: &mut VisualTestContext, scale: ScalePreset) {
    let viewport = bounds(cx, "preview-viewport");
    let toolbar = bounds(cx, "preview-overlay-toolbar");
    assert_eq!(toolbar.intersect(&viewport), toolbar);
    let controls = CONTROLS.map(|id| bounds(cx, id));
    for (index, control) in controls.iter().enumerate() {
        assert!(control.size.width > px(0.0) && control.size.height > px(0.0));
        assert_eq!(control.intersect(&toolbar), *control, "{}", CONTROLS[index]);
        for other in &controls[index + 1..] {
            assert!(disjoint(*control, *other), "overlapping edit controls");
        }
    }
    for id in [
        "preview-zoom-out",
        "preview-zoom-in",
        "preview-tool-rotate",
        "preview-tool-flip-horizontal",
        "preview-tool-flip-vertical",
        "preview-tool-crop",
        "preview-tool-overlay",
    ] {
        assert!(disjoint(toolbar, bounds(cx, id)), "toolbar covers {id}");
    }
    // Wrapping moves whole groups; it must not squeeze the opacity rail or icons.
    assert_eq!(controls[1].size.width, px(96.0 * scale.factor()));
    for index in [0, 2, 3] {
        assert_eq!(controls[index].size.width, px(28.0 * scale.factor()));
        assert_eq!(controls[index].size.height, px(28.0 * scale.factor()));
    }
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        draw(cx);
    }
}

fn assert_done_committed(
    root: &Entity<FrameRoot>,
    cx: &mut VisualTestContext,
    zoom: f64,
    expected: &PreviewOverlay,
) {
    root.read_with(cx, |root, _| {
        assert!(!root.preview_ui.overlay.overlay_mode());
        let overlay = root.selected_config().unwrap().overlay.as_ref().unwrap();
        assert_eq!(
            (
                &overlay.path,
                &overlay.anchor,
                overlay.enabled,
                overlay.x,
                overlay.y,
                overlay.width,
                overlay.opacity
            ),
            (
                &expected.path,
                &expected.anchor,
                expected.enabled,
                expected.x,
                expected.y,
                expected.width,
                expected.opacity
            ),
        );
        assert_eq!(root.preview_ui.canvas.target_zoom.to_bits(), zoom.to_bits());
    });
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Overlay image"
    );
}

#[test]
fn overlay_toolbar_stays_visible_and_done_does_not_also_click_zoom() {
    for theme in crate::appearance::ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            for (width, height) in [(1200.0, 800.0), (1800.0, 1000.0)] {
                let mut app = TestAppContext::single();
                let (root, cx) =
                    test_window(&mut app, VisualFixture::PreviewAnamorphicOverlay, scale);
                root.update(cx, |root, cx| {
                    root.appearance.color_theme = theme;
                    root.set_selected_overlay_opacity(0.37);
                    cx.notify();
                });
                cx.update(|window, _| window.set_a11y_forced(true));
                cx.simulate_resize(size(px(width), px(height)));
                settle(cx);
                assert_toolbar_geometry(cx, scale);
                let zoom = root.read_with(cx, |root, _| root.preview_ui.canvas.target_zoom);
                let expected = root.read_with(cx, |root, _| {
                    root.preview_ui.overlay.overlay().unwrap().clone()
                });
                click(cx, "preview-overlay-done");
                assert_done_committed(&root, cx, zoom, &expected);
            }
        }
    }
}

fn assert_focused_label(cx: &mut VisualTestContext, label: &str) {
    let node = keyboard_tests::focused_node(cx);
    assert_eq!(node["aria"]["label"], label);
    let rect = &node["bounds"];
    let visible = Bounds::new(
        point(
            px(crate::numeric::f64_to_f32(rect["x"].as_f64().unwrap())),
            px(crate::numeric::f64_to_f32(rect["y"].as_f64().unwrap())),
        ),
        size(
            px(crate::numeric::f64_to_f32(rect["width"].as_f64().unwrap())),
            px(crate::numeric::f64_to_f32(rect["height"].as_f64().unwrap())),
        ),
    );
    assert!(visible.size.width > px(0.0) && visible.size.height > px(0.0));
    assert_eq!(visible.intersect(&bounds(cx, "preview-viewport")), visible);
}

fn tab_to_replace(cx: &mut VisualTestContext) {
    for _ in 0..80 {
        press(cx, "tab");
        if keyboard_tests::focused_node(cx)["aria"]["label"] == "Replace overlay image" {
            assert_focused_label(cx, "Replace overlay image");
            return;
        }
    }
    panic!("Replace must be reachable through keyboard navigation");
}

#[test]
fn wrapped_overlay_toolbar_preserves_keyboard_order_and_activation() {
    for activation in ["enter", "space"] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(
            &mut app,
            VisualFixture::PreviewAnamorphicOverlay,
            ScalePreset::Percent200,
        );
        cx.update(|window, _| window.set_a11y_forced(true));
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        root.update(cx, |root, cx| {
            root.set_selected_overlay_opacity(0.37);
            cx.notify();
        });
        settle(cx);
        let zoom = root.read_with(cx, |root, _| root.preview_ui.canvas.target_zoom);
        tab_to_replace(cx);
        for label in ["Overlay opacity", "Remove overlay", "Done editing overlay"] {
            press(cx, "tab");
            assert_focused_label(cx, label);
        }
        // Editing is non-modal: Tab leaves the bar, and Shift-Tab returns.
        press(cx, "tab");
        assert_focused_label(cx, "Rotate preview");
        press(cx, "shift-tab");
        assert_focused_label(cx, "Done editing overlay");
        for label in ["Remove overlay", "Overlay opacity", "Replace overlay image"] {
            press(cx, "shift-tab");
            assert_focused_label(cx, label);
        }
        press(cx, "tab");
        press(cx, "right");
        root.read_with(cx, |root, _| {
            assert!((root.preview_ui.overlay.overlay().unwrap().opacity - 0.38).abs() < 1e-6);
        });
        press(cx, "left");
        press(cx, "tab");
        press(cx, "tab");
        let expected = root.read_with(cx, |root, _| {
            root.preview_ui.overlay.overlay().unwrap().clone()
        });
        press(cx, activation);
        assert_done_committed(&root, cx, zoom, &expected);
    }
}

#[test]
fn overlay_toolbar_reflows_on_resize_and_scale_without_discarding_the_draft() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewAnamorphicOverlay,
        ScalePreset::Percent100,
    );
    root.update(cx, |root, cx| {
        root.set_selected_overlay_opacity(0.37);
        cx.notify();
    });
    let original = root.read_with(cx, |root, _| root.preview_ui.overlay.overlay().cloned());
    for (width, height, scale) in [
        (1800.0, 1000.0, ScalePreset::Percent100),
        (1200.0, 800.0, ScalePreset::Percent100),
        (1200.0, 800.0, ScalePreset::Percent200),
        (1800.0, 1000.0, ScalePreset::Percent200),
        (1800.0, 1000.0, ScalePreset::Percent100),
    ] {
        root.update(cx, |root, cx| {
            root.appearance.ui_scale = scale;
            cx.notify();
        });
        cx.simulate_resize(size(px(width), px(height)));
        settle(cx);
        assert_toolbar_geometry(cx, scale);
        root.read_with(cx, |root, _| {
            assert!(root.preview_ui.overlay.overlay_mode());
            assert_eq!(root.preview_ui.overlay.overlay(), original.as_ref());
        });
    }
}

#[test]
fn wrapped_overlay_opacity_pointer_and_remove_controls_keep_their_actions() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewAnamorphicOverlay,
        ScalePreset::Percent200,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    settle(cx);
    let zoom = root.read_with(cx, |root, _| root.preview_ui.canvas.target_zoom);
    let slider = bounds(cx, "preview-overlay-opacity-slider");
    let rail =
        cx.update(|window, _| components::frame_slider_content_bounds(slider, window.rem_size()));
    let position = |fraction: f32| point(rail.left() + rail.size.width * fraction, rail.center().y);
    cx.simulate_click(position(0.25), Modifiers::default());
    draw(cx);
    root.read_with(cx, |root, _| {
        assert!((root.preview_ui.overlay.overlay().unwrap().opacity - 0.25).abs() < 1e-6);
    });
    cx.simulate_mouse_down(position(0.25), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(position(0.4), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(position(0.75), MouseButton::Left, Modifiers::default());
    assert!(cx.update(|_, cx| cx.has_active_drag()));
    cx.simulate_mouse_up(position(0.75), MouseButton::Left, Modifiers::default());
    draw(cx);
    root.read_with(cx, |root, _| {
        assert!((root.preview_ui.overlay.overlay().unwrap().opacity - 0.75).abs() < 1e-6);
        assert_eq!(root.preview_ui.canvas.target_zoom.to_bits(), zoom.to_bits());
    });
    click(cx, "preview-overlay-remove");
    root.read_with(cx, |root, _| {
        assert!(!root.preview_ui.overlay.overlay_mode());
        assert!(root.selected_config().unwrap().overlay.is_none());
        assert_eq!(root.preview_ui.canvas.target_zoom.to_bits(), zoom.to_bits());
    });
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Overlay image"
    );
}
