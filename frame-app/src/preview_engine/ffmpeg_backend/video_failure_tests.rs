use super::audio_exit::AudioProcessExit;
use super::audio_stream::{AudioWorkerStatus, PendingAudioStart, lock_pending_audio_start};
use super::clock::{
    AudioPlaybackStart, PlaybackStreamState, lock_playback, wait_for_playback_start,
};
use super::process::{ProcessHandles, RunningPreviewProcess, RunningProcessState, lock_process};
use super::*;
use crate::preview_engine::{LatestFrameStore, PreviewCommand, PreviewEngineError, PreviewSession};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

const WATCHDOG: Duration = Duration::from_secs(4);

fn config() -> PreviewSessionConfig {
    PreviewSessionConfig {
        file_id: "video-failure".into(),
        path: "/missing/video.mp4".into(),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(160),
        source_height: Some(90),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        selected_audio_track: None,
        duration_seconds: 3.0,
        max_width: 160,
        max_height: 90,
        fps: 30,
        conversion_config: crate::conversion_runner::core_config_from_gpui(
            &crate::settings::ConversionConfig::default(),
        ),
    }
}

fn new_process() -> RunningPreviewProcess {
    RunningPreviewProcess::new(
        config(),
        LatestFrameStore::new(),
        PreviewRuntimeMetricsStore::new(),
        "/missing/ffmpeg".into(),
    )
}

fn sleeping_handles() -> ProcessHandles {
    ProcessHandles {
        child: Some(
            Command::new("sh")
                .args(["-c", "exec sleep 10"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        ),
        stdout_worker: None,
        stderr_worker: None,
        stop_requested: Some(Arc::new(AtomicBool::new(false))),
        audio_start: None,
        audio_exit: None,
    }
}

fn install_video(process: &RunningPreviewProcess, handles: ProcessHandles) {
    *lock_process(&process.process) = RunningProcessState {
        child: handles.child,
        stdout_worker: handles.stdout_worker,
        stderr_worker: handles.stderr_worker,
        stop_requested: handles.stop_requested,
        audio_start: handles.audio_start,
        audio_exit: handles.audio_exit,
    };
}

fn child_alive(pid: u32) -> bool {
    crate::process_launch::status(
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null()),
    )
    .unwrap()
    .success()
}

#[test]
fn video_failure_marks_and_notifies_before_blocking_cleanup() {
    let process = Arc::new(new_process());
    process.park_playback_generation(0, 1.25);
    let (release_tx, release_rx) = mpsc::channel();
    lock_process(&process.process).stdout_worker = Some(thread::spawn(move || {
        release_rx.recv_timeout(WATCHDOG).unwrap();
    }));
    let start = Arc::new(AudioPlaybackStart::new(
        Arc::clone(&process.playback),
        Arc::clone(&process.playback_changed),
    ));
    let stop = Arc::new(AtomicBool::new(false));
    let worker_start = Arc::clone(&start);
    let worker_stop = Arc::clone(&stop);
    let (woke_tx, woke_rx) = mpsc::channel();
    let audio_worker = thread::spawn(move || {
        let _ = woke_tx.send(wait_for_playback_start(&worker_start, &worker_stop, 0));
    });
    *lock_process(&process.audio_process) = RunningProcessState {
        stdout_worker: Some(audio_worker),
        audio_start: Some(start),
        stop_requested: Some(stop),
        ..RunningProcessState::default()
    };
    let failed_process = Arc::clone(&process);
    let (done_tx, done_rx) = mpsc::channel();
    let failure = thread::spawn(move || {
        let error = failed_process.fail_video_start(
            0,
            PreviewEngineError::Ffmpeg("original start error".into()),
            None,
        );
        let _ = done_tx.send(error.to_string());
    });
    assert_eq!(
        woke_rx
            .recv_timeout(WATCHDOG)
            .expect("failure notification"),
        PlaybackStreamState::Stale
    );
    assert!(
        done_rx.try_recv().is_err(),
        "cleanup should still be blocked"
    );
    let snapshot = process.playback_reader().snapshot(3.0, true);
    assert!(!snapshot.playing);
    assert_eq!(snapshot.position_seconds.to_bits(), 1.25_f64.to_bits());
    assert!(lock_playback(&process.playback).failed);
    release_tx.send(()).unwrap();
    assert!(
        done_rx
            .recv_timeout(WATCHDOG)
            .expect("cleanup watchdog")
            .contains("original start error")
    );
    failure.join().unwrap();
}

#[test]
fn stale_video_failure_preserves_replacement_but_disposes_owned_candidate() {
    let process = new_process();
    let recovered_generation = process.prepare_playback_generation(0.5);
    process.start_playback_generation(recovered_generation, 0.5);
    let registered = sleeping_handles();
    let registered_pid = registered.child.as_ref().unwrap().id();
    let registered_stop = Arc::clone(registered.stop_requested.as_ref().unwrap());
    install_video(&process, registered);
    let candidate = sleeping_handles();
    let candidate_pid = candidate.child.as_ref().unwrap().id();
    let candidate_stop = Arc::clone(candidate.stop_requested.as_ref().unwrap());
    let audio = sleeping_handles();
    let audio_pid = audio.child.as_ref().unwrap().id();
    let audio_stop = Arc::clone(audio.stop_requested.as_ref().unwrap());
    let exit = Arc::new(AudioProcessExit::new(
        Arc::clone(&audio_stop),
        Arc::clone(&process.stderr),
    ));
    *lock_process(&process.audio_process) = RunningProcessState {
        child: audio.child,
        stop_requested: audio.stop_requested,
        audio_exit: Some(Arc::clone(&exit)),
        ..RunningProcessState::default()
    };
    let (ready_tx, ready_rx) = mpsc::channel();
    ready_tx.send(AudioWorkerStatus::Ready).unwrap();
    *lock_pending_audio_start(&process.pending_audio_start) = Some(PendingAudioStart::Waiting {
        generation: recovered_generation,
        ready_rx,
        exit,
    });
    let error = process.fail_video_start(
        0,
        PreviewEngineError::Ffmpeg("stale failure".into()),
        Some(candidate),
    );
    assert!(error.to_string().contains("stale failure"));
    assert!(!child_alive(candidate_pid));
    assert!(candidate_stop.load(Ordering::SeqCst));
    assert!(child_alive(registered_pid));
    assert!(!registered_stop.load(Ordering::SeqCst));
    assert!(child_alive(audio_pid));
    assert!(!audio_stop.load(Ordering::SeqCst));
    assert!(process.pending_audio_start_generation_matches(recovered_generation));
    let playback = lock_playback(&process.playback);
    assert!(playback.playing);
    assert!(!playback.failed);
    assert_eq!(playback.generation, recovered_generation);
    drop(playback);
    process.stop_running_processes().unwrap();
    assert!(!child_alive(registered_pid));
    assert!(!child_alive(audio_pid));
}

#[test]
fn preflight_and_paused_single_frame_errors_preserve_the_existing_generation() {
    let mut process = new_process();
    process.start_playback_generation(0, 0.25);
    let old = sleeping_handles();
    let pid = old.child.as_ref().unwrap().id();
    install_video(&process, old);
    assert!(process.seek(f64::NAN, false, true).is_err());
    assert!(
        process
            .reconfigure(process.config.clone(), 0.5, false, true)
            .is_err()
    );
    let playback = lock_playback(&process.playback);
    assert_eq!(playback.generation, 0);
    assert!(playback.playing);
    assert!(!playback.failed);
    drop(playback);
    assert!(child_alive(pid));
    process.stop_running_processes().unwrap();
}

#[test]
fn video_plan_spawn_and_first_frame_failures_cleanup_old_child_and_freeze_generation() {
    for stage in ["plan", "spawn", "first frame"] {
        let mut process = new_process();
        let old = sleeping_handles();
        let old_pid = old.child.as_ref().unwrap().id();
        install_video(&process, old);
        let mut config = process.config.clone();
        if stage == "plan" {
            config.max_width = 0;
        }
        if stage == "first frame" {
            process.executable = "/usr/bin/true".into();
        }
        assert!(
            process
                .start_video_streaming_with_config(&config, 0.5, true)
                .is_err(),
            "{stage}"
        );
        let playback = lock_playback(&process.playback);
        assert!(playback.failed, "{stage}");
        assert!(!playback.playing);
        assert_eq!(playback.last_frame_seconds.to_bits(), 0.5_f64.to_bits());
        drop(playback);
        assert!(lock_process(&process.process).child.is_none());
        assert!(!child_alive(old_pid), "{stage}");
    }
}

#[test]
fn video_failure_cancels_reused_audio_prewarm_generation() {
    let process = new_process();
    let mut handles = sleeping_handles();
    let pid = handles.child.as_ref().unwrap().id();
    let exit = Arc::new(AudioProcessExit::new(
        Arc::clone(handles.stop_requested.as_ref().unwrap()),
        Arc::clone(&process.stderr),
    ));
    handles.audio_exit = Some(Arc::clone(&exit));
    let (ready_tx, ready_rx) = mpsc::channel();
    ready_tx.send(AudioWorkerStatus::Ready).unwrap();
    *lock_pending_audio_start(&process.pending_audio_start) = Some(PendingAudioStart::Waiting {
        generation: 0,
        ready_rx,
        exit,
    });
    *lock_process(&process.audio_process) = RunningProcessState {
        child: handles.child,
        stdout_worker: handles.stdout_worker,
        stderr_worker: handles.stderr_worker,
        stop_requested: handles.stop_requested,
        audio_start: handles.audio_start,
        audio_exit: handles.audio_exit,
    };
    assert!(
        process
            .start_video_streaming_with_config(&process.config, 0.0, true)
            .is_err()
    );
    assert_eq!(lock_playback(&process.playback).generation, 0);
    assert!(lock_playback(&process.playback).failed);
    assert!(lock_pending_audio_start(&process.pending_audio_start).is_none());
    assert!(lock_process(&process.audio_process).child.is_none());
    assert!(!child_alive(pid));
}

fn fail_reconfigure(session: &PreviewSession, valid: &PreviewSessionConfig) {
    let mut bad = valid.clone();
    bad.conversion_config.subtitle_burn_path = Some("/missing/frame-video-failure.srt".into());
    assert!(session.reconfigure(bad).is_err());
    let failed = session.snapshot();
    let frame = session.latest_frame().unwrap();
    assert!(!failed.playback.playing);
    thread::sleep(Duration::from_millis(80));
    assert_eq!(
        session.snapshot().playback.position_seconds.to_bits(),
        failed.playback.position_seconds.to_bits()
    );
    assert!(Arc::ptr_eq(
        &session.latest_frame().unwrap().frame,
        &frame.frame
    ));
}

#[test]
#[ignore = "requires FFmpeg and a default audio output device; run with --ignored"]
fn real_video_failure_preserves_frame_pauses_ordinary_commands_and_allows_explicit_retry() {
    let path = std::env::temp_dir().join(format!("frame-video-failure-{}.mp4", std::process::id()));
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
                "testsrc2=size=160x90:rate=30:duration=3",
                "-c:v",
                "libx264",
            ])
            .arg(&path),
    )
    .unwrap();
    assert!(status.success());
    let mut valid = config();
    valid.path = path.clone();
    let session = PreviewSession::start(valid.clone()).unwrap();
    session.command(PreviewCommand::Play).unwrap();
    let mut invalid = valid.clone();
    invalid.fps = 0;
    assert!(
        session.reconfigure(invalid).is_err(),
        "preflight validation"
    );
    assert!(session.snapshot().playback.playing);
    let before = session.latest_frame().unwrap().generation;
    session.mark_frame_presented(before);
    let deadline = Instant::now() + WATCHDOG;
    while session.latest_frame().unwrap().generation <= before {
        assert!(
            Instant::now() < deadline,
            "preflight error stopped live video: {:?}",
            session.snapshot()
        );
        thread::sleep(Duration::from_millis(2));
    }
    fail_reconfigure(&session, &valid);
    let failed_generation = session.snapshot().runtime_metrics.playback_generation;
    session.command(PreviewCommand::Play).unwrap();
    assert!(session.snapshot().playback.playing);
    assert!(session.snapshot().runtime_metrics.playback_generation > failed_generation);
    for command in [
        PreviewCommand::SeekFast(0.3),
        PreviewCommand::SeekPrecise(0.4),
    ] {
        fail_reconfigure(&session, &valid);
        session.command(command).unwrap();
        assert!(
            !session.snapshot().playback.playing,
            "ordinary seek after failure"
        );
        session.command(PreviewCommand::Play).unwrap();
        assert!(
            session.snapshot().playback.playing,
            "explicit retry of retained valid config"
        );
    }
    fail_reconfigure(&session, &valid);
    session.reconfigure(valid.clone()).unwrap();
    assert!(
        !session.snapshot().playback.playing,
        "ordinary reconfigure after failure"
    );
    session.command(PreviewCommand::Play).unwrap();
    assert!(session.snapshot().playback.playing);
    valid.selected_audio_track = Some(99);
    session.reconfigure(valid).unwrap();
    assert!(
        session.snapshot().playback.playing,
        "audio-disabled fallback is nonfatal"
    );
    session.stop().unwrap();
    std::fs::remove_file(path).unwrap();
}
