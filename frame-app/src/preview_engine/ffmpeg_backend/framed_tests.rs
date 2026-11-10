use super::clock::lock_playback;
use super::decode::{FrameStreamSpec, RenderImageIdentity, read_stream_frame};
use super::framed;
use super::plan::{framed_preview_plan, preview_plan, snapshot_preview_plan};
use super::process::RunningPreviewProcess;
use super::tests::preview_config;
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use crate::preview_engine::{LatestFrameStore, PreviewRenderPresentation, PreviewSession};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn config() -> crate::preview_engine::PreviewSessionConfig {
    let mut config = preview_config();
    config.path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tooling/ffmpeg/frame-preview-v1/probes/dynamic.m2ts"
    )
    .into();
    config.source_is_mpeg_ts = true;
    config.source_width = Some(64);
    config.source_height = Some(48);
    config.max_width = 32;
    config.max_height = 24;
    config.fps = 12;
    config.duration_seconds = 3.0;
    config.request_token = 507;
    config.conversion_config.processing_mode = "copy".into();
    config
}

#[test]
fn framed_plans_preserve_seek_trim_fps_and_snapshot_semantics() {
    let mut config = config();
    config.conversion_config.end_time = Some("2.5170009".into());
    for snapshot in [false, true] {
        for precise in [false, true] {
            let ordinary = if snapshot {
                snapshot_preview_plan(&config, 0.217_000_9, precise)
            } else {
                preview_plan(&config, 0.217_000_9, true, precise)
            }
            .unwrap();
            let mut framed = framed_preview_plan(&config, 0.217_000_9, precise, snapshot).unwrap();
            framed.args.retain(|arg| {
                !matches!(
                    arg.as_str(),
                    "-native_frame_sar:v:0" | "-frame_preview_framed"
                )
            });
            let codec = framed.args.iter().position(|arg| arg == "-c:v").unwrap();
            framed.args.drain(codec..codec + 2);
            for arg in &mut framed.args {
                *arg = arg.replace(":frame_preview_capture=1", "");
            }
            assert_eq!(framed, ordinary);
        }
    }
    config.conversion_config.processing_mode = "reencode".into();
    assert!(framed_preview_plan(&config, 0.0, true, true).is_err());
}

#[test]
fn protocol_is_not_probed_for_unrelated_sources_or_output_preview() {
    let cancelled = AtomicBool::new(false);
    let mut config = config();
    config.source_is_mpeg_ts = false;
    assert!(
        framed::prepare("/missing/ffmpeg", &config, &cancelled)
            .unwrap()
            .is_none()
    );
    config.source_is_mpeg_ts = true;
    config.conversion_config.processing_mode = "reencode".into();
    assert!(
        framed::prepare("/missing/ffmpeg", &config, &cancelled)
            .unwrap()
            .is_none()
    );
}

#[test]
fn frame_reader_publishes_packet_presentation_with_its_request_token() {
    let wire = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tooling/ffmpeg/frame-preview-v1/probes/dynamic.fpv"
    ));
    let spec = FrameStreamSpec {
        framed: true,
        presentation: PreviewRenderPresentation::default(),
        request_token: 507,
        width: 4,
        height: 2,
        fps: 12,
        frame_bytes: 32,
        base_seconds: 0.0,
        first_frame_index: 12,
        generation: 9,
        spawned_at: Instant::now(),
    };
    let decoded = read_stream_frame(
        &mut Cursor::new(&wire[12 * 112..13 * 112]),
        &spec,
        1.0,
        RenderImageIdentity {
            image_id: gpui::RenderImage::new_image_id(),
            content_version: 7,
        },
    )
    .unwrap();
    assert_eq!(decoded.frame.request_token, 507);
    assert_eq!(
        decoded.frame.presentation.pixel_aspect_ratio.exact_ratio(),
        Some((4, 3))
    );
    assert_eq!(
        decoded
            .frame
            .presentation
            .output_raster_aspect_ratio
            .unwrap()
            .exact_ratio(),
        Some((4, 3))
    );
    assert_eq!(decoded.frame.timestamp_us, 1_000_000);
    assert_eq!(
        decoded.frame.render_image().as_bytes(0),
        Some(&wire[80..112])
    );
}

#[test]
fn pre_cancelled_session_does_not_launch_an_initial_preview() {
    assert!(
        PreviewSession::start_with_cancellation(config(), Arc::new(AtomicBool::new(true))).is_err()
    );
}

fn patched_executable() -> String {
    std::env::var("FRAME_TEST_FRAMED_FFMPEG")
        .expect("set FRAME_TEST_FRAMED_FFMPEG to the pinned patched runtime")
}

#[test]
#[ignore = "requires the pinned patched FFmpeg runtime"]
fn actual_framed_snapshots_follow_sar_and_keep_failed_replacement_pixels() {
    let mut process = RunningPreviewProcess::new(
        config(),
        LatestFrameStore::new(),
        PreviewRuntimeMetricsStore::new(),
        patched_executable(),
    );
    let mut pixels = None;
    for (seconds, expected) in [(0.0, 1), (1.25, 2), (2.25, 1)] {
        process.render_single_frame(seconds, true).unwrap();
        let latest = process.frame_store.latest().unwrap();
        assert_eq!(latest.frame.request_token, 507);
        assert_eq!(
            latest.frame.presentation.pixel_aspect_ratio.exact_ratio(),
            Some((expected, 1))
        );
        let bytes = latest.frame.render_image().as_bytes(0).unwrap().to_vec();
        if let Some(expected) = &pixels {
            assert_eq!(&bytes, expected);
        } else {
            pixels = Some(bytes);
        }
    }
    let retained = process.frame_store.latest().unwrap();
    let mut invalid = config();
    invalid.path = "/missing/replacement.m2ts".into();
    invalid.request_token = 508;
    assert!(process.reconfigure(invalid, 0.0, false, true).is_err());
    let after = process.frame_store.latest().unwrap();
    assert_eq!(after.generation, retained.generation);
    assert_eq!(after.frame, retained.frame);
}

#[test]
#[ignore = "requires the pinned patched FFmpeg runtime"]
fn actual_framed_playback_keeps_identical_pixels_paired_with_each_sar_phase() {
    let process = RunningPreviewProcess::new(
        config(),
        LatestFrameStore::new(),
        PreviewRuntimeMetricsStore::new(),
        patched_executable(),
    );
    process.start_streaming(0.0, true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut phases = Vec::new();
    let mut pixels = None;
    loop {
        if let Some(latest) = process.frame_store.latest() {
            let ratio = latest
                .frame
                .presentation
                .pixel_aspect_ratio
                .exact_ratio()
                .unwrap();
            if phases.last() != Some(&ratio) {
                phases.push(ratio);
            }
            assert_eq!(latest.frame.request_token, 507);
            let bytes = latest.frame.render_image().as_bytes(0).unwrap().to_vec();
            if let Some(expected) = &pixels {
                assert_eq!(&bytes, expected);
            } else {
                pixels = Some(bytes);
            }
            process.frame_store.mark_presented(latest.generation);
        }
        let clock = *lock_playback(&process.playback);
        assert!(!clock.failed);
        if clock.ended {
            break;
        }
        assert!(Instant::now() < deadline, "framed playback did not end");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(phases, vec![(1, 1), (2, 1), (1, 1)]);
    assert!(
        super::process::lock_process(&process.process)
            .child
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn cancellation_interrupts_first_framed_read_without_restarting_the_child() {
    use super::video_stream::SpawnedVideoStream;
    use std::process::{Command, Stdio};
    let mut child = Command::new("sh")
        .args(["-c", "exec sleep 30"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stream = SpawnedVideoStream {
        child,
        stdout: Box::new(stdout),
        stderr_worker: std::thread::spawn(|| {}),
        stop_requested: Arc::new(AtomicBool::new(false)),
        spec: FrameStreamSpec {
            framed: true,
            presentation: PreviewRenderPresentation::default(),
            request_token: 99,
            width: 4,
            height: 2,
            fps: 12,
            frame_bytes: 32,
            base_seconds: 0.0,
            first_frame_index: 0,
            generation: 0,
            spawned_at: Instant::now(),
        },
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        signal.store(true, Ordering::Release);
    });
    let started = Instant::now();
    assert!(
        super::framed_stream::read_first_frame(
            &mut stream,
            0.0,
            RenderImageIdentity {
                image_id: gpui::RenderImage::new_image_id(),
                content_version: 1
            },
            &cancelled
        )
        .is_err()
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(stream.child.try_wait().unwrap().is_some());
    sender.join().unwrap();
    stream.stderr_worker.join().unwrap();
}

#[cfg(unix)]
#[test]
fn framed_worker_owns_failure_and_exit_without_relabeling_retained_pixels() {
    use super::framed_stream::FramedStreamOwner;
    use super::process::{RunningProcessState, lock_process};
    use super::video_stream::{VideoStdoutWorkerConfig, spawn_stdout_worker};
    use std::process::{Command, Stdio};
    for (command, wire, should_fail) in [
        ("exec sleep 30", vec![0; 12], true),
        ("exit 14", Vec::new(), true),
        ("exit 0", Vec::new(), false),
    ] {
        let process = RunningPreviewProcess::new(
            config(),
            LatestFrameStore::new(),
            PreviewRuntimeMetricsStore::new(),
            "/unused".into(),
        );
        let retained = process.frame_store.publish(
            crate::preview_engine::rendered_frame_from_bgra_payload(4, 2, 16, 0, vec![42; 32])
                .unwrap()
                .with_presentation(PreviewRenderPresentation::default(), 506),
        );
        process.frame_store.mark_presented(retained.generation);
        let generation = process.prepare_playback_generation(0.0);
        let child = Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let spec = FrameStreamSpec {
            framed: true,
            presentation: PreviewRenderPresentation::default(),
            request_token: 507,
            width: 4,
            height: 2,
            fps: 12,
            frame_bytes: 32,
            base_seconds: 0.0,
            first_frame_index: 1,
            generation,
            spawned_at: Instant::now(),
        };
        let worker = spawn_stdout_worker(
            Cursor::new(wire),
            VideoStdoutWorkerConfig {
                frame_store: process.frame_store.clone(),
                playback: Arc::clone(&process.playback),
                playback_changed: Arc::clone(&process.playback_changed),
                metrics: process.metrics.clone(),
                stop_requested: Arc::clone(&stop),
                render_image_id: process.render_image_id,
                render_image_version: Arc::clone(&process.render_image_version),
                spec,
                framed_owner: Some(FramedStreamOwner {
                    process: Arc::clone(&process.process),
                    audio: Arc::clone(&process.audio_process),
                    pending_audio: Arc::clone(&process.pending_audio_start),
                    stderr: Arc::clone(&process.stderr),
                }),
            },
        )
        .unwrap();
        *lock_process(&process.process) = RunningProcessState {
            framed_video: true,
            child: Some(child),
            stdout_worker: Some(worker),
            stop_requested: Some(stop),
            ..RunningProcessState::default()
        };
        process.start_playback_generation(generation, 0.0);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            // Exercise the competing normal reaper while the reader owns EOF.
            process.reap_finished_processes();
            if lock_process(&process.process)
                .stdout_worker
                .as_ref()
                .is_some_and(std::thread::JoinHandle::is_finished)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "worker did not clean up {command}"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        let clock = *lock_playback(&process.playback);
        assert_eq!(clock.failed, should_fail);
        assert_eq!(clock.ended, !should_fail);
        assert!(lock_process(&process.process).child.is_none());
        let current = process.frame_store.latest().unwrap();
        assert_eq!(current.generation, retained.generation);
        assert_eq!(current.frame, retained.frame);
        process.stop_running_processes().unwrap();
    }
}
