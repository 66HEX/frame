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

use frame_core::{
    preview::{PreviewFfmpegOptions, PreviewFfmpegPlan, build_ffmpeg_preview_args},
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

pub struct RunningPreviewProcess {
    config: PreviewSessionConfig,
    frame_store: LatestFrameStore,
    executable: String,
    process: Mutex<RunningProcessState>,
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

#[derive(Clone, Copy, Debug)]
struct FrameStreamSpec {
    width: u32,
    height: u32,
    fps: u32,
    frame_bytes: usize,
    base_seconds: f64,
}

impl RunningPreviewProcess {
    /// Pauses preview playback by terminating the active `FFmpeg` process.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot be terminated or reaped.
    pub fn pause(&self) -> Result<(), PreviewEngineError> {
        self.stop_running_process()?;
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
        if self.config.source_kind == PreviewSourceKind::Audio {
            return Ok(());
        }

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
        if self.config.source_kind == PreviewSourceKind::Audio {
            return Ok(());
        }

        let seconds = clamp_seek_seconds(seconds, self.duration())?;
        if render_seek_frame_when_paused {
            self.stop_running_process()?;
            self.render_single_frame(seconds, precise)?;
            lock_playback(&self.playback).playing = false;
            return Ok(());
        }

        self.start_streaming(seconds, precise)
    }

    #[must_use]
    pub fn position(&self) -> f64 {
        self.reap_finished_process();
        let playback = lock_playback(&self.playback);
        if playback.playing {
            playback.last_frame_seconds.max(playback.base_seconds)
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
        self.reap_finished_process();
        lock_playback(&self.playback).ended
    }

    pub fn stop(&mut self) {
        let _ = self.stop_running_process();
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
        self.stop_running_process()?;
        let plan = preview_plan(&self.config, seconds, true, precise)?;
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
        let stdout_worker = spawn_stdout_worker(
            stdout,
            self.frame_store.clone(),
            Arc::clone(&self.playback),
            Arc::clone(&stop_requested),
            FrameStreamSpec {
                width: plan.width,
                height: plan.height,
                fps: plan.fps,
                frame_bytes: plan.frame_bytes,
                base_seconds: seconds,
            },
        )?;
        let stderr_worker = spawn_stderr_worker(
            stderr,
            Arc::clone(&self.stderr),
            Arc::clone(&stop_requested),
        )?;

        {
            let mut playback = lock_playback(&self.playback);
            playback.base_seconds = seconds;
            playback.last_frame_seconds = seconds;
            playback.started_at = Instant::now();
            playback.playing = true;
            playback.ended = false;
        }

        let mut state = lock_process(&self.process);
        *state = RunningProcessState {
            child: Some(child),
            stdout_worker: Some(stdout_worker),
            stderr_worker: Some(stderr_worker),
            stop_requested: Some(stop_requested),
        };
        drop(state);
        Ok(())
    }

    fn stop_running_process(&self) -> Result<(), PreviewEngineError> {
        let handles = self.take_process_handles();
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

    fn take_process_handles(&self) -> ProcessHandles {
        let mut state = lock_process(&self.process);
        ProcessHandles {
            child: state.child.take(),
            stdout_worker: state.stdout_worker.take(),
            stderr_worker: state.stderr_worker.take(),
            stop_requested: state.stop_requested.take(),
        }
    }

    fn reap_finished_process(&self) {
        let mut state = lock_process(&self.process);
        let Some(child) = state.child.as_mut() else {
            return;
        };
        let Ok(Some(_status)) = child.try_wait() else {
            return;
        };

        state.child = None;
        let stdout_worker = state.stdout_worker.take();
        let stderr_worker = state.stderr_worker.take();
        state.stop_requested = None;
        drop(state);

        join_worker(stdout_worker);
        join_worker(stderr_worker);
        let mut playback = lock_playback(&self.playback);
        playback.playing = false;
        playback.ended = true;
    }
}

impl Drop for RunningPreviewProcess {
    fn drop(&mut self) {
        let _ = self.stop_running_process();
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
            let mut frame_index = 0_u64;
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
    fn clamp_seek_seconds_keeps_seek_inside_duration() {
        let seconds = clamp_seek_seconds(12.5, 12.5).expect("seconds");

        assert!((seconds - 12.499).abs() <= f64::EPSILON);
    }
}
