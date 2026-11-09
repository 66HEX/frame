use crate::app::*;
use crate::preview_engine::{
    PreviewRenderPresentation, PreviewSession, rendered_frame_from_bgra_payload,
};
use frame_core::aspect::DisplayRatio;
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};
use std::sync::Arc;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
}

fn focused_visible_label(cx: &mut VisualTestContext) -> String {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        let focus = tree["gpui_focus"]
            .as_str()
            .expect("keyboard must have a focused node");
        let node = &tree["nodes"][focus];
        let bounds = &node["bounds"];
        assert!(
            bounds["width"].as_f64().unwrap() > 0.0 && bounds["height"].as_f64().unwrap() > 0.0,
            "{node}"
        );
        let x = bounds["x"].as_f64().unwrap();
        let y = bounds["y"].as_f64().unwrap();
        assert!(
            x >= 0.0
                && y >= 0.0
                && x + bounds["width"].as_f64().unwrap() <= 1800.01
                && y + bounds["height"].as_f64().unwrap() <= 1200.01,
            "{node}"
        );
        assert_ne!(node["aria"]["disabled"], true);
        node["aria"]["label"]
            .as_str()
            .expect("visible named focus target")
            .to_owned()
    })
}

fn key(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

fn begin_drag(cx: &mut VisualTestContext, selector: &'static str) -> Point<Pixels> {
    let start = cx.debug_bounds(selector).unwrap().center();
    cx.simulate_mouse_move(start, None, Modifiers::default());
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    let next = start + point(px(12.0), px(8.0));
    cx.simulate_mouse_move(next, MouseButton::Left, Modifiers::default());
    draw(cx);
    assert!(cx.update(|_, cx| cx.has_active_drag()));
    // The threshold move starts GPUI's payload; the next move reaches the
    // root handler and establishes its crop/overlay drag origin.
    let next = next + point(px(8.0), px(6.0));
    cx.simulate_mouse_move(next, MouseButton::Left, Modifiers::default());
    draw(cx);
    next
}

fn seed_session(root: &Entity<FrameRoot>, cx: &mut VisualTestContext) {
    root.update(cx, |root, cx| {
        let entry = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&entry).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(request.config));
        let frame = rendered_frame_from_bgra_payload(320, 180, 1280, 0, vec![123; 320 * 180 * 4])
            .unwrap()
            .with_presentation(
                PreviewRenderPresentation {
                    pixel_aspect_ratio: DisplayRatio::new(2, 1).unwrap(),
                    ..Default::default()
                },
                0,
            );
        let _ = session.frame_store().publish(frame);
        root.preview_ui.pending_runtime_key = None;
        root.preview_ui.runtime_key = Some(request.key.clone());
        root.preview_ui.desired_runtime_key = Some(request.key);
        root.preview_ui.session = Some(session);
        assert!(root.refresh_preview_render_image(cx));
    });
    draw(cx);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The native gesture regression preserves one uninterrupted drag across pending and accepted requests before checking recovery."
)]
fn stale_native_gestures_cannot_resume_after_a_reconfigured_frame() {
    for (fixture, selector) in [
        (
            VisualFixture::PreviewAnamorphicCrop,
            "preview-crop-move-handle",
        ),
        (
            VisualFixture::PreviewAnamorphicOverlay,
            "preview-overlay-layer",
        ),
    ] {
        let mut app = TestAppContext::single();
        let (root, cx) = app.add_window_view(|window, cx| {
            window.activate();
            window.set_a11y_forced(true);
            crate::app::init_app(cx, "Preview gesture ownership test");
            cx.set_reduce_motion(true);
            let mut root = FrameRoot::new();
            root.apply_visual_fixture(Some(fixture));
            // This test uses actual session/completion ownership, never the
            // deterministic snapshot hook from those visual fixtures.
            root.preview_ui.static_fixture_file_id = None;
            let entry = root.selected_source_metadata_entry();
            root.preview_ui.pending_runtime_key = root
                .selected_preview_runtime_request(&entry)
                .map(|request| request.key);
            root
        });
        cx.simulate_resize(size(px(1800.0), px(1200.0)));
        for _ in 0..3 {
            draw(cx);
        }
        seed_session(&root, cx);
        let handle = cx.debug_bounds(selector).unwrap();
        cx.simulate_click(handle.center(), Modifiers::default());
        draw(cx);
        let handle_label = focused_visible_label(cx);
        assert_eq!(
            handle_label,
            if fixture == VisualFixture::PreviewAnamorphicCrop {
                "Move crop selection"
            } else {
                "Move overlay image"
            }
        );
        let old_position = begin_drag(cx, selector);
        let toolbar = if fixture == VisualFixture::PreviewAnamorphicCrop {
            "preview-crop-toolbar"
        } else {
            "preview-overlay-toolbar"
        };
        let toolbar_bounds = cx.debug_bounds(toolbar).unwrap();
        root.read_with(cx, |root, _| {
            if fixture == VisualFixture::PreviewAnamorphicCrop {
                assert!(root.preview_ui.crop_drag.is_some());
            } else {
                assert!(root.preview_ui.overlay.is_dragging());
            }
        });
        let saved = root.read_with(cx, |root, _| {
            (
                root.preview_ui.draft_crop,
                root.preview_ui.overlay.overlay().cloned(),
                root.selected_config().unwrap().clone(),
            )
        });
        root.update(cx, |root, cx| {
            root.file_queue.selected_file_mut().unwrap().config.rotation = "90".into();
            let entry = root.selected_source_metadata_entry();
            let request = root.selected_preview_runtime_request(&entry).unwrap();
            root.sync_preview_runtime_for_selection(Some(request), cx);
            assert!(root.preview_ui.pending_runtime_key.is_some());
            let token = root.preview_ui.runtime_request_generation;
            assert!(token > 0);
            let frame =
                rendered_frame_from_bgra_payload(180, 320, 720, 0, vec![124; 180 * 320 * 4])
                    .unwrap()
                    .with_presentation(
                        PreviewRenderPresentation {
                            pixel_aspect_ratio: DisplayRatio::new(1, 2).unwrap(),
                            ..Default::default()
                        },
                        token,
                    );
            let _ = root
                .preview_ui
                .session
                .as_ref()
                .unwrap()
                .frame_store()
                .publish(frame);
        });
        // Preserve the toolbar and draft values while hiding only unsafe canvas
        // controls; the published B frame has not been accepted by the UI yet.
        draw(cx);
        assert_eq!(cx.debug_bounds(toolbar), Some(toolbar_bounds));
        assert!(cx.debug_bounds(selector).is_none());
        // A key aimed at the removed canvas control cannot change its geometry.
        key(cx, "right");
        root.read_with(cx, |root, _| {
            assert_eq!(root.preview_ui.draft_crop, saved.0);
            assert_eq!(root.preview_ui.overlay.overlay(), saved.1.as_ref());
        });
        for direction in ["tab", "shift-tab"] {
            key(cx, direction);
            assert_ne!(focused_visible_label(cx), handle_label);
        }
        cx.update(|window, cx| {
            root.update(cx, |root, cx| {
                let first = if fixture == VisualFixture::PreviewAnamorphicCrop {
                    "preview-crop-action-free"
                } else {
                    "preview-overlay-replace"
                };
                assert!(root.focus_registered_control(first, window, cx));
            });
        });
        draw(cx);
        assert_eq!(
            focused_visible_label(cx),
            if fixture == VisualFixture::PreviewAnamorphicCrop {
                "Crop aspect ratio"
            } else {
                "Replace overlay image"
            }
        );
        // No pointer movement occurs while B is pending. Its session has no
        // decoder in this test; the actual async completion accepts our B frame.
        cx.run_until_parked();
        draw(cx);
        root.read_with(cx, |root, _| {
            assert!(root.preview_ui.static_fixture_file_id.is_none());
            assert!(root.preview_ui.pending_runtime_key.is_none());
            assert!(root.preview_edit_frame_ready());
            assert_eq!(root.preview_ui.draft_crop, saved.0);
            assert_eq!(root.preview_ui.overlay.overlay(), saved.1.as_ref());
        });
        assert!(cx.update(|_, cx| cx.has_active_drag()));
        let stage = cx.debug_bounds("preview-media-stage").unwrap();
        cx.simulate_mouse_move(stage.center(), MouseButton::Left, Modifiers::default());
        draw(cx);
        assert!(!cx.update(|_, cx| cx.has_active_drag()));
        root.read_with(cx, |root, _| {
            assert!(root.preview_ui.crop_drag.is_none());
            assert!(!root.preview_ui.overlay.is_dragging());
            assert_eq!(root.preview_ui.draft_crop, saved.0);
            assert_eq!(root.preview_ui.overlay.overlay(), saved.1.as_ref());
            let mut expected = saved.2.clone();
            expected.rotation = "90".into();
            assert_eq!(root.selected_config(), Some(&expected));
        });
        cx.simulate_mouse_up(old_position, MouseButton::Left, Modifiers::default());
        let fresh = begin_drag(cx, selector);
        cx.simulate_mouse_move(
            fresh + point(px(18.0), px(12.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        draw(cx);
        root.read_with(cx, |root, _| {
            assert!(
                root.preview_ui.draft_crop != saved.0
                    || root.preview_ui.overlay.overlay() != saved.1.as_ref()
            );
        });
        cx.simulate_mouse_up(fresh, MouseButton::Left, Modifiers::default());
        root.update(cx, |root, cx| root.clear_preview_runtime(cx).detach());
        cx.run_until_parked();
    }
}
