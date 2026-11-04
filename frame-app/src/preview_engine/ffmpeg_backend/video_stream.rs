use crate::numeric::u64_to_f64;
use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_stream::{AudioStartStatus, PendingAudioStart};
use crate::preview_engine::ffmpeg_backend::clock::{
    PlaybackStreamState, PreviewPlaybackClock, lock_playback, seconds_to_us,
    update_last_frame_seconds, wait_for_frame_presentation_time,
};
use crate::preview_engine::ffmpeg_backend::decode::{
    FrameStreamSpec, RenderImageIdentity, read_stream_frame, rendered_preview_frame_from_payload,
};
use crate::preview_engine::ffmpeg_backend::plan::preview_plan;
use crate::preview_engine::ffmpeg_backend::process::{
    ProcessHandles, RunningPreviewProcess, RunningProcessState, lock_process, push_stderr_line,
    spawn_stderr_worker, stop_process_handles,
};
use crate::preview_engine::frame_store::LatestFrameStore;
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use crate::preview_engine::types::{PreviewDimensions, PreviewSessionConfig};
use gpui::ImageId;
use std::io::{ErrorKind, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

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
            if status == AudioStartStatus::TimedOut {
                return Err(PreviewEngineError::Audio(
                    "audio preview start readiness timed out".to_string(),
                ));
            }
            self.start_playback_generation(generation, seconds);
            return Ok(());
        }

        self.start_video_streaming_with_config(&self.config, seconds, precise)?;
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Video startup coordinates FFmpeg spawn, audio prewarm, first-frame readiness, and worker handoff in one transaction."
    )]
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

        let mut stream = match self.spawn_video_stream(config, seconds, precise) {
            Ok(stream) => stream,
            Err(error) => {
                let _ = self.stop_audio_process();
                return Err(error);
            }
        };

        let first_frame = match read_stream_frame(
            &mut stream.stdout,
            &stream.spec,
            seconds,
            self.next_render_image_identity(),
        ) {
            Ok(frame) => frame,
            Err(error) => {
                let _ = stop_spawned_video_stream(stream);
                let _ = self.stop_audio_process();
                return Err(error);
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
                AudioStartStatus::TimedOut => {
                    push_stderr_line(
                        &self.stderr,
                        "audio preview disabled: start readiness timed out".to_string(),
                    );
                }
            }
        }

        let stop_result = self.stop_video_process();
        if let Err(error) = stop_result {
            let _ = stop_spawned_video_stream(stream);
            let _ = self.stop_audio_process();
            return Err(error);
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
                };
                let _ = stop_process_handles(handles);
                let _ = self.stop_audio_process();
                return Err(error);
            }
        };

        let mut state = lock_process(&self.process);
        *state = RunningProcessState {
            child: Some(stream.child),
            stdout_worker: Some(stdout_worker),
            stderr_worker: Some(stream.stderr_worker),
            stop_requested: Some(stream.stop_requested),
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
    ) -> Result<SpawnedVideoStream, PreviewEngineError> {
        let plan = preview_plan(config, seconds, true, precise)?;
        self.metrics.record_video_process_spawn();
        let spawned_at = Instant::now();
        let mut child = Command::new(&self.executable)
            .args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to start: {err}")))?;

        let stdout = child.stdout.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg stdout was not captured".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg stderr was not captured".to_string())
        })?;
        let stop_requested = Arc::new(AtomicBool::new(false));
        let stderr_worker = match spawn_stderr_worker(
            stderr,
            Arc::clone(&self.stderr),
            Arc::clone(&stop_requested),
        ) {
            Ok(worker) => worker,
            Err(error) => {
                stop_requested.store(true, Ordering::SeqCst);
                if child.try_wait().map_err(PreviewEngineError::Io)?.is_none() {
                    child.kill().map_err(PreviewEngineError::Io)?;
                }
                let _ = child.wait();
                return Err(error);
            }
        };

        Ok(SpawnedVideoStream {
            child,
            stdout: Box::new(stdout),
            stderr_worker,
            stop_requested,
            spec: FrameStreamSpec {
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

pub(super) fn stop_spawned_video_stream(
    stream: SpawnedVideoStream,
) -> Result<(), PreviewEngineError> {
    stop_process_handles(ProcessHandles {
        child: Some(stream.child),
        stdout_worker: None,
        stderr_worker: Some(stream.stderr_worker),
        stop_requested: Some(stream.stop_requested),
    })
}

pub(super) fn spawn_stdout_worker(
    mut stdout: impl Read + Send + 'static,
    config: VideoStdoutWorkerConfig,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-stdout".to_string())
        .spawn(move || {
            let VideoStdoutWorkerConfig {
                frame_store,
                playback,
                metrics,
                stop_requested,
                render_image_id,
                render_image_version,
                spec,
            } = config;
            let mut frame_index = spec.first_frame_index;
            loop {
                if stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                let timestamp_seconds =
                    spec.base_seconds + (u64_to_f64(frame_index) / f64::from(spec.fps.max(1)));
                match wait_for_frame_presentation_time(
                    &playback,
                    &stop_requested,
                    spec.generation,
                    timestamp_seconds,
                ) {
                    PlaybackStreamState::Ready => {}
                    PlaybackStreamState::Stale => break,
                    PlaybackStreamState::Waiting => continue,
                }
                let mut payload = vec![0_u8; spec.frame_bytes];
                let read_started = Instant::now();
                match stdout.read_exact(&mut payload) {
                    Ok(()) => {
                        let read_elapsed = read_started.elapsed();
                        metrics.record_video_frame_read(
                            spec.generation,
                            spec.frame_bytes,
                            read_elapsed,
                        );
                        if frame_store.has_unpresented_frame() {
                            if update_last_frame_seconds(
                                &playback,
                                spec.generation,
                                timestamp_seconds,
                            ) {
                                metrics.record_video_frame_dropped(spec.generation);
                            }
                            frame_index = frame_index.saturating_add(1);
                            continue;
                        }

                        let Ok(frame) = rendered_preview_frame_from_payload(
                            spec.width,
                            spec.height,
                            spec.width.saturating_mul(4),
                            seconds_to_us(timestamp_seconds),
                            payload,
                            read_elapsed,
                            RenderImageIdentity {
                                image_id: render_image_id,
                                content_version: render_image_version
                                    .fetch_add(1, Ordering::Relaxed),
                            },
                        ) else {
                            frame_index = frame_index.saturating_add(1);
                            continue;
                        };
                        if update_last_frame_seconds(&playback, spec.generation, timestamp_seconds)
                        {
                            metrics.record_render_image_converted(
                                spec.generation,
                                frame.frame.byte_len,
                                frame.render_elapsed,
                            );
                            let _ = frame_store.publish(frame.frame);
                            metrics.record_video_frame_published(spec.generation);
                        }
                        frame_index = frame_index.saturating_add(1);
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe
                        ) =>
                    {
                        break;
                    }
                    Err(_) => break,
                }
            }

            if !stop_requested.load(Ordering::SeqCst) {
                let mut playback = lock_playback(&playback);
                if playback.generation_matches(spec.generation) {
                    playback.playing = false;
                    playback.ended = true;
                }
            }
        })
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to spawn stdout worker: {err}")))
}
