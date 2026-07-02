use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, ErrorKind, Read},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use frame_core::{
    preview::{
        PreviewAudioFfmpegOptions, PreviewFfmpegOptions, PreviewFfmpegPlan,
        build_ffmpeg_preview_args, build_ffmpeg_preview_audio_args,
    },
    utils::parse_time,
};

use crate::{
    numeric::{f64_to_u64, u64_to_f64},
    runtime_binaries::ffmpeg_executable,
};

use super::{
    LatestFrameStore, PreviewDimensions, PreviewEngineError, PreviewFrame, PreviewSessionConfig,
    PreviewSourceKind,
};

const STDERR_RING_LINES: usize = 24;
const SEEK_END_EPSILON: f64 = 0.001;
const AUDIO_BUFFER_SECONDS: usize = 3;
const AUDIO_READ_BUFFER_BYTES: usize = 8192;

pub struct RunningPreviewProcess {
    config: PreviewSessionConfig,
    frame_store: LatestFrameStore,
    executable: String,
    process: Mutex<RunningProcessState>,
    audio_process: Mutex<RunningProcessState>,
    playback: Arc<Mutex<FfmpegPlaybackState>>,
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
struct FfmpegPlaybackState {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioOutputSpec {
    sample_rate: u32,
    channels: u16,
}

struct AudioPreviewOutput {
    _stream: cpal::Stream,
    buffer: Arc<Mutex<AudioSampleBuffer>>,
}

struct AudioSampleBuffer {
    samples: VecDeque<f32>,
    capacity: usize,
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
    fn new() -> Result<Self, PreviewEngineError> {
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
        let err_fn = |err| eprintln!("frame preview audio stream error: {err}");

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let buffer = Arc::clone(&buffer);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _| fill_audio_output_f32(data, &buffer),
                        err_fn,
                        None,
                    )
                    .map_err(|err| {
                        PreviewEngineError::Audio(format!(
                            "failed to build f32 output stream: {err}"
                        ))
                    })?
            }
            cpal::SampleFormat::F64 => {
                let buffer = Arc::clone(&buffer);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f64], _| fill_audio_output_f64(data, &buffer),
                        err_fn,
                        None,
                    )
                    .map_err(|err| {
                        PreviewEngineError::Audio(format!(
                            "failed to build f64 output stream: {err}"
                        ))
                    })?
            }
            cpal::SampleFormat::I16 => {
                let buffer = Arc::clone(&buffer);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| fill_audio_output_i16(data, &buffer),
                        err_fn,
                        None,
                    )
                    .map_err(|err| {
                        PreviewEngineError::Audio(format!(
                            "failed to build i16 output stream: {err}"
                        ))
                    })?
            }
            cpal::SampleFormat::U16 => {
                let buffer = Arc::clone(&buffer);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _| fill_audio_output_u16(data, &buffer),
                        err_fn,
                        None,
                    )
                    .map_err(|err| {
                        PreviewEngineError::Audio(format!(
                            "failed to build u16 output stream: {err}"
                        ))
                    })?
            }
            format => {
                return Err(PreviewEngineError::Audio(format!(
                    "unsupported output sample format: {format:?}"
                )));
            }
        };

        stream.play().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to start output stream: {err}"))
        })?;

        Ok(Self {
            _stream: stream,
            buffer,
        })
    }
}

impl RunningPreviewProcess {
    /// Pauses preview playback by terminating the active `FFmpeg` process.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot be terminated or reaped.
    pub fn pause(&self) -> Result<(), PreviewEngineError> {
        self.stop_running_processes()?;
        lock_playback(&self.playback).playing = false;
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
                let mut playback = lock_playback(&self.playback);
                playback.base_seconds = seconds;
                playback.last_frame_seconds = seconds;
                playback.started_at = Instant::now();
                playback.playing = false;
                playback.ended = false;
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

    pub fn stop(&mut self) {
        let _ = self.stop_running_processes();
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
                self.start_video_streaming_with_config(&config, seconds, precise, true)?
            } else {
                let frame =
                    decode_single_preview_frame(&self.executable, &config, seconds, precise)?;
                let dimensions = frame.dimensions();
                self.stop_running_processes()?;
                let _ = self.frame_store.publish(frame);
                let mut playback = lock_playback(&self.playback);
                playback.base_seconds = seconds;
                playback.last_frame_seconds = seconds;
                playback.started_at = Instant::now();
                playback.playing = false;
                playback.ended = false;
                dimensions
            }
        } else {
            self.stop_running_processes()?;
            self.config = config.clone();
            if playing {
                self.start_audio_streaming(seconds, precise, true)?;
            }
            let mut playback = lock_playback(&self.playback);
            playback.base_seconds = seconds;
            playback.last_frame_seconds = seconds;
            playback.started_at = Instant::now();
            playback.playing = playing;
            playback.ended = false;
            drop(playback);
            config.target_dimensions()
        };

        self.config = config;
        Ok(dimensions)
    }

    fn new(
        config: PreviewSessionConfig,
        frame_store: LatestFrameStore,
        executable: String,
    ) -> Self {
        let start_seconds = initial_start_seconds(&config);
        Self {
            config,
            frame_store,
            executable,
            process: Mutex::new(RunningProcessState::default()),
            audio_process: Mutex::new(RunningProcessState::default()),
            playback: Arc::new(Mutex::new(FfmpegPlaybackState {
                base_seconds: start_seconds,
                last_frame_seconds: start_seconds,
                started_at: Instant::now(),
                playing: false,
                ended: false,
            })),
            stderr: Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_RING_LINES))),
        }
    }

    fn render_single_frame(
        &self,
        seconds: f64,
        precise: bool,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        let frame = decode_single_preview_frame(&self.executable, &self.config, seconds, precise)?;
        let dimensions = frame.dimensions();
        let _ = self.frame_store.publish(frame);
        let mut playback = lock_playback(&self.playback);
        playback.base_seconds = seconds;
        playback.last_frame_seconds = seconds;
        playback.started_at = Instant::now();
        playback.playing = false;
        playback.ended = false;
        drop(playback);
        Ok(dimensions)
    }

    fn start_streaming(&self, seconds: f64, precise: bool) -> Result<(), PreviewEngineError> {
        if !self.has_visual_stream() {
            self.stop_running_processes()?;
            self.start_audio_streaming(seconds, precise, true)?;
            let mut playback = lock_playback(&self.playback);
            playback.base_seconds = seconds;
            playback.last_frame_seconds = seconds;
            playback.started_at = Instant::now();
            playback.playing = true;
            playback.ended = false;
            drop(playback);
            return Ok(());
        }

        let buffered = has_running_child(&self.process);
        self.start_video_streaming_with_config(&self.config, seconds, precise, buffered)?;
        Ok(())
    }

    fn start_video_streaming_with_config(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        buffered: bool,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        let mut stream = self.spawn_video_stream(config, seconds, precise)?;

        let first_frame = if buffered {
            match read_stream_frame(&mut stream.stdout, &stream.spec, stream.spec.base_seconds) {
                Ok(frame) => Some(frame),
                Err(error) => {
                    let _ = stop_spawned_video_stream(stream);
                    return Err(error);
                }
            }
        } else {
            None
        };

        let stop_result = if buffered {
            self.stop_video_process()
        } else {
            self.stop_running_processes()
        };
        if let Err(error) = stop_result {
            let _ = stop_spawned_video_stream(stream);
            return Err(error);
        }

        let dimensions = PreviewDimensions {
            width: stream.spec.width,
            height: stream.spec.height,
        };
        let mut stdout_spec = stream.spec;
        if first_frame.is_some() {
            stdout_spec.first_frame_index = 1;
        }
        let stdout_worker = match spawn_stdout_worker(
            stream.stdout,
            self.frame_store.clone(),
            Arc::clone(&self.playback),
            Arc::clone(&stream.stop_requested),
            stdout_spec,
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

        let mut playback = lock_playback(&self.playback);
        playback.base_seconds = seconds;
        playback.last_frame_seconds = seconds;
        playback.started_at = Instant::now();
        playback.playing = true;
        playback.ended = false;
        drop(playback);

        if let Some(frame) = first_frame {
            let _ = self.frame_store.publish(frame);
        }

        if config.has_audio {
            if let Err(error) =
                self.start_audio_streaming_with_config(config, seconds, precise, false)
            {
                push_stderr_line(&self.stderr, format!("audio preview disabled: {error}"));
            }
        } else if let Err(error) = self.stop_audio_process() {
            push_stderr_line(
                &self.stderr,
                format!("audio preview cleanup failed: {error}"),
            );
        }

        Ok(dimensions)
    }

    fn spawn_video_stream(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
    ) -> Result<SpawnedVideoStream, PreviewEngineError> {
        let plan = preview_plan(config, seconds, true, precise)?;
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
    ) -> Result<(), PreviewEngineError> {
        self.start_audio_streaming_with_config(&self.config, seconds, precise, mark_ended_on_eof)
    }

    fn start_audio_streaming_with_config(
        &self,
        config: &PreviewSessionConfig,
        seconds: f64,
        precise: bool,
        mark_ended_on_eof: bool,
    ) -> Result<(), PreviewEngineError> {
        if !config.has_audio {
            return Ok(());
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

        let stdout = child.stdout.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg audio stdout was not captured".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            PreviewEngineError::Ffmpeg("ffmpeg audio stderr was not captured".to_string())
        })?;
        let stop_requested = Arc::new(AtomicBool::new(false));
        let stdout_worker = spawn_audio_stdout_worker(
            stdout,
            Arc::clone(&self.stderr),
            Arc::clone(&self.playback),
            Arc::clone(&stop_requested),
            mark_ended_on_eof,
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
        Ok(())
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

fn has_running_child(process: &Mutex<RunningProcessState>) -> bool {
    lock_process(process).child.is_some()
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
pub fn start_ffmpeg_preview_process(
    config: &PreviewSessionConfig,
    frame_store: LatestFrameStore,
) -> Result<(RunningPreviewProcess, PreviewDimensions, f64), PreviewEngineError> {
    let executable = ffmpeg_executable();
    let process = RunningPreviewProcess::new(config.clone(), frame_store, executable);
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
) -> Result<PreviewFrame, PreviewEngineError> {
    let plan = preview_plan(config, seconds, false, precise)?;
    let mut args = plan.args.clone();
    insert_frame_limit(&mut args, 1);
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

    PreviewFrame::bgra(
        plan.width,
        plan.height,
        plan.width.saturating_mul(4),
        seconds_to_us(seconds),
        output.stdout[..plan.frame_bytes].to_vec(),
    )
}

fn read_stream_frame(
    stdout: &mut dyn Read,
    spec: &FrameStreamSpec,
    timestamp_seconds: f64,
) -> Result<PreviewFrame, PreviewEngineError> {
    let mut payload = vec![0_u8; spec.frame_bytes];
    stdout.read_exact(&mut payload).map_err(|error| {
        PreviewEngineError::Ffmpeg(format!("failed to read first preview frame: {error}"))
    })?;
    PreviewFrame::bgra(
        spec.width,
        spec.height,
        spec.width.saturating_mul(4),
        seconds_to_us(timestamp_seconds),
        payload,
    )
}

fn spawn_stdout_worker(
    mut stdout: impl Read + Send + 'static,
    frame_store: LatestFrameStore,
    playback: Arc<Mutex<FfmpegPlaybackState>>,
    stop_requested: Arc<AtomicBool>,
    spec: FrameStreamSpec,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-stdout".to_string())
        .spawn(move || {
            let mut frame_index = spec.first_frame_index;
            loop {
                if stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                let mut payload = vec![0_u8; spec.frame_bytes];
                match stdout.read_exact(&mut payload) {
                    Ok(()) => {
                        let timestamp_seconds = spec.base_seconds
                            + (u64_to_f64(frame_index) / f64::from(spec.fps.max(1)));
                        if let Ok(frame) = PreviewFrame::bgra(
                            spec.width,
                            spec.height,
                            spec.width.saturating_mul(4),
                            seconds_to_us(timestamp_seconds),
                            payload,
                        ) {
                            let _ = frame_store.publish(frame);
                            let mut playback = lock_playback(&playback);
                            playback.last_frame_seconds = timestamp_seconds;
                            playback.ended = false;
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
                playback.playing = false;
                playback.ended = true;
            }
        })
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to spawn stdout worker: {err}")))
}

fn spawn_audio_stdout_worker(
    mut stdout: impl Read + Send + 'static,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
    playback: Arc<Mutex<FfmpegPlaybackState>>,
    stop_requested: Arc<AtomicBool>,
    mark_ended_on_eof: bool,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-audio-stdout".to_string())
        .spawn(move || {
            let output = match AudioPreviewOutput::new() {
                Ok(output) => Some(output),
                Err(error) => {
                    push_stderr_line(
                        &stderr_lines,
                        format!("audio preview output disabled: {error}"),
                    );
                    None
                }
            };
            let mut read_buffer = [0_u8; AUDIO_READ_BUFFER_BYTES];
            let mut pending = Vec::new();

            loop {
                if stop_requested.load(Ordering::SeqCst) {
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

                        let mut samples = Vec::with_capacity(complete_len / 4);
                        for bytes in pending[..complete_len].chunks_exact(4) {
                            samples
                                .push(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
                        }
                        if let Some(output) = &output {
                            push_audio_samples(&output.buffer, &samples);
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

            if mark_ended_on_eof && !stop_requested.load(Ordering::SeqCst) {
                let mut playback = lock_playback(&playback);
                playback.playing = false;
                playback.ended = true;
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

fn fill_audio_output_f32(data: &mut [f32], buffer: &Arc<Mutex<AudioSampleBuffer>>) {
    let mut buffer = lock_audio_buffer(buffer);
    for sample in data {
        *sample = buffer.samples.pop_front().unwrap_or(0.0);
    }
}

fn fill_audio_output_f64(data: &mut [f64], buffer: &Arc<Mutex<AudioSampleBuffer>>) {
    let mut buffer = lock_audio_buffer(buffer);
    for sample in data {
        *sample = f64::from(buffer.samples.pop_front().unwrap_or(0.0));
    }
}

fn fill_audio_output_i16(data: &mut [i16], buffer: &Arc<Mutex<AudioSampleBuffer>>) {
    let mut buffer = lock_audio_buffer(buffer);
    for sample in data {
        *sample = f32_to_i16(buffer.samples.pop_front().unwrap_or(0.0));
    }
}

fn fill_audio_output_u16(data: &mut [u16], buffer: &Arc<Mutex<AudioSampleBuffer>>) {
    let mut buffer = lock_audio_buffer(buffer);
    for sample in data {
        *sample = f32_to_u16(buffer.samples.pop_front().unwrap_or(0.0));
    }
}

fn push_audio_samples(buffer: &Arc<Mutex<AudioSampleBuffer>>, samples: &[f32]) {
    let mut buffer = lock_audio_buffer(buffer);
    for sample in samples {
        if buffer.samples.len() >= buffer.capacity {
            let _ = buffer.samples.pop_front();
        }
        buffer.samples.push_back(sample.clamp(-1.0, 1.0));
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

fn lock_playback(state: &Mutex<FfmpegPlaybackState>) -> MutexGuard<'_, FfmpegPlaybackState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_stderr(lines: &Mutex<VecDeque<String>>) -> MutexGuard<'_, VecDeque<String>> {
    lines
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
    fn clamp_seek_seconds_keeps_seek_inside_duration() {
        let seconds = clamp_seek_seconds(12.5, 12.5).expect("seconds");

        assert!((seconds - 12.499).abs() <= f64::EPSILON);
    }
}
