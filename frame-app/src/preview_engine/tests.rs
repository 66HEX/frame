#![expect(
    clippy::float_cmp,
    reason = "Preview engine tests compare exact deterministic timestamps and dimensions."
)]

use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use super::*;

fn default_core_config() -> frame_core::types::ConversionConfig {
    crate::conversion_runner::core_config_from_gpui(&crate::settings::ConversionConfig::default())
}

fn rendered_test_frame(timestamp_us: u64, bytes: Vec<u8>) -> PreviewRenderedFrame {
    rendered_frame_from_bgra_payload(1, 1, 4, timestamp_us, bytes).expect("rendered frame")
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
    let first = rendered_test_frame(0, vec![1, 2, 3, 4]);
    let second = rendered_test_frame(33_333, vec![5, 6, 7, 8]);

    let _ = store.publish(first);
    let latest = store.publish(second);

    assert_eq!(latest.generation, 2);
    assert_eq!(latest.stats.published_frames, 2);
    assert_eq!(latest.stats.overwritten_before_present, 1);
    assert_eq!(
        latest.frame.render_image().as_bytes(0),
        Some([5, 6, 7, 8].as_slice())
    );
}

#[test]
fn latest_frame_store_does_not_count_presented_frames_as_overwritten() {
    let store = LatestFrameStore::new();
    let first = rendered_test_frame(0, vec![1, 2, 3, 4]);
    let second = rendered_test_frame(33_333, vec![5, 6, 7, 8]);

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
fn rendered_frame_from_bgra_payload_reuses_tight_allocation() {
    let payload = vec![3, 2, 1, 255];
    let payload_ptr = payload.as_ptr();

    let frame = rendered_frame_from_bgra_payload(1, 1, 4, 0, payload).expect("rendered frame");
    let render_image = frame.render_image();
    let rendered_bytes = render_image.as_bytes(0).expect("rendered bytes");

    assert_eq!(rendered_bytes, &[3, 2, 1, 255]);
    assert_eq!(rendered_bytes.as_ptr(), payload_ptr);
}

#[test]
fn rendered_frame_from_bgra_payload_compacts_padded_rows() {
    let frame = rendered_frame_from_bgra_payload(
        1,
        2,
        8,
        0,
        vec![1, 2, 3, 4, 99, 99, 99, 99, 5, 6, 7, 8, 88, 88, 88, 88],
    )
    .expect("rendered frame");
    let render_image = frame.render_image();

    assert_eq!(
        render_image.as_bytes(0),
        Some([1, 2, 3, 4, 5, 6, 7, 8].as_slice())
    );
}

#[test]
fn rendered_frame_from_bgra_payload_can_reuse_image_identity_with_new_version() {
    let image_id = gpui::RenderImage::new_image_id();
    let first = rendered_frame_from_bgra_payload_with_image_id(
        1,
        1,
        4,
        0,
        Some((image_id, 1)),
        vec![1, 2, 3, 4],
    )
    .expect("first frame");
    let second = rendered_frame_from_bgra_payload_with_image_id(
        1,
        1,
        4,
        33_333,
        Some((image_id, 2)),
        vec![5, 6, 7, 8],
    )
    .expect("second frame");

    assert_eq!(first.render_image().id, second.render_image().id);
    assert_eq!(first.render_image().content_version(), 1);
    assert_eq!(second.render_image().content_version(), 2);
}

#[test]
fn latest_frame_snapshot_uses_shared_frame_storage() {
    let store = LatestFrameStore::new();
    let frame = rendered_test_frame(0, vec![1, 2, 3, 4]);

    let published = store.publish(frame);
    let latest = store.latest().expect("latest frame");

    assert!(Arc::ptr_eq(&published.frame, &latest.frame));
}

#[test]
fn latest_frame_store_reports_unpresented_frame_backpressure() {
    let store = LatestFrameStore::new();
    let first = store.publish(rendered_test_frame(0, vec![1, 2, 3, 4]));

    assert!(store.has_unpresented_frame());

    store.mark_presented(first.generation);

    assert!(!store.has_unpresented_frame());
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
fn test_preview_session_snapshot_exposes_runtime_metrics() {
    let config = PreviewSessionConfig {
        file_id: "test-metrics".to_string(),
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

    let snapshot = session.snapshot();

    assert_eq!(snapshot.runtime_metrics.video_frames_published, 0);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_preview_session_publishes_first_video_frame() {
    let path = temp_preview_video("first-frame");
    let config = real_video_preview_config(path.clone());

    let session = PreviewSession::start(config).expect("session");
    let _ = std::fs::remove_file(&path);

    let frame = session.latest_frame().expect("latest frame").frame;
    session.stop().expect("preview session should stop");

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
    session.stop().expect("preview session should stop");

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
    session.stop().expect("preview session should stop");

    assert!(before.position_seconds > 0.0);
    assert!(after.position_seconds >= before.position_seconds);
    assert!(after.playing);
}

#[test]
#[ignore = "requires FFmpeg plus a default audio output device; run with --ignored"]
fn ffmpeg_preview_session_collects_audio_video_metrics_for_sync_fixture() {
    let path = temp_preview_av_sync_video("av-sync");
    let config = real_av_preview_config(path.clone());

    let session = PreviewSession::start(config).expect("session");
    session.command(PreviewCommand::Play).expect("play");
    let playback_started_at = Instant::now();
    thread::sleep(Duration::from_millis(650));
    let snapshot = session.snapshot();
    let playback_elapsed_seconds = playback_started_at.elapsed().as_secs_f64();
    let _ = std::fs::remove_file(&path);
    session.stop().expect("preview session should stop");

    assert!(snapshot.runtime_metrics.video_process_spawns >= 1);
    assert!(snapshot.runtime_metrics.audio_process_spawns >= 1);
    assert!(snapshot.runtime_metrics.video_frames_published >= 1);
    assert!(snapshot.runtime_metrics.audio_pcm_chunks >= 1);
    assert!(
        snapshot
            .runtime_metrics
            .first_video_frame_published_ms
            .is_some_and(|elapsed_ms| elapsed_ms <= 250),
        "first video frame should publish quickly after play when audio prewarm is ready: {:?}",
        snapshot.runtime_metrics.first_video_frame_published_ms
    );
    assert!(
        snapshot.playback.position_seconds <= playback_elapsed_seconds + 0.150,
        "video playback advanced too far ahead of wall clock: position={:.3}s elapsed={:.3}s",
        snapshot.playback.position_seconds,
        playback_elapsed_seconds
    );
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

fn real_av_preview_config(path: PathBuf) -> PreviewSessionConfig {
    PreviewSessionConfig {
        file_id: "video-av-real".to_string(),
        path,
        source_kind: PreviewSourceKind::Video,
        source_width: Some(160),
        source_height: Some(90),
        has_audio: true,
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

fn temp_preview_av_sync_video(label: &str) -> PathBuf {
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
            "color=c=black:s=160x90:r=30:d=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:d=2",
            "-vf",
            "drawbox=x=0:y=0:w=iw:h=ih:color=white:t=fill:enable='between(t,0.5,0.6)'",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            path.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("spawn ffmpeg");
    assert!(
        status.success(),
        "ffmpeg av fixture generation failed: {status}"
    );
    path
}
