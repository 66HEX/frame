use super::*;

#[test]
fn selected_preview_runtime_request_keeps_dimensions_while_playing() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(900.0, 500.0));
    root.preview_ui.canvas.target_zoom = 1.0;
    root.preview_ui.playback_file_id = Some("video".to_string());
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.playback.handle_play();
    let metadata_entry = root.source_metadata.entry_for("video");
    let initial_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("initial request");

    root.preview_ui.canvas_bounds = Some(preview_test_bounds(1200.0, 700.0));
    root.preview_ui.canvas.current_zoom = 2.0;
    root.preview_ui.canvas.target_zoom = 2.0;
    let resized_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("resized request");

    assert_eq!(
        resized_request.key.preview_dimensions,
        initial_request.key.preview_dimensions
    );
    assert_eq!(
        resized_request.config.max_width,
        initial_request.config.max_width
    );
    assert_eq!(
        resized_request.config.max_height,
        initial_request.config.max_height
    );
}

#[test]
fn selected_preview_runtime_request_refreshes_dimensions_while_paused() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(900.0, 500.0));
    root.preview_ui.canvas.target_zoom = 1.0;
    root.preview_ui.playback_file_id = Some("video".to_string());
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    let metadata_entry = root.source_metadata.entry_for("video");
    let initial_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("initial request");

    root.preview_ui.canvas_bounds = Some(preview_test_bounds(1200.0, 700.0));
    root.preview_ui.canvas.current_zoom = 2.0;
    root.preview_ui.canvas.target_zoom = 2.0;
    let resized_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("resized request");

    assert_ne!(
        resized_request.key.preview_dimensions,
        initial_request.key.preview_dimensions
    );
    assert_eq!(
        resized_request.key.preview_dimensions,
        PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720
        }
    );
}

#[test]
fn selected_preview_runtime_request_refreshes_dimensions_after_pause() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(900.0, 500.0));
    root.preview_ui.canvas.target_zoom = 1.0;
    root.preview_ui.playback_file_id = Some("video".to_string());
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.playback.handle_play();
    let metadata_entry = root.source_metadata.entry_for("video");
    let initial_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("initial request");

    root.preview_ui.canvas_bounds = Some(preview_test_bounds(1200.0, 700.0));
    root.preview_ui.canvas.current_zoom = 2.0;
    root.preview_ui.canvas.target_zoom = 2.0;
    let playing_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("playing request");
    root.preview_ui.playback.handle_pause();
    let paused_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("paused request");

    assert_eq!(
        playing_request.key.preview_dimensions,
        initial_request.key.preview_dimensions
    );
    assert_eq!(
        paused_request.key.preview_dimensions,
        PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720
        }
    );
}

#[test]
fn selected_preview_runtime_request_defers_paused_zoom_until_canvas_settles() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(900.0, 500.0));
    root.preview_ui.canvas.current_zoom = 1.0;
    root.preview_ui.canvas.target_zoom = 1.0;
    root.preview_ui.playback_file_id = Some("video".to_string());
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    let metadata_entry = root.source_metadata.entry_for("video");
    let initial_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("initial request");

    root.preview_ui.canvas.target_zoom = 2.0;
    let animating_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("animating request");
    root.preview_ui.canvas.current_zoom = 2.0;
    let settled_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("settled request");

    assert_eq!(
        animating_request.key.preview_dimensions,
        initial_request.key.preview_dimensions
    );
    assert_eq!(
        settled_request.key.preview_dimensions,
        PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720
        }
    );
}

#[test]
fn selected_preview_runtime_request_debounces_paused_resize_dimensions() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(900.0, 500.0));
    root.preview_ui.canvas.current_zoom = 1.0;
    root.preview_ui.canvas.target_zoom = 1.0;
    root.preview_ui.playback_file_id = Some("video".to_string());
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    let metadata_entry = root.source_metadata.entry_for("video");
    let initial_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("initial request");

    root.preview_ui.canvas_bounds = Some(preview_test_bounds(1200.0, 700.0));
    root.preview_ui.preview_dimensions_debounce_until =
        Some(std::time::Instant::now() + PREVIEW_DIMENSION_DEBOUNCE_INTERVAL);
    let debounced_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("debounced request");
    root.preview_ui.preview_dimensions_debounce_until = Some(
        std::time::Instant::now()
            .checked_sub(PREVIEW_DIMENSION_DEBOUNCE_INTERVAL)
            .expect("deadline should move into the past"),
    );
    let settled_request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("settled request");

    assert_eq!(
        debounced_request.key.preview_dimensions,
        initial_request.key.preview_dimensions
    );
    assert_eq!(
        settled_request.key.preview_dimensions,
        PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720
        }
    );
}

#[test]
fn filter_preview_debounce_active_expires_deadline() {
    let mut root = FrameRoot::new();

    root.preview_ui.filter_preview_debounce_until =
        Some(std::time::Instant::now() + PREVIEW_FILTER_DEBOUNCE_INTERVAL);
    assert!(root.filter_preview_debounce_active());

    root.preview_ui.filter_preview_debounce_until = Some(
        std::time::Instant::now()
            .checked_sub(PREVIEW_FILTER_DEBOUNCE_INTERVAL)
            .expect("deadline should move into the past"),
    );
    assert!(!root.filter_preview_debounce_active());
    assert!(root.preview_ui.filter_preview_debounce_until.is_none());
}

#[test]
fn selected_preview_runtime_request_ignores_playing_latch_for_different_file() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "first", "/tmp/one.mp4");
    seed_ready_video(&mut root, "second", "/tmp/two.mp4");
    root.file_queue.select_existing_file("second");
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(900.0, 500.0));
    root.preview_ui.canvas.target_zoom = 1.0;
    root.preview_ui.playback_file_id = Some("first".to_string());
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.playback.handle_play();
    root.preview_ui.active_preview_dimensions = Some(PreviewRuntimeDimensions {
        max_width: 640,
        max_height: 360,
    });
    let metadata_entry = root.source_metadata.entry_for("second");

    let request = root
        .selected_preview_runtime_request(&metadata_entry)
        .expect("request");

    assert_eq!(
        request.key.preview_dimensions,
        PreviewRuntimeDimensions {
            max_width: 1088,
            max_height: 576
        }
    );
}

#[test]
fn preview_runtime_dimensions_fall_back_without_canvas_bounds() {
    let dimensions = preview_runtime_dimensions(None, 1.0);

    assert_eq!(
        dimensions,
        PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720
        }
    );
}

#[test]
fn preview_runtime_dimensions_clamp_small_canvas_to_minimum() {
    let dimensions = preview_runtime_dimensions(
        Some(Bounds::new(
            point(px(0.0), px(0.0)),
            size(px(200.0), px(120.0)),
        )),
        1.0,
    );

    assert_eq!(
        dimensions,
        PreviewRuntimeDimensions {
            max_width: 640,
            max_height: 360
        }
    );
}

#[test]
fn preview_runtime_dimensions_quantize_typical_canvas_below_720p() {
    let dimensions = preview_runtime_dimensions(
        Some(Bounds::new(
            point(px(0.0), px(0.0)),
            size(px(900.0), px(500.0)),
        )),
        1.0,
    );

    assert_eq!(
        dimensions,
        PreviewRuntimeDimensions {
            max_width: 1088,
            max_height: 576
        }
    );
}

#[test]
fn preview_runtime_dimensions_ignore_resize_within_same_quantum() {
    let first = preview_runtime_dimensions(
        Some(Bounds::new(
            point(px(0.0), px(0.0)),
            size(px(900.0), px(500.0)),
        )),
        1.0,
    );
    let second = preview_runtime_dimensions(
        Some(Bounds::new(
            point(px(0.0), px(0.0)),
            size(px(910.0), px(500.0)),
        )),
        1.0,
    );

    assert_eq!(first, second);
}

#[test]
fn preview_runtime_dimensions_zoom_up_without_exceeding_720p_cap() {
    let dimensions = preview_runtime_dimensions(
        Some(Bounds::new(
            point(px(0.0), px(0.0)),
            size(px(900.0), px(500.0)),
        )),
        2.0,
    );

    assert_eq!(
        dimensions,
        PreviewRuntimeDimensions {
            max_width: 1280,
            max_height: 720
        }
    );
}
