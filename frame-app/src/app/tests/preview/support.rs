use super::*;

pub(super) fn preview_test_bounds(width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(0.0), px(0.0)), size(px(width), px(height)))
}

pub(super) fn root_with_preview_canvas_media(width: u32, height: u32) -> FrameRoot {
    let mut root = FrameRoot::new();
    let row_bytes = width.checked_mul(4).expect("row bytes");
    let data_len = usize::try_from(row_bytes.checked_mul(height).expect("data length"))
        .expect("data length usize");
    let frame =
        PreviewFrame::bgra(width, height, row_bytes, 0, vec![0; data_len]).expect("preview frame");
    root.preview_ui.render_image = Some(render_image_from_frame(&frame).expect("render image"));
    root.preview_ui.rendered_presentation = PreviewRenderPresentation::default();
    root.preview_ui.canvas_bounds = Some(preview_test_bounds(1000.0, 500.0));
    root.preview_ui.canvas.auto_fit_pending = false;
    root
}

pub(super) fn seed_ready_video(root: &mut FrameRoot, id: &str, path: &str) {
    root.file_queue.add_file(FileItem::from_path(id, path, 1));
    root.source_metadata.mark_ready(
        id.to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        },
    );
}

pub(super) fn test_video_preview_session(file_id: &str) -> std::sync::Arc<PreviewSession> {
    std::sync::Arc::new(PreviewSession::new_for_test(PreviewSessionConfig {
        file_id: file_id.to_string(),
        path: PathBuf::from("/tmp/one.mp4"),
        source_kind: EnginePreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        selected_audio_track: None,
        duration_seconds: 90.0,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: crate::conversion_runner::core_config_from_gpui(
            &ConversionConfig::default(),
        ),
    }))
}

pub(super) fn root_with_overlay() -> FrameRoot {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );
    let overlay = PreviewOverlay {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.50,
        y: 0.50,
        width: 0.20,
        opacity: 1.0,
        anchor: "custom".to_string(),
    };
    root.preview_ui.overlay.sync_initial_overlay(Some(&overlay));
    root.preview_ui.overlay.set_overlay_mode(true, false);
    root.update_selected_config(|config| {
        config.overlay = Some(OverlaySettings {
            enabled: overlay.enabled,
            path: overlay.path.clone(),
            x: overlay.x,
            y: overlay.y,
            width: overlay.width,
            opacity: overlay.opacity,
            anchor: overlay.anchor.clone(),
        });
        true
    });
    root
}
