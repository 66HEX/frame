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
            revealed: std::cell::RefCell::default(),
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
fn source_frame_dialog_never_changes_the_preview_bounds() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, ColorTheme::Dark, scale);
        let before = cx.debug_bounds("preview-viewport").unwrap();
        let request = SourceFrameExportRequest::for_test(
            "dialog-layout".into(),
            "source-frame".into(),
            PathBuf::from("/tmp/source.mov"),
            0,
            0,
            PathBuf::from("/tmp/frame.png"),
        );
        root.update(cx, |root, _| test_support::begin(root, request));
        draw(cx);
        assert_eq!(
            cx.debug_bounds("preview-viewport").unwrap(),
            before,
            "starting an export must not shrink the preview"
        );
        root.update(cx, |root, _| {
            assert!(test_support::complete(
                root,
                "dialog-layout",
                SourceFrameExportOutcome::Published {
                    path: PathBuf::from("/tmp/published.png"),
                }
            ));
        });
        draw(cx);
        assert_eq!(
            cx.debug_bounds("preview-viewport").unwrap(),
            before,
            "publishing a result must not shrink the preview"
        );
        assert!(cx.debug_bounds("source-frame-export-toolbar").is_none());
        assert!(
            cx.debug_bounds("source-frame-dialog").is_some(),
            "completion opens a result dialog"
        );
    }
}

#[test]
fn source_frame_icon_does_not_blink_during_same_source_output_refresh() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, ColorTheme::Dark, scale);
        let before = cx.debug_bounds("source-frame-export").unwrap();
        root.update(cx, |root, cx| {
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .video_bitrate = "2500k".into();
            root.preview_ui.pending_runtime_key = root.preview_ui.runtime_key.clone();
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            cx.debug_bounds("source-frame-export"),
            Some(before),
            "refreshing processed output must not hide or move the source-frame icon"
        );
        root.read_with(cx, |root, _| {
            assert!(
                root.source_frame_export_render_state()
                    .disabled_reason
                    .is_some()
            );
        });
        cx.simulate_click(before.center(), gpui::Modifiers::default());
        cx.update(|_, cx| {
            assert!(
                cx.global::<SourceFrameRequests>()
                    .exports
                    .borrow()
                    .is_empty()
            );
        });
        root.update(cx, |root, cx| {
            root.preview_ui.pending_runtime_key = None;
            cx.notify();
        });
        draw(cx);
        assert_eq!(cx.debug_bounds("source-frame-export"), Some(before));
        root.read_with(cx, |root, _| {
            assert!(
                root.source_frame_export_render_state()
                    .disabled_reason
                    .is_none()
            );
        });
    }
}

#[test]
fn source_frame_dialog_traps_focus_until_cancel_settles_and_restores_trigger() {
    for error in [false, true] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
        tab_to(cx, "Export source frame");
        press(cx, "enter");
        let id = cx.update(|_, cx| {
            cx.global::<SourceFrameRequests>().exports.borrow()[0]
                .id
                .clone()
        });
        assert_eq!(focused_label(cx).as_deref(), Some("Cancel export"));
        assert!(cx.debug_bounds("source-frame-close").is_none());
        press(cx, "escape");
        assert!(cx.debug_bounds("source-frame-dialog").is_some());
        press(cx, "tab");
        assert_eq!(focused_label(cx).as_deref(), Some("Cancel export"));
        press(cx, "enter");
        assert!(cx.debug_bounds("source-frame-dialog").is_some());
        root.update(cx, |root, _| {
            if error {
                assert!(test_support::fail(root, &id, "The source was removed."));
            } else {
                assert!(test_support::complete(
                    root,
                    &id,
                    SourceFrameExportOutcome::Cancelled
                ));
            }
        });
        draw(cx);
        assert_eq!(focused_label(cx).as_deref(), Some("Close"));
        assert!(cx.debug_bounds("source-frame-open").is_none());
        press(cx, "tab");
        assert_eq!(
            focused_label(cx).as_deref(),
            Some("Close source frame dialog")
        );
        press(cx, "tab");
        assert_eq!(focused_label(cx).as_deref(), Some("Close"));
        press(cx, "escape");
        assert!(cx.debug_bounds("source-frame-dialog").is_none());
        assert_eq!(focused_label(cx).as_deref(), Some("Export source frame"));
    }
}

#[test]
fn source_frame_result_reveal_and_open_survive_source_removal_and_dialog_dismissal() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Light, ScalePreset::Percent100);
    let path = root.read_with(cx, |root, _| {
        root.default_output_directory
            .as_ref()
            .unwrap()
            .join("published.png")
    });
    std::fs::write(&path, b"actual published path fixture").unwrap();
    root.update(cx, |root, _| {
        test_support::begin(
            root,
            SourceFrameExportRequest::for_test(
                "result-actions".into(),
                "source-frame".into(),
                PathBuf::from("/tmp/source.mov"),
                0,
                0,
                path.clone(),
            ),
        );
        root.file_queue.remove_file("source-frame");
        assert!(test_support::complete(
            root,
            "result-actions",
            SourceFrameExportOutcome::Published { path: path.clone() }
        ));
    });
    draw(cx);
    assert!(cx.debug_bounds("preview-panel").is_none());
    tab_to(cx, "Show in folder");
    press(cx, "enter");
    tab_to(cx, "Open image");
    press(cx, "enter");
    cx.update(|_, cx| {
        assert_eq!(
            *cx.global::<SourceFrameRequests>().revealed.borrow(),
            vec![path.clone()]
        );
        assert_eq!(
            *cx.global::<SourceFrameRequests>().opened.borrow(),
            vec![path.clone()]
        );
    });
    press(cx, "escape");
    assert!(cx.debug_bounds("source-frame-dialog").is_none());
    assert!(cx.debug_bounds("source-frame-export-toolbar").is_none());
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.preview_ui.source_export.result.as_ref().unwrap().path,
            path
        );
    });
    cx.update(|window, cx| assert!(window.focused(cx).is_some()));
}

#[test]
fn source_frame_result_dialog_keeps_its_content_during_animated_exit() {
    use std::time::{Duration, Instant};
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    let now = Instant::now();
    cx.set_transition_time(now);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    root.update(cx, |root, cx| {
        test_support::begin(
            root,
            SourceFrameExportRequest::for_test(
                "animated".into(),
                "source-frame".into(),
                PathBuf::from("/tmp/source.mov"),
                0,
                0,
                PathBuf::from("/tmp/frame.png"),
            ),
        );
        assert!(test_support::complete(
            root,
            "animated",
            SourceFrameExportOutcome::Published {
                path: PathBuf::from("/tmp/frame.png")
            }
        ));
        cx.notify();
    });
    draw(cx);
    cx.set_transition_time(now + Duration::from_millis(100));
    draw(cx);
    press(cx, "escape");
    assert!(cx.debug_bounds("source-frame-dialog").is_some());
    cx.set_transition_time(now + Duration::from_millis(150));
    draw(cx);
    assert!(
        cx.debug_bounds("source-frame-open").is_some(),
        "result actions remain mounted through exit"
    );
    cx.set_transition_time(now + Duration::from_millis(200));
    draw(cx);
    assert!(cx.debug_bounds("source-frame-dialog").is_none());
    assert_eq!(focused_label(cx).as_deref(), Some("Export source frame"));
    root.read_with(cx, |root, _| {
        assert!(root.preview_ui.source_export.result.is_some());
    });
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
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(700));
    draw(cx);
    assert!(cx.debug_bounds("tooltip-source-frame-export").is_some());
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
    assert_eq!(focused_label(cx).as_deref(), Some("Cancel export"));
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
    tab_to(cx, "Cancel export");
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
    tab_to(cx, "Open image");
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
    tab_to(cx, "Open image");
    press(cx, "enter");
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        let (_, prompt) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Source frame not found"))
            .expect("missing result dialog");
        assert!(
            prompt
                .description()
                .unwrap()
                .contains(path.to_str().unwrap())
        );
    });
    let close = cx
        .debug_bounds("frame-prompt-action-0")
        .expect("custom prompt Close action");
    cx.simulate_click(close.center(), gpui::Modifiers::default());
    draw(cx);
    assert_eq!(focused_label(cx).as_deref(), Some("Open image"));
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
        tab_to(cx, "Cancel export");
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
fn source_frame_export_control_is_a_named_icon_inside_the_viewport_toolbar() {
    for theme in ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (_root, cx) = window(&mut app, theme, scale);
            let button = cx
                .debug_bounds("source-frame-export")
                .expect("Export source frame must be visible in the preview panel");
            let viewport = cx.debug_bounds("preview-viewport").unwrap();
            let panel = cx.debug_bounds("preview-panel").unwrap();
            assert!(button.top() >= viewport.top());
            assert!(
                button.bottom() <= viewport.bottom(),
                "button={button:?}, viewport={viewport:?}"
            );
            assert!((button.size.width - button.size.height).abs() <= px(1.0));
            assert!(cx.debug_bounds("source-frame-export-toolbar").is_none());
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

#[test]
fn source_frame_export_icon_is_hidden_when_the_source_cannot_export() {
    for reason in ["folder", "audio", "busy"] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
        root.update(cx, |root, cx| {
            match reason {
                "folder" => root.default_output_directory = None,
                "audio" => root.source_metadata.mark_ready(
                    "source-frame",
                    SourceMetadata {
                        media_kind: Some(SourceKind::Audio),
                        ..SourceMetadata::default()
                    },
                ),
                _ => {
                    test_support::begin(
                        root,
                        SourceFrameExportRequest::for_test(
                            "busy".into(),
                            "source-frame".into(),
                            PathBuf::from("/tmp/source.mov"),
                            0,
                            0,
                            PathBuf::from("/tmp/frame.png"),
                        ),
                    );
                }
            }
            cx.notify();
        });
        draw(cx);
        assert!(
            cx.debug_bounds("source-frame-export").is_none(),
            "hidden for {reason}"
        );
        cx.update(|window, _| {
            assert!(
                !window
                    .a11y_tree()
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|(_, node)| node.label() == Some("Export source frame"))
            );
        });
    }
}

#[test]
fn source_frame_export_icon_remains_available_when_transform_tools_are_hidden() {
    for audio_only in [true, false] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent200);
        root.update(cx, |root, cx| {
            if audio_only {
                root.file_queue
                    .selected_file_mut()
                    .unwrap()
                    .config
                    .container = "wav".into();
            } else {
                root.preview_ui.transport.open = true;
            }
            cx.notify();
        });
        draw(cx);
        root.read_with(cx, |root, _| {
            assert!(
                root.source_frame_export_render_state()
                    .disabled_reason
                    .is_none()
            );
        });
        let button = cx
            .debug_bounds("source-frame-export")
            .expect("source frame action remains available");
        let viewport = cx.debug_bounds("preview-viewport").unwrap();
        assert!(button.left() >= viewport.left() && button.top() >= viewport.top());
        assert!(button.right() <= viewport.right() && button.bottom() <= viewport.bottom());
        assert!(cx.debug_bounds("preview-tool-rotate").is_none());
        tab_to(cx, "Export source frame");
        press(cx, "enter");
        cx.update(|_, cx| assert_eq!(cx.global::<SourceFrameRequests>().exports.borrow().len(), 1));
    }
}
