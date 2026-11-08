use super::*;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn audio_fixture(milliseconds: u32) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "frame-audio-drain-{milliseconds}-{}.wav",
        std::process::id()
    ));
    let source = format!(
        "sine=frequency=440:sample_rate=48000:duration={}",
        f64::from(milliseconds) / 1_000.0
    );
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &source,
                "-c:a",
                "pcm_f32le",
            ])
            .arg(&path),
    )
    .expect("generate audio fixture");
    assert!(status.success());
    path
}

fn audio_config(path: PathBuf, milliseconds: u32) -> PreviewSessionConfig {
    let mut config = crate::conversion_runner::core_config_from_gpui(
        &crate::settings::ConversionConfig::default(),
    );
    config.audio_volume = 0.0;
    PreviewSessionConfig {
        file_id: "audio-drain".into(),
        path,
        source_kind: PreviewSourceKind::Audio,
        source_width: None,
        source_height: None,
        selected_audio_track: Some(0),
        duration_seconds: f64::from(milliseconds) / 1_000.0,
        max_width: 320,
        max_height: 180,
        fps: 30,
        conversion_config: config,
    }
}

fn wait_for_end(session: &PreviewSession) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while session.snapshot().playback.playing {
        assert!(Instant::now() < deadline, "audio drain did not publish EOF");
        thread::sleep(Duration::from_millis(2));
    }
}

fn with_watchdog(operation: impl FnOnce() + Send + 'static) {
    let (done_tx, done) = mpsc::channel();
    let worker = thread::spawn(move || {
        operation();
        let _ = done_tx.send(());
    });
    done.recv_timeout(Duration::from_secs(12))
        .expect("real audio drain watchdog");
    worker.join().expect("real audio drain worker");
}

#[test]
#[ignore = "requires FFmpeg and a default audio output device; run with --ignored"]
fn ffmpeg_audio_drain_preserves_all_samples_for_normal_short_sources() {
    for milliseconds in [100, 500, 1_000, 2_000] {
        let path = audio_fixture(milliseconds);
        let config = audio_config(path.clone(), milliseconds);
        with_watchdog(move || {
            let session = PreviewSession::start(config).expect("audio session");
            session.command(PreviewCommand::Play).expect("audio Play");
            let started = Instant::now();
            wait_for_end(&session);
            let elapsed = started.elapsed();
            let metrics = session.snapshot().runtime_metrics;
            assert!(metrics.audio_samples_queued > 0);
            assert_eq!(
                metrics.audio_samples_consumed, metrics.audio_samples_queued,
                "{milliseconds}ms source must deliver every decoded sample"
            );
            assert!(metrics.audio_output_callbacks > 0);
            assert!(
                elapsed >= Duration::from_millis(u64::from(milliseconds.saturating_sub(40))),
                "{milliseconds}ms source ended too early: {elapsed:?}"
            );
            eprintln!(
                "audio drain {milliseconds}ms: queued={} consumed={} callbacks={} underruns={} elapsed={elapsed:?}",
                metrics.audio_samples_queued,
                metrics.audio_samples_consumed,
                metrics.audio_output_callbacks,
                metrics.audio_output_underruns
            );
            session.stop().expect("stop drained audio");
        });
        let _ = std::fs::remove_file(path);
    }
}

#[test]
#[ignore = "requires FFmpeg and a default audio output device; run with --ignored"]
fn ffmpeg_audio_below_prebuffer_plays_every_complete_sample() {
    for milliseconds in [10, 20] {
        let path = audio_fixture(milliseconds);
        let config = audio_config(path.clone(), milliseconds);
        with_watchdog(move || {
            let session = PreviewSession::start(config).expect("short audio session");
            session
                .command(PreviewCommand::Play)
                .expect("short audio Play");
            wait_for_end(&session);
            let metrics = session.snapshot().runtime_metrics;
            assert!(metrics.audio_samples_queued > 0);
            assert_eq!(metrics.audio_samples_consumed, metrics.audio_samples_queued);
            session.stop().expect("stop short audio");
        });
        let _ = std::fs::remove_file(path);
    }
}

#[test]
#[ignore = "requires FFmpeg and a default audio output device; run with --ignored"]
fn ffmpeg_audio_drain_does_not_end_a_longer_video() {
    let path = std::env::temp_dir().join(format!(
        "frame-audio-drain-short-track-{}.mkv",
        std::process::id()
    ));
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=160x90:r=30:d=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=0.1",
                "-c:v",
                "libx264",
                "-c:a",
                "pcm_f32le",
            ])
            .arg(&path),
    )
    .expect("generate shorter audio track");
    assert!(status.success());
    let mut config = audio_config(path.clone(), 1_000);
    config.source_kind = PreviewSourceKind::Video;
    config.source_width = Some(160);
    config.source_height = Some(90);
    config.selected_audio_track = Some(1);
    with_watchdog(move || {
        let session = PreviewSession::start(config).expect("AV session");
        session.command(PreviewCommand::Play).expect("AV Play");
        thread::sleep(Duration::from_millis(350));
        let snapshot = session.snapshot();
        assert!(snapshot.playback.playing, "audio EOF must not end video");
        assert!(snapshot.runtime_metrics.audio_samples_queued > 0);
        assert_eq!(
            snapshot.runtime_metrics.audio_samples_consumed,
            snapshot.runtime_metrics.audio_samples_queued
        );
        wait_for_end(&session);
        assert_eq!(
            session.snapshot().runtime_metrics.audio_samples_consumed,
            snapshot.runtime_metrics.audio_samples_consumed
        );
        session.stop().expect("stop AV");
    });
    let _ = std::fs::remove_file(path);
}
