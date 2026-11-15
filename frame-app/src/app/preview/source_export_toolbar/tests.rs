use crate::app::preview::actions::source_export::test_support::{self, SourceFrameRequests};
use crate::app::root::FrameRoot;
use crate::appearance::{ColorTheme, ScalePreset};
use crate::file_queue::FileItem;
use crate::frame_export::{SourceFrameExportOutcome, SourceFrameExportRequest};
use crate::preview_engine::rendered_frame_from_bgra_payload;
use crate::settings::{SourceKind, SourceMetadata};
use gpui::{Entity, TestAppContext, VisualTestContext, px, size};
use std::path::PathBuf;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn window(
    app: &mut TestAppContext,
    theme: ColorTheme,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    static NEXT_SOURCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let source_directory = std::env::temp_dir().join(format!(
        "frame-source-ui-{}-{}",
        std::process::id(),
        NEXT_SOURCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    std::fs::create_dir(&source_directory).unwrap();
    let source_path = source_directory.join("source-frame-test.mov");
    std::fs::write(&source_path, b"source identity fixture").unwrap();
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Source frame export test");
        cx.set_reduce_motion(true);
        cx.set_global(SourceFrameRequests {
            source_directory: Some(source_directory.clone()),
            exports: std::cell::RefCell::default(),
            opened: std::cell::RefCell::default(),
        });
        let mut root = FrameRoot::new();
        root.appearance.color_theme = theme;
        root.appearance.ui_scale = scale;
        root.file_queue.add_file(FileItem::from_path(
            "source-frame",
            source_path.to_str().unwrap(),
            100,
        ));
        root.default_output_directory = Some(source_directory);
        root.source_metadata.mark_ready(
            "source-frame",
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                video_codec: Some("h264".into()),
                video_stream_index: Some(0),
                width: Some(320),
                height: Some(180),
                duration: Some("10".into()),
                ..SourceMetadata::default()
            },
        );
        let frame =
            rendered_frame_from_bgra_payload(320, 180, 1280, 0, vec![0; 320 * 180 * 4]).unwrap();
        root.preview_ui.render_image = Some(frame.render_image());
        root.preview_ui.static_fixture_file_id = Some("source-frame".into());
        root
    });
    cx.update(|window, _| {
        window.activate();
        window.set_a11y_forced(true);
    });
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    draw(cx);
    (root, cx)
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

fn focused_label(cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        tree["gpui_focus"].as_str().and_then(|id| {
            tree["nodes"][id]["aria"]["label"]
                .as_str()
                .map(str::to_owned)
        })
    })
}

fn tab_to(cx: &mut VisualTestContext, label: &str) {
    for _ in 0..100 {
        press(cx, "tab");
        if focused_label(cx).as_deref() == Some(label) {
            return;
        }
    }
    panic!("keyboard must reach {label}");
}

#[test]
fn source_frame_ui_keyboard_starts_a_frozen_source_request() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    root.update(cx, |root, _| {
        root.preview_ui
            .playback
            .sync_media(crate::preview::MediaSnapshot {
                current_time: 2.125_001,
                duration: 10.0,
                paused: true,
            });
    });
    tab_to(cx, "Export source frame");
    press(cx, "enter");
    let requests = cx.update(|_, cx| cx.global::<SourceFrameRequests>().exports.borrow().clone());
    assert_eq!(
        requests.len(),
        1,
        "keyboard action must begin exactly one frame export"
    );
    assert_eq!(requests[0].position_microseconds, 2_125_001);
    assert_eq!(requests[0].source_id, "source-frame");
    assert_eq!(
        requests[0].source_path,
        root.read_with(cx, |root, _| PathBuf::from(
            &root.file_queue.selected_file().unwrap().path
        ))
    );
    assert_eq!(
        focused_label(cx).as_deref(),
        Some("Cancel source frame export")
    );
}

#[test]
fn source_frame_ui_cancel_uses_the_dedicated_attempt() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent200);
    let request = SourceFrameExportRequest::for_test(
        "ui-export".into(),
        "source-frame".into(),
        PathBuf::from("/tmp/source-frame-test.mov"),
        0,
        0,
        PathBuf::from("/tmp/nominal.png"),
    );
    let controller = root.update(cx, |root, _| test_support::begin(root, request));
    draw(cx);
    tab_to(cx, "Cancel source frame export");
    press(cx, "enter");
    assert!(
        controller.is_cancelled("ui-export"),
        "Cancel must signal the dedicated export id"
    );
    assert!(!controller.is_cancelled("source-frame"));
}

#[test]
fn source_frame_ui_open_uses_the_actual_result_after_output_settings_change() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent200);
    let request = SourceFrameExportRequest::for_test(
        "ui-export".into(),
        "source-frame".into(),
        PathBuf::from("/tmp/source-frame-test.mov"),
        0,
        0,
        PathBuf::from("/tmp/nominal.png"),
    );
    root.update(cx, |root, _| test_support::begin(root, request));
    let path =
        std::env::temp_dir().join(format!("frame-source-ui-result-{}.png", std::process::id()));
    std::fs::write(&path, b"published PNG test path").unwrap();
    root.update(cx, |root, _| {
        root.default_output_directory = Some(PathBuf::from("/tmp/later-folder"));
        root.file_queue.selected_file_mut().unwrap().output_name = "changed-after-export".into();
        assert!(test_support::complete(
            root,
            "ui-export",
            SourceFrameExportOutcome::Published { path: path.clone() }
        ));
    });
    draw(cx);
    tab_to(cx, "Open result");
    press(cx, "enter");
    let opened = cx.update(|_, cx| cx.global::<SourceFrameRequests>().opened.borrow().clone());
    std::fs::remove_file(&path).unwrap();
    assert_eq!(opened, vec![path]);
    root.read_with(cx, |root, _| {
        assert!(
            root.file_queue
                .selected_file()
                .unwrap()
                .completed_output_path()
                .is_none()
        );
    });
}

#[test]
fn source_frame_ui_open_reports_a_missing_result_without_changing_the_queue() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    let path = std::env::temp_dir().join(format!(
        "frame-missing-source-result-{}.png",
        std::process::id()
    ));
    let request = SourceFrameExportRequest::for_test(
        "missing-export".into(),
        "source-frame".into(),
        PathBuf::from("/tmp/source-frame-test.mov"),
        0,
        0,
        path.clone(),
    );
    root.update(cx, |root, _| {
        test_support::begin(root, request);
        assert!(test_support::complete(
            root,
            "missing-export",
            SourceFrameExportOutcome::Published { path: path.clone() }
        ));
    });
    let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
    draw(cx);
    tab_to(cx, "Open result");
    press(cx, "enter");
    let (title, detail) = cx
        .pending_prompt()
        .expect("missing source frame must produce a visible error");
    assert_eq!(title, "Source frame not found");
    assert!(detail.contains(path.to_str().unwrap()));
    cx.simulate_prompt_answer("Close");
    draw(cx);
    assert_eq!(focused_label(cx).as_deref(), Some("Open result"));
    assert_eq!(
        root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
        before
    );
}

#[test]
fn source_frame_ui_keeps_actions_when_the_preview_disappears() {
    for show_logs in [false, true] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent200);
        let request = SourceFrameExportRequest::for_test(
            "detached-export".into(),
            "source-frame".into(),
            PathBuf::from("/tmp/source-frame-test.mov"),
            0,
            0,
            PathBuf::from("/tmp/frame.png"),
        );
        let controller = root.update(cx, |root, _| {
            let controller = test_support::begin(root, request);
            if show_logs {
                root.active_view = crate::app::model::ActiveView::Logs;
            } else {
                root.file_queue.remove_file("source-frame");
            }
            controller
        });
        draw(cx);
        assert!(
            cx.debug_bounds("source-frame-cancel").is_some(),
            "Cancel must remain reachable outside the preview"
        );
        tab_to(cx, "Cancel source frame export");
        press(cx, "enter");
        assert!(controller.is_cancelled("detached-export"));
        cx.update(|window, _| {
            let tree: serde_json::Value =
                serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
            let focus = tree["gpui_focus"]
                .as_str()
                .expect("cancelling must keep focus on a visible element");
            assert!(
                !tree["nodes"][focus].is_null(),
                "focus must not move to the hidden preview"
            );
        });
        root.update(cx, |root, _| {
            assert!(test_support::complete(
                root,
                "detached-export",
                SourceFrameExportOutcome::Published {
                    path: PathBuf::from("/tmp/final.png")
                }
            ));
        });
        draw(cx);
        assert!(
            cx.debug_bounds("source-frame-open").is_some(),
            "Published result must remain reachable outside the preview"
        );
    }
}

#[test]
fn source_frame_export_control_is_named_and_outside_the_viewport() {
    for theme in ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (_root, cx) = window(&mut app, theme, scale);
            let button = cx
                .debug_bounds("source-frame-export")
                .expect("Export source frame must be visible in the preview panel");
            let viewport = cx.debug_bounds("preview-viewport").unwrap();
            let panel = cx.debug_bounds("preview-panel").unwrap();
            assert!(button.top() >= viewport.bottom());
            assert!(button.left() >= panel.left());
            assert!(button.right() <= panel.right());
            assert!(button.bottom() <= panel.bottom());
            let tree: serde_json::Value = cx.update(|window, _| {
                serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap()
            });
            assert!(tree["nodes"].as_object().unwrap().values().any(|node| {
                node["aria"]["label"] == "Export source frame"
                    && node["aria"]["description"]
                        .as_str()
                        .is_some_and(|description| {
                            description.contains("without conversion settings")
                        })
            }));
        }
    }
}
