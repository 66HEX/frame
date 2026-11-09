#[cfg(unix)]
use super::audio_stream::{
    AudioStartStatus, AudioWorkerStatus, PendingAudioStart, lock_pending_audio_start,
};
use super::clock::{
    AudioPlaybackStart, PlaybackStreamState, lock_playback, wait_for_playback_start,
};
use super::process::{RunningPreviewProcess, RunningProcessState, lock_process};
use super::*;
use crate::preview_engine::LatestFrameStore;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

const WATCHDOG: Duration = Duration::from_secs(3);

fn new_process() -> RunningPreviewProcess {
    RunningPreviewProcess::new(
        PreviewSessionConfig {
            file_id: "audio-start-wait".into(),
            path: "/missing/preview.mp4".into(),
            source_kind: PreviewSourceKind::Video,
            source_width: Some(160),
            source_height: Some(90),
            source_is_palette: false,
            selected_audio_track: Some(1),
            duration_seconds: 2.0,
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

struct Waiter {
    start: Arc<AudioPlaybackStart>,
    stop: Arc<AtomicBool>,
    done: Receiver<PlaybackStreamState>,
    worker: Option<JoinHandle<()>>,
}

impl Waiter {
    fn new(process: &RunningPreviewProcess) -> Self {
        let start = Arc::new(AudioPlaybackStart::new(
            Arc::clone(&process.playback),
            Arc::clone(&process.playback_changed),
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let (done_tx, done) = mpsc::channel();
        let worker_start = Arc::clone(&start);
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            let state = wait_for_playback_start(&worker_start, &worker_stop, 0);
            let _ = done_tx.send(state);
        });
        Self {
            start,
            stop,
            done,
            worker: Some(worker),
        }
    }

    fn wait_until_parked(&self, process: &RunningPreviewProcess, minimum_waits: usize) {
        let deadline = Instant::now() + WATCHDOG;
        while self.start.wait_count.load(Ordering::SeqCst) < minimum_waits {
            assert!(
                Instant::now() < deadline,
                "audio worker did not enter its wait"
            );
            thread::sleep(Duration::from_millis(1));
        }
        // The counter increments with the clock locked. Acquiring it here
        // proves the worker released the mutex by entering Condvar::wait.
        drop(lock_playback(&process.playback));
        assert!(matches!(
            self.done.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    fn finish(mut self, expected: PlaybackStreamState) {
        assert_eq!(
            self.done
                .recv_timeout(WATCHDOG)
                .expect("audio wait watchdog"),
            expected
        );
        if let Some(worker) = self.worker.take() {
            worker.join().expect("audio wait worker panicked");
        }
    }

    fn install(&mut self, process: &RunningPreviewProcess) {
        *lock_process(&process.audio_process) = RunningProcessState {
            stdout_worker: self.worker.take(),
            stop_requested: Some(Arc::clone(&self.stop)),
            audio_start: Some(Arc::clone(&self.start)),
            ..RunningProcessState::default()
        };
    }
}

#[test]
fn parked_audio_wait_rechecks_spurious_wakes_and_starts() {
    let process = new_process();
    let waiter = Waiter::new(&process);
    waiter.wait_until_parked(&process, 1);
    process.playback_changed.notify_all();
    waiter.wait_until_parked(&process, 2);
    process.start_playback_generation(0, 0.0);
    waiter.finish(PlaybackStreamState::Ready);
}

#[test]
fn parked_audio_wait_is_released_by_stop_generation_replacement_and_end() {
    for event in ["stop", "generation", "ended"] {
        let process = new_process();
        let waiter = Waiter::new(&process);
        waiter.wait_until_parked(&process, 1);
        match event {
            "stop" => waiter.start.request_stop(&waiter.stop),
            "generation" => {
                process.prepare_playback_generation(1.0);
            }
            "ended" => {
                waiter.start.mark_ended(0);
            }
            _ => unreachable!(),
        }
        waiter.finish(PlaybackStreamState::Stale);
    }
}

#[test]
fn audio_start_and_stop_do_not_lose_wakes_before_or_during_wait() {
    for _ in 0..64 {
        let process = new_process();
        process.start_playback_generation(0, 0.0);
        Waiter::new(&process).finish(PlaybackStreamState::Ready);

        let process = new_process();
        let waiter = Waiter::new(&process);
        waiter.start.request_stop(&waiter.stop);
        waiter.finish(PlaybackStreamState::Stale);

        let process = new_process();
        let waiter = Waiter::new(&process);
        process.start_playback_generation(0, 0.0);
        waiter.finish(PlaybackStreamState::Ready);
    }
}

#[test]
fn stopping_and_dropping_process_join_a_parked_audio_worker() {
    for drop_process in [false, true] {
        let process = new_process();
        let mut waiter = Waiter::new(&process);
        waiter.wait_until_parked(&process, 1);
        waiter.install(&process);
        #[cfg(unix)]
        {
            lock_process(&process.audio_process).child = Some(exited_child());
        }
        let (done_tx, done) = mpsc::channel();
        let cleanup = thread::spawn(move || {
            if drop_process {
                drop(process);
            } else {
                process.stop_audio_process().expect("stop audio");
            }
            let _ = done_tx.send(());
        });
        done.recv_timeout(WATCHDOG)
            .expect("process cleanup watchdog");
        cleanup.join().expect("cleanup worker");
        waiter.finish(PlaybackStreamState::Stale);
    }
}

#[cfg(unix)]
fn exited_child() -> std::process::Child {
    let mut child = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .unwrap();
    child.wait().expect("child exit");
    child
}

#[cfg(unix)]
#[test]
fn reaping_exited_child_preserves_parked_audio_until_play() {
    let process = Arc::new(new_process());
    let mut waiter = Waiter::new(&process);
    waiter.wait_until_parked(&process, 1);
    waiter.install(&process);
    lock_process(&process.audio_process).child = Some(exited_child());
    let (ready_tx, ready_rx) = mpsc::channel();
    ready_tx.send(AudioWorkerStatus::Ready).unwrap();
    *lock_pending_audio_start(&process.pending_audio_start) = Some(PendingAudioStart::Waiting {
        generation: 0,
        ready_rx,
        exit: Arc::new(super::audio_exit::AudioProcessExit::new(
            Arc::clone(&waiter.stop),
            Arc::clone(&process.stderr),
        )),
    });
    let (done_tx, done) = mpsc::channel();
    let reaper_process = Arc::clone(&process);
    let reaper = thread::spawn(move || {
        assert!(!reaper_process.reap_finished_process(&reaper_process.audio_process, false));
        let _ = done_tx.send(());
    });
    done.recv_timeout(WATCHDOG).expect("reap watchdog");
    reaper.join().expect("reaper worker");
    assert!(
        lock_process(&process.audio_process)
            .stop_requested
            .is_some()
    );
    assert!(!waiter.stop.load(Ordering::SeqCst));
    let pending = process
        .take_pending_audio_start_for_generation(0)
        .expect("reaping must retain prewarmed readiness");
    assert_eq!(process.finish_audio_start(pending), AudioStartStatus::Ready);
    process.start_playback_generation(0, 0.0);
    waiter.finish(PlaybackStreamState::Ready);
    let deadline = Instant::now() + WATCHDOG;
    while !process.reap_finished_process(&process.audio_process, false) {
        assert!(Instant::now() < deadline, "finished worker was not reaped");
        thread::sleep(Duration::from_millis(1));
    }
}
