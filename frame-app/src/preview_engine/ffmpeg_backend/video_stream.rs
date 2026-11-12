use crate::bounded_process::{ExecutionDomain, ProcessReservation};
use crate::numeric::u64_to_f64;
use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_stream::{AudioStartStatus, PendingAudioStart};
use crate::preview_engine::ffmpeg_backend::clock::{
    PlaybackStreamState, PreviewPlaybackClock, mark_playback_ended, seconds_to_us,
    update_last_frame_seconds, wait_for_frame_presentation_time,
};
use crate::preview_engine::ffmpeg_backend::decode::{
    DecodedPreviewFrame, FrameStreamSpec, RenderImageIdentity, read_stream_frame,
    rendered_preview_frame_from_payload,
};
use crate::preview_engine::ffmpeg_backend::plan::preview_plan;
use crate::preview_engine::ffmpeg_backend::process::{
    ProcessHandles, RunningPreviewProcess, RunningProcessState, lock_process, push_stderr_line,
    spawn_stderr_worker,
};
use crate::preview_engine::ffmpeg_backend::startup_pipe::{STARTUP_TIMEOUT, StartupPipe};
use crate::preview_engine::frame_store::LatestFrameStore;
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use crate::preview_engine::types::{PreviewDimensions, PreviewSessionConfig};
use gpui::ImageId;
use std::io::Read;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(super) struct SpawnedVideoStream {
    pub(super) child: Child,
    pub(super) stdout: ChildStdout,
    pub(super) stderr_worker: JoinHandle<()>,
    pub(super) stop_requested: Arc<AtomicBool>,
    pub(super) spec: FrameStreamSpec,
    reservation: ProcessReservation,
    deadline: Instant,
}

pub(super) struct VideoStdoutWorkerConfig {
    pub(super) frame_store: LatestFrameStore,
    pub(super) playback: Arc<Mutex<PreviewPlaybackClock>>,
    pub(super) playback_changed: Arc<Condvar>,
    pub(super) metrics: PreviewRuntimeMetricsStore,
    pub(super) stop_requested: Arc<AtomicBool>,
    pub(super) render_image_id: ImageId,
    pub(super) render_image_version: Arc<AtomicU64>,
    pub(super) spec: FrameStreamSpec,
}

impl RunningPreviewProcess {
    pub(super) fn start_streaming(
        &self,
        seconds: f64,
        precise: bool,
    ) -> Result<(), PreviewEngineError> {
        if !self.has_visual_stream() {
            self.stop_running_processes()?;
            let generation = self.prepare_playback_generation(seconds);
            let status = self.start_audio_streaming(seconds, precise, true, generation)?;
            if matches!(
                status,
                AudioStartStatus::TimedOut | AudioStartStatus::Failed
            ) {
                return Err(PreviewEngineError::Audio(format!(
                    "audio preview start failed: {status:?}"
                )));
            }
            self.start_playback_generation(generation, seconds);
            return Ok(());
        }

        self.start_video_streaming_with_config(&self.config, seconds, precise)?;
        Ok(())
    }

    pub(super) fn start_video_streaming_with_config(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        static STARTUPS: ExecutionDomain = ExecutionDomain::new();
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let reservation = STARTUPS
            .reserve(deadline, || self.cancelled.load(Ordering::Acquire))
            .map_err(|error| {
                PreviewEngineError::Ffmpeg(format!("preview startup stopped: {error}"))
            })?;
        let (generation, prewarmed_audio) = self.prepare_video_start_generation(seconds);
        let pending_audio = prewarmed_audio.map_or_else(
            || self.start_audio_for_video_generation(config, seconds, precise, generation),
            Some,
        );

        let mut stream =
            self.spawn_video_stream(config, seconds, precise, generation, reservation, deadline)?;

        let first_frame = match read_stream_frame(
            &mut StartupPipe::new(&mut stream.stdout, &self.cancelled, stream.deadline),
            &stream.spec,
            seconds,
            self.next_render_image_identity(),
        ) {
            Ok(frame) => frame,
            Err(error) => {
                let (handles, reservation) = stream.into_handles();
                return Err(self.fail_video_start_bounded(
                    generation,
                    error,
                    Some(handles),
                    reservation,
                ));
            }
        };
        let first_frame_read_elapsed = stream
            .spec
            .spawned_at
            .elapsed()
            .saturating_sub(first_frame.render_elapsed);
        let first_frame_read_ms =
            u64::try_from(first_frame_read_elapsed.as_millis()).unwrap_or(u64::MAX);

        if let Some(pending_audio) = pending_audio {
            match self.finish_audio_start(pending_audio) {
                AudioStartStatus::Ready | AudioStartStatus::OutputDisabled => {}
                AudioStartStatus::TimedOut | AudioStartStatus::Failed => {
                    push_stderr_line(
                        &self.stderr,
                        "audio preview disabled: start failed".to_string(),
                    );
                }
            }
        }

        if self.cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
            let (handles, reservation) = stream.into_handles();
            return Err(self.fail_video_start_bounded(
                generation,
                PreviewEngineError::Ffmpeg("preview startup cancelled or timed out".into()),
                Some(handles),
                reservation,
            ));
        }

        self.finish_video_start(
            stream,
            first_frame,
            seconds,
            generation,
            first_frame_read_ms,
        )
    }

    fn finish_video_start(
        &self,
        stream: SpawnedVideoStream,
        first_frame: DecodedPreviewFrame,
        seconds: f64,
        generation: u64,
        first_frame_read_ms: u64,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        let stop_result = self.stop_video_process();
        if let Err(error) = stop_result {
            let (handles, reservation) = stream.into_handles();
            return Err(self.fail_video_start_bounded(
                generation,
                error,
                Some(handles),
                reservation,
            ));
        }

        let dimensions = PreviewDimensions {
            width: stream.spec.width,
            height: stream.spec.height,
        };
        let mut stdout_spec = stream.spec;
        stdout_spec.first_frame_index = 1;
        stdout_spec.generation = generation;
        let stdout_worker = match spawn_stdout_worker(
            stream.stdout,
            VideoStdoutWorkerConfig {
                frame_store: self.frame_store.clone(),
                playback: Arc::clone(&self.playback),
                playback_changed: Arc::clone(&self.playback_changed),
                metrics: self.metrics.clone(),
                stop_requested: Arc::clone(&stream.stop_requested),
                render_image_id: self.render_image_id,
                render_image_version: Arc::clone(&self.render_image_version),
                spec: stdout_spec,
            },
        ) {
            Ok(worker) => worker,
            Err(error) => {
                let handles = ProcessHandles {
                    child: Some(stream.child),
                    stdout_worker: None,
                    stderr_worker: Some(stream.stderr_worker),
                    stop_requested: Some(stream.stop_requested),
                    audio_start: None,
                    audio_exit: None,
                };
                return Err(self.fail_video_start_bounded(
                    generation,
                    error,
                    Some(handles),
                    stream.reservation,
                ));
            }
        };

        let mut state = lock_process(&self.process);
        *state = RunningProcessState {
            child: Some(stream.child),
            stdout_worker: Some(stdout_worker),
            stderr_worker: Some(stream.stderr_worker),
            stop_requested: Some(stream.stop_requested),
            audio_start: None,
            audio_exit: None,
        };
        drop(state);
        drop(stream.reservation);

        self.metrics
            .record_video_first_frame_read_duration(generation, first_frame_read_ms);
        self.metrics.record_video_frame_read(
            generation,
            first_frame.frame.byte_len,
            first_frame.read_elapsed,
        );
        self.publish_frame_for_generation(
            first_frame.frame,
            first_frame.render_elapsed,
            generation,
        );

        self.start_playback_generation(generation, seconds);
        Ok(dimensions)
    }

    pub(super) fn prepare_video_start_generation(
        &self,
        seconds: f64,
    ) -> (u64, Option<PendingAudioStart>) {
        let prewarmed_audio = self
            .parked_generation_at(seconds)
            .and_then(|generation| self.take_pending_audio_start_for_generation(generation));
        prewarmed_audio.map_or_else(
            || (self.prepare_playback_generation(seconds), None),
            |pending_audio| {
                let generation = pending_audio
                    .generation()
                    .unwrap_or_else(|| self.prepare_playback_generation(seconds));
                self.metrics.begin_generation(generation);
                (generation, Some(pending_audio))
            },
        )
    }

    pub(super) fn spawn_video_stream(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        generation: u64,
        reservation: ProcessReservation,
        deadline: Instant,
    ) -> Result<SpawnedVideoStream, PreviewEngineError> {
        let plan = match preview_plan(config, seconds, true, precise) {
            Ok(plan) => plan,
            Err(error) => {
                return Err(self.fail_video_start_bounded(generation, error, None, reservation));
            }
        };
        let spawned_at = Instant::now();
        let child = super::startup_process::spawn(
            Command::new(&self.executable)
                .args(&plan.args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
            &self.cancelled,
            deadline,
        );
        let mut child = match child {
            Ok(child) => child,
            Err(error) => {
                return Err(self.fail_video_start_bounded(
                    generation,
                    PreviewEngineError::Ffmpeg(format!("failed to start: {error}")),
                    None,
                    reservation,
                ));
            }
        };
        self.metrics.record_video_process_spawn();

        let stop_requested = Arc::new(AtomicBool::new(false));
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
            return Err(self.fail_video_start_bounded(
                generation,
                PreviewEngineError::Ffmpeg("ffmpeg output pipes were not captured".to_string()),
                Some(ProcessHandles {
                    child: Some(child),
                    stdout_worker: None,
                    stderr_worker: None,
                    stop_requested: Some(stop_requested),
                    audio_start: None,
                    audio_exit: None,
                }),
                reservation,
            ));
        };
        let stderr_worker = match spawn_stderr_worker(
            stderr,
            Arc::clone(&self.stderr),
            Arc::clone(&stop_requested),
        ) {
            Ok(worker) => worker,
            Err(error) => {
                return Err(self.fail_video_start_bounded(
                    generation,
                    error,
                    Some(ProcessHandles {
                        child: Some(child),
                        stdout_worker: None,
                        stderr_worker: None,
                        stop_requested: Some(stop_requested),
                        audio_start: None,
                        audio_exit: None,
                    }),
                    reservation,
                ));
            }
        };

        Ok(SpawnedVideoStream {
            child,
            stdout,
            stderr_worker,
            stop_requested,
            reservation,
            deadline,
            spec: FrameStreamSpec {
                presentation: crate::preview_engine::PreviewRenderPresentation {
                    pixel_aspect_ratio: plan.pixel_aspect_ratio,
                    output_raster_aspect_ratio: plan.output_raster_aspect_ratio,
                    ..Default::default()
                },
                request_token: config.request_token,
                width: plan.width,
                height: plan.height,
                fps: plan.fps,
                frame_bytes: plan.frame_bytes,
                base_seconds: seconds,
                first_frame_index: 0,
                generation: 0,
                spawned_at,
            },
        })
    }
}

impl SpawnedVideoStream {
    fn into_handles(self) -> (ProcessHandles, ProcessReservation) {
        (
            ProcessHandles {
                child: Some(self.child),
                stdout_worker: None,
                stderr_worker: Some(self.stderr_worker),
                stop_requested: Some(self.stop_requested),
                audio_start: None,
                audio_exit: None,
            },
            self.reservation,
        )
    }
}

impl VideoStdoutWorkerConfig {
    fn publish_payload(&self, payload: Vec<u8>, timestamp_seconds: f64, read_elapsed: Duration) {
        let spec = self.spec;
        self.metrics
            .record_video_frame_read(spec.generation, spec.frame_bytes, read_elapsed);
        if self.frame_store.has_unpresented_frame() {
            if update_last_frame_seconds(&self.playback, spec.generation, timestamp_seconds) {
                self.metrics.record_video_frame_dropped(spec.generation);
            }
            return;
        }
        let Ok(frame) = rendered_preview_frame_from_payload(
            spec.width,
            spec.height,
            spec.width.saturating_mul(4),
            seconds_to_us(timestamp_seconds),
            payload,
            read_elapsed,
            RenderImageIdentity {
                image_id: self.render_image_id,
                content_version: self.render_image_version.fetch_add(1, Ordering::Relaxed),
            },
        ) else {
            return;
        };
        if update_last_frame_seconds(&self.playback, spec.generation, timestamp_seconds) {
            self.metrics.record_render_image_converted(
                spec.generation,
                frame.frame.byte_len,
                frame.render_elapsed,
            );
            let _ = self.frame_store.publish(
                frame
                    .frame
                    .with_presentation(spec.presentation, spec.request_token),
            );
            self.metrics.record_video_frame_published(spec.generation);
        }
    }
}

pub(super) fn spawn_stdout_worker(
    mut stdout: impl Read + Send + 'static,
    config: VideoStdoutWorkerConfig,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-stdout".to_string())
        .spawn(move || {
            let spec = config.spec;
            let mut frame_index = spec.first_frame_index;
            loop {
                if config.stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                let timestamp_seconds =
                    spec.base_seconds + (u64_to_f64(frame_index) / f64::from(spec.fps.max(1)));
                match wait_for_frame_presentation_time(
                    &config.playback,
                    &config.stop_requested,
                    spec.generation,
                    timestamp_seconds,
                ) {
                    PlaybackStreamState::Ready => {}
                    PlaybackStreamState::Stale => break,
                    PlaybackStreamState::Waiting => continue,
                }
                let read_started = Instant::now();
                let mut payload = vec![0; spec.frame_bytes];
                if stdout.read_exact(&mut payload).is_err() {
                    break;
                }
                config.publish_payload(payload, timestamp_seconds, read_started.elapsed());
                frame_index = frame_index.saturating_add(1);
            }
            if !config.stop_requested.load(Ordering::SeqCst) {
                mark_playback_ended(&config.playback, &config.playback_changed, spec.generation);
            }
        })
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to spawn stdout worker: {err}")))
}
