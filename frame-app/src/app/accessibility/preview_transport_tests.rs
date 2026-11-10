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

fn assert_painted_surface(cx: &mut VisualTestContext, bounds: Bounds<Pixels>) {
    cx.update(|window, _| {
        let bounds = bounds.scale(window.scale_factor());
        assert!(
            window
                .painted_quads()
                .iter()
                .any(|quad| quad.bounds == bounds && !quad.background.is_transparent()),
            "the visible trim grip must paint a surface, not only reserve a hit target"
        );
    });
}

fn assert_trim_handles_share_track_y(cx: &mut VisualTestContext) {
    let track_y = cx
        .debug_bounds("preview-timeline-track")
        .unwrap()
        .center()
        .y;
    for id in [
        "preview-timeline-start-handle",
        "preview-timeline-end-handle",
        "preview-timeline-start-grip",
        "preview-timeline-end-grip",
    ] {
        let bounds = cx.debug_bounds(id).unwrap();
        assert!(
            (bounds.center().y - track_y).abs() <= px(0.5),
            "{id} is off the timeline: {bounds:?}, expected center y={track_y:?}"
        );
    }
}

fn drag_trim_grip(cx: &mut VisualTestContext, id: &'static str, delta: Pixels) {
    let from = cx.debug_bounds(id).unwrap().center();
    let to = from + point(delta, px(0.0));
    cx.simulate_mouse_move(from, None, Modifiers::default());
    draw(cx);
    assert_eq!(cx.cursor_style(), gpui::CursorStyle::ResizeLeftRight);
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(
        from + point(px(4.0), px(0.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    draw(cx);
    assert!(cx.update(|_, cx| cx.has_active_drag()));
    assert_eq!(cx.cursor_style(), gpui::CursorStyle::ResizeLeftRight);
    assert_trim_handles_share_track_y(cx);
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    draw(cx);
}

#[test]
fn preview_trim_grips_are_visible_and_align_with_the_range_at_every_scale() {
    for color_theme in crate::appearance::ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
            root.update(cx, |root, cx| {
                root.appearance.color_theme = color_theme;
                cx.notify();
            });
            open_preview_trim(cx);
            for (start, end) in [(0.0, 90.4), (45.0, 45.1), (0.0, 0.1), (90.3, 90.4)] {
                root.update(cx, |root, cx| {
                    root.update_selected_config(|config| {
                        config.start_time = Some(format_time(start));
                        config.end_time = Some(format_time(end));
                        true
                    });
                    cx.notify();
                });
                draw(cx);
                let panel = cx.debug_bounds("preview-trim-panel").unwrap();
                let selection = cx.debug_bounds("preview-timeline-selection").unwrap();
                let start_hit = cx.debug_bounds("preview-timeline-start-handle").unwrap();
                let end_hit = cx.debug_bounds("preview-timeline-end-handle").unwrap();
                let start_grip = cx.debug_bounds("preview-timeline-start-grip").unwrap();
                let end_grip = cx.debug_bounds("preview-timeline-end-grip").unwrap();
                assert_trim_handles_share_track_y(cx);
                assert_disjoint(start_hit, end_hit);
                assert_disjoint(start_grip, end_grip);
                for (hit, grip) in [(start_hit, start_grip), (end_hit, end_grip)] {
                    assert_inside(panel, hit);
                    assert_inside(hit, grip);
                    assert!(hit.size.width >= px(24.0_f32.mul_add(scale.factor(), -0.1)));
                    assert!(hit.size.height >= px(24.0_f32.mul_add(scale.factor(), -0.1)));
                    assert!(grip.size.width >= px(6.0 * scale.factor()));
                    assert!(grip.size.height >= px(16.0 * scale.factor()));
                    assert_painted_surface(cx, grip);
                }
                assert!((start_grip.right() - selection.left()).abs() <= px(0.5));
                assert!((end_grip.left() - selection.right()).abs() <= px(0.5));
                let inner =
                    root.read_with(cx, |root, _| root.preview_ui.timeline_track_bounds.unwrap());
                let duration = root.read_with(cx, |root, _| root.preview_ui.playback.duration());
                let expected_start = inner.left()
                    + inner.size.width * crate::numeric::unit_f64_to_f32(start / duration);
                let expected_end = inner.left()
                    + inner.size.width * crate::numeric::unit_f64_to_f32(end / duration);
                assert!(
                    (selection.left() - expected_start).abs() <= px(0.5),
                    "start mismatch: theme={color_theme:?} scale={scale:?} range={start}..{end} duration={duration} inner={inner:?} selection={selection:?} expected={expected_start:?}; state={:?}",
                    root.read_with(cx, |root, _| (
                        root.preview_ui.playback.start_value(),
                        root.preview_ui.playback.end_value()
                    ))
                );
                assert!(
                    (selection.right() - expected_end).abs() <= px(0.5),
                    "end mismatch: range={start}..{end} inner={inner:?} selection={selection:?} expected={expected_end:?}"
                );
                assert!(cx.debug_bounds("preview-timeline-playhead").is_some());
            }
        }
    }
}

#[test]
fn preview_trim_visible_grips_resize_independently_and_the_track_only_seeks() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        open_preview_trim(cx);
        let end = root.read_with(cx, |root, _| root.preview_ui.playback.end_value());
        let inner = root.read_with(cx, |root, _| root.preview_ui.timeline_track_bounds.unwrap());
        drag_trim_grip(cx, "preview-timeline-start-grip", inner.size.width * 0.2);
        let start = root.read_with(cx, |root, _| {
            assert!(root.preview_ui.playback.start_value() > 0.0);
            assert!((root.preview_ui.playback.end_value() - end).abs() < f64::EPSILON);
            assert!(root.preview_ui.playback.dragging().is_none());
            root.preview_ui.playback.start_value()
        });
        drag_trim_grip(cx, "preview-timeline-end-grip", -inner.size.width * 0.2);
        let end = root.read_with(cx, |root, _| {
            assert!(root.preview_ui.playback.end_value() < end);
            assert!((root.preview_ui.playback.start_value() - start).abs() < f64::EPSILON);
            assert!(root.preview_ui.playback.dragging().is_none());
            root.preview_ui.playback.end_value()
        });
        let seek = point(inner.left() + inner.size.width * 0.5, inner.center().y);
        cx.simulate_mouse_move(seek, None, Modifiers::default());
        draw(cx);
        assert_ne!(cx.cursor_style(), gpui::CursorStyle::ResizeLeftRight);
        cx.simulate_click(seek, Modifiers::default());
        draw(cx);
        root.read_with(cx, |root, _| {
            let playback = &root.preview_ui.playback;
            assert!(
                playback
                    .duration()
                    .mul_add(-0.5, playback.current_time())
                    .abs()
                    < 0.01
            );
            assert!((playback.start_value() - start).abs() < f64::EPSILON);
            assert!((playback.end_value() - end).abs() < f64::EPSILON);
        });
    }
}

#[test]
fn preview_trim_off_center_grabs_follow_a_single_pointer_move_without_jumping() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        root.update(cx, |root, cx| {
            root.update_selected_config(|config| {
                config.start_time = Some(format_time(20.0));
                config.end_time = Some(format_time(70.0));
                true
            });
            cx.notify();
        });
        open_preview_trim(cx);
        for (is_end, release_pixels) in [(false, 0.0), (true, 0.0), (false, 3.0), (true, -3.0)] {
            let id = if is_end {
                "preview-timeline-end-handle"
            } else {
                "preview-timeline-start-handle"
            };
            let hit = cx.debug_bounds(id).unwrap();
            // Grab the invisible padding, well away from the visible endpoint.
            let from = point(
                if is_end {
                    hit.right() - px(1.0)
                } else {
                    hit.left() + px(1.0)
                },
                hit.center().y,
            );
            let before = root.read_with(cx, |root, _| {
                (
                    root.preview_ui.playback.start_value(),
                    root.preview_ui.playback.end_value(),
                )
            });
            let inner =
                root.read_with(cx, |root, _| root.preview_ui.timeline_track_bounds.unwrap());
            let duration = root.read_with(cx, |root, _| root.preview_ui.playback.duration());
            cx.simulate_mouse_move(from, None, Modifiers::default());
            draw(cx);
            assert_eq!(cx.cursor_style(), gpui::CursorStyle::ResizeLeftRight);
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            let delta = px(if is_end { -8.0 } else { 8.0 } * scale.factor());
            let to = from + point(delta, px(0.0));
            // A real drag can cross the threshold and release after one move.
            // It must apply that move and retain the original point of contact.
            cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
            draw(cx);
            assert!(cx.update(|_, cx| cx.has_active_drag()));
            let after = root.read_with(cx, |root, _| {
                (
                    root.preview_ui.playback.start_value(),
                    root.preview_ui.playback.end_value(),
                )
            });
            let time_delta = if is_end {
                assert!(
                    (after.0 - before.0).abs() < f64::EPSILON,
                    "end drag changed the start"
                );
                after.1 - before.1
            } else {
                assert!(
                    (after.1 - before.1).abs() < f64::EPSILON,
                    "start drag changed the end"
                );
                after.0 - before.0
            };
            let displacement_px = time_delta / duration * f64::from(inner.size.width.as_f32());
            assert!(
                (displacement_px - f64::from(delta.as_f32())).abs() <= 0.5,
                "off-center single-move drag moved {displacement_px}px instead of {delta:?}: {before:?} -> {after:?}"
            );
            assert_trim_handles_share_track_y(cx);
            // The pointer can advance between the last move and mouseup;
            // commit the release position as well, without another move event.
            let release_delta = px(release_pixels * scale.factor());
            let release = to + point(release_delta, px(0.0));
            cx.simulate_mouse_up(release, MouseButton::Left, Modifiers::default());
            draw(cx);
            assert!(!cx.update(|_, cx| cx.has_active_drag()));
            let release_time =
                f64::from(release_delta.as_f32()) / f64::from(inner.size.width.as_f32()) * duration;
            root.read_with(cx, |root, _| {
                assert!(root.preview_ui.playback.dragging().is_none());
                let expected_start = after.0 + if is_end { 0.0 } else { release_time };
                let expected_end = after.1 + if is_end { release_time } else { 0.0 };
                // Persisted timecodes have millisecond precision.
                assert!(
                    (root.preview_ui.playback.start_value() - expected_start).abs() <= 0.0005,
                    "release start mismatch: scale={scale:?} is_end={is_end} release_pixels={release_pixels} expected={expected_start} actual={} before={before:?} after={after:?}",
                    root.preview_ui.playback.start_value()
                );
                assert!(
                    (root.preview_ui.playback.end_value() - expected_end).abs() <= 0.0005,
                    "release end mismatch: scale={scale:?} is_end={is_end} release_pixels={release_pixels} expected={expected_end} actual={} before={before:?} after={after:?}",
                    root.preview_ui.playback.end_value()
                );
            });
        }
    }
}

#[test]
fn preview_trim_short_range_handles_remain_independent_across_repeated_drags() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        root.update(cx, |root, cx| {
            root.update_selected_config(|config| {
                config.start_time = Some(format_time(45.0));
                config.end_time = Some(format_time(45.1));
                true
            });
            cx.notify();
        });
        open_preview_trim(cx);
        let track_y = cx
            .debug_bounds("preview-timeline-track")
            .unwrap()
            .center()
            .y;
        for _ in 0..3 {
            for is_end in [false, true] {
                let before = root.read_with(cx, |root, _| {
                    (
                        root.preview_ui.playback.start_value(),
                        root.preview_ui.playback.end_value(),
                    )
                });
                assert_disjoint(
                    cx.debug_bounds("preview-timeline-start-handle").unwrap(),
                    cx.debug_bounds("preview-timeline-end-handle").unwrap(),
                );
                let (id, direction) = if is_end {
                    ("preview-timeline-end-grip", 1.0)
                } else {
                    ("preview-timeline-start-grip", -1.0)
                };
                drag_trim_grip(cx, id, px(direction * 8.0 * scale.factor()));
                root.read_with(cx, |root, _| {
                    let after = (
                        root.preview_ui.playback.start_value(),
                        root.preview_ui.playback.end_value(),
                    );
                    if is_end {
                        assert!(
                            (after.0 - before.0).abs() < f64::EPSILON,
                            "dragging the end changed the start"
                        );
                        assert!(
                            after.1 > before.1,
                            "end grip did not extend the short range"
                        );
                    } else {
                        assert!(
                            after.0 < before.0,
                            "start grip did not extend the short range"
                        );
                        assert!(
                            (after.1 - before.1).abs() < f64::EPSILON,
                            "dragging the start changed the end"
                        );
                    }
                    assert!(root.preview_ui.playback.dragging().is_none());
                });
                assert_trim_handles_share_track_y(cx);
                assert!(
                    (cx.debug_bounds("preview-timeline-track")
                        .unwrap()
                        .center()
                        .y
                        - track_y)
                        .abs()
                        <= px(0.5)
                );
            }
        }
    }
}

#[test]
fn preview_trim_end_cap_at_full_duration_handles_its_center_and_edges_without_seeking() {
    for scale in [
        ScalePreset::Percent100,
        ScalePreset::Percent125,
        ScalePreset::Percent200,
    ] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        open_preview_trim(cx);
        for offset in [-2.0, 0.0, 2.0] {
            root.update(cx, |root, cx| {
                root.update_selected_config(|config| {
                    config.start_time = Some(format_time(0.0));
                    config.end_time = Some(format_time(90.4));
                    true
                });
                cx.notify();
            });
            draw(cx);
            let inner =
                root.read_with(cx, |root, _| root.preview_ui.timeline_track_bounds.unwrap());
            cx.simulate_click(inner.center(), Modifiers::default());
            draw(cx);
            let position = root.read_with(cx, |root, _| root.preview_ui.playback.current_time());
            let end = root.read_with(cx, |root, _| root.preview_ui.playback.end_value());
            let grip = cx.debug_bounds("preview-timeline-end-grip").unwrap();
            let from = grip.center() + point(px(offset * scale.factor()), px(0.0));
            cx.simulate_mouse_move(from, None, Modifiers::default());
            draw(cx);
            assert_eq!(
                cx.cursor_style(),
                gpui::CursorStyle::ResizeLeftRight,
                "missing trim cursor at end cap offset {offset}"
            );
            cx.simulate_click(from, Modifiers::default());
            draw(cx);
            root.read_with(cx, |root, _| {
                assert!(
                    (root.preview_ui.playback.current_time() - position).abs() < f64::EPSILON,
                    "click at end cap offset {offset} sought instead of targeting trim"
                );
                assert!((root.preview_ui.playback.end_value() - end).abs() < f64::EPSILON);
            });
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            let to = from - point(px(12.0 * scale.factor()), px(0.0));
            cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
            draw(cx);
            root.read_with(cx, |root, _| {
                assert_eq!(
                    root.preview_ui.playback.dragging(),
                    Some(crate::preview::TimelineDragTarget::End),
                    "end cap offset {offset} did not start an end trim"
                );
                assert!(root.preview_ui.playback.end_value() < end);
                assert!(root.preview_ui.playback.start_value().abs() < f64::EPSILON);
            });
            cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
            draw(cx);
            assert_trim_handles_share_track_y(cx);
        }
    }
}

#[test]
fn preview_trim_endpoints_align_on_the_first_render_without_a_measurement_frame() {
    for scale in [
        ScalePreset::Percent100,
        ScalePreset::Percent125,
        ScalePreset::Percent200,
    ] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        root.update(cx, |root, cx| {
            root.preview_ui.transport.open = true;
            cx.notify();
        });
        // Do not use draw(): its second frame can hide stale layout measurements.
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let inner = root.read_with(cx, |root, _| root.preview_ui.timeline_track_bounds.unwrap());
        let start = cx.debug_bounds("preview-timeline-start-grip").unwrap();
        let end = cx.debug_bounds("preview-timeline-end-grip").unwrap();
        assert!(
            (start.right() - inner.left()).abs() <= px(0.5),
            "start is misplaced on the first frame: {start:?}, range {inner:?}"
        );
        assert!(
            (end.left() - inner.right()).abs() <= px(0.5),
            "end is misplaced on the first frame: {end:?}, range {inner:?}"
        );
        assert_trim_handles_share_track_y(cx);
    }
}

fn painted_end_cap_bounds(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let layout = cx.debug_bounds("preview-timeline-end-grip").unwrap();
    cx.update(|window, _| {
        let display_scale = window.scale_factor();
        window
            .painted_quads()
            .into_iter()
            .find_map(|quad| {
                let bounds = Bounds::new(
                    quad.bounds
                        .origin
                        .map(|value| px(value.as_f32() / display_scale)),
                    quad.bounds
                        .size
                        .map(|value| px(value.as_f32() / display_scale)),
                );
                (!quad.background.is_transparent()
                    && bounds.size.width >= layout.size.width * 0.9
                    && bounds.size.width <= layout.size.width
                    && bounds.size.height >= layout.size.height * 0.9
                    && bounds.size.height <= layout.size.height
                    && (bounds.center().x - layout.center().x).abs() < px(20.0)
                    && (bounds.center().y - layout.center().y).abs() < px(20.0))
                .then_some(bounds)
            })
            .expect("painted End cap during surface motion")
    })
}

#[test]
fn preview_trim_end_cap_accepts_pointer_input_with_surface_motion_enabled() {
    for elapsed_ms in [16, 140] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(
            &mut app,
            VisualFixture::PreviewReady,
            ScalePreset::Percent125,
        );
        let now = std::time::Instant::now();
        cx.set_transition_time(now);
        cx.update(|_, cx| cx.set_reduce_motion(false));
        root.update(cx, |root, cx| {
            root.preview_ui.transport.open = true;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.set_transition_time(now + std::time::Duration::from_millis(elapsed_ms));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        // Paint-space bounds include the entrance scale; debug layout bounds do not.
        let from = painted_end_cap_bounds(cx).center();
        cx.simulate_mouse_move(from, None, Modifiers::default());
        assert_eq!(
            cx.cursor_style(),
            gpui::CursorStyle::ResizeLeftRight,
            "End cap hover failed {elapsed_ms}ms after opening"
        );
        let from = painted_end_cap_bounds(cx).center();
        let position = root.read_with(cx, |root, _| root.preview_ui.playback.current_time());
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        root.read_with(cx, |root, _| {
            assert!(
                (root.preview_ui.playback.current_time() - position).abs() < f64::EPSILON,
                "painted End cap sought on mouse-down {elapsed_ms}ms after opening"
            );
        });
        let to = from - point(px(20.0), px(0.0));
        cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
        root.read_with(cx, |root, _| {
            assert_eq!(
                root.preview_ui.playback.dragging(),
                Some(crate::preview::TimelineDragTarget::End),
                "painted End cap sought instead of trimming {elapsed_ms}ms after opening"
            );
            assert!(root.preview_ui.playback.end_value() < root.preview_ui.playback.duration());
        });
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    }
}

#[test]
fn preview_trim_and_position_remain_separate_keyboard_sliders() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    open_preview_trim(cx);
    let original_end = root.read_with(cx, |root, _| root.preview_ui.playback.end_value());
    tab_to_label(cx, "Trim start");
    press(cx, "right");
    let start = root.read_with(cx, |root, _| {
        assert!(root.preview_ui.playback.start_value() > 0.0);
        assert!((root.preview_ui.playback.end_value() - original_end).abs() < f64::EPSILON);
        root.preview_ui.playback.start_value()
    });
    tab_to_label(cx, "Trim end");
    press(cx, "left");
    let end = root.read_with(cx, |root, _| {
        assert!(root.preview_ui.playback.end_value() < original_end);
        assert!((root.preview_ui.playback.start_value() - start).abs() < f64::EPSILON);
        root.preview_ui.playback.end_value()
    });
    tab_to_label(cx, "Preview position");
    press(cx, "pageup");
    root.read_with(cx, |root, _| {
        let playback = &root.preview_ui.playback;
        assert!(playback.current_time() > start);
        assert!((playback.start_value() - start).abs() < f64::EPSILON);
        assert!((playback.end_value() - end).abs() < f64::EPSILON);
    });
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
