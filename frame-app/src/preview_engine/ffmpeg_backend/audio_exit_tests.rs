use super::audio_exit::{AudioExitOutcome, AudioProcessExit};
use super::audio_stream::{AudioStartStatus, AudioWorkerStatus, PendingAudioStart};
use super::clock::{AudioPlaybackStart, lock_playback};
use super::process::{
    RunningPreviewProcess, RunningProcessState, lock_process, spawn_stderr_worker_with_completion,
};
use super::*;
use crate::preview_engine::LatestFrameStore;
use std::io::Cursor;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

const WATCHDOG: Duration = Duration::from_secs(4);

fn new_process() -> RunningPreviewProcess {
    RunningPreviewProcess::new(
        PreviewSessionConfig {
            file_id: "audio-exit-proof".into(),
            path: "/missing/audio.wav".into(),
            source_kind: PreviewSourceKind::Audio,
            source_width: None,
            source_height: None,
            source_is_palette: false,
            source_sample_aspect_ratio: None,
            request_token: 0,
            source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
            selected_audio_track: Some(0),
            duration_seconds: 1.0,
            max_width: 320,
            max_height: 180,
            fps: 30,
            conversion_config: crate::conversion_runner::core_config_from_gpui(
                &crate::settings::ConversionConfig::default(),
            ),
        },
        LatestFrameStore::new(),
        PreviewRuntimeMetricsStore::new(),
        "/missing/ffmpeg".into(),
    )
}

fn install_child(
    process: &RunningPreviewProcess,
    script: &str,
    exited: bool,
) -> Arc<AudioProcessExit> {
    let mut child = Command::new("sh")
        .args(["-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    if exited {
        child.wait().unwrap();
    }
    let stop = Arc::new(AtomicBool::new(false));
    let exit = Arc::new(AudioProcessExit::new(
        Arc::clone(&stop),
        Arc::clone(&process.stderr),
    ));
    *lock_process(&process.audio_process) = RunningProcessState {
        child: Some(child),
        stop_requested: Some(stop),
        audio_exit: Some(Arc::clone(&exit)),
        audio_start: Some(Arc::new(AudioPlaybackStart::new(
            Arc::clone(&process.playback),
            Arc::clone(&process.playback_changed),
        ))),
        ..RunningProcessState::default()
    };
    exit
}

fn pending(exit: Arc<AudioProcessExit>, status: AudioWorkerStatus) -> PendingAudioStart {
    let (ready_tx, ready_rx) = mpsc::channel();
    ready_tx.send(status).unwrap();
    PendingAudioStart::Waiting {
        generation: 0,
        ready_rx,
        exit,
    }
}

#[test]
fn short_pcm_requires_successful_child_exit() {
    for (script, expected) in [
        ("exit 0", AudioStartStatus::Ready),
        ("exit 7", AudioStartStatus::Failed),
    ] {
        let process = new_process();
        let exit = install_child(&process, script, true);
        assert_eq!(
            process.finish_audio_start(pending(exit, AudioWorkerStatus::ReadyAtEof)),
            expected
        );
        if expected == AudioStartStatus::Failed {
            assert!(lock_playback(&process.playback).failed);
            assert!(lock_process(&process.audio_process).child.is_none());
            assert!(
                process
                    .stderr
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|line| line.contains("exit status: 7"))
            );
        }
    }
}

#[test]
fn disconnected_readiness_fails_and_cleans_up() {
    let process = new_process();
    let exit = install_child(&process, "exec sleep 10", false);
    let (ready_tx, ready_rx) = mpsc::channel();
    drop(ready_tx);
    assert_eq!(
        process.finish_audio_start(PendingAudioStart::Waiting {
            generation: 0,
            ready_rx,
            exit
        }),
        AudioStartStatus::Failed
    );
    assert!(lock_playback(&process.playback).failed);
    assert!(lock_process(&process.audio_process).child.is_none());
}

#[test]
fn closed_pipes_without_child_exit_fail_bounded_proof_and_cleanup() {
    let process = new_process();
    let exit = install_child(&process, "exec sleep 10", false);
    let began = Instant::now();
    assert_eq!(
        process.finish_audio_start(pending(exit, AudioWorkerStatus::ReadyAtEof)),
        AudioStartStatus::Failed
    );
    assert!(began.elapsed() < WATCHDOG);
    assert!(lock_process(&process.audio_process).child.is_none());
    assert!(
        process
            .stderr
            .lock()
            .unwrap()
            .iter()
            .any(|line| line.contains("timed out after pipe closure"))
    );
}

#[test]
fn stderr_observer_waits_for_registration_and_records_an_instant_exit() {
    let process = new_process();
    let stop = Arc::new(AtomicBool::new(false));
    let exit = Arc::new(AudioProcessExit::new(
        Arc::clone(&stop),
        Arc::clone(&process.stderr),
    ));
    let (registered_tx, registered_rx) = mpsc::channel();
    let observed = Arc::clone(&exit);
    let state = Arc::clone(&process.audio_process);
    let worker = spawn_stderr_worker_with_completion(
        Cursor::new(Vec::<u8>::new()),
        Arc::clone(&process.stderr),
        Arc::clone(&stop),
        move || {
            observed.observe_after_registration(&state, &registered_rx);
        },
    )
    .unwrap();
    // EOF precedes installation. Registration is the only wake needed here.
    thread::sleep(Duration::from_millis(20));
    assert!(!worker.is_finished());
    let mut child = Command::new("sh").args(["-c", "exit 9"]).spawn().unwrap();
    child.wait().unwrap();
    *lock_process(&process.audio_process) = RunningProcessState {
        child: Some(child),
        stderr_worker: Some(worker),
        stop_requested: Some(stop),
        audio_exit: Some(Arc::clone(&exit)),
        ..RunningProcessState::default()
    };
    registered_tx.send(()).unwrap();
    assert!(
        matches!(exit.wait_for_exit(&process.audio_process, WATCHDOG), AudioExitOutcome::Exited(status) if status.code() == Some(9))
    );
    process.stop_audio_process().unwrap();
    assert_eq!(
        process
            .stderr
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.contains("exit status: 9"))
            .count(),
        1
    );
}

#[test]
fn abandoned_registration_releases_observer() {
    let process = new_process();
    let exit = Arc::new(AudioProcessExit::new(
        Arc::new(AtomicBool::new(false)),
        Arc::clone(&process.stderr),
    ));
    let (registered_tx, registered_rx) = mpsc::channel();
    drop(registered_tx);
    exit.observe_after_registration(&process.audio_process, &registered_rx);
    assert_eq!(
        exit.wait_for_exit(&process.audio_process, WATCHDOG),
        AudioExitOutcome::Cancelled
    );
}

#[test]
fn observer_proof_and_reaper_share_one_exit_classification() {
    for _ in 0..32 {
        let process = Arc::new(new_process());
        let exit = install_child(&process, "exit 6", true);
        let mut workers = Vec::new();
        for _ in 0..2 {
            let observed_process = Arc::clone(&process);
            let observed_exit = Arc::clone(&exit);
            let (done_tx, done_rx) = mpsc::channel();
            let worker = thread::spawn(move || {
                let outcome =
                    observed_exit.wait_for_exit(&observed_process.audio_process, WATCHDOG);
                let _ = done_tx.send(outcome);
            });
            workers.push((worker, done_rx));
        }
        let reaped_process = Arc::clone(&process);
        let (reaped_tx, reaped_rx) = mpsc::channel();
        let reaper = thread::spawn(move || {
            reaped_process.reap_finished_process(&reaped_process.audio_process, false);
            let _ = reaped_tx.send(());
        });
        reaped_rx.recv_timeout(WATCHDOG).expect("reaper watchdog");
        reaper.join().unwrap();
        for (worker, done_rx) in workers {
            assert!(
                matches!(done_rx.recv_timeout(WATCHDOG).expect("exit proof watchdog"), AudioExitOutcome::Exited(status) if status.code() == Some(6))
            );
            worker.join().unwrap();
        }
        assert_eq!(
            process
                .stderr
                .lock()
                .unwrap()
                .iter()
                .filter(|line| line.contains("exit status: 6"))
                .count(),
            1
        );
    }
}

#[test]
fn cancellation_interrupts_exit_observer_and_never_reports_intentional_kill() {
    for drop_process in [false, true] {
        let process = new_process();
        let exit = install_child(&process, "exec sleep 10", false);
        let observed_exit = Arc::clone(&exit);
        let state = Arc::clone(&process.audio_process);
        let lines = Arc::clone(&process.stderr);
        let (done_tx, done_rx) = mpsc::channel();
        let observer = thread::spawn(move || {
            let _ = done_tx.send(observed_exit.wait_for_exit(&state, WATCHDOG));
        });
        lock_process(&process.audio_process).stderr_worker = Some(observer);
        let began = Instant::now();
        let (cleaned_tx, cleaned_rx) = mpsc::channel();
        let cleanup = thread::spawn(move || {
            if drop_process {
                drop(process);
            } else {
                process.stop_audio_process().unwrap();
            }
            let _ = cleaned_tx.send(());
        });
        cleaned_rx
            .recv_timeout(WATCHDOG)
            .expect("stop/drop watchdog");
        cleanup.join().unwrap();
        assert_eq!(
            done_rx.recv_timeout(WATCHDOG).unwrap(),
            AudioExitOutcome::Cancelled
        );
        assert!(began.elapsed() < Duration::from_secs(1));
        assert!(lines.lock().unwrap().is_empty());
    }
}

#[test]
fn stale_short_proof_does_not_stop_replacement_child() {
    let process = new_process();
    let old = install_child(&process, "exec sleep 10", false);
    process.stop_audio_process().unwrap();
    let replacement = install_child(&process, "exec sleep 10", false);
    assert_eq!(
        process.finish_audio_start(pending(old, AudioWorkerStatus::ReadyAtEof)),
        AudioStartStatus::Failed
    );
    let state = lock_process(&process.audio_process);
    assert!(state.child.is_some());
    assert!(Arc::ptr_eq(
        state.audio_exit.as_ref().unwrap(),
        &replacement
    ));
    drop(state);
    assert!(!lock_playback(&process.playback).failed);
}

#[test]
fn late_nonzero_exit_after_ready_is_reported_without_reaping_or_ui_polling() {
    let process = new_process();
    let exit = install_child(&process, "sleep 0.05; exit 17", false);
    let observed_exit = Arc::clone(&exit);
    let state = Arc::clone(&process.audio_process);
    let (registered_tx, registered_rx) = mpsc::channel();
    registered_tx.send(()).unwrap();
    let observer = spawn_stderr_worker_with_completion(
        Cursor::new(Vec::<u8>::new()),
        Arc::clone(&process.stderr),
        Arc::clone(&exit.stop_requested),
        move || {
            observed_exit.observe_after_registration(&state, &registered_rx);
        },
    )
    .unwrap();
    assert_eq!(
        process.finish_audio_start(pending(exit, AudioWorkerStatus::Ready)),
        AudioStartStatus::Ready
    );
    let (done_tx, done_rx) = mpsc::channel();
    let joiner = thread::spawn(move || {
        let _ = done_tx.send(observer.join());
    });
    done_rx
        .recv_timeout(WATCHDOG)
        .expect("exit observer watchdog")
        .unwrap();
    joiner.join().unwrap();
    assert!(
        process
            .stderr
            .lock()
            .unwrap()
            .iter()
            .any(|line| line.contains("exit status: 17"))
    );
}

struct PcmReader {
    bytes: Cursor<Vec<u8>>,
    error: Option<std::io::ErrorKind>,
}

impl std::io::Read for PcmReader {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = std::io::Read::read(&mut self.bytes, bytes)?;
        if count == 0
            && let Some(error) = self.error
        {
            return Err(std::io::Error::new(
                error,
                "injected short PCM read failure",
            ));
        }
        Ok(count)
    }
}

#[test]
#[ignore = "requires a default audio output device; run with --ignored"]
fn real_output_short_pcm_eof_matrix_preserves_frames_and_rejects_invalid_input() {
    use super::audio_stream::{
        AudioStdoutWorkerConfig, audio_prebuffer_samples, spawn_audio_stdout_worker,
    };
    let spec = AudioOutputSpec::default_output().unwrap();
    let frame_bytes = usize::from(spec.channels) * 4;
    let frames_10ms = usize::try_from(spec.sample_rate).unwrap() / 100;
    let mut cases = vec![
        (frame_bytes, None, true),
        (frames_10ms * frame_bytes, None, true),
        (frames_10ms * 2 * frame_bytes, None, true),
        (0, None, false),
    ];
    for remainder in 1..frame_bytes {
        cases.push((frame_bytes + remainder, None, false));
        cases.push((remainder, None, false));
    }
    for error in [
        std::io::ErrorKind::Other,
        std::io::ErrorKind::UnexpectedEof,
        std::io::ErrorKind::BrokenPipe,
    ] {
        cases.push((frame_bytes, Some(error), false));
    }
    for (bytes, error, valid) in cases {
        let process = new_process();
        let exit = install_child(&process, "exit 0", true);
        let start = Arc::clone(
            lock_process(&process.audio_process)
                .audio_start
                .as_ref()
                .unwrap(),
        );
        let (ready_tx, ready_rx) = mpsc::channel();
        let worker = spawn_audio_stdout_worker(
            PcmReader {
                bytes: Cursor::new(vec![0; bytes]),
                error,
            },
            AudioStdoutWorkerConfig {
                stderr_lines: Arc::clone(&process.stderr),
                playback: Arc::clone(&process.playback),
                metrics: process.metrics.clone(),
                stop_requested: Arc::clone(&exit.stop_requested),
                audio_start: Arc::clone(&start),
                mark_ended_on_eof: true,
                generation: 0,
                prebuffer_samples: audio_prebuffer_samples(spec),
                ready_tx,
            },
        )
        .unwrap();
        lock_process(&process.audio_process).stdout_worker = Some(worker);
        let result = process.finish_audio_start(PendingAudioStart::Waiting {
            generation: 0,
            ready_rx,
            exit,
        });
        assert_eq!(
            result,
            if valid {
                AudioStartStatus::Ready
            } else {
                AudioStartStatus::Failed
            },
            "bytes={bytes}, error={error:?}"
        );
        if valid {
            process.start_playback_generation(0, 0.0);
            let deadline = Instant::now() + WATCHDOG;
            while !lock_process(&process.audio_process)
                .stdout_worker
                .as_ref()
                .unwrap()
                .is_finished()
            {
                assert!(
                    Instant::now() < deadline,
                    "short PCM drain watchdog, bytes={bytes}"
                );
                thread::sleep(Duration::from_millis(2));
            }
            let metrics = process.metrics.snapshot();
            assert_eq!(
                metrics.audio_samples_queued,
                u64::try_from(bytes / 4).unwrap()
            );
            assert_eq!(
                metrics.audio_samples_consumed,
                u64::try_from(bytes / 4).unwrap()
            );
            assert!(lock_playback(&process.playback).ended);
        } else {
            assert!(lock_playback(&process.playback).failed);
            assert!(!lock_playback(&process.playback).ended);
            assert!(lock_process(&process.audio_process).child.is_none());
        }
    }
}

#[test]
#[ignore = "requires a default audio output device; run with --ignored"]
fn real_output_observes_short_and_late_decoder_failures_with_empty_stderr() {
    use std::os::unix::fs::PermissionsExt;
    let spec = AudioOutputSpec::default_output().unwrap();
    let frame_bytes = usize::from(spec.channels) * 4;
    let prebuffer_bytes = super::audio_stream::audio_prebuffer_samples(spec) * 4;
    let cases = [
        (frame_bytes, "exit 0", AudioStartStatus::Ready, None),
        (
            frame_bytes,
            "exit 23",
            AudioStartStatus::Failed,
            Some("exit status: 23"),
        ),
        (
            frame_bytes,
            "exec 1>&- 2>&-; exec sleep 10",
            AudioStartStatus::Failed,
            Some("timed out after pipe closure"),
        ),
        (
            prebuffer_bytes,
            "sleep 0.05; exit 29",
            AudioStartStatus::Ready,
            Some("exit status: 29"),
        ),
    ];
    for (index, (bytes, ending, expected, diagnostic)) in cases.into_iter().enumerate() {
        let path = std::env::temp_dir().join(format!(
            "frame-audio-exit-{}-{index}.sh",
            std::process::id()
        ));
        std::fs::write(
            &path,
            format!("#!/bin/sh\nhead -c {bytes} /dev/zero\n{ending}\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut process = new_process();
        process.executable = path.to_str().unwrap().into();
        let result = process.start_audio_streaming(0.0, true, true, 0).unwrap();
        assert_eq!(result, expected, "case {index}");
        if result == AudioStartStatus::Ready {
            process.start_playback_generation(0, 0.0);
        }
        if let Some(diagnostic) = diagnostic {
            let deadline = Instant::now() + WATCHDOG;
            // Inspect only diagnostics: no process reaper or UI snapshot drives
            // this late-status observation.
            while !process
                .stderr
                .lock()
                .unwrap()
                .iter()
                .any(|line| line.contains(diagnostic))
            {
                assert!(
                    Instant::now() < deadline,
                    "missing diagnostic for case {index}: {diagnostic}"
                );
                thread::sleep(Duration::from_millis(2));
            }
        }
        if expected == AudioStartStatus::Failed {
            assert!(lock_process(&process.audio_process).child.is_none());
            assert!(lock_playback(&process.playback).failed);
        }
        drop(process);
        std::fs::remove_file(path).unwrap();
    }
}
