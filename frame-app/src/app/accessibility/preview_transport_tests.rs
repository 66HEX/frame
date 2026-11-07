use super::*;

fn move_outside_preview(cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(point(px(-100.0), px(-100.0)), None, Modifiers::default());
    draw(cx);
}

fn tab_to_label(cx: &mut VisualTestContext, label: &str) {
    for _ in 0..80 {
        press(cx, "tab");
        if keyboard_tests::focused_node(cx)["aria"]["label"] == label {
            return;
        }
    }
    panic!("unreachable preview control: {label}");
}

fn assert_inside(outer: Bounds<Pixels>, inner: Bounds<Pixels>) {
    let tolerance = px(0.5);
    assert!(
        inner.left() >= outer.left() - tolerance
            && inner.top() >= outer.top() - tolerance
            && inner.right() <= outer.right() + tolerance
            && inner.bottom() <= outer.bottom() + tolerance,
        "control escapes preview: {inner:?}, viewport {outer:?}"
    );
}

#[test]
fn preview_transport_reveals_on_keyboard_focus_and_returns_focus_after_trim() {
    for fixture in [VisualFixture::PreviewReady, VisualFixture::WorkspaceAudio] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (_, cx) = test_window(&mut app, fixture, scale);
            move_outside_preview(cx);
            assert!(cx.debug_bounds("preview-playback-toggle").is_none());
            assert!(cx.debug_bounds("preview-trim-trigger").is_none());
            assert!(cx.debug_bounds("preview-start-time").is_none());
            cx.update(|window, _| window.set_a11y_forced(true));
            draw(cx);

            tab_to_label(cx, "Preview canvas");
            assert!(cx.debug_bounds("preview-playback-toggle").is_some());
            assert!(cx.debug_bounds("preview-trim-trigger").is_some());
            tab_to_label(cx, "Trim preview");
            let trigger_focus = focus(cx);
            press(cx, "enter");
            assert!(cx.debug_bounds("preview-trim-panel").is_some());
            assert!(cx.debug_bounds("preview-start-time").is_some());
            assert!(cx.debug_bounds("preview-end-time").is_some());
            assert_ne!(focus(cx), trigger_focus, "opening trim enters its controls");

            move_outside_preview(cx);
            assert!(cx.debug_bounds("preview-trim-panel").is_some());
            press(cx, "escape");
            assert!(cx.debug_bounds("preview-trim-panel").is_none());
            assert_eq!(focus(cx), trigger_focus);

            press(cx, "enter");
            tab_to_label(cx, "Preview start time");
            tab_to_label(cx, "Preview end time");
            tab_to_label(cx, "Preview position");
            press(cx, "escape");
            assert!(cx.debug_bounds("preview-trim-panel").is_none());
            assert_eq!(focus(cx), trigger_focus);
            tab_to_label(cx, "Metadata");
            assert!(cx.debug_bounds("preview-playback-toggle").is_none());
        }
    }
}

#[test]
fn preview_trim_floats_without_reflow_or_collisions_at_the_minimum_window_size() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (_, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        move_outside_preview(cx);
        let viewport = cx.debug_bounds("preview-viewport").unwrap();
        let card = cx.debug_bounds("preview-panel").unwrap();
        let padding = px(crate::app::layout::PREVIEW_PANEL_PADDING * scale.factor());
        assert!((viewport.top() - card.top() - padding).abs() < px(0.5));
        assert!((card.bottom() - viewport.bottom() - padding).abs() < px(0.5));

        cx.simulate_mouse_move(viewport.center(), None, Modifiers::default());
        draw(cx);
        let play = cx.debug_bounds("preview-playback-toggle").unwrap();
        let trigger = cx.debug_bounds("preview-trim-trigger").unwrap();
        assert_inside(viewport, play);
        assert_inside(viewport, trigger);
        assert_disjoint(play, trigger);
        move_outside_preview(cx);
        assert!(cx.debug_bounds("preview-playback-toggle").is_none());

        open_preview_trim(cx);
        let panel = cx.debug_bounds("preview-trim-panel").unwrap();
        assert_eq!(cx.debug_bounds("preview-viewport").unwrap(), viewport);
        assert_inside(viewport, panel);
        assert!(panel.bottom() <= play.top(), "trim opens above transport");
        for id in [
            "preview-tool-rotate",
            "preview-tool-flip-horizontal",
            "preview-tool-flip-vertical",
            "preview-tool-crop",
            "preview-tool-overlay",
            "preview-zoom-out",
            "preview-zoom-in",
        ] {
            if let Some(tool) = cx.debug_bounds(id) {
                assert_disjoint(panel, tool);
                assert_disjoint(play, tool);
                assert_disjoint(trigger, tool);
            }
        }
        for id in [
            "preview-start-time",
            "preview-end-time",
            "preview-timeline-start-handle",
            "preview-timeline-end-handle",
        ] {
            assert_inside(panel, cx.debug_bounds(id).unwrap());
        }
        press(cx, "escape");
        assert_eq!(cx.debug_bounds("preview-viewport").unwrap(), viewport);
    }
}

#[test]
fn preview_edit_modes_replace_transport_and_non_temporal_previews_have_no_transport() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    root.update(cx, |root, cx| {
        root.update_selected_config(|config| {
            config.overlay = Some(OverlaySettings {
                enabled: true,
                path: "/tmp/preview-transport-overlay.png".into(),
                x: 0.5,
                y: 0.5,
                width: 0.3,
                opacity: 1.0,
                anchor: "custom".into(),
            });
            true
        });
        cx.notify();
    });
    draw(cx);
    for (trigger, mode_control) in [
        ("preview-tool-crop", "preview-crop-ratio"),
        ("preview-tool-overlay", "preview-overlay-done"),
    ] {
        open_preview_trim(cx);
        click(cx, trigger);
        assert!(
            cx.debug_bounds(mode_control).is_some(),
            "missing {mode_control}"
        );
        assert!(cx.debug_bounds("preview-trim-panel").is_none());
        assert!(cx.debug_bounds("preview-playback-toggle").is_none());
        press(cx, "escape");
        assert!(cx.debug_bounds(mode_control).is_none());
        assert!(cx.debug_bounds("preview-trim-panel").is_none());
    }

    for fixture in [VisualFixture::WorkspaceImage, VisualFixture::WorkspaceEmpty] {
        let mut app = TestAppContext::single();
        let (_, cx) = test_window(&mut app, fixture, ScalePreset::Percent100);
        if let Some(viewport) = cx.debug_bounds("preview-viewport") {
            cx.simulate_mouse_move(viewport.center(), None, Modifiers::default());
            draw(cx);
        }
        assert!(cx.debug_bounds("preview-playback-toggle").is_none());
        assert!(cx.debug_bounds("preview-trim-trigger").is_none());
        assert!(cx.debug_bounds("preview-start-time").is_none());
    }
}

#[test]
fn preview_trim_escape_cancels_draft_and_outside_click_commits_without_stealing_focus() {
    use crate::app::input::FrameTextInputKind;

    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    open_preview_trim(cx);
    click(cx, "preview-start-time");
    press(cx, "cmd-a");
    cx.simulate_input("00:00:05.000");
    draw(cx);
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.text_input_value(FrameTextInputKind::PreviewStartTime),
            "00:00:05.000"
        );
        assert!(root.preview_ui.playback.start_value().abs() < f64::EPSILON);
    });

    press(cx, "escape");
    assert!(cx.debug_bounds("preview-trim-panel").is_some());
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Preview start time"
    );
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.text_input_value(FrameTextInputKind::PreviewStartTime),
            "00:00:00.000"
        );
        assert!(root.preview_ui.playback.start_value().abs() < f64::EPSILON);
    });
    press(cx, "escape");
    assert!(cx.debug_bounds("preview-trim-panel").is_none());
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Trim preview"
    );

    press(cx, "enter");
    click(cx, "preview-start-time");
    press(cx, "cmd-a");
    cx.simulate_input("00:00:05.000");
    draw(cx);
    let zoom_before = root.read_with(cx, |root, _| root.preview_ui.canvas.target_zoom);
    click(cx, "preview-zoom-in");
    assert!(root.read_with(cx, |root, _| root.preview_ui.canvas.target_zoom) > zoom_before);
    assert!(cx.debug_bounds("preview-trim-panel").is_none());
    root.read_with(cx, |root, _| {
        assert!((root.preview_ui.playback.start_value() - 5.0).abs() < f64::EPSILON);
        assert_eq!(
            root.selected_config().unwrap().start_time.as_deref(),
            Some("00:00:05.000")
        );
    });
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Preview canvas"
    );

    open_preview_trim(cx);
    click(cx, "preview-end-time");
    press(cx, "cmd-a");
    cx.simulate_input("00:01:00.000");
    draw(cx);
    click(cx, "titlebar-settings");
    assert!(cx.debug_bounds("preview-trim-panel").is_none());
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Close settings"
    );
    root.read_with(cx, |root, _| {
        assert!((root.preview_ui.playback.end_value() - 60.0).abs() < f64::EPSILON);
        assert_eq!(
            root.selected_config().unwrap().end_time.as_deref(),
            Some("00:01:00.000")
        );
    });
}

#[test]
fn preview_trim_dismissal_ends_drag_and_media_selection_preserves_visible_focus() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    let image_runtime_key = root.update(cx, |root, cx| {
        root.apply_workspace_image_fixture();
        // Prepare the same inert runtime sentinel used by test_window for the second fixture.
        let selected = root.file_queue.selected_file_id().map(str::to_owned);
        root.file_queue
            .select_file(Some("fixture-image".to_owned()));
        let metadata = root.selected_source_metadata_entry();
        let image_key = root
            .selected_preview_runtime_request(&metadata)
            .unwrap()
            .key;
        root.file_queue.select_file(selected);
        cx.notify();
        image_key
    });
    cx.update(|window, _| window.set_a11y_forced(true));
    open_preview_trim(cx);
    let from = cx
        .debug_bounds("preview-timeline-start-handle")
        .unwrap()
        .center();
    let to = from + point(px(40.0), px(0.0));
    cx.simulate_mouse_move(from, None, Modifiers::default());
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(
        from + point(px(4.0), px(0.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    draw(cx);
    assert!(root.read_with(cx, |root, _| root.preview_ui.playback.dragging().is_some()));

    press(cx, "escape");
    assert!(cx.debug_bounds("preview-trim-panel").is_none());
    root.read_with(cx, |root, _| {
        assert!(root.preview_ui.playback.dragging().is_none());
        assert!(!root.preview_media_snapshot_sync_blocked());
    });
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    draw(cx);

    open_preview_trim(cx);
    let image_row = cx
        .debug_bounds("file-list-row-fixture-image")
        .unwrap()
        .center();
    cx.simulate_mouse_down(image_row, MouseButton::Left, Modifiers::default());
    // Mouse-down closes trim; selection changes on mouse-up. Keep each source inert
    // while simulate_event drains its executor between the two parts of the click.
    root.update(cx, |root, _| {
        root.preview_ui.pending_runtime_key = Some(image_runtime_key);
    });
    cx.simulate_mouse_up(image_row, MouseButton::Left, Modifiers::default());
    draw(cx);
    assert_eq!(
        root.read_with(cx, |root, _| root
            .file_queue
            .selected_file_id()
            .map(str::to_owned)),
        Some("fixture-image".to_owned())
    );
    assert!(cx.debug_bounds("preview-trim-panel").is_none());
    assert!(cx.debug_bounds("preview-playback-toggle").is_none());
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "File queue"
    );
}
