#![expect(
    clippy::float_cmp,
    reason = "Preview engine tests compare exact deterministic timestamps and dimensions."
)]

use std::{path::PathBuf, process::Command, sync::Arc, thread, time::Duration};

use super::*;

fn default_core_config() -> frame_core::types::ConversionConfig {
    crate::conversion_runner::core_config_from_gpui(&crate::settings::ConversionConfig::default())
}

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
        has_audio: false,
        selected_audio_track: None,
        duration_seconds: 10.0,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: default_core_config(),
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

    let _ = store.publish(first);
    let latest = store.publish(second);

    assert_eq!(latest.generation, 2);
    assert_eq!(latest.stats.published_frames, 2);
    assert_eq!(latest.stats.overwritten_before_present, 1);
    assert_eq!(latest.frame.bytes(), &[5, 6, 7, 8]);
}

#[test]
fn latest_frame_store_does_not_count_presented_frames_as_overwritten() {
    let store = LatestFrameStore::new();
    let first = PreviewFrame::bgra(1, 1, 4, 0, vec![1, 2, 3, 4]).expect("first frame");
    let second = PreviewFrame::bgra(1, 1, 4, 33_333, vec![5, 6, 7, 8]).expect("second frame");

    let first = store.publish(first);
    store.mark_presented(first.generation);
    let latest = store.publish(second);

    assert_eq!(latest.stats.presented_frames, 1);
    assert_eq!(latest.stats.overwritten_before_present, 0);
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
fn test_preview_session_command_is_noop_without_pipeline() {
    let config = PreviewSessionConfig {
        file_id: "test-1".to_string(),
        path: PathBuf::from("/tmp/test.mp4"),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        has_audio: false,
        selected_audio_track: None,
        duration_seconds: 12.5,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: default_core_config(),
    };
    let session = PreviewSession::new_for_test(config);

    session
        .command(PreviewCommand::SeekFast(2.0))
        .expect("command");

    assert_eq!(session.snapshot().playback.duration_seconds, 12.5);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_preview_session_publishes_first_video_frame() {
    let path = temp_preview_video("first-frame");
    let config = real_video_preview_config(path.clone());

    let session = PreviewSession::start(config).expect("session");
    let _ = std::fs::remove_file(&path);

    let frame = session.latest_frame().expect("latest frame").frame;
    session.stop();

    assert_eq!(
        frame.dimensions(),
        PreviewDimensions {
            width: 160,
            height: 90
        }
    );
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_preview_session_can_play_pause_and_seek() {
    let path = temp_preview_video("playback");
    let config = real_video_preview_config(path.clone());

    let session = PreviewSession::start(config).expect("session");
    session.command(PreviewCommand::Play).expect("play");
    thread::sleep(Duration::from_millis(450));
    session.command(PreviewCommand::Pause).expect("pause");
    let played_position = session.snapshot().playback.position_seconds;

    session
        .command(PreviewCommand::SeekPrecise(1.0))
        .expect("seek");
    let seek_position = session.snapshot().playback.position_seconds;
    let _ = std::fs::remove_file(&path);
    session.stop();

    assert!(played_position > 0.0);
    assert!(seek_position >= 0.99);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_preview_session_reconfigure_preserves_playback_position() {
    let path = temp_preview_video("reconfigure");
    let config = real_video_preview_config(path.clone());

    let session = PreviewSession::start(config.clone()).expect("session");
    session.command(PreviewCommand::Play).expect("play");
    thread::sleep(Duration::from_millis(350));
    let before = session.snapshot().playback;
    let mut next_config = config;
    next_config.conversion_config.flip_horizontal = true;

    session.reconfigure(next_config).expect("reconfigure");
    let after = session.snapshot().playback;
    let _ = std::fs::remove_file(&path);
    session.stop();

    assert!(before.position_seconds > 0.0);
    assert!(after.position_seconds >= before.position_seconds);
    assert!(after.playing);
}

fn real_video_preview_config(path: PathBuf) -> PreviewSessionConfig {
    PreviewSessionConfig {
        file_id: "video-real".to_string(),
        path,
        source_kind: PreviewSourceKind::Video,
        source_width: Some(160),
        source_height: Some(90),
        has_audio: false,
        selected_audio_track: None,
        duration_seconds: 2.0,
        max_width: 320,
        max_height: 180,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: default_core_config(),
    }
}

fn temp_preview_video(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("frame-preview-{label}-{}.mp4", std::process::id()));
    let status = Command::new(crate::runtime_binaries::ffmpeg_executable())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x90:rate=30:duration=2",
            "-pix_fmt",
            "yuv420p",
            path.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("spawn ffmpeg");
    assert!(
        status.success(),
        "ffmpeg fixture generation failed: {status}"
    );
    path
}
