//! Exercise native file-drag events through the rendered app, including its overlay.
use crate::VisualFixture;
use crate::app::FrameRoot;
use crate::appearance::ScalePreset;
use gpui::{
    Bounds, Entity, ExternalPaths, FileDropEvent, FocusHandle, Pixels, TestAppContext,
    VisualTestContext, point, px, size,
};
use std::{path::PathBuf, time::Duration};

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
        crate::app::init_app(cx, "Frame file drop test");
        cx.set_reduce_motion(true);
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(fixture));
        root.appearance.ui_scale = scale;
        // Keep populated fixtures from starting an asynchronous media preview.
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        root
    });
    cx.update(|window, _| window.activate());
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    draw(cx);
    (root, cx)
}

fn focus(cx: &mut VisualTestContext) -> FocusHandle {
    cx.update(|window, cx| {
        window
            .focused(cx)
            .expect("the existing control stays focused")
    })
}

fn viewport_bounds(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.update(|window, _| Bounds::new(point(px(0.0), px(0.0)), window.viewport_size()))
}

fn assert_full_window_drop_target(cx: &mut VisualTestContext) {
    let overlay = cx
        .debug_bounds("drag-drop-overlay")
        .expect("an active native drag must have a visible drop target");
    let app = viewport_bounds(cx);
    assert_eq!(
        overlay, app,
        "the entire application, including its titlebar, must accept a drop"
    );
}

#[test]
fn native_drag_stays_visible_without_motion_exits_and_reenters_without_stealing_focus() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        for fixture in [VisualFixture::WorkspaceEmpty, VisualFixture::SettingsVideo] {
            let mut app = TestAppContext::single();
            let (root, cx) = test_window(&mut app, fixture, scale);
            let trigger = if fixture == VisualFixture::WorkspaceEmpty {
                "welcome-open-file"
            } else {
                "titlebar-settings"
            };
            cx.update(|window, cx| {
                root.update(cx, |root, cx| {
                    assert!(root.focus_registered_control(trigger, window, cx));
                });
            });
            draw(cx);
            let original_focus = focus(cx);
            let original_count = root.read_with(cx, |root, _| root.file_queue.files().len());
            let position = viewport_bounds(cx).center();

            for _ in 0..2 {
                cx.simulate_event(FileDropEvent::Entered {
                    position,
                    paths: ExternalPaths(smallvec::smallvec![PathBuf::from(
                        "/tmp/frame-drag-lifecycle.mp4",
                    )]),
                });
                draw(cx);
                assert_full_window_drop_target(cx);
                assert_eq!(focus(cx), original_focus);

                cx.simulate_event(FileDropEvent::Pending { position });
                draw(cx);
                // A paused pointer must not dismiss the destination between OS events.
                cx.executor().advance_clock(Duration::from_secs(2));
                draw(cx);
                assert_full_window_drop_target(cx);
                assert_eq!(focus(cx), original_focus);

                cx.simulate_event(FileDropEvent::Exited);
                draw(cx);
                assert!(cx.debug_bounds("drag-drop-overlay").is_none());
                assert_eq!(focus(cx), original_focus);
                assert_eq!(
                    root.read_with(cx, |root, _| root.file_queue.files().len()),
                    original_count,
                    "leaving the window must not import anything"
                );
            }
        }
    }
}

#[test]
fn native_drop_imports_once_at_the_center_and_window_edge() {
    for at_edge in [false, true] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(
            &mut app,
            VisualFixture::WorkspaceEmpty,
            ScalePreset::Percent100,
        );
        let bounds = viewport_bounds(cx);
        let position = if at_edge {
            point(bounds.left() + px(2.0), bounds.top() + px(2.0))
        } else {
            bounds.center()
        };
        // Queueing is independent of metadata success. An absent supported path
        // exercises the real importer without requiring media fixtures or FFmpeg.
        let path = std::env::temp_dir().join(format!(
            "frame-drop-absent-source-{}-{at_edge}.png",
            std::process::id()
        ));
        cx.simulate_event(FileDropEvent::Entered {
            position,
            paths: ExternalPaths(smallvec::smallvec![path.clone()]),
        });
        draw(cx);
        assert_full_window_drop_target(cx);
        cx.simulate_event(FileDropEvent::Submit { position });
        draw(cx);

        root.read_with(cx, |root, _| {
            let files = root.file_queue.files();
            assert_eq!(
                files.len(),
                1,
                "overlay and root must not import the same drop twice"
            );
            assert_eq!(PathBuf::from(&files[0].path), path);
        });
        assert!(cx.debug_bounds("drag-drop-overlay").is_none());

        // Some platforms send these cleanup events after the successful drop.
        cx.simulate_event(FileDropEvent::Exited);
        cx.simulate_event(FileDropEvent::Ended);
        draw(cx);
        assert_eq!(
            root.read_with(cx, |root, _| root.file_queue.files().len()),
            1
        );
        assert!(cx.debug_bounds("drag-drop-overlay").is_none());
    }
}
