use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_drain::AudioDrainOutcome;
use crate::preview_engine::ffmpeg_backend::audio_output::{
    AudioOutputSpec, AudioPreviewOutput, lock_audio_buffer, push_audio_sample_bytes,
};
use crate::preview_engine::ffmpeg_backend::clock::{
    AudioPlaybackStart, PlaybackStreamState, PreviewPlaybackClock, clock_generation_matches,
    wait_for_playback_start,
};
use crate::preview_engine::ffmpeg_backend::plan::preview_audio_plan;
use crate::preview_engine::ffmpeg_backend::process::{
    ProcessHandles, RunningPreviewProcess, RunningProcessState, lock_process, push_stderr_line,
    spawn_stderr_worker, stop_process_handles,
};
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use crate::preview_engine::types::PreviewSessionConfig;
use std::collections::VecDeque;
use std::io::{ErrorKind, Read};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub(super) const AUDIO_READ_BUFFER_BYTES: usize = 8192;

pub(super) const AUDIO_START_READY_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) const AUDIO_PREBUFFER_MS: u32 = 30;

pub(super) struct AudioStdoutWorkerConfig {
    pub(super) stderr_lines: Arc<Mutex<VecDeque<String>>>,
    pub(super) playback: Arc<Mutex<PreviewPlaybackClock>>,
    pub(super) metrics: PreviewRuntimeMetricsStore,
    pub(super) stop_requested: Arc<AtomicBool>,
    pub(super) audio_start: Arc<AudioPlaybackStart>,
    pub(super) mark_ended_on_eof: bool,
    pub(super) generation: u64,
    pub(super) prebuffer_samples: usize,
    pub(super) ready_tx: mpsc::Sender<AudioStartStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AudioStartStatus {
    Ready,
    OutputDisabled,
    TimedOut,
}

pub(super) enum PendingAudioStart {
    Waiting {
        generation: u64,
        ready_rx: mpsc::Receiver<AudioStartStatus>,
    },
    OutputDisabled,
}

impl RunningPreviewProcess {
    pub(super) fn start_audio_for_video_generation(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        generation: u64,
    ) -> Option<PendingAudioStart> {
        if config.selected_audio_track.is_some() {
            return match self
                .spawn_audio_streaming_with_config(config, seconds, precise, false, generation)
            {
                Ok(pending) => Some(pending),
                Err(error) => {
                    push_stderr_line(&self.stderr, format!("audio preview disabled: {error}"));
                    None
                }
            };
        }
        if let Err(error) = self.stop_audio_process() {
            push_stderr_line(
                &self.stderr,
                format!("audio preview cleanup failed: {error}"),
            );
        }
        None
    }

    pub(super) fn start_audio_streaming(
        &self,
        seconds: f64,
        precise: bool,
        mark_ended_on_eof: bool,
        generation: u64,
    ) -> Result<AudioStartStatus, PreviewEngineError> {
        self.start_audio_streaming_with_config(
            &self.config,
            seconds,
            precise,
            mark_ended_on_eof,
            generation,
        )
    }

    pub(super) fn start_audio_streaming_with_config(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        mark_ended_on_eof: bool,
        generation: u64,
    ) -> Result<AudioStartStatus, PreviewEngineError> {
        let pending = self.spawn_audio_streaming_with_config(
            config,
            seconds,
            precise,
            mark_ended_on_eof,
            generation,
        )?;
        Ok(self.finish_audio_start(pending))
    }

    pub(super) fn spawn_audio_streaming_with_config(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        mark_ended_on_eof: bool,
        generation: u64,
    ) -> Result<PendingAudioStart, PreviewEngineError> {
        if config.selected_audio_track.is_none() {
            return Ok(PendingAudioStart::OutputDisabled);
        }

        self.stop_audio_process()?;
        let output_spec = AudioOutputSpec::default_output()?;
        let plan = preview_audio_plan(config, seconds, true, precise, output_spec)?;
        let mut child = Command::new(&self.executable)
            .args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                PreviewEngineError::Ffmpeg(format!("failed to start audio preview: {err}"))
            })?;
        self.metrics.record_audio_process_spawn();

        let stdout = child.stdout.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg audio stdout was not captured".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg audio stderr was not captured".to_string())
        })?;
        let stop_requested = Arc::new(AtomicBool::new(false));
        let audio_start = Arc::new(AudioPlaybackStart::new(
            Arc::clone(&self.playback),
            Arc::clone(&self.playback_changed),
        ));
        let (ready_tx, ready_rx) = mpsc::channel();
        let stdout_worker = spawn_audio_stdout_worker(
            stdout,
            AudioStdoutWorkerConfig {
                stderr_lines: Arc::clone(&self.stderr),
                playback: Arc::clone(&self.playback),
                metrics: self.metrics.clone(),
                stop_requested: Arc::clone(&stop_requested),
                audio_start: Arc::clone(&audio_start),
                mark_ended_on_eof,
                generation,
                prebuffer_samples: audio_prebuffer_samples(output_spec),
                ready_tx,
            },
        )?;
        let stderr_worker = match spawn_stderr_worker(
            stderr,
            Arc::clone(&self.stderr),
            Arc::clone(&stop_requested),
        ) {
            Ok(worker) => worker,
            Err(error) => {
                let _ = stop_process_handles(ProcessHandles {
                    child: Some(child),
                    stdout_worker: Some(stdout_worker),
                    stderr_worker: None,
                    stop_requested: Some(stop_requested),
                    audio_start: Some(audio_start),
                });
                return Err(error);
            }
        };

        let mut state = lock_process(&self.audio_process);
        *state = RunningProcessState {
            child: Some(child),
            stdout_worker: Some(stdout_worker),
            stderr_worker: Some(stderr_worker),
            stop_requested: Some(stop_requested),
            audio_start: Some(audio_start),
        };
        drop(state);
        Ok(PendingAudioStart::Waiting {
            generation,
            ready_rx,
        })
    }

    pub(super) fn finish_audio_start(&self, pending: PendingAudioStart) -> AudioStartStatus {
        let PendingAudioStart::Waiting { ready_rx, .. } = pending else {
            return AudioStartStatus::OutputDisabled;
        };
        let status = wait_for_audio_start(&ready_rx);
        if status != AudioStartStatus::TimedOut {
            return status;
        }
        push_stderr_line(
            &self.stderr,
            "audio preview start readiness timed out".to_string(),
        );
        let _ = self.stop_audio_process();
        status
    }

    pub(super) fn prewarm_audio_for_generation(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        generation: u64,
    ) {
        if config.selected_audio_track.is_none() || !self.has_visual_stream() {
            return;
        }
        if self.pending_audio_start_generation_matches(generation) {
            return;
        }
        match self.spawn_audio_streaming_with_config(config, seconds, precise, false, generation) {
            Ok(pending_audio @ PendingAudioStart::Waiting { .. }) => {
                *lock_pending_audio_start(&self.pending_audio_start) = Some(pending_audio);
            }
            Ok(PendingAudioStart::OutputDisabled) => {}
            Err(error) => {
                push_stderr_line(
                    &self.stderr,
                    format!("audio preview prewarm failed: {error}"),
                );
            }
        }
    }

    pub(super) fn pending_audio_start_generation_matches(&self, generation: u64) -> bool {
        lock_pending_audio_start(&self.pending_audio_start)
            .as_ref()
            .and_then(PendingAudioStart::generation)
            == Some(generation)
    }

    pub(super) fn take_pending_audio_start_for_generation(
        &self,
        generation: u64,
    ) -> Option<PendingAudioStart> {
        let mut pending_audio = lock_pending_audio_start(&self.pending_audio_start);
        if pending_audio
            .as_ref()
            .and_then(PendingAudioStart::generation)
            == Some(generation)
        {
            return pending_audio.take();
        }
        None
    }
}

pub(super) fn audio_prebuffer_samples(output: AudioOutputSpec) -> usize {
    let samples_per_second = usize::try_from(output.sample_rate)
        .unwrap_or(48_000)
        .saturating_mul(usize::from(output.channels));
    samples_per_second.saturating_mul(usize::try_from(AUDIO_PREBUFFER_MS).unwrap_or(30)) / 1_000
}

pub(super) fn wait_for_audio_start(
    receiver: &mpsc::Receiver<AudioStartStatus>,
) -> AudioStartStatus {
    receiver
        .recv_timeout(AUDIO_START_READY_TIMEOUT)
        .unwrap_or(AudioStartStatus::TimedOut)
}

#[expect(
    clippy::too_many_lines,
    reason = "PCM reading coordinates readiness, cancellable playback start, and output drain in one worker."
)]
pub(super) fn spawn_audio_stdout_worker(
    mut stdout: impl Read + Send + 'static,
    config: AudioStdoutWorkerConfig,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-audio-stdout".to_string())
        .spawn(move || {
            let output = match AudioPreviewOutput::new(
                &config.metrics,
                &config.playback,
                &config.audio_start,
                &config.stderr_lines,
                config.mark_ended_on_eof,
                config.generation,
            ) {
                Ok(output) => Some(output),
                Err(error) => {
                    push_stderr_line(
                        &config.stderr_lines,
                        format!("audio preview output disabled: {error}"),
                    );
                    let _ = config.ready_tx.send(AudioStartStatus::OutputDisabled);
                    None
                }
            };
            let mut read_buffer = [0_u8; AUDIO_READ_BUFFER_BYTES];
            let mut pending = Vec::new();
            let mut queued_for_prebuffer = 0_usize;
            let mut ready_sent = output.is_none();
            let mut output_started = output.is_none();
            let mut pipe_exhausted = false;

            loop {
                if config.stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                if !clock_generation_matches(&config.playback, config.generation) {
                    break;
                }
                if output
                    .as_ref()
                    .is_some_and(|output| lock_audio_buffer(&output.context.buffer).drain.failed())
                {
                    break;
                }

                match stdout.read(&mut read_buffer) {
                    Ok(0) => {
                        pipe_exhausted = true;
                        break;
                    }
                    Ok(bytes_read) => {
                        pending.extend_from_slice(&read_buffer[..bytes_read]);
                        let frame_bytes = output.as_ref().map_or(4, |output| output.frame_bytes);
                        let complete_len = pending.len() - (pending.len() % frame_bytes);
                        if complete_len == 0 {
                            continue;
                        }

                        if let Some(output) = &output {
                            let samples = push_audio_sample_bytes(
                                &output.context.buffer,
                                &pending[..complete_len],
                            );
                            config.metrics.record_audio_pcm_chunk(
                                config.generation,
                                complete_len,
                                samples,
                            );
                            queued_for_prebuffer = queued_for_prebuffer.saturating_add(samples);
                            if !ready_sent && queued_for_prebuffer >= config.prebuffer_samples {
                                let _ = config.ready_tx.send(AudioStartStatus::Ready);
                                ready_sent = true;
                            }
                            if ready_sent && !output_started {
                                match wait_for_playback_start(
                                    &config.audio_start,
                                    &config.stop_requested,
                                    config.generation,
                                ) {
                                    PlaybackStreamState::Ready => match output.play() {
                                        Ok(()) => {}
                                        Err(_) => break,
                                    },
                                    PlaybackStreamState::Stale => break,
                                    PlaybackStreamState::Waiting => {}
                                }
                                output_started = true;
                            }
                        } else {
                            config.metrics.record_audio_pcm_chunk(
                                config.generation,
                                complete_len,
                                complete_len / 4,
                            );
                        }
                        pending.drain(..complete_len);
                    }
                    // Retry interrupted reads without consuming or discarding PCM.
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => {
                        let message = format!("audio preview read failed: {error}");
                        if let Some(output) = &output {
                            output.context.fail(message);
                        } else {
                            push_stderr_line(&config.stderr_lines, message);
                            config.audio_start.mark_output_failed(
                                config.mark_ended_on_eof.then_some(config.generation),
                            );
                        }
                        break;
                    }
                }
            }

            if !ready_sent {
                let _ = config.ready_tx.send(AudioStartStatus::TimedOut);
            }
            let drained = pipe_exhausted
                && output_started
                && output.as_ref().is_none_or(|output| {
                    output
                        .context
                        .drain_after_pipe_exhausted(&config.stop_requested)
                        == AudioDrainOutcome::Drained
                });
            if config.mark_ended_on_eof && drained && !config.stop_requested.load(Ordering::SeqCst)
            {
                config.audio_start.mark_ended(config.generation);
            }
        })
        .map_err(|err| {
            PreviewEngineError::Ffmpeg(format!("failed to spawn audio stdout worker: {err}"))
        })
}

pub(super) fn lock_pending_audio_start(
    pending_audio: &Mutex<Option<PendingAudioStart>>,
) -> MutexGuard<'_, Option<PendingAudioStart>> {
    pending_audio
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl PendingAudioStart {
    pub(super) const fn generation(&self) -> Option<u64> {
        match self {
            Self::Waiting { generation, .. } => Some(*generation),
            Self::OutputDisabled => None,
        }
    }
}
