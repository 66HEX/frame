use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, ErrorKind, Read},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use frame_core::{
    preview::{
        PreviewAudioFfmpegOptions, PreviewFfmpegOptions, PreviewFfmpegPlan,
        build_ffmpeg_preview_args, build_ffmpeg_preview_audio_args,
    },
    utils::parse_time,
};
use gpui::{ImageId, RenderImage};

use crate::{
    numeric::{f64_to_u64, u64_to_f64},
    runtime_binaries::ffmpeg_executable,
};

use super::metrics::{PreviewRuntimeMetricsStore, log_preview_runtime_metrics};
use super::{
    LatestFrameStore, PreviewDimensions, PreviewEngineError, PreviewRenderedFrame,
    PreviewSessionConfig, PreviewSourceKind, rendered_frame_from_bgra_payload_with_image_id,
};

const STDERR_RING_LINES: usize = 24;
const SEEK_END_EPSILON: f64 = 0.001;
const AUDIO_BUFFER_SECONDS: usize = 3;
const AUDIO_READ_BUFFER_BYTES: usize = 8192;
const AUDIO_START_READY_TIMEOUT: Duration = Duration::from_secs(2);
const AUDIO_PREBUFFER_MS: u32 = 30;
const VIDEO_START_WAIT_INTERVAL: Duration = Duration::from_millis(2);
const VIDEO_FRAME_TIMING_EPSILON_SECONDS: f64 = 0.002;

pub struct RunningPreviewProcess {
    config: PreviewSessionConfig,
    frame_store: LatestFrameStore,
    metrics: PreviewRuntimeMetricsStore,
    executable: String,
    process: Mutex<RunningProcessState>,
    audio_process: Mutex<RunningProcessState>,
    pending_audio_start: Mutex<Option<PendingAudioStart>>,
    playback: Arc<Mutex<PreviewPlaybackClock>>,
    render_image_id: ImageId,
    render_image_version: Arc<AtomicU64>,
    stderr: Arc<Mutex<VecDeque<String>>>,
}

#[derive(Debug, Default)]
struct RunningProcessState {
    child: Option<Child>,
    stdout_worker: Option<JoinHandle<()>>,
    stderr_worker: Option<JoinHandle<()>>,
    stop_requested: Option<Arc<AtomicBool>>,
}

#[derive(Clone, Copy, Debug)]
struct PreviewPlaybackClock {
    generation: u64,
    base_seconds: f64,
    last_frame_seconds: f64,
    started_at: Instant,
    playing: bool,
    ended: bool,
}

struct ProcessHandles {
    child: Option<Child>,
    stdout_worker: Option<JoinHandle<()>>,
    stderr_worker: Option<JoinHandle<()>>,
    stop_requested: Option<Arc<AtomicBool>>,
}

struct SpawnedVideoStream {
    child: Child,
    stdout: Box<dyn Read + Send>,
    stderr_worker: JoinHandle<()>,
    stop_requested: Arc<AtomicBool>,
    spec: FrameStreamSpec,
}

#[derive(Clone, Copy, Debug)]
struct FrameStreamSpec {
    width: u32,
    height: u32,
    fps: u32,
    frame_bytes: usize,
    base_seconds: f64,
    first_frame_index: u64,
    generation: u64,
    spawned_at: Instant,
}

struct DecodedPreviewFrame {
    frame: PreviewRenderedFrame,
    read_elapsed: Duration,
    render_elapsed: Duration,
}

#[derive(Clone, Copy, Debug)]
struct RenderImageIdentity {
    image_id: ImageId,
    content_version: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioOutputSpec {
    sample_rate: u32,
    channels: u16,
}

struct AudioPreviewOutput {
    stream: cpal::Stream,
    buffer: Arc<Mutex<AudioSampleBuffer>>,
}

struct AudioSampleBuffer {
    samples: VecDeque<f32>,
    capacity: usize,
}

struct AudioStdoutWorkerConfig {
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
    playback: Arc<Mutex<PreviewPlaybackClock>>,
    metrics: PreviewRuntimeMetricsStore,
    stop_requested: Arc<AtomicBool>,
    mark_ended_on_eof: bool,
    generation: u64,
    prebuffer_samples: usize,
    ready_tx: mpsc::Sender<AudioStartStatus>,
}

struct VideoStdoutWorkerConfig {
    frame_store: LatestFrameStore,
    playback: Arc<Mutex<PreviewPlaybackClock>>,
    metrics: PreviewRuntimeMetricsStore,
    stop_requested: Arc<AtomicBool>,
    render_image_id: ImageId,
    render_image_version: Arc<AtomicU64>,
    spec: FrameStreamSpec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AudioStartStatus {
    Ready,
    OutputDisabled,
    TimedOut,
}

enum PendingAudioStart {
    Waiting {
        generation: u64,
        ready_rx: mpsc::Receiver<AudioStartStatus>,
    },
    OutputDisabled,
}

impl PendingAudioStart {
    const fn generation(&self) -> Option<u64> {
        match self {
            Self::Waiting { generation, .. } => Some(*generation),
            Self::OutputDisabled => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaybackStreamState {
    Ready,
    Waiting,
    Stale,
}

impl PreviewPlaybackClock {
    fn prepare_generation(&mut self, seconds: f64) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = false;
        self.ended = false;
        self.generation
    }

    fn start_generation(&mut self, generation: u64, seconds: f64) {
        if self.generation != generation {
            return;
        }
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = true;
        self.ended = false;
    }

    fn park_generation(&mut self, generation: u64, seconds: f64) {
        if self.generation != generation {
            return;
        }
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = false;
        self.ended = false;
    }

    const fn generation_matches(&self, generation: u64) -> bool {
        self.generation == generation
    }
}

impl AudioOutputSpec {
    fn default_output() -> Result<Self, PreviewEngineError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            PreviewEngineError::Audio("no default audio output device available".to_string())
        })?;
        let supported_config = device.default_output_config().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to read default output config: {err}"))
        })?;
        let config = supported_config.config();
        Ok(Self {
            sample_rate: config.sample_rate.0,
            channels: config.channels,
        })
    }
}

impl AudioPreviewOutput {
    fn new(
        metrics: &PreviewRuntimeMetricsStore,
        playback: &Arc<Mutex<PreviewPlaybackClock>>,
        generation: u64,
    ) -> Result<Self, PreviewEngineError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            PreviewEngineError::Audio("no default audio output device available".to_string())
        })?;
        let supported_config = device.default_output_config().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to read default output config: {err}"))
        })?;
        let sample_format = supported_config.sample_format();
        let config = supported_config.config();
        let channels = config.channels;
        let sample_rate = config.sample_rate.0;
        let capacity = usize::try_from(sample_rate)
            .unwrap_or(48_000)
            .saturating_mul(usize::from(channels))
            .saturating_mul(AUDIO_BUFFER_SECONDS);
        let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }));

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_audio_output_stream_f32(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            cpal::SampleFormat::F64 => build_audio_output_stream_f64(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            cpal::SampleFormat::I16 => build_audio_output_stream_i16(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            cpal::SampleFormat::U16 => build_audio_output_stream_u16(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            format => {
                return Err(PreviewEngineError::Audio(format!(
                    "unsupported output sample format: {format:?}"
                )));
            }
        };

        Ok(Self { stream, buffer })
    }

    fn play(&self) -> Result<(), PreviewEngineError> {
        self.stream.play().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to start output stream: {err}"))
        })
    }
}

fn build_audio_output_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                fill_audio_output_f32(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build f32 output stream: {err}"))
        })
}

fn build_audio_output_stream_f64(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            config,
            move |data: &mut [f64], _| {
                fill_audio_output_f64(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build f64 output stream: {err}"))
        })
}

fn build_audio_output_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            config,
            move |data: &mut [i16], _| {
                fill_audio_output_i16(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build i16 output stream: {err}"))
        })
}

fn build_audio_output_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            config,
            move |data: &mut [u16], _| {
                fill_audio_output_u16(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build u16 output stream: {err}"))
        })
}

impl RunningPreviewProcess {
    /// Pauses preview playback by terminating the active `FFmpeg` process.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot be terminated or reaped.
    pub fn pause(&self) -> Result<(), PreviewEngineError> {
        self.stop_running_processes()?;
        let (generation, seconds) = {
            let mut playback = lock_playback(&self.playback);
            playback.playing = false;
            (playback.generation, playback.last_frame_seconds)
        };
        self.prewarm_audio_for_generation(&self.config, seconds, true, generation);
        Ok(())
    }

    /// Resumes preview playback by spawning a new `FFmpeg` process at the last
    /// published frame timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when `FFmpeg` cannot be spawned.
    pub fn resume(&self) -> Result<(), PreviewEngineError> {
        let seconds = {
            let playback = lock_playback(&self.playback);
            if playback.ended {
                initial_start_seconds(&self.config)
            } else {
                playback.last_frame_seconds
            }
        };
        self.start_streaming(seconds, true)
    }

    /// Seeks preview playback to a source timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the timestamp is invalid, frame rendering fails,
    /// or the replacement `FFmpeg` process cannot be spawned.
    pub fn seek(
        &self,
        seconds: f64,
        render_seek_frame_when_paused: bool,
        precise: bool,
    ) -> Result<(), PreviewEngineError> {
        let seconds = clamp_seek_seconds(seconds, self.duration())?;
        if render_seek_frame_when_paused {
            self.stop_running_processes()?;
            if self.has_visual_stream() {
                self.render_single_frame(seconds, precise)?;
            } else {
                let generation = self.prepare_playback_generation(seconds);
                self.park_playback_generation(generation, seconds);
            }
            lock_playback(&self.playback).playing = false;
            return Ok(());
        }

        self.start_streaming(seconds, precise)
    }

    #[must_use]
    pub fn position(&self) -> f64 {
        self.reap_finished_processes();
        let playback = lock_playback(&self.playback);
        if playback.playing {
            let clock_seconds = playback.base_seconds + playback.started_at.elapsed().as_secs_f64();
            clock_seconds.max(playback.last_frame_seconds)
        } else {
            playback.last_frame_seconds
        }
    }

    #[must_use]
    pub const fn duration(&self) -> f64 {
        self.config.duration_seconds
    }

    #[must_use]
    pub fn ended(&self) -> bool {
        self.reap_finished_processes();
        lock_playback(&self.playback).ended
    }

    /// Stops every `FFmpeg` process owned by the preview pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error when a preview video or audio process cannot be stopped.
    pub fn stop(&mut self) -> Result<(), PreviewEngineError> {
        let result = self.stop_running_processes();
        log_preview_runtime_metrics("stop", self.metrics.snapshot());
        result
    }

    /// Rebuilds the `FFmpeg` preview command for a new conversion config while
    /// preserving the current source timestamp whenever possible.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement preview process cannot render its
    /// first frame or cannot be started.
    pub fn reconfigure(
        &mut self,
        config: PreviewSessionConfig,
        seconds: f64,
        playing: bool,
        precise: bool,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        let seconds = clamp_seek_seconds(seconds, config.duration_seconds)?;
        let dimensions = if matches!(
            config.source_kind,
            PreviewSourceKind::Video | PreviewSourceKind::Image
        ) {
            if playing && config.source_kind == PreviewSourceKind::Video {
                self.start_video_streaming_with_config(&config, seconds, precise)?
            } else {
                let started_at = Instant::now();
                let frame = decode_single_preview_frame(
                    &self.executable,
                    &config,
                    seconds,
                    precise,
                    self.next_render_image_identity(),
                )?;
                let dimensions = frame.frame.dimensions();
                self.stop_running_processes()?;
                let generation = self.prepare_playback_generation_at(seconds, started_at);
                self.metrics.record_video_frame_read(
                    generation,
                    frame.frame.byte_len,
                    frame.read_elapsed,
                );
                self.publish_frame_for_generation(frame.frame, frame.render_elapsed, generation);
                self.park_playback_generation(generation, seconds);
                self.prewarm_audio_for_generation(&config, seconds, precise, generation);
                dimensions
            }
        } else {
            self.stop_running_processes()?;
            self.config = config.clone();
            let generation = self.prepare_playback_generation(seconds);
            if playing {
                let status = self.start_audio_streaming(seconds, precise, true, generation)?;
                if status == AudioStartStatus::TimedOut {
                    return Err(PreviewEngineError::Audio(
                        "audio preview start readiness timed out".to_string(),
                    ));
                }
                self.start_playback_generation(generation, seconds);
            } else {
                self.park_playback_generation(generation, seconds);
            }
            config.target_dimensions()
        };

        self.config = config;
        Ok(dimensions)
    }

    fn new(
        config: PreviewSessionConfig,
        frame_store: LatestFrameStore,
        metrics: PreviewRuntimeMetricsStore,
        executable: String,
    ) -> Self {
        let start_seconds = initial_start_seconds(&config);
        Self {
            config,
            frame_store,
            metrics,
            executable,
            process: Mutex::new(RunningProcessState::default()),
            audio_process: Mutex::new(RunningProcessState::default()),
            pending_audio_start: Mutex::new(None),
            playback: Arc::new(Mutex::new(PreviewPlaybackClock {
                generation: 0,
                base_seconds: start_seconds,
                last_frame_seconds: start_seconds,
                started_at: Instant::now(),
                playing: false,
                ended: false,
            })),
            render_image_id: RenderImage::new_image_id(),
            render_image_version: Arc::new(AtomicU64::new(1)),
            stderr: Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_RING_LINES))),
        }
    }

    fn next_render_image_identity(&self) -> RenderImageIdentity {
        RenderImageIdentity {
            image_id: self.render_image_id,
            content_version: self.render_image_version.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn render_single_frame(
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

    fn prepare_playback_generation(&self, seconds: f64) -> u64 {
        let generation = {
            let mut playback = lock_playback(&self.playback);
            playback.prepare_generation(seconds)
        };
        self.metrics.begin_generation(generation);
        generation
    }

    fn prepare_playback_generation_at(&self, seconds: f64, started_at: Instant) -> u64 {
        let generation = {
            let mut playback = lock_playback(&self.playback);
            playback.prepare_generation(seconds)
        };
        self.metrics.begin_generation_at(generation, started_at);
        generation
    }

    fn start_playback_generation(&self, generation: u64, seconds: f64) {
        lock_playback(&self.playback).start_generation(generation, seconds);
    }

    fn park_playback_generation(&self, generation: u64, seconds: f64) {
        lock_playback(&self.playback).park_generation(generation, seconds);
    }

    fn parked_generation_at(&self, seconds: f64) -> Option<u64> {
        let playback = lock_playback(&self.playback);
        if playback.playing || playback.ended {
            return None;
        }
        if (playback.last_frame_seconds - seconds).abs() > SEEK_END_EPSILON {
            return None;
        }
        Some(playback.generation)
    }

    fn playback_generation_matches(&self, generation: u64) -> bool {
        lock_playback(&self.playback).generation_matches(generation)
    }

    fn publish_frame_for_generation(
        &self,
        frame: PreviewRenderedFrame,
        render_elapsed: Duration,
        generation: u64,
    ) {
        if !self.playback_generation_matches(generation) {
            return;
        }
        self.metrics
            .record_render_image_converted(generation, frame.byte_len, render_elapsed);
        let _ = self.frame_store.publish(frame);
        self.metrics.record_video_frame_published(generation);
    }

    fn start_streaming(&self, seconds: f64, precise: bool) -> Result<(), PreviewEngineError> {
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
    fn start_video_streaming_with_config(
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

    fn prepare_video_start_generation(&self, seconds: f64) -> (u64, Option<PendingAudioStart>) {
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

    fn start_audio_for_video_generation(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        generation: u64,
    ) -> Option<PendingAudioStart> {
        if config.has_audio {
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

    fn spawn_video_stream(
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

    const fn has_visual_stream(&self) -> bool {
        matches!(
            self.config.source_kind,
            PreviewSourceKind::Video | PreviewSourceKind::Image
        )
    }

    fn start_audio_streaming(
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

    fn start_audio_streaming_with_config(
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

    fn spawn_audio_streaming_with_config(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        mark_ended_on_eof: bool,
        generation: u64,
    ) -> Result<PendingAudioStart, PreviewEngineError> {
        if !config.has_audio {
            return Ok(PendingAudioStart::OutputDisabled);
        }

        self.stop_audio_process()?;
        let output_spec = AudioOutputSpec::default_output()?;
        let plan = preview_audio_plan(config, seconds, true, precise, output_spec)?;
        self.metrics.record_audio_process_spawn();
        let mut child = Command::new(&self.executable)
            .args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                PreviewEngineError::Ffmpeg(format!("failed to start audio preview: {err}"))
            })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg audio stdout was not captured".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg audio stderr was not captured".to_string())
        })?;
        let stop_requested = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel();
        let stdout_worker = spawn_audio_stdout_worker(
            stdout,
            AudioStdoutWorkerConfig {
                stderr_lines: Arc::clone(&self.stderr),
                playback: Arc::clone(&self.playback),
                metrics: self.metrics.clone(),
                stop_requested: Arc::clone(&stop_requested),
                mark_ended_on_eof,
                generation,
                prebuffer_samples: audio_prebuffer_samples(output_spec),
                ready_tx,
            },
        )?;
        let stderr_worker = spawn_stderr_worker(
            stderr,
            Arc::clone(&self.stderr),
            Arc::clone(&stop_requested),
        )?;

        let mut state = lock_process(&self.audio_process);
        *state = RunningProcessState {
            child: Some(child),
            stdout_worker: Some(stdout_worker),
            stderr_worker: Some(stderr_worker),
            stop_requested: Some(stop_requested),
        };
        drop(state);
        Ok(PendingAudioStart::Waiting {
            generation,
            ready_rx,
        })
    }

    fn finish_audio_start(&self, pending: PendingAudioStart) -> AudioStartStatus {
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

    fn prewarm_audio_for_generation(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        generation: u64,
    ) {
        if !config.has_audio || !self.has_visual_stream() {
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

    fn pending_audio_start_generation_matches(&self, generation: u64) -> bool {
        lock_pending_audio_start(&self.pending_audio_start)
            .as_ref()
            .and_then(PendingAudioStart::generation)
            == Some(generation)
    }

    fn take_pending_audio_start_for_generation(
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

    fn stop_running_processes(&self) -> Result<(), PreviewEngineError> {
        self.stop_video_process()?;
        self.stop_audio_process()?;
        Ok(())
    }

    fn stop_video_process(&self) -> Result<(), PreviewEngineError> {
        let handles = take_process_handles(&self.process);
        stop_process_handles(handles)
    }

    fn stop_audio_process(&self) -> Result<(), PreviewEngineError> {
        *lock_pending_audio_start(&self.pending_audio_start) = None;
        let handles = take_process_handles(&self.audio_process);
        stop_process_handles(handles)
    }

    fn reap_finished_processes(&self) {
        let video_finished = self.reap_finished_process(&self.process, true);
        if video_finished {
            let _ = self.stop_audio_process();
        }
        self.reap_finished_process(
            &self.audio_process,
            self.config.source_kind == PreviewSourceKind::Audio,
        );
    }

    fn reap_finished_process(
        &self,
        process: &Mutex<RunningProcessState>,
        mark_ended: bool,
    ) -> bool {
        let mut state = lock_process(process);
        let Some(child) = state.child.as_mut() else {
            return false;
        };
        let Ok(Some(_status)) = child.try_wait() else {
            return false;
        };

        state.child = None;
        let stdout_worker = state.stdout_worker.take();
        let stderr_worker = state.stderr_worker.take();
        state.stop_requested = None;
        drop(state);

        join_worker(stdout_worker);
        join_worker(stderr_worker);
        if mark_ended {
            let mut playback = lock_playback(&self.playback);
            playback.playing = false;
            playback.ended = true;
        }
        true
    }
}

fn stop_process_handles(handles: ProcessHandles) -> Result<(), PreviewEngineError> {
    if let Some(stop_requested) = &handles.stop_requested {
        stop_requested.store(true, Ordering::SeqCst);
    }
    if let Some(mut child) = handles.child {
        if child.try_wait().map_err(PreviewEngineError::Io)?.is_none() {
            child.kill().map_err(PreviewEngineError::Io)?;
        }
        let _ = child.wait();
    }
    join_worker(handles.stdout_worker);
    join_worker(handles.stderr_worker);
    Ok(())
}

fn stop_spawned_video_stream(stream: SpawnedVideoStream) -> Result<(), PreviewEngineError> {
    stop_process_handles(ProcessHandles {
        child: Some(stream.child),
        stdout_worker: None,
        stderr_worker: Some(stream.stderr_worker),
        stop_requested: Some(stream.stop_requested),
    })
}

fn take_process_handles(process: &Mutex<RunningProcessState>) -> ProcessHandles {
    let mut state = lock_process(process);
    ProcessHandles {
        child: state.child.take(),
        stdout_worker: state.stdout_worker.take(),
        stderr_worker: state.stderr_worker.take(),
        stop_requested: state.stop_requested.take(),
    }
}

fn audio_prebuffer_samples(output: AudioOutputSpec) -> usize {
    let samples_per_second = usize::try_from(output.sample_rate)
        .unwrap_or(48_000)
        .saturating_mul(usize::from(output.channels));
    samples_per_second.saturating_mul(usize::try_from(AUDIO_PREBUFFER_MS).unwrap_or(30)) / 1_000
}

fn wait_for_audio_start(receiver: &mpsc::Receiver<AudioStartStatus>) -> AudioStartStatus {
    receiver
        .recv_timeout(AUDIO_START_READY_TIMEOUT)
        .unwrap_or(AudioStartStatus::TimedOut)
}

impl Drop for RunningPreviewProcess {
    fn drop(&mut self) {
        let _ = self.stop_running_processes();
    }
}

/// Starts an `FFmpeg`-backed preview runtime.
///
/// # Errors
///
/// Returns an error when the initial preview frame cannot be rendered or the
/// `FFmpeg` executable cannot be launched.
pub(super) fn start_ffmpeg_preview_process(
    config: &PreviewSessionConfig,
    frame_store: LatestFrameStore,
    metrics: PreviewRuntimeMetricsStore,
) -> Result<(RunningPreviewProcess, PreviewDimensions, f64), PreviewEngineError> {
    let executable = ffmpeg_executable();
    let process = RunningPreviewProcess::new(config.clone(), frame_store, metrics, executable);
    let dimensions = if matches!(
        config.source_kind,
        PreviewSourceKind::Video | PreviewSourceKind::Image
    ) {
        process.render_single_frame(initial_start_seconds(config), true)?
    } else {
        config.target_dimensions()
    };

    Ok((process, dimensions, config.duration_seconds))
}

fn decode_single_preview_frame(
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

fn read_stream_frame(
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

fn rendered_preview_frame_from_payload(
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

fn clock_generation_matches(playback: &Arc<Mutex<PreviewPlaybackClock>>, generation: u64) -> bool {
    lock_playback(playback).generation_matches(generation)
}

fn playback_stream_state(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> PlaybackStreamState {
    let playback = lock_playback(playback);
    if !playback.generation_matches(generation) || playback.ended {
        return PlaybackStreamState::Stale;
    }
    if playback.playing {
        PlaybackStreamState::Ready
    } else {
        PlaybackStreamState::Waiting
    }
}

fn frame_presentation_state(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    timestamp_seconds: f64,
) -> PlaybackStreamState {
    let playback = lock_playback(playback);
    if !playback.generation_matches(generation) || playback.ended {
        return PlaybackStreamState::Stale;
    }
    if !playback.playing {
        return PlaybackStreamState::Waiting;
    }
    let clock_seconds = playback.base_seconds + playback.started_at.elapsed().as_secs_f64();
    if clock_seconds + VIDEO_FRAME_TIMING_EPSILON_SECONDS >= timestamp_seconds {
        PlaybackStreamState::Ready
    } else {
        PlaybackStreamState::Waiting
    }
}

fn wait_for_playback_start(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    stop_requested: &AtomicBool,
    generation: u64,
) -> PlaybackStreamState {
    loop {
        if stop_requested.load(Ordering::SeqCst) {
            return PlaybackStreamState::Stale;
        }
        match playback_stream_state(playback, generation) {
            PlaybackStreamState::Ready => return PlaybackStreamState::Ready,
            PlaybackStreamState::Stale => return PlaybackStreamState::Stale,
            PlaybackStreamState::Waiting => thread::sleep(VIDEO_START_WAIT_INTERVAL),
        }
    }
}

fn wait_for_frame_presentation_time(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    stop_requested: &AtomicBool,
    generation: u64,
    timestamp_seconds: f64,
) -> PlaybackStreamState {
    loop {
        if stop_requested.load(Ordering::SeqCst) {
            return PlaybackStreamState::Stale;
        }
        match frame_presentation_state(playback, generation, timestamp_seconds) {
            PlaybackStreamState::Ready => return PlaybackStreamState::Ready,
            PlaybackStreamState::Stale => return PlaybackStreamState::Stale,
            PlaybackStreamState::Waiting => thread::sleep(VIDEO_START_WAIT_INTERVAL),
        }
    }
}

fn update_last_frame_seconds(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    timestamp_seconds: f64,
) -> bool {
    let mut playback = lock_playback(playback);
    if !playback.generation_matches(generation) {
        return false;
    }
    playback.last_frame_seconds = timestamp_seconds;
    playback.ended = false;
    true
}

fn spawn_stdout_worker(
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

#[expect(
    clippy::too_many_lines,
    reason = "Audio stdout worker coordinates PCM buffering, prebuffer readiness, and CPAL start state."
)]
fn spawn_audio_stdout_worker(
    mut stdout: impl Read + Send + 'static,
    config: AudioStdoutWorkerConfig,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-audio-stdout".to_string())
        .spawn(move || {
            let output =
                match AudioPreviewOutput::new(&config.metrics, &config.playback, config.generation)
                {
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

            loop {
                if config.stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                if !clock_generation_matches(&config.playback, config.generation) {
                    break;
                }

                match stdout.read(&mut read_buffer) {
                    Ok(0) => break,
                    Ok(bytes_read) => {
                        pending.extend_from_slice(&read_buffer[..bytes_read]);
                        let complete_len = pending.len() - (pending.len() % 4);
                        if complete_len == 0 {
                            continue;
                        }

                        if let Some(output) = &output {
                            let samples =
                                push_audio_sample_bytes(&output.buffer, &pending[..complete_len]);
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
                                    &config.playback,
                                    &config.stop_requested,
                                    config.generation,
                                ) {
                                    PlaybackStreamState::Ready => match output.play() {
                                        Ok(()) => {}
                                        Err(error) => {
                                            push_stderr_line(
                                                &config.stderr_lines,
                                                format!("audio preview output disabled: {error}"),
                                            );
                                        }
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

            if !ready_sent {
                let _ = config.ready_tx.send(AudioStartStatus::TimedOut);
            }
            if config.mark_ended_on_eof && !config.stop_requested.load(Ordering::SeqCst) {
                let mut playback = lock_playback(&config.playback);
                if playback.generation_matches(config.generation) {
                    playback.playing = false;
                    playback.ended = true;
                }
            }
        })
        .map_err(|err| {
            PreviewEngineError::Ffmpeg(format!("failed to spawn audio stdout worker: {err}"))
        })
}

fn spawn_stderr_worker(
    stderr: impl Read + Send + 'static,
    lines: Arc<Mutex<VecDeque<String>>>,
    stop_requested: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-stderr".to_string())
        .spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(line) = line else {
                    break;
                };
                push_stderr_line(&lines, line);
            }
        })
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to spawn stderr worker: {err}")))
}

fn fill_audio_output_f32(
    data: &mut [f32],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(0.0);
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = value;
        } else {
            *sample = 0.0;
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

fn fill_audio_output_f64(
    data: &mut [f64],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(0.0);
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = f64::from(value);
        } else {
            *sample = 0.0;
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

fn fill_audio_output_i16(
    data: &mut [i16],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(0);
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = f32_to_i16(value);
        } else {
            *sample = 0;
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

fn fill_audio_output_u16(
    data: &mut [u16],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(f32_to_u16(0.0));
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = f32_to_u16(value);
        } else {
            *sample = f32_to_u16(0.0);
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

fn push_audio_sample_bytes(buffer: &Arc<Mutex<AudioSampleBuffer>>, bytes: &[u8]) -> usize {
    let mut samples = 0_usize;
    {
        let mut buffer = lock_audio_buffer(buffer);
        for bytes in bytes.chunks_exact(4) {
            let sample = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if buffer.samples.len() >= buffer.capacity {
                let _ = buffer.samples.pop_front();
            }
            buffer.samples.push_back(sample.clamp(-1.0, 1.0));
            samples = samples.saturating_add(1);
        }
        drop(buffer);
    }
    samples
}

fn audio_output_is_playing(playback: &Arc<Mutex<PreviewPlaybackClock>>, generation: u64) -> bool {
    let playback = lock_playback(playback);
    playback.generation_matches(generation) && playback.playing && !playback.ended
}

fn record_audio_callback_if_current(
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    underrun: bool,
) {
    if clock_generation_matches(playback, generation) {
        metrics.record_audio_output_callback(generation, underrun);
    }
}

fn f32_to_i16(sample: f32) -> i16 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio samples are clamped to the i16 range before conversion"
    )]
    let converted = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
    converted
}

fn f32_to_u16(sample: f32) -> u16 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "audio samples are clamped to the u16 range before conversion"
    )]
    let converted =
        (sample.clamp(-1.0, 1.0).mul_add(0.5, 0.5) * f32::from(u16::MAX)).round() as u16;
    converted
}

fn preview_plan(
    config: &PreviewSessionConfig,
    seconds: f64,
    realtime: bool,
    precise: bool,
) -> Result<PreviewFfmpegPlan, PreviewEngineError> {
    let end_seconds = parsed_time(config.conversion_config.end_time.as_deref())
        .filter(|end_seconds| *end_seconds > seconds);
    build_ffmpeg_preview_args(
        config.path.to_string_lossy().as_ref(),
        &config.conversion_config,
        &PreviewFfmpegOptions {
            start_seconds: seconds,
            end_seconds,
            source_width: config.source_width,
            source_height: config.source_height,
            max_width: config.max_width,
            max_height: config.max_height,
            fps: config.fps,
            realtime,
            precise_seek: precise,
            source_is_image: config.source_kind == PreviewSourceKind::Image,
        },
    )
    .map_err(|err| PreviewEngineError::Ffmpeg(err.to_string()))
}

fn preview_audio_plan(
    config: &PreviewSessionConfig,
    seconds: f64,
    realtime: bool,
    precise: bool,
    output: AudioOutputSpec,
) -> Result<frame_core::preview::PreviewAudioFfmpegPlan, PreviewEngineError> {
    let end_seconds = parsed_time(config.conversion_config.end_time.as_deref())
        .filter(|end_seconds| *end_seconds > seconds);
    build_ffmpeg_preview_audio_args(
        config.path.to_string_lossy().as_ref(),
        &config.conversion_config,
        &PreviewAudioFfmpegOptions {
            start_seconds: seconds,
            end_seconds,
            sample_rate: output.sample_rate,
            channels: output.channels,
            realtime,
            precise_seek: precise,
            selected_track: config.selected_audio_track,
        },
    )
    .map_err(|err| PreviewEngineError::Ffmpeg(err.to_string()))
}

fn insert_frame_limit(args: &mut Vec<String>, frames: u32) {
    let insert_at = args.len().saturating_sub(1);
    args.insert(insert_at, "-frames:v".to_string());
    args.insert(insert_at + 1, frames.to_string());
}

fn parsed_time(value: Option<&str>) -> Option<f64> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(parse_time)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn initial_start_seconds(config: &PreviewSessionConfig) -> f64 {
    parsed_time(config.conversion_config.start_time.as_deref()).unwrap_or(0.0)
}

fn clamp_seek_seconds(seconds: f64, duration: f64) -> Result<f64, PreviewEngineError> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(PreviewEngineError::InvalidInput(
            "seek position must be a positive finite number".to_string(),
        ));
    }

    if !duration.is_finite() || duration <= 0.0 {
        return Ok(seconds);
    }

    Ok(seconds.min((duration - SEEK_END_EPSILON).max(0.0)))
}

fn seconds_to_us(seconds: f64) -> u64 {
    f64_to_u64(seconds.max(0.0) * 1_000_000.0)
}

fn push_stderr_line(lines: &Arc<Mutex<VecDeque<String>>>, line: String) {
    let mut lines = lock_stderr(lines);
    if lines.len() >= STDERR_RING_LINES {
        let _ = lines.pop_front();
    }
    lines.push_back(line);
}

fn stderr_detail(stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!(": {trimmed}")
    }
}

fn join_worker(worker: Option<JoinHandle<()>>) {
    if let Some(worker) = worker {
        let _ = worker.join();
    }
}

fn lock_process(state: &Mutex<RunningProcessState>) -> MutexGuard<'_, RunningProcessState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_playback(state: &Mutex<PreviewPlaybackClock>) -> MutexGuard<'_, PreviewPlaybackClock> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_stderr(lines: &Mutex<VecDeque<String>>) -> MutexGuard<'_, VecDeque<String>> {
    lines
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_pending_audio_start(
    pending_audio: &Mutex<Option<PendingAudioStart>>,
) -> MutexGuard<'_, Option<PendingAudioStart>> {
    pending_audio
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_audio_buffer(buffer: &Mutex<AudioSampleBuffer>) -> MutexGuard<'_, AudioSampleBuffer> {
    buffer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{conversion_runner::core_config_from_gpui, settings::ConversionConfig};
    use std::path::PathBuf;

    fn preview_config() -> PreviewSessionConfig {
        PreviewSessionConfig {
            file_id: "video-1".to_string(),
            path: PathBuf::from("/tmp/video.mp4"),
            source_kind: PreviewSourceKind::Video,
            source_width: Some(1920),
            source_height: Some(1080),
            has_audio: false,
            selected_audio_track: None,
            duration_seconds: 12.5,
            max_width: 1280,
            max_height: 720,
            fps: 30,
            conversion_config: core_config_from_gpui(&ConversionConfig::default()),
        }
    }

    #[test]
    fn insert_frame_limit_adds_single_frame_before_pipe_output() {
        let mut args = vec![
            "-f".to_string(),
            "rawvideo".to_string(),
            "pipe:1".to_string(),
        ];

        insert_frame_limit(&mut args, 1);

        assert_eq!(args[2], "-frames:v");
        assert_eq!(args[3], "1");
        assert_eq!(args.last(), Some(&"pipe:1".to_string()));
    }

    #[test]
    fn preview_plan_uses_configured_trim_end_as_duration_bound() {
        let mut config = preview_config();
        config.conversion_config.end_time = Some("00:00:10.000".to_string());

        let plan = preview_plan(&config, 2.0, true, true).expect("plan");

        assert!(plan.args.windows(2).any(|args| args == ["-t", "8.000"]));
    }

    #[test]
    fn preview_audio_plan_streams_selected_track_as_device_pcm() {
        let mut config = preview_config();
        config.has_audio = true;
        config.selected_audio_track = Some(2);
        config.conversion_config.end_time = Some("00:00:05.000".to_string());

        let plan = preview_audio_plan(
            &config,
            1.0,
            true,
            true,
            AudioOutputSpec {
                sample_rate: 44_100,
                channels: 2,
            },
        )
        .expect("audio plan");

        assert!(plan.args.windows(2).any(|args| args == ["-map", "0:2"]));
        assert!(plan.args.windows(2).any(|args| args == ["-t", "4.000"]));
        assert!(plan.args.windows(2).any(|args| args == ["-ar", "44100"]));
        assert!(plan.args.windows(2).any(|args| args == ["-ac", "2"]));
        assert!(plan.args.windows(2).any(|args| args == ["-f", "f32le"]));
    }

    #[test]
    fn preview_playback_clock_prepares_distinct_generations() {
        let mut clock = PreviewPlaybackClock {
            generation: 0,
            base_seconds: 0.0,
            last_frame_seconds: 0.0,
            started_at: Instant::now(),
            playing: true,
            ended: true,
        };

        let generation = clock.prepare_generation(2.5);

        assert_eq!(generation, 1);
        assert!((clock.base_seconds - 2.5).abs() <= f64::EPSILON);
        assert!((clock.last_frame_seconds - 2.5).abs() <= f64::EPSILON);
        assert!(!clock.playing);
        assert!(!clock.ended);
    }

    #[test]
    fn playback_stream_state_waits_until_generation_is_started() {
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 0,
            base_seconds: 0.0,
            last_frame_seconds: 0.0,
            started_at: Instant::now(),
            playing: true,
            ended: false,
        }));
        let generation = lock_playback(&playback).prepare_generation(2.5);

        assert_eq!(
            playback_stream_state(&playback, generation),
            PlaybackStreamState::Waiting
        );

        lock_playback(&playback).start_generation(generation, 2.5);

        assert_eq!(
            playback_stream_state(&playback, generation),
            PlaybackStreamState::Ready
        );
    }

    #[test]
    fn frame_presentation_state_waits_for_future_video_timestamp() {
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 1,
            base_seconds: 10.0,
            last_frame_seconds: 10.0,
            started_at: Instant::now(),
            playing: true,
            ended: false,
        }));

        assert_eq!(
            frame_presentation_state(&playback, 1, 10.5),
            PlaybackStreamState::Waiting
        );
    }

    #[test]
    fn frame_presentation_state_allows_due_video_timestamp() {
        let started_at = Instant::now()
            .checked_sub(Duration::from_millis(40))
            .expect("test timestamp should be representable");
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 1,
            base_seconds: 10.0,
            last_frame_seconds: 10.0,
            started_at,
            playing: true,
            ended: false,
        }));

        assert_eq!(
            frame_presentation_state(&playback, 1, 10.033),
            PlaybackStreamState::Ready
        );
    }

    #[test]
    fn wait_for_playback_start_returns_stale_when_stop_is_requested() {
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 1,
            base_seconds: 0.0,
            last_frame_seconds: 0.0,
            started_at: Instant::now(),
            playing: false,
            ended: false,
        }));
        let stop_requested = AtomicBool::new(true);

        assert_eq!(
            wait_for_playback_start(&playback, &stop_requested, 1),
            PlaybackStreamState::Stale
        );
    }

    #[test]
    fn update_last_frame_seconds_rejects_stale_generations() {
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 2,
            base_seconds: 0.0,
            last_frame_seconds: 0.0,
            started_at: Instant::now(),
            playing: true,
            ended: false,
        }));

        let updated = update_last_frame_seconds(&playback, 1, 4.0);

        assert!(!updated);
        assert!(lock_playback(&playback).last_frame_seconds.abs() <= f64::EPSILON);
    }

    #[test]
    fn audio_prebuffer_samples_uses_sample_rate_and_channels() {
        let samples = audio_prebuffer_samples(AudioOutputSpec {
            sample_rate: 48_000,
            channels: 2,
        });

        assert_eq!(samples, 2_880);
    }

    #[test]
    fn push_audio_sample_bytes_clamps_samples_and_returns_count() {
        let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
            samples: VecDeque::new(),
            capacity: 4,
        }));
        let samples = [
            2.0_f32.to_le_bytes(),
            (-2.0_f32).to_le_bytes(),
            0.25_f32.to_le_bytes(),
        ]
        .concat();

        let count = push_audio_sample_bytes(&buffer, &samples);

        let queued = {
            let buffer = lock_audio_buffer(&buffer);
            buffer.samples.iter().copied().collect::<Vec<_>>()
        };
        assert_eq!(count, 3);
        assert!((queued[0] - 1.0).abs() <= f32::EPSILON);
        assert!((queued[1] + 1.0).abs() <= f32::EPSILON);
        assert!((queued[2] - 0.25).abs() <= f32::EPSILON);
    }

    #[test]
    fn fill_audio_output_f32_mutes_when_playback_is_paused() {
        let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
            samples: VecDeque::from([0.5, -0.5]),
            capacity: 4,
        }));
        let metrics = PreviewRuntimeMetricsStore::new();
        metrics.begin_generation(1);
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 1,
            base_seconds: 0.0,
            last_frame_seconds: 0.0,
            started_at: Instant::now(),
            playing: false,
            ended: false,
        }));
        let mut data = [1.0_f32, 1.0];

        fill_audio_output_f32(&mut data, &buffer, &metrics, &playback, 1);

        assert!(data.iter().all(|sample| sample.abs() <= f32::EPSILON));
        assert_eq!(lock_audio_buffer(&buffer).samples.len(), 2);
        assert_eq!(metrics.snapshot().audio_output_callbacks, 0);
    }

    #[test]
    fn fill_audio_output_f32_drains_samples_when_playback_is_playing() {
        let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
            samples: VecDeque::from([0.25]),
            capacity: 4,
        }));
        let metrics = PreviewRuntimeMetricsStore::new();
        metrics.begin_generation(1);
        let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 1,
            base_seconds: 0.0,
            last_frame_seconds: 0.0,
            started_at: Instant::now(),
            playing: true,
            ended: false,
        }));
        let mut data = [0.0_f32, 0.0];

        fill_audio_output_f32(&mut data, &buffer, &metrics, &playback, 1);

        assert!((data[0] - 0.25).abs() <= f32::EPSILON);
        assert!(data[1].abs() <= f32::EPSILON);
        assert!(lock_audio_buffer(&buffer).samples.is_empty());
        assert_eq!(metrics.snapshot().audio_output_callbacks, 1);
    }

    #[test]
    fn clamp_seek_seconds_keeps_seek_inside_duration() {
        let seconds = clamp_seek_seconds(12.5, 12.5).expect("seconds");

        assert!((seconds - 12.499).abs() <= f64::EPSILON);
    }
}
