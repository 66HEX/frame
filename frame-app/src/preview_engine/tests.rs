#![expect(
    clippy::float_cmp,
    reason = "Preview engine tests compare exact deterministic timestamps and dimensions."
)]

use super::*;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn default_core_config() -> frame_core::types::ConversionConfig {
    crate::conversion_runner::core_config_from_gpui(&crate::settings::ConversionConfig::default())
}

fn rendered_test_frame(timestamp_us: u64, bytes: Vec<u8>) -> PreviewRenderedFrame {
    rendered_frame_from_bgra_payload(1, 1, 4, timestamp_us, bytes).expect("rendered frame")
}

#[test]
fn fit_dimensions_preserves_aspect_without_rounding_odd_rasters() {
    let dimensions = fit_dimensions(1920, 1080, 1280, 720);

    assert_eq!(
        dimensions,
        PreviewDimensions {
            width: 1280,
            height: 720
        }
    );
    assert_eq!(
        fit_dimensions(127, 129, 1280, 720),
        PreviewDimensions {
            width: 127,
            height: 129
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
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
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
fn session_config_accepts_silent_audio_source_without_selected_track() {
    let config = PreviewSessionConfig {
        file_id: "audio-1".to_string(),
        path: PathBuf::from("/tmp/audio.wav"),
        source_kind: PreviewSourceKind::Audio,
        source_width: None,
        source_height: None,
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        selected_audio_track: None,
        duration_seconds: 10.0,
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: default_core_config(),
    };

    assert!(config.validate().is_ok());
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
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
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
    session
        .command(PreviewCommand::SeekAndPlay {
            seconds: 2.0,
            precise: true,
        })
        .expect("stopped session must not restart for queued playback");

    assert_eq!(session.snapshot().playback.duration_seconds, 12.5);
    assert!(!session.snapshot().playback.playing);
}

#[test]
fn test_preview_session_snapshot_exposes_runtime_metrics() {
    let config = PreviewSessionConfig {
        file_id: "test-metrics".to_string(),
        path: PathBuf::from("/tmp/test.mp4"),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
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
fn ffmpeg_seek_and_play_matches_precise_frame_with_one_process_spawn() {
    let path = temp_preview_video("atomic-seek-play");
    let config = real_video_preview_config(path.clone());
    let baseline = PreviewSession::start(config.clone()).expect("baseline session");
    let baseline_spawns = baseline.snapshot().runtime_metrics.video_process_spawns;
    baseline
        .command(PreviewCommand::SeekPrecise(1.0))
        .expect("paused precise seek");
    let expected_frame = baseline.latest_frame().expect("precise frame").frame;
    baseline
        .command(PreviewCommand::Play)
        .expect("baseline play");
    let baseline_spawn_delta =
        baseline.snapshot().runtime_metrics.video_process_spawns - baseline_spawns;
    baseline.stop().expect("stop baseline");

    let atomic = PreviewSession::start(config).expect("atomic session");
    let atomic_spawns = atomic.snapshot().runtime_metrics.video_process_spawns;
    atomic
        .command(PreviewCommand::SeekAndPlay {
            seconds: 1.0,
            precise: true,
        })
        .expect("atomic precise seek and play");
    // The first frame remains unpresented, so backpressure keeps it available
    // for exact comparison even if the decoder has read subsequent frames.
    let actual_frame = atomic.latest_frame().expect("atomic frame").frame;
    let snapshot = atomic.snapshot();
    atomic.stop().expect("stop atomic");
    let _ = std::fs::remove_file(&path);

    assert_eq!(baseline_spawn_delta, 2);
    assert_eq!(
        snapshot.runtime_metrics.video_process_spawns - atomic_spawns,
        1
    );
    assert!(snapshot.playback.playing);
    assert_eq!(actual_frame.timestamp_us, 1_000_000);
    assert_eq!(actual_frame.timestamp_us, expected_frame.timestamp_us);
    assert_eq!(actual_frame.dimensions(), expected_frame.dimensions());
    assert_eq!(
        actual_frame.render_image().as_bytes(0),
        expected_frame.render_image().as_bytes(0)
    );
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_play_is_idempotent_and_restarts_after_eof() {
    let path = temp_preview_video("idempotent-play-eof");
    let mut config = real_video_preview_config(path.clone());
    config.conversion_config.start_time = Some("00:00:00.250".into());
    let session = PreviewSession::start(config).expect("session");
    session.command(PreviewCommand::Play).expect("play");
    let playing_spawns = session.snapshot().runtime_metrics.video_process_spawns;
    let first_generation = session.latest_frame().expect("playing frame").generation;
    for _ in 0..10 {
        session
            .command(PreviewCommand::Play)
            .expect("repeated play");
    }
    assert_eq!(
        session.snapshot().runtime_metrics.video_process_spawns,
        playing_spawns
    );
    assert_eq!(session.latest_frame().unwrap().generation, first_generation);

    session
        .command(PreviewCommand::SeekAndPlay {
            seconds: 1.8,
            precise: true,
        })
        .expect("play near EOF");
    let deadline = Instant::now() + Duration::from_secs(5);
    let ended = loop {
        let playback = session.snapshot().playback;
        if !playback.playing || Instant::now() >= deadline {
            break playback;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let ended_spawns = session.snapshot().runtime_metrics.video_process_spawns;
    session
        .command(PreviewCommand::Play)
        .expect("replay after EOF");
    let replay = session.snapshot();
    let replay_frame = session.latest_frame().expect("replay frame").frame;
    session.stop().expect("stop");
    let _ = std::fs::remove_file(&path);

    assert!(!ended.playing, "preview must reach EOF before replay");
    assert_eq!(ended.position_seconds, ended.duration_seconds);
    assert!(replay.playback.playing);
    assert_eq!(
        replay.runtime_metrics.video_process_spawns,
        ended_spawns + 1
    );
    assert_eq!(replay_frame.timestamp_us, 250_000);
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
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_preview_session_reports_eof_through_read_only_snapshots() {
    let path = temp_preview_video("read-only-eof");
    let session = PreviewSession::start(real_video_preview_config(path.clone())).expect("session");
    session.command(PreviewCommand::Play).expect("play");
    let deadline = Instant::now() + Duration::from_secs(5);
    let ended = loop {
        let playback = session.snapshot().playback;
        if !playback.playing || Instant::now() >= deadline {
            break playback;
        }
        thread::sleep(Duration::from_millis(10));
    };
    session.stop().expect("stop");
    let _ = std::fs::remove_file(&path);

    assert!(
        !ended.playing,
        "the worker must publish EOF without UI process polling"
    );
    assert_eq!(ended.position_seconds, ended.duration_seconds);
}

#[test]
#[ignore = "requires FFmpeg plus a default audio output device; run with --ignored"]
fn ffmpeg_preview_session_collects_audio_video_metrics_for_sync_fixture() {
    let path = temp_preview_av_sync_video("av-sync");
    let mut config = real_av_preview_config(path.clone());
    config.conversion_config.audio_volume = 0.0;

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

#[test]
#[ignore = "requires FFmpeg plus a default audio output device; run with --ignored"]
fn ffmpeg_audio_prewarm_wakes_through_preview_session_lifecycle() {
    let path = temp_preview_av_sync_video("audio-prewarm-lifecycle");
    let mut config = real_av_preview_config(path.clone());
    config.conversion_config.audio_volume = 0.0;
    let (stage_tx, stage_rx) = std::sync::mpsc::channel();
    let worker = thread::spawn(move || {
        let session = PreviewSession::start(config.clone()).expect("session");
        assert_audio_stays_paused(&session);
        stage_tx.send("initial prewarm").unwrap();

        session.command(PreviewCommand::Play).expect("play");
        assert_audio_callbacks_resume(&session);
        stage_tx.send("play").unwrap();

        session.command(PreviewCommand::Pause).expect("pause");
        assert_audio_stays_paused(&session);
        stage_tx.send("pause").unwrap();

        session
            .command(PreviewCommand::Play)
            .expect("resume prewarm");
        assert_audio_callbacks_resume(&session);
        stage_tx.send("resume").unwrap();

        session
            .command(PreviewCommand::Pause)
            .expect("pause before seek");
        session
            .command(PreviewCommand::SeekPrecise(0.3))
            .expect("paused seek");
        assert_audio_stays_paused(&session);
        stage_tx.send("paused seek").unwrap();

        session
            .command(PreviewCommand::SeekAndPlay {
                seconds: 0.5,
                precise: true,
            })
            .expect("seek and play");
        assert_audio_callbacks_resume(&session);
        stage_tx.send("seek and play").unwrap();

        session
            .command(PreviewCommand::Pause)
            .expect("pause before rebuild");
        config.conversion_config.flip_horizontal = true;
        session.reconfigure(config).expect("paused rebuild");
        assert_audio_stays_paused(&session);
        stage_tx.send("paused rebuild").unwrap();

        session
            .command(PreviewCommand::Play)
            .expect("play rebuilt prewarm");
        assert_audio_callbacks_resume(&session);
        session
            .command(PreviewCommand::Pause)
            .expect("pause before stop");
        assert_audio_stays_paused(&session);
        session.stop().expect("stop parked prewarm");
        stage_tx.send("stop").unwrap();
    });
    for expected in [
        "initial prewarm",
        "play",
        "pause",
        "resume",
        "paused seek",
        "seek and play",
        "paused rebuild",
        "stop",
    ] {
        assert_eq!(
            stage_rx
                .recv_timeout(Duration::from_secs(8))
                .unwrap_or_else(|error| panic!("audio lifecycle watchdog at {expected}: {error}")),
            expected
        );
    }
    worker.join().expect("audio lifecycle worker");
    let _ = std::fs::remove_file(path);
}

fn assert_audio_stays_paused(session: &PreviewSession) {
    let before = session.snapshot();
    assert!(!before.playback.playing);
    thread::sleep(Duration::from_millis(100));
    let after = session.snapshot();
    assert!(!after.playback.playing);
    assert_eq!(
        before.runtime_metrics.audio_output_callbacks, after.runtime_metrics.audio_output_callbacks,
        "parked audio must not start its output stream",
    );
}

fn assert_audio_callbacks_resume(session: &PreviewSession) {
    let before = session.snapshot().runtime_metrics.audio_output_callbacks;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let snapshot = session.snapshot();
        assert!(snapshot.playback.playing, "audio should still be playing");
        if snapshot.runtime_metrics.audio_output_callbacks > before {
            return;
        }
        assert!(Instant::now() < deadline, "audio callbacks did not resume");
        thread::sleep(Duration::from_millis(5));
    }
}

fn real_video_preview_config(path: PathBuf) -> PreviewSessionConfig {
    PreviewSessionConfig {
        file_id: "video-real".to_string(),
        path,
        source_kind: PreviewSourceKind::Video,
        source_width: Some(160),
        source_height: Some(90),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
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
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        selected_audio_track: Some(1),
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
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable()).args([
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
        ]),
    )
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
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable()).args([
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
        ]),
    )
    .expect("spawn ffmpeg");
    assert!(
        status.success(),
        "ffmpeg av fixture generation failed: {status}"
    );
    path
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_snapshot_stream_seek_and_reconfigure_keep_frame_owned_presentation() {
    use frame_core::aspect::{DisplayRatio, SampleAspectRatio};
    let path = temp_preview_video("presentation-ownership");
    let mut config = real_video_preview_config(path.clone());
    config.source_sample_aspect_ratio = SampleAspectRatio::new(2, 1);
    config.request_token = 101;
    let session = PreviewSession::start(config.clone()).expect("session");
    let initial = session.latest_frame().unwrap().frame;
    assert_eq!(initial.request_token, 101);
    assert_eq!(
        initial.presentation.pixel_aspect_ratio,
        DisplayRatio::new(2, 1).unwrap()
    );
    assert_eq!(initial.presentation.transform, PreviewTransform::default());
    assert_eq!(
        initial.presentation.output_raster_aspect_ratio,
        DisplayRatio::new(16, 9)
    );
    for command in [
        PreviewCommand::SeekPrecise(0.5),
        PreviewCommand::SeekAndPlay {
            seconds: 0.5,
            precise: true,
        },
    ] {
        session.command(command).unwrap();
        let frame = session.latest_frame().unwrap().frame;
        assert_eq!(frame.request_token, 101);
        assert_eq!(frame.presentation, initial.presentation);
        assert_eq!(frame.render_image().id, initial.render_image().id);
        assert!(frame.render_image().content_version() > initial.render_image().content_version());
        session.command(PreviewCommand::Pause).unwrap();
    }
    config.request_token = 102;
    config.conversion_config.rotation = "90".into();
    session.reconfigure(config).unwrap();
    let rotated = session.latest_frame().unwrap().frame;
    assert_eq!(rotated.request_token, 102);
    assert_eq!((rotated.width, rotated.height), (90, 160));
    assert_eq!(
        rotated.presentation.pixel_aspect_ratio,
        DisplayRatio::new(1, 2).unwrap()
    );
    assert_eq!(rotated.presentation.transform, PreviewTransform::default());
    assert_eq!(
        rotated.presentation.output_raster_aspect_ratio,
        DisplayRatio::new(9, 16)
    );
    assert_eq!(rotated.render_image().id, initial.render_image().id);
    assert!(rotated.render_image().content_version() > initial.render_image().content_version());
    session.stop().unwrap();
    let _ = std::fs::remove_file(path);
}

#[path = "tests/small_sources.rs"]
mod small_sources;
