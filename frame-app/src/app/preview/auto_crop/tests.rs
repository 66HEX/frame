use super::*;
use crate::app::preview::crop::full_crop_rect;
use crate::app::updates::state::UpdateStatus;
use crate::file_queue::{FileItem, FileStatus};
use crate::preview::{DragHandle, Point};
use crate::settings::{CropSettings, ProcessingMode, SourceMetadata};
use gpui::{TestAppContext, VisualTestContext, px, size};

fn video_root() -> FrameRoot {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/auto-crop.mp4", 1));
    root.source_metadata.mark_ready(
        "video",
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("120".into()),
            video_stream_index: Some(3),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );
    sync_selection(&mut root);
    assert!(root.toggle_selected_crop_mode());
    root
}

fn sync_selection(root: &mut FrameRoot) {
    let id = root.file_queue.selected_file_id().map(str::to_string);
    let config = root.selected_config().cloned().unwrap_or_default();
    root.sync_preview_crop_for_selection(id.as_deref(), &config);
}

fn detected_crop() -> DetectedCrop {
    DetectedCrop {
        x: 100,
        y: 60,
        width: 1600,
        height: 900,
    }
}

fn complete_detection(root: &mut FrameRoot) {
    let (_, cancel) = root.begin_auto_crop().expect("video supports auto crop");
    assert!(root.complete_auto_crop(&cancel, Ok(detected_crop())));
}

#[test]
fn detected_frame_remains_a_draft_and_manual_correction_is_applied() {
    let mut root = video_root();
    let original = root.selected_config().unwrap().clone();
    complete_detection(&mut root);
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Ready);
    assert!(root.preview_ui.crop_mode);
    assert_eq!(root.selected_config(), Some(&original));

    let detected_draft = root.preview_ui.draft_crop;
    assert!(root.apply_preview_crop_drag(DragHandle::Move, Point { x: 0.5, y: 0.5 }));
    assert_eq!(root.preview_ui.draft_crop, detected_draft);
    assert!(root.apply_preview_crop_drag(DragHandle::Move, Point { x: 0.55, y: 0.5 }));
    assert_eq!(root.selected_config(), Some(&original));
    assert!(root.apply_selected_crop());
    let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (196, 60, 1600, 900)
    );
    assert!(!root.preview_ui.crop_mode);
}

#[test]
fn detected_original_coordinates_follow_rotation_and_flips_on_apply() {
    for (rotation, flip_horizontal, flip_vertical, expected) in [
        ("0", false, false, (100, 60, 1600, 900)),
        ("90", false, false, (120, 100, 900, 1600)),
        ("90", true, false, (120, 220, 900, 1600)),
        ("270", false, true, (120, 220, 900, 1600)),
    ] {
        let mut root = video_root();
        let config = &mut root.file_queue.selected_file_mut().unwrap().config;
        config.rotation = rotation.into();
        config.flip_horizontal = flip_horizontal;
        config.flip_vertical = flip_vertical;
        complete_detection(&mut root);
        assert!(root.apply_selected_crop());
        let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
        assert_eq!(
            (crop.x, crop.y, crop.width, crop.height),
            expected,
            "rotation={rotation}, horizontal={flip_horizontal}, vertical={flip_vertical}"
        );
        let dimensions = if rotation == "0" {
            (1920, 1080)
        } else {
            (1080, 1920)
        };
        assert_eq!(
            (crop.source_width, crop.source_height),
            (Some(dimensions.0), Some(dimensions.1))
        );
    }
}

#[test]
fn detection_clears_old_custom_fields_before_the_result_is_applied() {
    let mut root = video_root();
    root.preview_ui.crop_options.custom = true;
    root.preview_ui.crop_options.custom_dirty = true;
    root.preview_ui.crop_options.is_ratio = false;
    root.preview_ui.crop_options.width = "400".into();
    root.preview_ui.crop_options.height = "300".into();
    root.preview_ui.crop_options.custom_edit_base = Some(full_crop_rect());
    complete_detection(&mut root);
    assert_eq!(root.preview_ui.crop_aspect, "free");
    assert!(!root.preview_ui.crop_options.custom);
    assert!(!root.preview_ui.crop_options.custom_dirty);
    assert!(root.preview_ui.crop_options.custom_edit_base.is_none());
    assert!(root.apply_selected_crop());
    let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
    assert_eq!((crop.width, crop.height), (1600, 900));
}

#[test]
fn no_bars_clears_an_existing_crop_only_after_apply() {
    let mut root = video_root();
    let existing = CropSettings {
        enabled: true,
        x: 200,
        y: 200,
        width: 1000,
        height: 600,
        source_width: Some(1920),
        source_height: Some(1080),
        aspect_ratio: None,
    };
    root.file_queue.selected_file_mut().unwrap().config.crop = Some(existing.clone());
    let (_, cancel) = root.begin_auto_crop().unwrap();
    assert!(root.complete_auto_crop(
        &cancel,
        Ok(DetectedCrop {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080
        })
    ));
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::NoBars);
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
    assert_eq!(
        root.selected_config().unwrap().crop.as_ref(),
        Some(&existing)
    );
    assert!(root.apply_selected_crop());
    assert!(root.selected_config().unwrap().crop.is_none());
}

#[test]
fn an_unreliable_result_preserves_the_draft_and_can_be_retried() {
    let mut root = video_root();
    assert!(root.select_preview_crop_aspect("1:1"));
    let draft = root.preview_ui.draft_crop;
    let config = root.selected_config().unwrap().clone();
    let (_, cancel) = root.begin_auto_crop().unwrap();
    assert!(root.complete_auto_crop(&cancel, Err(AutoCropError::NoReliableCrop)));
    assert!(matches!(
        root.preview_ui.auto_crop.status,
        AutoCropStatus::Error(_)
    ));
    assert_eq!(root.preview_ui.draft_crop, draft);
    assert_eq!(root.selected_config(), Some(&config));
    assert!(root.begin_auto_crop().is_some());
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Analyzing);
}

#[test]
fn replacing_or_cancelling_a_job_rejects_its_late_result() {
    let mut root = video_root();
    let (_, first) = root.begin_auto_crop().unwrap();
    let (_, second) = root.begin_auto_crop().unwrap();
    assert!(first.load(Ordering::Relaxed));
    assert!(!root.complete_auto_crop(&first, Ok(detected_crop())));
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Analyzing);
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
    assert!(root.cancel_selected_auto_crop());
    assert!(second.load(Ordering::Relaxed));
    assert!(!root.complete_auto_crop(&second, Ok(detected_crop())));
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Idle);
}

#[test]
fn resetting_an_unchanged_frame_notifies_when_it_cancels_analysis() {
    let mut root = video_root();
    let (_, cancel) = root.begin_auto_crop().unwrap();
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
    assert!(root.reset_preview_crop_selection());
    assert!(cancel.load(Ordering::Relaxed));
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Idle);
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
}

#[test]
fn starting_a_drag_with_dirty_custom_fields_notifies_when_analysis_is_cancelled() {
    let mut root = video_root();
    root.preview_ui.crop_options.custom = true;
    root.preview_ui.crop_options.custom_dirty = true;
    root.preview_ui.crop_options.is_ratio = false;
    let draft = root.preview_ui.draft_crop;
    let (_, cancel) = root.begin_auto_crop().unwrap();

    assert!(root.apply_preview_crop_drag(DragHandle::Move, Point { x: 0.5, y: 0.5 }));
    assert!(cancel.load(Ordering::Relaxed));
    assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Idle);
    assert_eq!(root.preview_ui.draft_crop, draft);
    assert!(!root.preview_ui.crop_options.custom_dirty);
}

#[test]
fn closing_and_reopening_the_same_crop_session_rejects_an_old_job() {
    let mut root = video_root();
    let (_, cancel) = root.begin_auto_crop().unwrap();
    assert!(root.toggle_selected_crop_mode());
    assert!(root.toggle_selected_crop_mode());
    assert!(cancel.load(Ordering::Relaxed));
    assert!(!root.complete_auto_crop(&cancel, Ok(detected_crop())));
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
}

#[test]
fn switching_away_and_back_does_not_restore_an_old_detection() {
    let mut root = video_root();
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/second.mp4", 1));
    let (_, cancel) = root.begin_auto_crop().unwrap();
    root.file_queue.select_existing_file("second");
    sync_selection(&mut root);
    root.file_queue.select_existing_file("video");
    sync_selection(&mut root);
    assert!(root.toggle_selected_crop_mode());
    assert!(cancel.load(Ordering::Relaxed));
    assert!(!root.complete_auto_crop(&cancel, Ok(detected_crop())));
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
}

#[test]
fn manual_changes_cancel_analysis_before_it_can_replace_the_draft() {
    let edits: [fn(&mut FrameRoot); 5] = [
        |root| {
            root.select_preview_crop_aspect("1:1");
        },
        |root| {
            root.reset_preview_crop_selection();
        },
        |root| {
            root.apply_preview_crop_drag(DragHandle::Move, Point { x: 0.5, y: 0.5 });
        },
        |root| {
            root.adjust_preview_crop_from_keyboard_with_step(DragHandle::East, "left", false);
        },
        |root| {
            root.preview_ui.crop_options.custom = true;
            root.preview_ui.crop_options.is_ratio = false;
            root.preview_ui.crop_options.width = "400".into();
            root.preview_ui.crop_options.height = "300".into();
            root.preview_custom_crop_values();
        },
    ];
    for edit in edits {
        let mut root = video_root();
        let (_, cancel) = root.begin_auto_crop().unwrap();
        edit(&mut root);
        let draft = root.preview_ui.draft_crop;
        assert!(cancel.load(Ordering::Relaxed));
        assert!(!root.complete_auto_crop(&cancel, Ok(detected_crop())));
        assert_eq!(root.preview_ui.draft_crop, draft);
        assert!(root.selected_config().unwrap().crop.is_none());
    }
}

#[test]
fn requests_use_selected_stream_and_clamp_sampling_to_the_trimmed_video() {
    let mut root = video_root();
    let config = &mut root.file_queue.selected_file_mut().unwrap().config;
    config.start_time = Some("00:00:10.000".into());
    config.end_time = Some("00:03:00.000".into());
    let (request, _) = root.begin_auto_crop().unwrap();
    assert_eq!(request.video_stream_index, Some(3));
    assert!((request.start_seconds - 10.0).abs() < f64::EPSILON);
    assert!((request.end_seconds - 120.0).abs() < f64::EPSILON);
}

#[test]
fn unavailable_media_and_invalid_time_ranges_cannot_start_detection() {
    for duration in [None, Some("0"), Some("NaN"), Some("inf"), Some("invalid")] {
        let mut root = video_root();
        let mut metadata = root.selected_source_metadata().unwrap();
        metadata.duration = duration.map(str::to_string);
        root.source_metadata.mark_ready("video", metadata);
        assert!(root.begin_auto_crop().is_none());
    }
    for kind in [SourceKind::Image, SourceKind::Audio] {
        let mut root = video_root();
        let mut metadata = root.selected_source_metadata().unwrap();
        metadata.media_kind = Some(kind);
        root.source_metadata.mark_ready("video", metadata);
        assert!(root.begin_auto_crop().is_none());
    }
    let mut root = video_root();
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .start_time = Some("00:03:00.000".into());
    assert!(root.begin_auto_crop().is_none());
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .start_time = None;
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .processing_mode = ProcessingMode::Copy;
    assert!(root.begin_auto_crop().is_none());
}

#[test]
fn changed_trim_or_locked_file_rejects_an_in_flight_result() {
    let changes: [fn(&mut FrameRoot); 3] = [
        |root| {
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .start_time = Some("00:00:30.000".into());
        },
        |root| {
            root.file_queue
                .update_status("video", FileStatus::Converting, 0);
        },
        |root| {
            root.update_ui.status = UpdateStatus::Installing;
        },
    ];
    for change in changes {
        let mut root = video_root();
        let (_, cancel) = root.begin_auto_crop().unwrap();
        change(&mut root);
        root.complete_auto_crop(&cancel, Ok(detected_crop()));
        assert!(cancel.load(Ordering::Relaxed));
        assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
        assert!(root.selected_config().unwrap().crop.is_none());
    }
}

#[test]
fn dropping_the_view_cancels_its_background_analysis() {
    let mut root = video_root();
    let (_, cancel) = root.begin_auto_crop().unwrap();
    drop(root);
    assert!(cancel.load(Ordering::Relaxed));
}

fn draw_auto_crop_window(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn assert_rendered_auto_crop_status(cx: &mut VisualTestContext, label: &str) {
    let viewport = cx.debug_bounds("preview-viewport").unwrap();
    for id in [
        "preview-crop-ratio",
        "preview-crop-auto",
        "preview-crop-action-reset",
        "preview-crop-action-apply",
    ] {
        let bounds = cx.debug_bounds(id).unwrap();
        assert!(
            bounds.left() >= viewport.left()
                && bounds.right() <= viewport.right()
                && bounds.top() >= viewport.top()
                && bounds.bottom() <= viewport.bottom(),
            "{id} must remain inside the preview for {label}: {bounds:?} in {viewport:?}"
        );
    }
    assert_auto_crop_live_status(cx, label);
}

fn assert_auto_crop_live_status(cx: &mut VisualTestContext, label: &str) {
    let bounds = cx.debug_bounds("preview-crop-auto-status").unwrap();
    assert!(
        bounds.size.width <= px(1.0) && bounds.size.height <= px(1.0),
        "status feedback must not occupy a visible toolbar row: {bounds:?}"
    );
    cx.update(|window, _| {
        let tree = window.a11y_tree().expect("rendered accessibility tree");
        let (id, node) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == gpui::Role::Status && node.label() == Some(label))
            .expect("status must expose its current message to assistive technology");
        assert!(window.a11y_node_bounds(*id).is_some());
        assert_eq!(node.live(), Some(gpui::accesskit::Live::Polite));
        assert!(node.is_live_atomic());
    });
}

fn auto_crop_toolbar_bounds(cx: &mut VisualTestContext) -> [gpui::Bounds<gpui::Pixels>; 2] {
    ["preview-crop-toolbar", "preview-crop-action-apply"].map(|id| cx.debug_bounds(id).unwrap())
}

fn tab_auto_crop_window(cx: &mut VisualTestContext) -> String {
    cx.simulate_keystrokes("tab");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("tab").unwrap(),
    });
    cx.update(gpui::Window::simulate_next_frame);
    draw_auto_crop_window(cx);
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        tree["gpui_focus"]
            .as_str()
            .and_then(|id| tree["nodes"][id]["aria"]["label"].as_str())
            .unwrap_or_default()
            .to_owned()
    })
}

#[test]
fn rendered_analysis_status_updates_without_applying_the_detected_draft() {
    use crate::appearance::ScalePreset;

    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = app.add_window_view(|_, cx| {
            crate::app::init_app(cx, "Frame auto crop test");
            cx.set_reduce_motion(true);
            let mut root = video_root();
            root.appearance.ui_scale = scale;
            // Render the real view while keeping the fixture path away from FFmpeg.
            let metadata = root.selected_source_metadata_entry();
            root.preview_ui.pending_runtime_key = root
                .selected_preview_runtime_request(&metadata)
                .map(|request| request.key);
            root
        });
        cx.update(|window, _| {
            window.activate();
            window.set_a11y_forced(true);
        });
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        draw_auto_crop_window(cx);
        let idle_bounds = auto_crop_toolbar_bounds(cx);
        let original = root.read_with(cx, |root, _| root.selected_config().unwrap().clone());
        let cancelled = root.update(cx, |root, cx| {
            let (_, cancelled) = root.begin_auto_crop().unwrap();
            cx.notify();
            cancelled
        });
        draw_auto_crop_window(cx);
        assert_rendered_auto_crop_status(cx, "Analyzing video…");
        assert_eq!(auto_crop_toolbar_bounds(cx), idle_bounds);
        assert!(
            (0..80).any(|_| tab_auto_crop_window(cx) == "Cancel analysis"),
            "analysis cancellation must remain keyboard reachable at {scale:?}"
        );
        assert_eq!(tab_auto_crop_window(cx), "Reset");
        assert_eq!(tab_auto_crop_window(cx), "Apply");

        root.update(cx, |root, cx| {
            assert!(root.complete_auto_crop(&cancelled, Ok(detected_crop())));
            assert!(root.preview_ui.crop_mode);
            assert_eq!(root.selected_config(), Some(&original));
            cx.notify();
        });
        draw_auto_crop_window(cx);
        assert_rendered_auto_crop_status(cx, "Adjust the crop if needed, then Apply.");
        assert_eq!(auto_crop_toolbar_bounds(cx), idle_bounds);
        root.update(cx, |root, cx| {
            let (_, cancelled) = root.begin_auto_crop().unwrap();
            assert!(root.complete_auto_crop(
                &cancelled,
                Ok(DetectedCrop {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080
                })
            ));
            assert_eq!(root.selected_config(), Some(&original));
            cx.notify();
        });
        draw_auto_crop_window(cx);
        assert_rendered_auto_crop_status(cx, "No black bars detected.");
        assert_eq!(auto_crop_toolbar_bounds(cx), idle_bounds);
        let detected_draft = root.read_with(cx, |root, _| root.preview_ui.draft_crop);
        root.update(cx, |root, cx| {
            let (_, cancelled) = root.begin_auto_crop().unwrap();
            assert!(root.complete_auto_crop(&cancelled, Err(AutoCropError::NoReliableCrop)));
            assert!(root.preview_ui.crop_mode);
            assert_eq!(root.preview_ui.draft_crop, detected_draft);
            assert_eq!(root.selected_config(), Some(&original));
            cx.notify();
        });
        draw_auto_crop_window(cx);
        assert_rendered_auto_crop_status(
            cx,
            "Could not detect a reliable crop. Adjust the frame manually.",
        );
        assert_eq!(auto_crop_toolbar_bounds(cx), idle_bounds);
    }
}

fn assert_auto_crop_bounds_inside(
    cx: &mut VisualTestContext,
    id: &'static str,
    parent: &'static str,
) {
    let bounds = cx.debug_bounds(id).unwrap();
    let parent_bounds = cx.debug_bounds(parent).unwrap();
    assert!(
        bounds.left() >= parent_bounds.left()
            && bounds.right() <= parent_bounds.right()
            && bounds.top() >= parent_bounds.top()
            && bounds.bottom() <= parent_bounds.bottom(),
        "{id} must be visible inside {parent}: {bounds:?} in {parent_bounds:?}"
    );
}

fn assert_scrolling_auto_crop_toolbar(cx: &mut VisualTestContext, action: &str, message: &str) {
    assert_auto_crop_live_status(cx, message);
    assert_auto_crop_bounds_inside(cx, "preview-crop-toolbar", "preview-viewport");
    assert_auto_crop_bounds_inside(cx, "preview-crop-auto-status", "preview-crop-toolbar");
    assert!(
        (0..80).any(|_| tab_auto_crop_window(cx) == action),
        "{action} must remain keyboard reachable"
    );
    for (label, id) in [
        (action, "preview-crop-auto"),
        ("Reset", "preview-crop-action-reset"),
        ("Apply", "preview-crop-action-apply"),
    ] {
        if label != action {
            assert_eq!(tab_auto_crop_window(cx), label);
        }
        assert_auto_crop_bounds_inside(cx, id, "preview-crop-toolbar-controls");
        assert_auto_crop_bounds_inside(cx, "preview-crop-auto-status", "preview-crop-toolbar");
    }
}

#[test]
fn custom_crop_feedback_keeps_toolbar_geometry_and_analysis_controls_reachable() {
    use crate::appearance::ScalePreset;

    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = app.add_window_view(|_, cx| {
            crate::app::init_app(cx, "Frame custom auto crop test");
            cx.set_reduce_motion(true);
            let mut root = video_root();
            root.appearance.ui_scale = scale;
            root.preview_ui.crop_options.custom = true;
            root.set_crop_custom_mode(false);
            let metadata = root.selected_source_metadata_entry();
            root.preview_ui.pending_runtime_key = root
                .selected_preview_runtime_request(&metadata)
                .map(|request| request.key);
            root
        });
        cx.update(|window, _| {
            window.activate();
            window.set_a11y_forced(true);
        });
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        draw_auto_crop_window(cx);
        let idle_bounds = auto_crop_toolbar_bounds(cx);
        let cancelled = root.update(cx, |root, cx| {
            let (_, cancelled) = root.begin_auto_crop().unwrap();
            cx.notify();
            cancelled
        });
        draw_auto_crop_window(cx);
        root.read_with(cx, |root, _| {
            assert_eq!(root.preview_ui.auto_crop.status, AutoCropStatus::Analyzing);
            assert!(!cancelled.load(Ordering::Relaxed));
        });
        assert_eq!(auto_crop_toolbar_bounds(cx), idle_bounds);
        assert_scrolling_auto_crop_toolbar(cx, "Cancel analysis", "Analyzing video…");
        let focused_bounds = auto_crop_toolbar_bounds(cx);

        root.update(cx, |root, cx| {
            assert!(root.complete_auto_crop(&cancelled, Err(AutoCropError::NoReliableCrop)));
            assert!(root.preview_ui.crop_mode);
            assert!(root.preview_ui.crop_options.custom);
            assert!(root.selected_config().unwrap().crop.is_none());
            cx.notify();
        });
        draw_auto_crop_window(cx);
        assert_eq!(auto_crop_toolbar_bounds(cx), focused_bounds);
        assert_scrolling_auto_crop_toolbar(
            cx,
            "Auto crop",
            "Could not detect a reliable crop. Adjust the frame manually.",
        );
    }
}
