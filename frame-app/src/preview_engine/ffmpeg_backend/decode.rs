use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::clock::seconds_to_us;
use crate::preview_engine::ffmpeg_backend::plan::{insert_frame_limit, preview_plan};
use crate::preview_engine::ffmpeg_backend::process::{RunningPreviewProcess, stderr_detail};
use crate::preview_engine::renderer::rendered_frame_from_bgra_payload_with_image_id;
use crate::preview_engine::types::{PreviewDimensions, PreviewRenderedFrame, PreviewSessionConfig};
use gpui::ImageId;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(super) struct FrameStreamSpec {
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
) -> Result<DecodedPreviewFrame, PreviewEngineError> {
    let plan = preview_plan(config, seconds, false, precise)?;
    let mut args = plan.args.clone();
    insert_frame_limit(&mut args, 1);
    let read_started = Instant::now();
    let output = Command::new(executable)
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to render frame: {err}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PreviewEngineError::Ffmpeg(format!(
            "single frame render exited with status {}{}",
            output.status,
            stderr_detail(&stderr)
        )));
    }
    if output.stdout.len() < plan.frame_bytes {
        return Err(PreviewEngineError::Ffmpeg(format!(
            "single frame render returned {} bytes, expected {}{}",
            output.stdout.len(),
            plan.frame_bytes,
            stderr_detail(&String::from_utf8_lossy(&output.stderr))
        )));
    }

    let mut payload = output.stdout;
    payload.truncate(plan.frame_bytes);
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
}

pub(super) fn read_stream_frame(
    stdout: &mut dyn Read,
    spec: &FrameStreamSpec,
    timestamp_seconds: f64,
    image_identity: RenderImageIdentity,
) -> Result<DecodedPreviewFrame, PreviewEngineError> {
    let mut payload = vec![0_u8; spec.frame_bytes];
    let read_started = Instant::now();
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
