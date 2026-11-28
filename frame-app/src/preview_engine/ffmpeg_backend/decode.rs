use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::clock::seconds_to_us;
use crate::preview_engine::ffmpeg_backend::plan::{insert_frame_limit, snapshot_preview_plan};
use crate::preview_engine::ffmpeg_backend::process::{RunningPreviewProcess, stderr_detail};
use crate::preview_engine::ffmpeg_backend::startup_pipe::STARTUP_TIMEOUT;
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use crate::preview_engine::renderer::rendered_frame_from_bgra_payload_with_image_id;
use crate::preview_engine::types::{PreviewDimensions, PreviewRenderedFrame, PreviewSessionConfig};
use gpui::ImageId;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(super) struct FrameStreamSpec {
    pub(super) presentation: crate::preview_engine::PreviewRenderPresentation,
    pub(super) request_token: u64,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) fps: u32,
    pub(super) frame_bytes: usize,
    pub(super) base_seconds: f64,
    pub(super) first_frame_index: u64,
    pub(super) generation: u64,
    pub(super) spawned_at: Instant,
}

pub(super) struct DecodedPreviewFrame {
    pub(super) frame: PreviewRenderedFrame,
    pub(super) read_elapsed: Duration,
    pub(super) render_elapsed: Duration,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderImageIdentity {
    pub(super) image_id: ImageId,
    pub(super) content_version: u64,
}

impl RunningPreviewProcess {
    pub(super) fn render_single_frame(
        &self,
        seconds: f64,
        precise: bool,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        let started_at = Instant::now();
        let frame = decode_single_preview_frame(
            &self.executable,
            &self.config,
            seconds,
            precise,
            self.next_render_image_identity(),
            &self.metrics,
            &self.cancelled,
        )?;
        let dimensions = frame.frame.dimensions();
        let generation = self.prepare_playback_generation_at(seconds, started_at);
        self.metrics
            .record_video_frame_read(generation, frame.frame.byte_len, frame.read_elapsed);
        self.publish_frame_for_generation(frame.frame, frame.render_elapsed, generation);
        self.park_playback_generation(generation, seconds);
        self.prewarm_audio_for_generation(&self.config, seconds, precise, generation);
        Ok(dimensions)
    }
}

pub(super) fn decode_single_preview_frame(
    executable: &str,
    config: &PreviewSessionConfig,
    seconds: f64,
    precise: bool,
    image_identity: RenderImageIdentity,
    metrics: &PreviewRuntimeMetricsStore,
    cancelled: &AtomicBool,
) -> Result<DecodedPreviewFrame, PreviewEngineError> {
    let plan = snapshot_preview_plan(config, seconds, precise)?;
    let mut args = plan.args.clone();
    insert_frame_limit(&mut args, 1);
    let read_started = Instant::now();
    let payload = raw_snapshot(executable, &args, plan.frame_bytes, metrics, cancelled)?;
    let presentation = crate::preview_engine::PreviewRenderPresentation {
        pixel_aspect_ratio: plan.pixel_aspect_ratio,
        output_raster_aspect_ratio: plan.output_raster_aspect_ratio,
        ..Default::default()
    };
    let read_elapsed = read_started.elapsed();

    rendered_preview_frame_from_payload(
        plan.width,
        plan.height,
        plan.width.saturating_mul(4),
        seconds_to_us(seconds),
        payload,
        read_elapsed,
        image_identity,
    )
    .map(|mut decoded| {
        decoded.frame = decoded
            .frame
            .with_presentation(presentation, config.request_token);
        decoded
    })
}

fn raw_snapshot(
    executable: &str,
    args: &[String],
    frame_bytes: usize,
    metrics: &PreviewRuntimeMetricsStore,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, PreviewEngineError> {
    let mut command = CommandSpec::new(executable);
    command.args = args.iter().map(Into::into).collect();
    if let Some(operation) = crate::runtime_binaries::worker_subcommand(&command.program, "preview")
    {
        command.args.insert(0, operation.into());
    }
    #[cfg(windows)]
    {
        let (program, interpreter) =
            crate::bounded_process::resolve_windows_program(std::ffi::OsStr::new(executable))
                .map_err(|error| {
                    PreviewEngineError::Ffmpeg(format!("failed to render frame: {error}"))
                })?;
        command.program = program;
        command.interpreter = interpreter;
    }
    capture_snapshot(&command, frame_bytes, metrics, cancelled, STARTUP_TIMEOUT)
}

fn capture_snapshot(
    command: &CommandSpec,
    frame_bytes: usize,
    metrics: &PreviewRuntimeMetricsStore,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>, PreviewEngineError> {
    static SNAPSHOTS: ExecutionDomain = ExecutionDomain::new();
    if cancelled.load(Ordering::Acquire) {
        return Err(PreviewEngineError::Ffmpeg(
            "single frame render cancelled".into(),
        ));
    }
    let output = SNAPSHOTS
        .run_observed(
            command,
            &[],
            Limits {
                timeout,
                cleanup_timeout: Duration::from_secs(1),
                stdout_bytes: frame_bytes,
                stderr_bytes: 64 * 1024,
            },
            || cancelled.load(Ordering::Acquire),
            || metrics.record_video_process_spawn(),
        )
        .map_err(|error| {
            PreviewEngineError::Ffmpeg(format!(
                "single frame render stopped: {error}{}",
                stderr_detail(&String::from_utf8_lossy(&error.stderr.bytes))
            ))
        })?;
    if cancelled.load(Ordering::Acquire) {
        return Err(PreviewEngineError::Ffmpeg(
            "single frame render cancelled".into(),
        ));
    }
    if !output.success {
        return Err(PreviewEngineError::Ffmpeg(format!(
            "single frame render exited with code {:?}{}",
            output.code,
            stderr_detail(&String::from_utf8_lossy(&output.stderr.bytes))
        )));
    }
    if output.stdout.bytes.len() < frame_bytes {
        return Err(PreviewEngineError::Ffmpeg(format!(
            "single frame render returned {} bytes, expected {}{}",
            output.stdout.bytes.len(),
            frame_bytes,
            stderr_detail(&String::from_utf8_lossy(&output.stderr.bytes))
        )));
    }

    Ok(output.stdout.bytes)
}

pub(super) fn read_stream_frame(
    stdout: &mut dyn Read,
    spec: &FrameStreamSpec,
    timestamp_seconds: f64,
    image_identity: RenderImageIdentity,
) -> Result<DecodedPreviewFrame, PreviewEngineError> {
    let read_started = Instant::now();
    let mut payload = vec![0; spec.frame_bytes];
    stdout.read_exact(&mut payload).map_err(|error| {
        PreviewEngineError::Ffmpeg(format!("failed to read first preview frame: {error}"))
    })?;
    let read_elapsed = read_started.elapsed();
    rendered_preview_frame_from_payload(
        spec.width,
        spec.height,
        spec.width.saturating_mul(4),
        seconds_to_us(timestamp_seconds),
        payload,
        read_elapsed,
        image_identity,
    )
    .map(|mut decoded| {
        decoded.frame = decoded
            .frame
            .with_presentation(spec.presentation, spec.request_token);
        decoded
    })
}

pub(super) fn rendered_preview_frame_from_payload(
    width: u32,
    height: u32,
    stride: u32,
    timestamp_us: u64,
    payload: Vec<u8>,
    read_elapsed: Duration,
    image_identity: RenderImageIdentity,
) -> Result<DecodedPreviewFrame, PreviewEngineError> {
    let render_started = Instant::now();
    let frame = rendered_frame_from_bgra_payload_with_image_id(
        width,
        height,
        stride,
        timestamp_us,
        Some((image_identity.image_id, image_identity.content_version)),
        payload,
    )?;
    let render_elapsed = render_started.elapsed();
    Ok(DecodedPreviewFrame {
        frame,
        read_elapsed,
        render_elapsed,
    })
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use std::io::Write;

    fn fixture(mode: &str) -> CommandSpec {
        let mut command = CommandSpec::new(std::env::current_exe().unwrap());
        command.args = [
            "--exact",
            "preview_engine::ffmpeg_backend::decode::snapshot_tests::fake_snapshot",
            "--ignored",
            "--nocapture",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        command
            .env
            .push(("FRAME_SNAPSHOT_MODE".into(), mode.into()));
        command
    }

    #[test]
    #[ignore = "native snapshot fixture"]
    fn fake_snapshot() {
        let Ok(mode) = std::env::var("FRAME_SNAPSHOT_MODE") else {
            return;
        };
        if mode != "partial" {
            std::io::stdout().write_all(&vec![37; 64 * 1024]).unwrap();
            std::io::stdout().flush().unwrap();
            std::io::stderr()
                .write_all(&vec![b'x'; 128 * 1024])
                .unwrap();
        }
        if mode == "hang-after-frame" {
            std::thread::sleep(Duration::from_secs(2));
        }
        std::process::exit(0);
    }

    #[test]
    fn snapshot_retains_a_whole_frame_larger_than_the_probe_default() {
        let _fixture = crate::preview_engine::ffmpeg_backend::test_support::process_fixture();
        let payload = capture_snapshot(
            &fixture("complete"),
            64 * 1024,
            &PreviewRuntimeMetricsStore::new(),
            &AtomicBool::new(false),
            Duration::from_secs(3),
        )
        .unwrap();
        assert_eq!(payload.len(), 64 * 1024);
    }

    #[test]
    fn snapshot_requires_complete_payload_and_successful_process_exit() {
        let _fixture = crate::preview_engine::ffmpeg_backend::test_support::process_fixture();
        let metrics = PreviewRuntimeMetricsStore::new();
        let partial = capture_snapshot(
            &fixture("partial"),
            4096,
            &metrics,
            &AtomicBool::new(false),
            Duration::from_secs(3),
        )
        .unwrap_err();
        assert!(partial.to_string().contains("expected 4096"), "{partial}");
        let started = Instant::now();
        let incomplete = capture_snapshot(
            &fixture("hang-after-frame"),
            4096,
            &metrics,
            &AtomicBool::new(false),
            Duration::from_millis(200),
        )
        .unwrap_err();
        assert!(incomplete.to_string().contains("TimedOut"), "{incomplete}");
        assert!(started.elapsed() < Duration::from_secs(1));
        capture_snapshot(
            &fixture("complete"),
            4096,
            &metrics,
            &AtomicBool::new(false),
            Duration::from_secs(3),
        )
        .unwrap();
    }
}

#[cfg(test)]
mod spawn_metric_tests {
    use super::*;

    #[test]
    fn missing_snapshot_executable_does_not_increment_spawn_metrics() {
        let _fixture = crate::preview_engine::ffmpeg_backend::test_support::process_fixture();
        let metrics = PreviewRuntimeMetricsStore::new();
        let result = capture_snapshot(
            &CommandSpec::new("/missing/frame-snapshot-ffmpeg"),
            4,
            &metrics,
            &AtomicBool::new(false),
            Duration::from_secs(1),
        );
        assert!(result.is_err());
        assert_eq!(
            metrics.snapshot().video_process_spawns,
            0,
            "a failed launch must not count as a spawned FFmpeg process"
        );
    }
}
