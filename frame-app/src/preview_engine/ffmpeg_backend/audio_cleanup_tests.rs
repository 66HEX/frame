use super::audio_exit::AudioProcessExit;
use super::audio_stream::{AudioWorkerStatus, PendingAudioStart, lock_pending_audio_start};
use super::process::{
    RunningPreviewProcess, RunningProcessState, lock_process, spawn_stderr_worker,
};
use super::*;
use crate::preview_engine::LatestFrameStore;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;

struct Directory(std::path::PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-audio-fallback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fallback_config() -> PreviewSessionConfig {
    let mut config = super::tests::preview_config();
    config.source_width = Some(160);
    config.source_height = Some(90);
    config.max_width = 160;
    config.max_height = 90;
    config.selected_audio_track = None;
    config
}

#[test]
fn failed_audio_fallback_does_not_block_video_on_an_inherited_pipe() {
    let directory = Directory::new();
    let executable = directory.0.join("video-ffmpeg");
    fs::write(
        &executable,
        "#!/bin/sh\nexec dd if=/dev/zero bs=57600 count=1 2>/dev/null\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let config = fallback_config();
    let process = Arc::new(
        RunningPreviewProcess::new(
            config.clone(),
            LatestFrameStore::new(),
            PreviewRuntimeMetricsStore::new(),
            executable.to_string_lossy().into_owned(),
        )
        .unwrap(),
    );
    process.park_playback_generation(0, 0.0);
    let marker = directory.0.join("audio-started");
    let marker_quoted = marker.to_string_lossy().replace('\'', "'\\''");
    let mut child = crate::process_launch::spawn(
        Command::new("sh")
            .args([
                "-c",
                &format!("sleep 3 &\nprintf started > '{marker_quoted}'\nexec sleep 3"),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .unwrap();
    let child_pid = child.id();
    let stop = Arc::new(AtomicBool::new(false));
    let exit = Arc::new(AudioProcessExit::new(
        Arc::clone(&stop),
        Arc::clone(&process.stderr),
    ));
    let stderr_worker = spawn_stderr_worker(
        child.stderr.take().unwrap(),
        Arc::clone(&process.stderr),
        Arc::clone(&stop),
    )
    .unwrap();
    *lock_process(&process.audio_process) = RunningProcessState {
        child: Some(child),
        stderr_worker: Some(stderr_worker),
        stop_requested: Some(stop),
        audio_exit: Some(Arc::clone(&exit)),
        ..RunningProcessState::default()
    };
    let (ready_tx, ready_rx) = mpsc::channel();
    ready_tx.send(AudioWorkerStatus::Failed).unwrap();
    *lock_pending_audio_start(&process.pending_audio_start) = Some(PendingAudioStart::Waiting {
        generation: 0,
        ready_rx,
        exit,
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let (done_tx, done_rx) = mpsc::channel();
    let starting = Arc::clone(&process);
    let next_config = config.clone();
    let worker = thread::spawn(move || {
        done_tx
            .send(starting.start_video_streaming_with_config(&next_config, 0.0, true))
            .unwrap();
    });
    let result = done_rx.recv_timeout(Duration::from_secs(2));
    worker.join().unwrap();
    process.stop_running_processes().unwrap();
    let error = result
        .expect("failed audio fallback must not join an inherited stderr pipe without a bound")
        .expect_err("unfinished audio cleanup must reject the new video candidate");
    assert!(error.to_string().contains("cleanup"), "{error}");
    // SAFETY: signal zero only queries our directly owned child's PID.
    assert_eq!(
        unsafe { libc::kill(i32::try_from(child_pid).unwrap(), 0) },
        -1
    );
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match process.start_video_streaming_with_config(&config, 0.0, true) {
            Ok(_) => break,
            Err(error) => {
                assert!(error.to_string().contains("CleanupInProgress"), "{error}");
                assert!(
                    Instant::now() < deadline,
                    "audio cleanup never released its permit"
                );
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    process.stop_running_processes().unwrap();
}
