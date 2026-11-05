use super::*;
use std::time::Duration;

fn assert_crop_outline_tracks_handles(cx: &mut VisualTestContext) {
    let outline = cx.debug_bounds("preview-crop-move-handle").unwrap();
    for (id, corner) in [
        ("preview-crop-handle-nw", outline.origin),
        ("preview-crop-handle-se", outline.bottom_right()),
    ] {
        let handle = cx.debug_bounds(id).unwrap().center();
        assert!(
            (handle.x - corner.x).abs() <= px(1.0) && (handle.y - corner.y).abs() <= px(1.0),
            "crop outline lags behind {id}: outline {outline:?}, handle {handle:?}"
        );
    }
}

#[test]
fn crop_outline_tracks_handles_on_every_drag_frame() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        for resize in [false, true] {
            let mut app = TestAppContext::single();
            let (_, cx) = test_window(&mut app, VisualFixture::PreviewCropCustom, scale);
            cx.update(|_, cx| cx.set_reduce_motion(false));
            let id = if resize {
                "preview-crop-handle-nw"
            } else {
                "preview-crop-move-handle"
            };
            let from = cx.debug_bounds(id).unwrap().center();
            cx.simulate_mouse_move(from, None, Modifiers::default());
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            for offset in [4.0, 25.0, 50.0, 15.0, -20.0] {
                cx.simulate_mouse_move(
                    from + point(px(offset), px(offset)),
                    MouseButton::Left,
                    Modifiers::default(),
                );
                draw(cx);
                assert_crop_outline_tracks_handles(cx);
                cx.executor().advance_clock(Duration::from_millis(16));
            }
            assert!(cx.update(|_, cx| cx.has_active_drag()));
            cx.simulate_mouse_up(from, MouseButton::Left, Modifiers::default());
            draw(cx);
            assert_crop_outline_tracks_handles(cx);
        }
    }
}

#[test]
fn overlay_position_updates_without_a_vertical_animation_tail() {
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
                path: "/tmp/motion-overlay.png".into(),
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
    cx.update(|_, cx| cx.set_reduce_motion(false));
    for key in ["down", "up"] {
        let before = cx.debug_bounds("preview-overlay-layer").unwrap();
        root.update(cx, |root, cx| {
            assert!(root.adjust_preview_overlay_from_keyboard_with_step(
                crate::preview::OverlayDragHandle::Move,
                key,
                None,
                true,
            ));
            cx.notify();
        });
        draw(cx);
        let immediate = cx.debug_bounds("preview-overlay-layer").unwrap();
        assert_ne!(immediate.origin, before.origin);
        cx.executor().advance_clock(Duration::from_millis(75));
        draw(cx);
        assert_eq!(cx.debug_bounds("preview-overlay-layer").unwrap(), immediate);
    }
}
