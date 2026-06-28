use std::{path::PathBuf, sync::Arc};

use super::*;

#[test]
fn fit_dimensions_preserves_aspect_and_even_dimensions() {
    let dimensions = fit_dimensions(1920, 1080, 1280, 720);

    assert_eq!(
        dimensions,
        PreviewDimensions {
            width: 1280,
            height: 720
        }
    );
}

#[test]
fn session_config_rejects_unpaired_source_dimensions() {
    let config = PreviewSessionConfig {
        file_id: "file-1".to_string(),
        path: PathBuf::from("/tmp/video.mp4"),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: None,
        duration_seconds: 10.0,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        transform: PreviewTransform::default(),
    };

    let error = config
        .validate()
        .expect_err("unpaired dimensions should fail");

    assert!(error.to_string().contains("provided together"));
}

#[test]
fn latest_frame_store_keeps_only_newest_frame() {
    let store = LatestFrameStore::new();
    let first = PreviewFrame::bgra(1, 1, 4, 0, vec![1, 2, 3, 4]).expect("first frame");
    let second = PreviewFrame::bgra(1, 1, 4, 33_333, vec![5, 6, 7, 8]).expect("second frame");

    store.publish(first);
    let latest = store.publish(second);

    assert_eq!(latest.generation, 2);
    assert_eq!(latest.dropped_frames, 1);
    assert_eq!(latest.frame.bytes(), &[5, 6, 7, 8]);
}

#[test]
fn render_image_from_frame_accepts_tight_bgra_frames() {
    let frame = PreviewFrame::bgra(1, 1, 4, 0, vec![3, 2, 1, 255]).expect("frame");

    let render_image = render_image_from_frame(&frame).expect("render image");

    assert_eq!(render_image.size(0).width.0, 1);
    assert_eq!(render_image.size(0).height.0, 1);
    assert_eq!(render_image.as_bytes(0), Some([3, 2, 1, 255].as_slice()));
}

#[test]
fn latest_frame_snapshot_uses_shared_frame_storage() {
    let store = LatestFrameStore::new();
    let frame = PreviewFrame::bgra(1, 1, 4, 0, vec![1, 2, 3, 4]).expect("frame");

    let published = store.publish(frame);
    let latest = store.latest().expect("latest frame");

    assert!(Arc::ptr_eq(&published.frame, &latest.frame));
}

#[test]
fn load_still_image_frame_converts_rgba_to_bgra_without_alpha_unpremultiply() {
    let mut path = std::env::temp_dir();
    path.push(format!("frame-preview-alpha-{}.png", std::process::id()));
    let rgba = image::RgbaImage::from_pixel(1, 1, image::Rgba([64, 32, 16, 128]));
    rgba.save(&path).expect("write test png");

    let frame =
        load_still_image_frame(&path, PreviewTransform::default()).expect("load still frame");
    let _ = std::fs::remove_file(&path);

    assert_eq!(frame.bytes(), &[16, 32, 64, 128]);
}

#[test]
fn image_preview_session_publishes_first_frame() {
    let path = temp_preview_png("session", image::Rgba([8, 16, 24, 255]));
    let config = PreviewSessionConfig {
        file_id: "image-1".to_string(),
        path: path.clone(),
        source_kind: PreviewSourceKind::Image,
        source_width: None,
        source_height: None,
        duration_seconds: 0.0,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        transform: PreviewTransform::default(),
    };

    let session = PreviewSession::start(config).expect("session");
    let _ = std::fs::remove_file(&path);

    let snapshot = session.snapshot();
    assert_eq!(snapshot.status, PreviewSessionStatus::Ready);
    assert_eq!(snapshot.frame_generation, 1);
    assert_eq!(
        session.latest_frame().expect("latest frame").frame.bytes(),
        &[24, 16, 8, 255]
    );
}

#[test]
fn test_preview_session_command_is_noop_without_pipeline() {
    let config = PreviewSessionConfig {
        file_id: "test-1".to_string(),
        path: PathBuf::from("/tmp/test.mp4"),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        duration_seconds: 12.5,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        transform: PreviewTransform::default(),
    };
    let session = PreviewSession::new_for_test(config);

    session
        .command(PreviewCommand::SeekFast(2.0))
        .expect("command");

    assert_eq!(session.snapshot().playback.duration_seconds, 12.5);
}

fn temp_preview_png(label: &str, pixel: image::Rgba<u8>) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("frame-preview-{label}-{}.png", std::process::id()));
    image::RgbaImage::from_pixel(1, 1, pixel)
        .save(&path)
        .expect("write test png");
    path
}
