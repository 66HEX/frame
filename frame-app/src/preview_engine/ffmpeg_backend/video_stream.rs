use crate::numeric::u64_to_f64;
use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_stream::{AudioStartStatus, PendingAudioStart};
use crate::preview_engine::ffmpeg_backend::clock::{
    PlaybackStreamState, PreviewPlaybackClock, mark_playback_ended, seconds_to_us,
    update_last_frame_seconds, wait_for_frame_presentation_time,
};
use crate::preview_engine::ffmpeg_backend::decode::{
    FrameStreamSpec, RenderImageIdentity, rendered_preview_frame_from_payload,
};
use crate::preview_engine::ffmpeg_backend::plan::preview_plan;
use crate::preview_engine::ffmpeg_backend::process::{
    ProcessHandles, RunningPreviewProcess, RunningProcessState, lock_process, push_stderr_line,
    spawn_stderr_worker,
};
use crate::preview_engine::frame_store::LatestFrameStore;
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use crate::preview_engine::types::{PreviewDimensions, PreviewSessionConfig};
use gpui::ImageId;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(super) struct SpawnedVideoStream {
    pub(super) child: Child,
    pub(super) stdout: Box<dyn Read + Send>,
    pub(super) stderr_worker: JoinHandle<()>,
    pub(super) stop_requested: Arc<AtomicBool>,
    pub(super) spec: FrameStreamSpec,
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
    pub(super) framed_owner: Option<super::framed_stream::FramedStreamOwner>,
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
        let (generation, prewarmed_audio) = self.prepare_video_start_generation(seconds);
        let pending_audio = prewarmed_audio.map_or_else(
            || self.start_audio_for_video_generation(config, seconds, precise, generation),
            Some,
        );

        let mut stream = self.spawn_video_stream(config, seconds, precise, generation)?;

        let first_frame = match super::framed_stream::read_first_frame(
            &mut stream,
            seconds,
            self.next_render_image_identity(),
            &self.cancelled,
        ) {
            Ok(frame) => frame,
            Err(error) => {
                return Err(self.fail_video_start(generation, error, Some(stream.into_handles())));
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

        let stop_result = self.stop_video_process();
        if let Err(error) = stop_result {
            return Err(self.fail_video_start(generation, error, Some(stream.into_handles())));
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
                framed_owner: stdout_spec.framed.then(|| self.framed_stream_owner()),
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
                return Err(self.fail_video_start(generation, error, Some(handles)));
            }
        };

        let mut state = lock_process(&self.process);
        *state = RunningProcessState {
            framed_video: stdout_spec.framed,
            child: Some(stream.child),
            stdout_worker: Some(stdout_worker),
            stderr_worker: Some(stream.stderr_worker),
            stop_requested: Some(stream.stop_requested),
            audio_start: None,
            audio_exit: None,
        };
        drop(state);

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

    fn framed_stream_owner(&self) -> super::framed_stream::FramedStreamOwner {
        super::framed_stream::FramedStreamOwner {
            process: Arc::clone(&self.process),
            audio: Arc::clone(&self.audio_process),
            pending_audio: Arc::clone(&self.pending_audio_start),
            stderr: Arc::clone(&self.stderr),
        }
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
    ) -> Result<SpawnedVideoStream, PreviewEngineError> {
        let verified = super::framed::prepare(&self.executable, config, &self.cancelled)
            .map_err(|error| self.fail_video_start(generation, error, None))?;
        let plan = if verified.is_some() {
            super::plan::framed_preview_plan(config, seconds, precise, false)
        } else {
            preview_plan(config, seconds, true, precise)
        }
        .map_err(|error| self.fail_video_start(generation, error, None))?;
        if let Some(verified) = &verified {
            verified
                .recheck(&self.cancelled)
                .map_err(|error| self.fail_video_start(generation, error, None))?;
        }
        let spawned_at = Instant::now();
        let program = verified.as_ref().map_or_else(
            || std::path::Path::new(&self.executable),
            super::framed::VerifiedTransport::program,
        );
        let mut child = crate::process_launch::spawn(
            Command::new(program)
                .args(&plan.args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .map_err(|err| {
            self.fail_video_start(
                generation,
                PreviewEngineError::Ffmpeg(format!("failed to start: {err}")),
                None,
            )
        })?;
        self.metrics.record_video_process_spawn();

        let stop_requested = Arc::new(AtomicBool::new(false));
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
            return Err(self.fail_video_start(
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
            ));
        };
        let stderr_worker = match spawn_stderr_worker(
            stderr,
            Arc::clone(&self.stderr),
            Arc::clone(&stop_requested),
        ) {
            Ok(worker) => worker,
            Err(error) => {
                return Err(self.fail_video_start(
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
                ));
            }
        };

        Ok(SpawnedVideoStream {
            child,
            stdout: Box::new(stdout),
            stderr_worker,
            stop_requested,
            spec: FrameStreamSpec {
                framed: verified.is_some(),
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
    fn into_handles(self) -> ProcessHandles {
        ProcessHandles {
            child: Some(self.child),
            stdout_worker: None,
            stderr_worker: Some(self.stderr_worker),
            stop_requested: Some(self.stop_requested),
            audio_start: None,
            audio_exit: None,
        }
    }
}

impl VideoStdoutWorkerConfig {
    fn publish_payload(
        &self,
        payload: Vec<u8>,
        presentation: crate::preview_engine::PreviewRenderPresentation,
        timestamp_seconds: f64,
        read_elapsed: Duration,
    ) {
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
                    .with_presentation(presentation, spec.request_token),
            );
            self.metrics.record_video_frame_published(spec.generation);
        }
    }

    fn finish_framed(&self, error: Option<String>) {
        if let Some(owner) = &self.framed_owner {
            owner.finish(
                self.spec.generation,
                &self.playback,
                &self.playback_changed,
                &self.stop_requested,
                error,
            );
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
                match super::framed::read_payload(&mut stdout, &spec, frame_index) {
                    Ok(Some((payload, presentation))) => {
                        config.publish_payload(
                            payload,
                            presentation,
                            timestamp_seconds,
                            read_started.elapsed(),
                        );
                        frame_index = frame_index.saturating_add(1);
                    }
                    Ok(None) => {
                        config.finish_framed(None);
                        break;
                    }
                    Err(error) => {
                        if spec.framed {
                            config.finish_framed(Some(error.to_string()));
                        }
                        break;
                    }
                }
            }
            if !spec.framed && !config.stop_requested.load(Ordering::SeqCst) {
                mark_playback_ended(&config.playback, &config.playback_changed, spec.generation);
            }
        })
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to spawn stdout worker: {err}")))
}
