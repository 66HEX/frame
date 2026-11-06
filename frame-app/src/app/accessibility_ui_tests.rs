use super::*;
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
}

fn test_window(
    app: &mut TestAppContext,
    fixture: VisualFixture,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        init_app(cx, "Frame accessibility test");
        cx.set_reduce_motion(true);
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(fixture));
        root.appearance.ui_scale = scale;
        root.available_filters.eq = true;
        root.preview_ui.render_image = Some(Arc::new(RenderImage::new(smallvec::smallvec![
            image::Frame::new(image::RgbaImage::new(400, 225))
        ])));
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        root
    });
    cx.update(|window, _| window.activate());
    cx.simulate_resize(size(px(2200.0), px(1800.0)));
    draw(cx);
    (root, cx)
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("missing {id}"));
    cx.simulate_click(bounds.center(), Modifiers::default());
    draw(cx);
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

fn focus(cx: &mut VisualTestContext) -> FocusHandle {
    cx.update(|window, cx| window.focused(cx).expect("focus must stay in the window"))
}

#[path = "accessibility/keyboard_tests.rs"]
mod keyboard_tests;

#[path = "accessibility/preview_motion_tests.rs"]
mod preview_motion_tests;

#[test]
fn preview_canvas_focus_ring_paints_above_the_clipped_content() {
    for theme in crate::appearance::ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = test_window(&mut app, VisualFixture::SettingsVideo, scale);
            root.update(cx, |root, cx| {
                root.appearance.color_theme = theme;
                cx.notify();
            });
            press(cx, "tab");
            cx.update(|window, cx| {
                root.update(cx, |root, cx| {
                    assert!(root.focus_registered_control("preview-viewport", window, cx));
                });
            });
            draw(cx);
            let viewport = cx.debug_bounds("preview-viewport").unwrap();
            cx.update(|window, _| {
                let scale = window.scale_factor();
                let viewport = viewport.scale(scale);
                let quads = window.painted_quads();
                let background = quads
                    .iter()
                    .find(|q| q.bounds == viewport && !q.background.is_transparent())
                    .expect("canvas background");
                let ring_bounds =
                    viewport.dilate(gpui::ScaledPixels(theme::metrics::FOCUS_WIDTH * scale));
                let ring = quads
                    .iter()
                    .find(|q| q.bounds == ring_bounds)
                    .expect("canvas ring");
                assert!(
                    ring.order > background.order,
                    "the ring must paint after canvas content and its corner cutouts"
                );
                assert_eq!(
                    ring.bounds.intersect(&ring.content_mask.bounds),
                    ring.bounds,
                    "the full outside ring must remain inside its ancestor clip"
                );
                assert_eq!(
                    ring.border_widths.top,
                    gpui::ScaledPixels(theme::metrics::FOCUS_WIDTH * scale)
                );
            });
        }
    }
}

fn assert_active_tab_background(
    cx: &mut VisualTestContext,
    rail_id: &'static str,
    trigger_id: &'static str,
    mode: crate::appearance::ColorTheme,
) {
    let rail = cx.debug_bounds(rail_id).expect("tab rail bounds");
    let trigger = cx.debug_bounds(trigger_id).expect("tab trigger bounds");
    cx.update(|window, _| {
        let scale = window.scale_factor();
        let rail = rail.scale(scale);
        let expected = theme::palette(mode).tab_active;
        let backgrounds: Vec<_> = window
            .painted_quads()
            .into_iter()
            .filter(|quad| {
                quad.background.as_solid().is_some_and(|fill| {
                    let fill = gpui::hsla_to_rgba(fill);
                    (fill.red - expected.red).abs() < 0.001
                        && (fill.green - expected.green).abs() < 0.001
                        && (fill.blue - expected.blue).abs() < 0.001
                        && (fill.alpha - expected.alpha).abs() < 0.001
                }) && rail.contains(&quad.bounds.center())
            })
            .collect();
        assert_eq!(backgrounds.len(), 1, "one shared background in {rail_id}");
        assert_eq!(backgrounds[0].bounds, trigger.scale(scale));
        assert_eq!(
            backgrounds[0]
                .bounds
                .intersect(&backgrounds[0].content_mask.bounds),
            backgrounds[0].bounds,
            "the selected background remains inside the rail clip"
        );
    });
}

#[test]
fn tab_rails_paint_one_active_background_matching_the_selected_trigger() {
    for mode in crate::appearance::ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = test_window(&mut app, VisualFixture::SettingsVideo, scale);
            root.update(cx, |root, cx| {
                root.appearance.color_theme = mode;
                cx.notify();
            });
            for trigger in ["titlebar-logs", "titlebar-workspace"] {
                click(cx, trigger);
                assert_active_tab_background(cx, "titlebar-main-view-tabs", trigger, mode);
            }
            assert!(
                cx.debug_bounds("titlebar-workspace").unwrap().size.width
                    > cx.debug_bounds("titlebar-logs").unwrap().size.width,
                "the moving background must fit different label widths"
            );
            for trigger in [
                "settings-tab-metadata",
                "settings-tab-presets",
                "settings-tab-output",
            ] {
                click(cx, trigger);
                assert_active_tab_background(cx, "settings-tab-list", trigger, mode);
            }
            press(cx, "home");
            assert_active_tab_background(cx, "settings-tab-list", "settings-tab-metadata", mode);
            press(cx, "right");
            assert_active_tab_background(cx, "settings-tab-list", "settings-tab-output", mode);
        }
    }
}

#[test]
fn logs_tab_indicator_does_not_make_fitted_rails_scrollable() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(&mut app, VisualFixture::LogsActive, ScalePreset::Percent100);
    for count in [1, 3] {
        root.update(cx, |root, cx| {
            for index in root.file_queue.files().len()..count {
                let id = format!("fitted-log-{index}");
                root.file_queue
                    .add_file(crate::file_queue::FileItem::from_path(
                        &id,
                        format!("/tmp/clip_{index}.mov"),
                        1000,
                    ));
                root.file_queue
                    .update_status(&id, crate::file_queue::FileStatus::Converting, 50);
                root.conversion_events.apply_conversion_event(
                    &mut root.file_queue,
                    frame_core::events::ConversionEvent::log(&id, "Progress"),
                );
            }
            cx.notify();
        });
        draw(cx);
        let selected = cx.debug_bounds("logs-tab-fixture-video").unwrap();
        let rail = cx.debug_bounds("logs-tab-list").unwrap();
        for direction in [-1.0, 1.0] {
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: rail.center(),
                delta: gpui::ScrollDelta::Pixels(point(px(direction * 1000.0), px(0.0))),
                ..Default::default()
            });
            draw(cx);
            assert_eq!(
                cx.debug_bounds("logs-tab-fixture-video").unwrap(),
                selected,
                "the decorative indicator cannot add overflow to a fitted {count}-tab rail"
            );
        }
    }
}

#[test]
fn logs_tab_indicator_tracks_the_selected_file_after_keyboard_scroll() {
    for mode in crate::appearance::ColorTheme::ALL {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::LogsActive, ScalePreset::Percent100);
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        root.update(cx, |root, cx| {
            root.appearance.color_theme = mode;
            for index in 0..30 {
                let id = format!("indicator-log-{index:02}");
                root.file_queue
                    .add_file(crate::file_queue::FileItem::from_path(
                        &id,
                        format!("/tmp/long_conversion_filename_{index:02}.mov"),
                        1000,
                    ));
                root.file_queue
                    .update_status(&id, crate::file_queue::FileStatus::Converting, 50);
                root.conversion_events.apply_conversion_event(
                    &mut root.file_queue,
                    frame_core::events::ConversionEvent::log(&id, "Progress"),
                );
            }
            cx.notify();
        });
        draw(cx);
        click(cx, "logs-tab-fixture-video");
        press(cx, "end");
        assert_eq!(
            root.read_with(cx, |root, _| root
                .conversion_events
                .selected_log_file_id()
                .map(str::to_owned)),
            Some("indicator-log-29".to_owned())
        );
        assert_active_tab_background(cx, "logs-tab-list", "logs-tab-indicator-log-29", mode);
        press(cx, "home");
        assert_active_tab_background(cx, "logs-tab-list", "logs-tab-fixture-video", mode);
    }
}

#[test]
fn tab_rails_move_focus_with_each_arrow_and_offer_one_tab_stop() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::SettingsVideo, scale);
        click(cx, "titlebar-workspace");
        let workspace = focus(cx);
        for (key, expected) in [
            ("right", ActiveView::Logs),
            ("right", ActiveView::Workspace),
            ("end", ActiveView::Logs),
            ("home", ActiveView::Workspace),
            ("left", ActiveView::Logs),
        ] {
            press(cx, key);
            assert_eq!(root.read_with(cx, |root, _| root.active_view), expected);
            assert_eq!(focus(cx) == workspace, expected == ActiveView::Workspace);
        }
        let selected = focus(cx);
        press(cx, "tab");
        assert_ne!(focus(cx), selected);
        assert_ne!(focus(cx), workspace);
        press(cx, "shift-tab");
        assert_eq!(focus(cx), selected);

        click(cx, "titlebar-workspace");
        click(cx, "settings-tab-metadata");
        let source = focus(cx);
        for (key, expected) in [
            ("right", SettingsTab::Output),
            ("right", SettingsTab::Presets),
            ("right", SettingsTab::Metadata),
            ("left", SettingsTab::Presets),
            ("home", SettingsTab::Metadata),
            ("right", SettingsTab::Output),
            ("end", SettingsTab::Presets),
            ("right", SettingsTab::Metadata),
            ("left", SettingsTab::Presets),
            ("home", SettingsTab::Metadata),
        ] {
            let previous = focus(cx);
            press(cx, key);
            assert_eq!(
                root.read_with(cx, |root, _| root.settings_ui.active_tab),
                expected
            );
            assert_ne!(focus(cx), previous, "focus did not move for {key}");
            assert_eq!(focus(cx) == source, expected == SettingsTab::Metadata);
        }
        press(cx, "tab");
        assert_ne!(focus(cx), source);
        press(cx, "shift-tab");
        assert_eq!(focus(cx), source);
    }
}

#[test]
fn inspector_tab_enters_controls_directly_and_shift_tab_returns_to_the_selected_tab() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (_, cx) = test_window(&mut app, VisualFixture::SettingsVideo, scale);
        cx.update(|window, _| window.set_a11y_forced(true));
        draw(cx);
        for (id, expected) in [
            ("settings-tab-metadata", None),
            ("settings-tab-output", Some("General")),
            ("settings-tab-presets", Some("Preset name")),
        ] {
            click(cx, id);
            let selected_tab = focus(cx);
            press(cx, "tab");
            let control = focus(cx);
            assert_ne!(control, selected_tab);
            let node = cx.update(|window, _| {
                let tree: serde_json::Value =
                    serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
                tree["nodes"][tree["gpui_focus"].as_str().unwrap()].clone()
            });
            assert_ne!(
                node["aria"]["role"], "TabPanel",
                "panel must not be a tab stop"
            );
            if let Some(label) = expected {
                assert_eq!(node["aria"]["label"], label, "first control of {id}");
            }
            press(cx, "shift-tab");
            assert_eq!(focus(cx), selected_tab);
            press(cx, "tab");
            assert_eq!(focus(cx), control);
        }
    }
}

#[test]
fn output_navigation_survives_changes_to_section_availability() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    click(cx, "settings-tab-output");
    let output = focus(cx);
    root.update(cx, |root, cx| {
        root.update_selected_config(|config| {
            config.processing_mode = ProcessingMode::Copy;
            true
        });
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        root.read_with(cx, |root, _| root.settings_ui.active_tab),
        SettingsTab::Output
    );
    assert_eq!(focus(cx), output);
    assert!(cx.debug_bounds("output-section-video-trigger").is_none());
    press(cx, "left");
    assert_eq!(
        root.read_with(cx, |root, _| root.settings_ui.active_tab),
        SettingsTab::Metadata
    );
}

#[test]
fn enabling_reduced_motion_settles_an_in_flight_canvas_transform_on_the_next_draw() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    cx.update(|_, cx| cx.set_reduce_motion(false));
    root.update(cx, |root, cx| {
        root.preview_ui.canvas.auto_fit_pending = false;
        root.preview_ui.canvas.target_zoom = 2.0;
        root.preview_ui.canvas.target_pan_x = 0.1;
        root.preview_ui.canvas.target_pan_y = -0.1;
        cx.notify();
    });
    draw(cx);
    assert!(
        root.read_with(cx, |root, _| (root.preview_ui.canvas.current_zoom - 2.0)
            .abs())
            > 0.01
    );
    let playback = root.read_with(cx, |root, _| root.preview_ui.playback.clone());
    cx.update(|_, cx| cx.set_reduce_motion(true));
    draw(cx);
    root.read_with(cx, |root, _| {
        let canvas = &root.preview_ui.canvas;
        assert!((canvas.current_zoom - 2.0).abs() < f64::EPSILON);
        assert!((canvas.current_pan_x - 0.1).abs() < f64::EPSILON);
        assert!((canvas.current_pan_y + 0.1).abs() < f64::EPSILON);
        assert!(canvas.last_animation_tick.is_none());
        assert_eq!(root.preview_ui.playback, playback);
    });
    cx.update(|_, cx| cx.set_reduce_motion(false));
    root.update(cx, |root, _| {
        root.preview_ui.canvas.target_zoom = 3.0;
        root.tick_preview_canvas_animation();
        assert!(root.preview_ui.canvas.current_zoom > 2.0);
        assert!(root.preview_ui.canvas.current_zoom < 3.0);
    });
}

fn drag(cx: &mut VisualTestContext, from: Point<Pixels>, to: Point<Pixels>) {
    cx.simulate_mouse_move(from, None, Modifiers::default());
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(
        from + point(px(4.0), px(4.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(
        to + point(px(2.0), px(2.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    assert!(
        cx.update(|_, cx| cx.has_active_drag()),
        "expanded area did not start a drag"
    );
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    draw(cx);
}

#[test]
fn crop_resize_and_scrollbar_drags_start_outside_the_visible_markers() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewCropCustom, scale);
        let factor = scale.factor();
        let bounds = cx.debug_bounds("preview-crop-handle-ne").unwrap();
        assert!((24.0_f32.mul_add(-factor, bounds.size.width.as_f32())).abs() < 0.01);
        let before = root.read_with(cx, |root, _| root.preview_ui.draft_crop);
        let from = bounds.center() + point(px(-9.0 * factor), px(0.0));
        drag(cx, from, from + point(px(-60.0), px(50.0)));
        assert_ne!(
            root.read_with(cx, |root, _| root.preview_ui.draft_crop),
            before
        );

        click(cx, "preview-crop-ratio");
        let bounds = cx.debug_bounds("crop-options-scrollbar").unwrap();
        let option = cx.debug_bounds("crop-ratio-free").unwrap();
        assert!(
            option.right() > bounds.left(),
            "scrollbar must overlay the option instead of reserving a gutter"
        );
        let before = root.read_with(cx, |root, _| root.preview_ui.crop_options.scroll.offset());
        let from = point(bounds.left() + px(factor), bounds.top() + px(4.0 * factor));
        drag(cx, from, from + point(px(0.0), px(70.0 * factor)));
        assert_ne!(
            root.read_with(cx, |root, _| root.preview_ui.crop_options.scroll.offset()),
            before
        );
    }
}

#[test]
fn slider_padding_starts_drags_but_disabled_sliders_ignore_it() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::SettingsVideoFilters, scale);
        let bounds = cx
            .debug_bounds("settings-video-filter-brightness-slider")
            .unwrap();
        assert!((24.0_f32.mul_add(-scale.factor(), bounds.size.height.as_f32())).abs() < 0.01);
        let before = root.read_with(cx, |root, _| {
            root.selected_config()
                .unwrap()
                .video_filters
                .color
                .brightness
        });
        let from = point(
            bounds.left() + bounds.size.width / 4.0,
            bounds.top() + px(scale.factor()),
        );
        drag(cx, from, from + point(px(40.0), px(0.0)));
        let after = root.read_with(cx, |root, _| {
            root.selected_config()
                .unwrap()
                .video_filters
                .color
                .brightness
        });
        assert_ne!(after, before);
        root.update(cx, |root, cx| {
            root.update_selected_config(|config| {
                config.video_filters.color.brightness.enabled = false;
                true
            });
            cx.notify();
        });
        draw(cx);
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(
            from + point(px(50.0), px(0.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(!cx.update(|_, cx| cx.has_active_drag()));
        cx.simulate_mouse_up(from, MouseButton::Left, Modifiers::default());
        assert_eq!(
            root.read_with(cx, |root, _| root
                .selected_config()
                .unwrap()
                .video_filters
                .color
                .brightness
                .value),
            after.value
        );
    }
}

#[test]
fn slider_values_reach_both_inset_endpoints_at_each_scale() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::SettingsVideoFilters, scale);
        let bounds = cx
            .debug_bounds("settings-video-filter-brightness-slider")
            .unwrap();
        let inset = px(theme::metrics::SLIDER_THUMB / 2.0 * scale.factor());
        let left = bounds.left() + inset;
        let width = bounds.size.width - inset * 2.0;
        for (fraction, expected) in [(0.0, -100), (0.25, -50), (0.75, 50), (1.0, 100)] {
            let from = bounds.center();
            let to = point(left + width * fraction, from.y);
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_move(
                from + point(px(5.0), px(0.0)),
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
            draw(cx);
            assert_eq!(
                root.read_with(cx, |root, _| root
                    .selected_config()
                    .unwrap()
                    .video_filters
                    .color
                    .brightness
                    .value),
                expected
            );
        }
    }
}

#[test]
fn timeline_seek_and_trim_targets_follow_the_inset_value_range() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        let bounds = cx.debug_bounds("preview-timeline-track").unwrap();
        let inset = px(theme::metrics::SLIDER_THUMB / 2.0 * scale.factor());
        let inner = root.read_with(cx, |root, _| root.preview_ui.timeline_track_bounds.unwrap());
        assert!((inner.left() - bounds.left() - inset).abs() < px(0.01));
        assert!((bounds.right() - inner.right() - inset).abs() < px(0.01));
        for (id, endpoint) in [
            ("preview-timeline-start-handle", inner.left()),
            ("preview-timeline-end-handle", inner.right()),
        ] {
            let handle = cx.debug_bounds(id).unwrap();
            assert!((handle.center().x - endpoint).abs() < px(0.01));
        }
        for fraction in [0.25, 0.75] {
            cx.simulate_click(
                point(inner.left() + inner.size.width * fraction, inner.center().y),
                Modifiers::default(),
            );
            draw(cx);
            root.read_with(cx, |root, _| {
                let playback = &root.preview_ui.playback;
                let expected = playback.duration() * f64::from(fraction);
                assert!((playback.current_time() - expected).abs() < 0.01);
            });
        }
    }
}

fn assert_disjoint(a: Bounds<Pixels>, b: Bounds<Pixels>) {
    let overlap_x = (a.right().min(b.right()) - a.left().max(b.left())).as_f32();
    let overlap_y = (a.bottom().min(b.bottom()) - a.top().max(b.top())).as_f32();
    assert!(
        overlap_x < 0.01 || overlap_y < 0.01,
        "overlapping targets: {a:?}, {b:?}"
    );
}

#[test]
fn tiny_crop_handles_remain_separate_at_both_interface_scales() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewCrop, scale);
        root.update(cx, |root, cx| {
            root.preview_ui.draft_crop = Some(CropRect {
                x: 0.4,
                y: 0.4,
                width: 0.001,
                height: 0.001,
            });
            cx.notify();
        });
        draw(cx);
        let bounds: Vec<_> = [
            "preview-crop-handle-nw",
            "preview-crop-handle-n",
            "preview-crop-handle-ne",
            "preview-crop-handle-e",
            "preview-crop-handle-se",
            "preview-crop-handle-s",
            "preview-crop-handle-sw",
            "preview-crop-handle-w",
        ]
        .map(|id| cx.debug_bounds(id).unwrap())
        .into_iter()
        .collect();
        for (index, a) in bounds.iter().enumerate() {
            assert!(a.size.width > px(0.0) && a.size.height > px(0.0));
            for b in &bounds[index + 1..] {
                assert_disjoint(*a, *b);
            }
        }
    }
}

#[test]
fn overlay_corner_padding_starts_resize() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        root.update(cx, |root, cx| {
            root.update_selected_config(|config| {
                config.overlay = Some(OverlaySettings {
                    enabled: true,
                    path: "/tmp/frame-overlay-hit-test.png".into(),
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
        root.update(cx, |root, cx| {
            root.set_selected_overlay_mode(true);
            cx.notify();
        });
        draw(cx);
        let bounds = cx.debug_bounds("preview-overlay-handle-se").unwrap();
        assert!((24.0_f32.mul_add(-scale.factor(), bounds.size.width.as_f32())).abs() < 0.01);
        let before = root.read_with(cx, |root, _| root.preview_ui.overlay.overlay().cloned());
        let from = bounds.center() + point(px(9.0 * scale.factor()), px(0.0));
        drag(cx, from, from + point(px(35.0), px(25.0)));
        assert_ne!(
            root.read_with(cx, |root, _| root.preview_ui.overlay.overlay().cloned()),
            before
        );
    }
}

#[test]
fn trim_targets_stay_separate_and_do_not_cover_play() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
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
            let start = cx.debug_bounds("preview-timeline-start-handle").unwrap();
            let end = cx.debug_bounds("preview-timeline-end-handle").unwrap();
            assert_disjoint(start, end);
            assert!(start.size.width > px(0.0) && end.size.width > px(0.0));
            let play = cx.debug_bounds("preview-playback-toggle").unwrap();
            assert_disjoint(end, play);
        }
    }
}

#[test]
fn trim_handle_padding_changes_start_without_moving_end() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    let bounds = cx.debug_bounds("preview-timeline-start-handle").unwrap();
    let end = root.read_with(cx, |root, _| root.preview_ui.playback.end_value());
    let from = point(bounds.right() - px(1.0), bounds.center().y);
    drag(cx, from, from + point(px(50.0), px(0.0)));
    root.read_with(cx, |root, _| {
        assert!(root.preview_ui.playback.start_value() > 0.0);
        assert!((root.preview_ui.playback.end_value() - end).abs() < f64::EPSILON);
    });
}

#[test]
fn both_color_picker_hue_sliders_accept_clicks_in_the_new_padding() {
    for (trigger, slider, outline) in [
        (
            "settings-subtitle-font-color",
            "settings-subtitle-font-color-hue",
            false,
        ),
        (
            "settings-subtitle-outline-color",
            "settings-subtitle-outline-color-hue",
            true,
        ),
    ] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = test_window(&mut app, VisualFixture::SettingsSubtitles, scale);
            root.update(cx, |root, cx| {
                root.subtitle_ui.mode = SettingsSubtitleMode::BurnIn;
                root.update_selected_config(|config| {
                    config.subtitle_font_color = Some("#ff0000".into());
                    config.subtitle_outline_color = Some("#ff0000".into());
                    true
                });
                cx.notify();
            });
            draw(cx);
            click(cx, trigger);
            let bounds = cx.debug_bounds(slider).unwrap();
            assert!(
                24.0_f32
                    .mul_add(-scale.factor(), bounds.size.height.as_f32())
                    .abs()
                    < 0.01
            );
            cx.simulate_click(
                point(bounds.center().x, bounds.top() + px(scale.factor())),
                Modifiers::default(),
            );
            draw(cx);
            let color = root.read_with(cx, |root, _| {
                let config = root.selected_config().unwrap();
                if outline {
                    config.subtitle_outline_color.clone()
                } else {
                    config.subtitle_font_color.clone()
                }
            });
            assert_ne!(color.as_deref(), Some("#ff0000"));
        }
    }
}

#[test]
fn output_sections_start_closed_and_keyboard_enters_only_expanded_controls() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        assert!(root.read_with(cx, |root, _| {
            root.settings_ui.expanded_output_sections.is_empty()
        }));
        click(cx, "settings-tab-output");
        for id in [
            "output-mode-reencode",
            "resolution-tabs",
            "audio-codec-select",
            "subtitle-type",
            "metadata-title-field",
        ] {
            assert!(cx.debug_bounds(id).is_none(), "closed section mounted {id}");
        }
        let tab = focus(cx);
        press(cx, "tab");
        let general = focus(cx);
        press(cx, "space");
        assert!(cx.debug_bounds("output-mode-reencode").is_some());
        assert_eq!(focus(cx), general);
        press(cx, "tab");
        assert_ne!(focus(cx), general);
        press(cx, "shift-tab");
        assert_eq!(focus(cx), general);
        press(cx, "enter");
        assert!(cx.debug_bounds("output-mode-reencode").is_none());
        press(cx, "shift-tab");
        assert_eq!(focus(cx), tab);
        press(cx, "tab");
        press(cx, "tab");
        let video = focus(cx);
        press(cx, "enter");
        assert!(cx.debug_bounds("resolution-tabs").is_some());
        press(cx, "tab");
        let resolution = focus(cx);
        assert_ne!(resolution, video);
        press(cx, "end");
        assert_eq!(
            root.read_with(cx, |root, _| root
                .selected_config()
                .unwrap()
                .resolution
                .clone()),
            "custom"
        );
        press(cx, "shift-tab");
        assert_eq!(focus(cx), video);
        press(cx, "space");
        assert!(cx.debug_bounds("resolution-tabs").is_none());
    }
}

#[test]
fn resolution_tabs_preserve_value_and_focus_when_reopened_after_section_collapse() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    click(cx, "video-resolution-custom");
    press(cx, "home");
    assert_eq!(
        root.read_with(cx, |root, _| root
            .selected_config()
            .unwrap()
            .resolution
            .clone()),
        "original"
    );
    press(cx, "tab");
    assert_ne!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Video width"
    );
    click(cx, "output-section-video-trigger");
    assert!(cx.debug_bounds("resolution-tabs").is_none());
    press(cx, "enter");
    assert!(
        cx.debug_bounds("resolution-tabs").is_some(),
        "Enter must reopen the focused section"
    );
    press(cx, "tab");
    press(cx, "end");
    assert_eq!(
        root.read_with(cx, |root, _| root
            .selected_config()
            .unwrap()
            .resolution
            .clone()),
        "custom"
    );
    press(cx, "shift-tab");
    press(cx, "tab");
    press(cx, "tab");
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Video width"
    );
}
