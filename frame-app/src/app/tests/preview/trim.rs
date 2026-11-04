use super::*;

#[test]
fn trim_preview_seek_state_overwrites_pending_without_starting_worker() {
    let mut state = TrimPreviewSeekState::default();

    state.queue(10.0);
    state.queue(20.0);

    assert_eq!(state.pending_seconds, Some(20.0));
    assert!(!state.worker_active);
}

#[test]
fn trim_preview_seek_state_skips_duplicate_pending_within_epsilon() {
    let mut state = TrimPreviewSeekState {
        last_sent_seconds: Some(10.0),
        ..Default::default()
    };
    state.queue(10.0 + (TRIM_PREVIEW_SEEK_EPSILON_SECONDS / 2.0));

    assert_eq!(state.take_next(), None);
    assert_eq!(state.pending_seconds, None);
}

#[test]
fn trim_preview_seek_state_finishes_with_restore_and_clears_after_completion() {
    let mut state = TrimPreviewSeekState::default();
    state.begin_drag(30.0);
    state.queue(60.0);

    assert!(state.finish());
    assert_eq!(
        state.take_next(),
        Some(TrimPreviewSeekRequest {
            seconds: 30.0,
            pause_first: false,
        })
    );
    assert!(state.complete_restore(30.0));

    assert_eq!(
        state,
        TrimPreviewSeekState {
            generation: 1,
            ..Default::default()
        }
    );
}

#[test]
fn trim_preview_seek_state_marks_only_next_request_for_pause() {
    let mut state = TrimPreviewSeekState::default();
    state.pause_before_next_seek();
    state.queue(10.0);

    assert_eq!(
        state.take_next(),
        Some(TrimPreviewSeekRequest {
            seconds: 10.0,
            pause_first: true,
        })
    );

    state.queue(20.0);
    assert_eq!(
        state.take_next(),
        Some(TrimPreviewSeekRequest {
            seconds: 20.0,
            pause_first: false,
        })
    );
}

#[test]
fn preview_media_snapshot_sync_is_blocked_while_trim_preview_is_active() {
    let mut root = FrameRoot::new();
    root.preview_ui.trim_preview_seek.begin_drag(30.0);

    assert!(root.preview_media_snapshot_sync_blocked());
}

#[test]
fn apply_preview_timeline_drag_updates_playback_start_without_committing_config() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        },
    );
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.session = Some(test_video_preview_session("video"));

    let changed = root.apply_preview_timeline_drag(TimelineDragTarget::Start, 0.25);

    assert!(changed);
    assert!((root.preview_ui.playback.start_value() - 22.5).abs() < 0.000_001);
    assert!((root.preview_ui.playback.current_time() - 0.0).abs() < 0.000_001);
    assert_eq!(root.preview_ui.trim_preview_seek.restore_seconds, Some(0.0));
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(22.5)
    );
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.start_time.as_deref()),
        None
    );
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.end_time.as_deref()),
        None
    );

    assert!(root.end_preview_timeline_drag());
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.start_time.as_deref()),
        Some("00:00:22.500")
    );
}

#[test]
fn apply_preview_timeline_handle_drag_pauses_active_playback() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        },
    );
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.session = Some(test_video_preview_session("video"));
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 30.0,
        duration: 90.0,
        paused: false,
    });

    assert!(root.apply_preview_timeline_drag(TimelineDragTarget::End, 0.75));

    assert!(!root.preview_ui.playback.is_playing());
    assert!((root.preview_ui.playback.end_value() - 67.5).abs() < 0.000_001);
    assert!((root.preview_ui.playback.current_time() - 30.0).abs() < 0.000_001);
    assert_eq!(
        root.preview_ui.trim_preview_seek.restore_seconds,
        Some(30.0)
    );
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(67.5)
    );
    assert!(root.preview_ui.trim_preview_seek.pause_before_next_seek);
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.end_time.as_deref()),
        None
    );

    assert!(root.end_preview_timeline_drag());
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(30.0)
    );
    assert!(root.preview_ui.trim_preview_seek.finish_after_restore);
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.end_time.as_deref()),
        Some("00:01:07.500")
    );
}

#[test]
fn apply_preview_timeline_scrub_queues_preview_seek_for_paused_video() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.session = Some(test_video_preview_session("video"));

    assert!(root.apply_preview_timeline_drag(TimelineDragTarget::Scrub, 0.5));

    assert!(!root.preview_ui.playback.is_playing());
    assert_eq!(
        root.preview_ui.playback.dragging(),
        Some(TimelineDragTarget::Scrub)
    );
    assert!((root.preview_ui.playback.current_time() - 45.0).abs() < 0.000_001);
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(45.0)
    );
    assert!(!root.preview_ui.trim_preview_seek.pause_before_next_seek);
    assert_eq!(root.preview_ui.trim_preview_seek.restore_seconds, None);
}

#[test]
fn apply_preview_timeline_scrub_pauses_and_marks_first_preview_seek_when_playing() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.session = Some(test_video_preview_session("video"));
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 30.0,
        duration: 90.0,
        paused: false,
    });

    assert!(root.apply_preview_timeline_drag(TimelineDragTarget::Scrub, 0.5));

    assert!(!root.preview_ui.playback.is_playing());
    assert!((root.preview_ui.playback.current_time() - 45.0).abs() < 0.000_001);
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(45.0)
    );
    assert!(root.preview_ui.trim_preview_seek.pause_before_next_seek);
    assert_eq!(root.preview_ui.trim_preview_seek.restore_seconds, None);

    assert!(root.end_preview_timeline_drag());
    assert!(root.preview_ui.playback.is_playing());
    assert!((root.preview_ui.playback.current_time() - 45.0).abs() < 0.000_001);
    assert_eq!(root.preview_ui.trim_preview_seek.restore_seconds, None);
    assert!(!root.preview_ui.trim_preview_seek.finish_after_restore);
}

#[test]
fn commit_preview_timeline_seek_cancels_pending_trim_restore() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.session = Some(test_video_preview_session("video"));
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 30.0,
        duration: 90.0,
        paused: false,
    });
    root.set_preview_timeline_track_bounds(Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(100.0), px(30.0)),
    });

    assert!(root.apply_preview_timeline_drag(TimelineDragTarget::End, 0.75));
    assert!(root.end_preview_timeline_drag());
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(30.0)
    );
    assert!(root.preview_ui.trim_preview_seek.finish_after_restore);

    assert!(root.commit_preview_timeline_seek_at_position(point(px(50.0), px(0.0))));

    assert!((root.preview_ui.playback.current_time() - 45.0).abs() < 0.000_001);
    assert_eq!(root.preview_ui.trim_preview_seek.pending_seconds, None);
    assert_eq!(root.preview_ui.trim_preview_seek.restore_seconds, None);
    assert!(!root.preview_ui.trim_preview_seek.finish_after_restore);
}

#[test]
fn apply_preview_timeline_scrub_cancels_pending_trim_restore() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "video", "/tmp/one.mp4");
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.session = Some(test_video_preview_session("video"));
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 30.0,
        duration: 90.0,
        paused: false,
    });

    assert!(root.apply_preview_timeline_drag(TimelineDragTarget::End, 0.75));
    assert!(root.end_preview_timeline_drag());
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(30.0)
    );
    assert!(root.preview_ui.trim_preview_seek.finish_after_restore);

    assert!(root.apply_preview_timeline_drag(TimelineDragTarget::Scrub, 0.5));

    assert!((root.preview_ui.playback.current_time() - 45.0).abs() < 0.000_001);
    assert_eq!(
        root.preview_ui.trim_preview_seek.pending_seconds,
        Some(45.0)
    );
    assert_eq!(root.preview_ui.trim_preview_seek.restore_seconds, None);
    assert!(!root.preview_ui.trim_preview_seek.finish_after_restore);
}

#[test]
fn apply_preview_timeline_drag_preserves_gap_when_end_moves_before_start() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        },
    );
    root.update_selected_config(|config| {
        config.start_time = Some("00:00:20.000".to_string());
        true
    });
    root.preview_ui.playback =
        preview_playback_state(PreviewMediaKind::Video, 90.0, Some("00:00:20.000"), None);

    let changed = root.apply_preview_timeline_drag(TimelineDragTarget::End, 0.10);

    assert!(changed);
    assert!((root.preview_ui.playback.end_value() - 21.0).abs() < 0.000_001);
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.end_time.as_deref()),
        None
    );

    assert!(root.end_preview_timeline_drag());
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.end_time.as_deref()),
        Some("00:00:21.000")
    );
}

#[test]
fn apply_preview_timeline_drag_ignores_image_sources() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("image", "/tmp/one.png", 1));
    root.source_metadata.mark_ready(
        "image".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Image),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        },
    );

    let changed = root.apply_preview_timeline_drag(TimelineDragTarget::Start, 0.25);

    assert!(!changed);
    assert_eq!(
        root.file_queue
            .file_by_id("image")
            .and_then(|file| file.config.start_time.as_deref()),
        None
    );
}

#[test]
fn commit_preview_timeline_seek_at_position_updates_local_playhead() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        },
    );
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.set_preview_timeline_track_bounds(Bounds {
        origin: point(px(10.0), px(0.0)),
        size: size(px(100.0), px(30.0)),
    });

    let changed = root.commit_preview_timeline_seek_at_position(point(px(60.0), px(0.0)));

    assert!(changed);
    assert_eq!(root.preview_ui.playback.current_time(), 45.0);
}
