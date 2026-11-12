use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

struct SilentPreviewChild {
    directory: PathBuf,
    executable: PathBuf,
    marker: PathBuf,
}

impl SilentPreviewChild {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "frame-preview-start-cancel-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let executable = directory.join("silent-ffmpeg");
        let marker = directory.join("child.pid");
        let marker_quoted = marker.to_string_lossy().replace('\'', "'\\''");
        fs::write(
            &executable,
            format!("#!/bin/sh\nprintf '%s' \"$$\" > '{marker_quoted}'\nexec sleep 30\n"),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory,
            executable,
            marker,
        }
    }

    fn wait_for_pid(&self) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Ok(value) = fs::read_to_string(&self.marker)
                && let Ok(pid) = value.parse()
            {
                return pid;
            }
            assert!(Instant::now() < deadline, "preview child did not start");
            thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for SilentPreviewChild {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn video_config() -> PreviewSessionConfig {
    PreviewSessionConfig {
        file_id: "startup-cancellation".into(),
        path: "/missing/preview.mp4".into(),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(160),
        source_height: Some(90),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 1,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        selected_audio_track: None,
        duration_seconds: 2.0,
        max_width: 320,
        max_height: 180,
        fps: 30,
        conversion_config: crate::conversion_runner::core_config_from_gpui(
            &crate::settings::ConversionConfig::default(),
        ),
    }
}

#[test]
fn stop_cancels_an_inflight_snapshot_before_waiting_for_pipeline_lock() {
    let fake = SilentPreviewChild::new();
    let video = video_config();
    let mut audio = video.clone();
    audio.source_kind = PreviewSourceKind::Audio;
    audio.source_width = None;
    audio.source_height = None;
    let session = Arc::new(PreviewSession::start(audio).unwrap());
    lock(&session.pipeline)
        .as_mut()
        .unwrap()
        .set_test_executable(fake.executable.to_string_lossy().into_owned());

    let worker_session = Arc::clone(&session);
    let worker = thread::spawn(move || worker_session.reconfigure(video));
    let pid = fake.wait_for_pid();
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let stopping_session = Arc::clone(&session);
    let stop_worker = thread::spawn(move || {
        stopped_tx.send(stopping_session.stop()).unwrap();
    });
    let stopped = stopped_rx.recv_timeout(Duration::from_secs(1));
    if stopped.is_err() {
        // Release the controlled child before asserting, including on the RED run.
        // SAFETY: this PID was just written by our still-sleeping, unreaped child.
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    let rendered = worker.join().unwrap();
    stop_worker.join().unwrap();

    stopped
        .expect("Stop must cancel the snapshot without waiting for its process to exit naturally")
        .expect("stopped preview");
    assert!(rendered.is_err(), "cancelled snapshot must not succeed");
    assert!(session.latest_frame().is_none());
    // SAFETY: signal zero only queries whether the child still exists.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "child was not reaped");
}

#[test]
fn stop_cancels_stream_start_before_the_first_frame_arrives() {
    let fake = SilentPreviewChild::new();
    let silence = fs::read(&fake.executable).unwrap();
    fs::write(
        &fake.executable,
        "#!/bin/sh\nexec dd if=/dev/zero bs=57600 count=1 2>/dev/null\n",
    )
    .unwrap();
    let video = video_config();
    let mut audio = video.clone();
    audio.source_kind = PreviewSourceKind::Audio;
    let session = Arc::new(PreviewSession::start(audio).unwrap());
    lock(&session.pipeline)
        .as_mut()
        .unwrap()
        .set_test_executable(fake.executable.to_string_lossy().into_owned());
    session.reconfigure(video).unwrap();
    let generation = session.latest_frame().unwrap().generation;
    fs::write(&fake.executable, silence).unwrap();

    let playing_session = Arc::clone(&session);
    let playing = thread::spawn(move || playing_session.command(PreviewCommand::Play));
    let pid = fake.wait_for_pid();
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let stopping_session = Arc::clone(&session);
    let stopping = thread::spawn(move || {
        stopped_tx.send(stopping_session.stop()).unwrap();
    });
    let stopped = stopped_rx.recv_timeout(Duration::from_secs(1));
    if stopped.is_err() {
        // SAFETY: the controlled child is still alive and has not been reaped.
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    let result = playing.join().unwrap();
    stopping.join().unwrap();
    stopped
        .expect("Stop must cancel first-frame I/O without waiting for the startup lock")
        .unwrap();
    assert!(result.is_err());
    assert_eq!(session.latest_frame().unwrap().generation, generation);
    assert!(!session.snapshot().playback.playing);
    // SAFETY: signal zero only queries whether the child still exists.
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "startup child was not reaped"
    );
}

#[test]
fn cancelled_stream_start_does_not_join_an_inherited_stderr_pipe_forever() {
    let fake = SilentPreviewChild::new();
    fs::write(
        &fake.executable,
        "#!/bin/sh\nexec dd if=/dev/zero bs=57600 count=1 2>/dev/null\n",
    )
    .unwrap();
    let video = video_config();
    let mut audio = video.clone();
    audio.source_kind = PreviewSourceKind::Audio;
    let session = Arc::new(PreviewSession::start(audio).unwrap());
    lock(&session.pipeline)
        .as_mut()
        .unwrap()
        .set_test_executable(fake.executable.to_string_lossy().into_owned());
    session.reconfigure(video).unwrap();
    let marker_quoted = fake.marker.to_string_lossy().replace('\'', "'\\''");
    fs::write(
        &fake.executable,
        format!("#!/bin/sh\nsleep 3 &\nprintf '%s' \"$$\" > '{marker_quoted}'\nexec sleep 3\n"),
    )
    .unwrap();
    let playing_session = Arc::clone(&session);
    let playing = thread::spawn(move || playing_session.command(PreviewCommand::Play));
    let pid = fake.wait_for_pid();
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let stopping_session = Arc::clone(&session);
    let stopping = thread::spawn(move || {
        stopped_tx.send(stopping_session.stop()).unwrap();
    });
    let stopped = stopped_rx.recv_timeout(Duration::from_secs(2));
    let result = playing.join().unwrap();
    stopping.join().unwrap();
    stopped
        .expect("startup cleanup must return before the inherited pipe's writer exits")
        .unwrap();
    assert!(result.is_err());
    // SAFETY: signal zero only queries whether our direct child remains alive.
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "direct child was not reaped"
    );

    let retry_fake = SilentPreviewChild::new();
    let retry_marker = retry_fake.marker.to_string_lossy().replace('\'', "'\\''");
    fs::write(
        &retry_fake.executable,
        format!("#!/bin/sh\nprintf '%s' \"$$\" > '{retry_marker}'\nexec dd if=/dev/zero bs=57600 count=1 2>/dev/null\n"),
    )
    .unwrap();
    let video = video_config();
    let mut audio = video.clone();
    audio.source_kind = PreviewSourceKind::Audio;
    let retry = PreviewSession::start(audio).unwrap();
    lock(&retry.pipeline)
        .as_mut()
        .unwrap()
        .set_test_executable(retry_fake.executable.to_string_lossy().into_owned());
    retry.reconfigure(video).unwrap();
    fs::remove_file(&retry_fake.marker).unwrap();
    let blocked = retry.command(PreviewCommand::Play).unwrap_err();
    assert!(
        blocked.to_string().contains("CleanupInProgress"),
        "{blocked}"
    );
    assert!(
        !retry_fake.marker.exists(),
        "pending startup cleanup allowed another child"
    );
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match retry.command(PreviewCommand::Play) {
            Ok(()) => break,
            Err(error) => {
                assert!(
                    Instant::now() < deadline,
                    "cleanup never released the startup budget: {error}"
                );
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    assert!(retry_fake.marker.exists());
    retry.stop().unwrap();
}

#[test]
fn cancellation_guard_rejects_a_queued_play_command() {
    let mut config = video_config();
    config.source_kind = PreviewSourceKind::Audio;
    let session = PreviewSession::start(config).unwrap();
    session.cancellation_token().store(true, Ordering::Release);
    assert!(session.command(PreviewCommand::Play).is_err());
    assert!(!session.snapshot().playback.playing);
}
