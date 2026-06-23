//! Native GPUI conversion runner backed by the shared Frame ffmpeg argument builder.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env,
    io::Read,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

use frame_core::{
    args::{
        build_ffmpeg_args, build_output_path, validate_stream_copy_compatibility,
        validate_task_input,
    },
    error::ConversionError,
    events::ConversionEvent,
    media_rules,
    probe::{ffprobe_json_args, parse_ffprobe_stdout},
    types::{
        ConversionConfig as CoreConversionConfig, ConversionTask, CropConfig,
        DEFAULT_MAX_CONCURRENCY, MetadataConfig, ProbeMetadata,
    },
    utils::{DURATION_REGEX, TIME_REGEX, parse_time},
};

use crate::{
    file_queue::FileItem,
    settings::{ConversionConfig as GpuiConversionConfig, CropSettings},
};

const DEFAULT_VIDEO_BITRATE: &str = "5000";
const DEFAULT_AUDIO_BITRATE: &str = "128";
const DEFAULT_AUDIO_QUALITY: &str = "4";
const DEFAULT_CRF: u8 = 23;
const DEFAULT_QUALITY: u32 = 50;
const DEFAULT_PRESET: &str = "medium";

#[derive(Clone, Debug, Default)]
pub struct ConversionProcessController {
    state: Arc<Mutex<ConversionProcessState>>,
}

#[derive(Debug)]
struct ConversionProcessState {
    active_processes: HashMap<String, ActiveConversionProcess>,
    cancelled_tasks: HashSet<String>,
    max_concurrency: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveConversionProcess {
    pid: u32,
}

impl Default for ConversionProcessState {
    fn default() -> Self {
        Self {
            active_processes: HashMap::new(),
            cancelled_tasks: HashSet::new(),
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
        }
    }
}

impl ConversionProcessController {
    pub fn update_max_concurrency(&self, value: usize) -> Result<(), ConversionError> {
        if value == 0 {
            return Err(ConversionError::InvalidInput(
                "Max concurrency must be at least 1".to_string(),
            ));
        }

        let mut state = self.lock_state()?;
        state.max_concurrency = value;
        Ok(())
    }

    pub fn current_max_concurrency(&self) -> Result<usize, ConversionError> {
        Ok(self.lock_state()?.max_concurrency.max(1))
    }

    #[must_use]
    pub fn active_pid(&self, id: &str) -> Option<u32> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.active_processes.get(id).map(|process| process.pid))
    }

    #[must_use]
    pub fn is_cancelled(&self, id: &str) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.cancelled_tasks.contains(id))
    }

    pub fn register_started_process(&self, id: &str, pid: u32) -> Result<bool, ConversionError> {
        let was_cancelled = {
            let mut state = self.lock_state()?;
            state
                .active_processes
                .insert(id.to_string(), ActiveConversionProcess { pid });
            state.cancelled_tasks.contains(id)
        };

        if was_cancelled && pid > 0 {
            terminate_process(pid)?;
        }

        Ok(was_cancelled)
    }

    pub fn finish_task(&self, id: &str) -> Result<bool, ConversionError> {
        let mut state = self.lock_state()?;
        state.active_processes.remove(id);
        Ok(state.cancelled_tasks.remove(id))
    }

    pub fn cancel_task(&self, id: &str) -> Result<(), ConversionError> {
        let pid = {
            let mut state = self.lock_state()?;
            state.cancelled_tasks.insert(id.to_string());
            state.active_processes.get(id).map(|process| process.pid)
        };

        if let Some(pid) = pid
            && pid > 0
        {
            terminate_process(pid)?;
        }

        Ok(())
    }

    pub fn pause_task(&self, id: &str) -> Result<(), ConversionError> {
        let pid = self
            .active_pid(id)
            .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;
        pause_process(pid)
    }

    pub fn resume_task(&self, id: &str) -> Result<(), ConversionError> {
        let pid = self
            .active_pid(id)
            .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;
        resume_process(pid)
    }

    pub fn take_cancelled(&self, id: &str) -> Result<bool, ConversionError> {
        let mut state = self.lock_state()?;
        Ok(state.cancelled_tasks.remove(id))
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, ConversionProcessState>, ConversionError> {
        self.state.lock().map_err(|error| {
            ConversionError::Worker(format!("process controller poisoned: {error}"))
        })
    }
}

#[must_use]
pub fn conversion_task_from_file(file: &FileItem) -> ConversionTask {
    let output_name = crate::settings::sanitize_output_name(&file.output_name);

    ConversionTask {
        id: file.id.clone(),
        file_path: file.path.clone(),
        output_name: (!output_name.is_empty()).then_some(output_name),
        config: core_config_from_gpui(&file.config),
    }
}

#[must_use]
pub fn core_config_from_gpui(config: &GpuiConversionConfig) -> CoreConversionConfig {
    CoreConversionConfig {
        processing_mode: config.processing_mode.id().to_string(),
        container: config.container.clone(),
        video_codec: default_video_codec_for_container(&config.container),
        video_bitrate_mode: "crf".to_string(),
        video_bitrate: DEFAULT_VIDEO_BITRATE.to_string(),
        audio_codec: config.audio_codec.clone(),
        audio_bitrate: DEFAULT_AUDIO_BITRATE.to_string(),
        audio_bitrate_mode: "bitrate".to_string(),
        audio_quality: DEFAULT_AUDIO_QUALITY.to_string(),
        audio_channels: "original".to_string(),
        audio_volume: 100.0,
        audio_normalize: false,
        selected_audio_tracks: config.selected_audio_tracks.clone(),
        selected_subtitle_tracks: config.selected_subtitle_tracks.clone(),
        subtitle_burn_path: None,
        subtitle_font_name: None,
        subtitle_font_size: None,
        subtitle_font_color: None,
        subtitle_outline_color: None,
        subtitle_position: None,
        resolution: "original".to_string(),
        custom_width: None,
        custom_height: None,
        scaling_algorithm: "bicubic".to_string(),
        fps: "original".to_string(),
        crf: DEFAULT_CRF,
        quality: DEFAULT_QUALITY,
        preset: DEFAULT_PRESET.to_string(),
        start_time: config.start_time.clone(),
        end_time: config.end_time.clone(),
        metadata: MetadataConfig::default(),
        rotation: config.rotation.clone(),
        flip_horizontal: config.flip_horizontal,
        flip_vertical: config.flip_vertical,
        ml_upscale: None,
        crop: config.crop.as_ref().map(core_crop_from_gpui),
        overlay: None,
        nvenc_spatial_aq: false,
        nvenc_temporal_aq: false,
        videotoolbox_allow_sw: false,
        hw_decode: false,
        pixel_format: "auto".to_string(),
        gif_colors: 256,
        gif_dither: "sierra2_4a".to_string(),
        gif_loop: 0,
    }
}

pub fn run_conversion_task(
    task: ConversionTask,
    mut emit: impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    run_conversion_task_with_control(task, &ConversionProcessController::default(), &mut emit)
}

pub fn run_conversion_batch_with_control(
    tasks: Vec<ConversionTask>,
    controller: ConversionProcessController,
    mut emit: impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let mut pending = VecDeque::from(tasks);
    let mut running_count = 0_usize;
    let (event_tx, event_rx) = mpsc::channel::<ConversionEvent>();
    let (done_tx, done_rx) = mpsc::channel::<(String, Result<(), ConversionError>)>();

    while !pending.is_empty() || running_count > 0 {
        let launch_count = next_batch_launch_count(
            pending.len(),
            running_count,
            controller.current_max_concurrency()?,
        );

        for _ in 0..launch_count {
            let Some(task) = pending.pop_front() else {
                break;
            };
            running_count += 1;
            spawn_batch_worker(task, controller.clone(), event_tx.clone(), done_tx.clone());
        }

        drain_batch_events(&event_rx, &mut emit);
        if running_count == 0 {
            continue;
        }

        match done_rx.recv_timeout(Duration::from_millis(50)) {
            Ok((task_id, result)) => {
                running_count = running_count.saturating_sub(1);
                drain_batch_events(&event_rx, &mut emit);
                if let Err(error) = result {
                    emit(ConversionEvent::error(task_id, error.to_string()));
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(ConversionError::Channel(
                    "conversion batch worker channel disconnected".to_string(),
                ));
            }
        }
    }

    drain_batch_events(&event_rx, &mut emit);
    Ok(())
}

pub fn run_conversion_task_with_control(
    task: ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    if controller.take_cancelled(&task.id)? {
        emit_cancelled_task(&task.id, emit);
        return Ok(());
    }

    validate_task_input(&task.file_path, &task.config)?;
    if task.config.processing_mode == "copy" {
        let probe = probe_media_file(&task.file_path)?;
        validate_stream_copy_compatibility(&task.config, &probe)?;
    }

    let output_path = build_output_path(
        &task.file_path,
        &task.config.container,
        task.output_name.as_deref(),
    );
    let args = build_ffmpeg_args(&task.file_path, &output_path, &task.config);
    let executable = ffmpeg_executable();

    emit(ConversionEvent::log(
        task.id.clone(),
        format!("[INFO] Running {executable} {}", args.join(" ")),
    ));

    let mut child = Command::new(&executable)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(ConversionError::Io)?;

    let started_cancelled = controller.register_started_process(&task.id, child.id())?;
    if started_cancelled {
        let _ = child.wait();
        let _ = controller.finish_task(&task.id);
        emit_cancelled_task(&task.id, emit);
        return Ok(());
    }

    emit(ConversionEvent::started(task.id.clone()));
    emit(ConversionEvent::progress(task.id.clone(), 0.0));

    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| ConversionError::Worker("ffmpeg stderr was not captured".to_string()))?;
    let stream_result = stream_ffmpeg_stderr(&mut stderr, &task, emit);

    let status = child.wait().map_err(ConversionError::Io);
    let was_cancelled = controller.finish_task(&task.id)?;
    if was_cancelled {
        emit_cancelled_task(&task.id, emit);
        return Ok(());
    }

    stream_result?;
    let status = status?;
    if status.success() {
        emit(ConversionEvent::completed(task.id, output_path));
        Ok(())
    } else {
        Err(ConversionError::Worker(format!(
            "ffmpeg exited with status {status}"
        )))
    }
}

fn spawn_batch_worker(
    task: ConversionTask,
    controller: ConversionProcessController,
    event_tx: mpsc::Sender<ConversionEvent>,
    done_tx: mpsc::Sender<(String, Result<(), ConversionError>)>,
) {
    let task_id = task.id.clone();
    thread::spawn(move || {
        let result = run_conversion_task_with_control(task, &controller, &mut |event| {
            let _ = event_tx.send(event);
        });
        let _ = done_tx.send((task_id, result));
    });
}

fn drain_batch_events(
    event_rx: &mpsc::Receiver<ConversionEvent>,
    emit: &mut impl FnMut(ConversionEvent),
) {
    while let Ok(event) = event_rx.try_recv() {
        emit(event);
    }
}

fn next_batch_launch_count(
    pending_count: usize,
    running_count: usize,
    max_concurrency: usize,
) -> usize {
    let available_slots = max_concurrency.max(1).saturating_sub(running_count);
    pending_count.min(available_slots)
}

fn emit_cancelled_task(id: &str, emit: &mut impl FnMut(ConversionEvent)) {
    emit(ConversionEvent::log(
        id.to_string(),
        "[INFO] Task cancelled",
    ));
    emit(ConversionEvent::cancelled(id.to_string()));
}

fn ffmpeg_executable() -> String {
    env::var("FRAME_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string())
}

fn ffprobe_executable() -> String {
    env::var("FRAME_FFPROBE_PATH").unwrap_or_else(|_| "ffprobe".to_string())
}

fn probe_media_file(file_path: &str) -> Result<ProbeMetadata, ConversionError> {
    let output = Command::new(ffprobe_executable())
        .args(ffprobe_json_args(file_path))
        .output()
        .map_err(ConversionError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = if stderr.trim().is_empty() {
            format!("ffprobe exited with status {}", output.status)
        } else {
            stderr.trim().to_string()
        };
        return Err(ConversionError::Probe(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_stdout(file_path, stdout)
}

fn stream_ffmpeg_stderr(
    stderr: &mut impl Read,
    task: &ConversionTask,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let mut buffer = [0_u8; 4096];
    let mut pending = String::new();
    let mut total_duration = None;
    let expected_duration = expected_duration_seconds(&task.config);

    loop {
        let read = stderr.read(&mut buffer).map_err(ConversionError::Io)?;
        if read == 0 {
            break;
        }

        pending.push_str(&String::from_utf8_lossy(&buffer[..read]));
        drain_ffmpeg_segments(
            &mut pending,
            task,
            expected_duration,
            &mut total_duration,
            emit,
        );
    }

    if !pending.trim().is_empty() {
        handle_ffmpeg_line(
            pending.trim(),
            task,
            expected_duration,
            &mut total_duration,
            emit,
        );
    }

    Ok(())
}

fn drain_ffmpeg_segments(
    pending: &mut String,
    task: &ConversionTask,
    expected_duration: f64,
    total_duration: &mut Option<f64>,
    emit: &mut impl FnMut(ConversionEvent),
) {
    while let Some(separator_index) = pending.find(['\r', '\n']) {
        let segment = pending[..separator_index].trim().to_string();
        pending.drain(..=separator_index);
        if !segment.is_empty() {
            handle_ffmpeg_line(&segment, task, expected_duration, total_duration, emit);
        }
    }
}

fn handle_ffmpeg_line(
    line: &str,
    task: &ConversionTask,
    expected_duration: f64,
    total_duration: &mut Option<f64>,
    emit: &mut impl FnMut(ConversionEvent),
) {
    emit(ConversionEvent::log(task.id.clone(), line));
    if let Some(progress) = ffmpeg_progress_from_line(line, expected_duration, total_duration) {
        emit(ConversionEvent::progress(task.id.clone(), progress));
    }
}

fn expected_duration_seconds(config: &CoreConversionConfig) -> f64 {
    let start = config
        .start_time
        .as_deref()
        .and_then(parse_time)
        .unwrap_or(0.0);
    let Some(end) = config.end_time.as_deref().and_then(parse_time) else {
        return 0.0;
    };

    (end - start).max(0.0)
}

fn ffmpeg_progress_from_line(
    line: &str,
    expected_duration: f64,
    total_duration: &mut Option<f64>,
) -> Option<f64> {
    if let Some(caps) = DURATION_REGEX.captures(line)
        && let Some(duration) = caps.get(1).and_then(|m| parse_time(m.as_str()))
    {
        *total_duration = Some(duration);
    }

    let current_time = TIME_REGEX
        .captures(line)
        .and_then(|caps| caps.get(1))
        .and_then(|m| parse_time(m.as_str()))?;
    let duration = if expected_duration > 0.0 {
        expected_duration
    } else {
        total_duration.unwrap_or(0.0)
    };

    (duration > 0.0).then(|| (current_time / duration * 100.0).clamp(0.0, 100.0))
}

fn default_video_codec_for_container(container: &str) -> String {
    if media_rules::is_gif_container(container) {
        return "gif".to_string();
    }

    media_rules::video_codec_fallback_order()
        .iter()
        .find(|codec| media_rules::is_video_codec_allowed(container, codec))
        .cloned()
        .unwrap_or_else(|| "libx264".to_string())
}

fn core_crop_from_gpui(crop: &CropSettings) -> CropConfig {
    CropConfig {
        enabled: crop.enabled,
        x: f64::from(crop.x),
        y: f64::from(crop.y),
        width: f64::from(crop.width),
        height: f64::from(crop.height),
        source_width: crop.source_width.map(f64::from),
        source_height: crop.source_height.map(f64::from),
        aspect_ratio: crop.aspect_ratio.clone(),
    }
}

#[cfg(unix)]
fn pause_process(pid: u32) -> Result<(), ConversionError> {
    signal_process(pid, libc::SIGSTOP, "SIGSTOP")
}

#[cfg(not(unix))]
fn pause_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Pausing conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
fn resume_process(pid: u32) -> Result<(), ConversionError> {
    signal_process(pid, libc::SIGCONT, "SIGCONT")
}

#[cfg(not(unix))]
fn resume_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Resuming conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
fn terminate_process(pid: u32) -> Result<(), ConversionError> {
    let unix_pid = pid_to_unix_pid(pid)?;
    unsafe {
        let _ = libc::kill(unix_pid, libc::SIGCONT);
        if libc::kill(unix_pid, libc::SIGKILL) != 0 {
            return Err(ConversionError::Shell("Failed to send SIGKILL".to_string()));
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn terminate_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Cancelling running conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
fn signal_process(pid: u32, signal: libc::c_int, label: &str) -> Result<(), ConversionError> {
    let unix_pid = pid_to_unix_pid(pid)?;
    unsafe {
        if libc::kill(unix_pid, signal) != 0 {
            return Err(ConversionError::Shell(format!("Failed to send {label}")));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn pid_to_unix_pid(pid: u32) -> Result<libc::pid_t, ConversionError> {
    libc::pid_t::try_from(pid)
        .map_err(|_| ConversionError::Shell(format!("PID {pid} is out of range for libc::pid_t")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{CropSettings, ProcessingMode};

    #[test]
    fn core_config_from_gpui_preserves_active_conversion_fields() {
        let config = GpuiConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "mov".to_string(),
            audio_codec: "aac".to_string(),
            start_time: Some("00:00:05.000".to_string()),
            end_time: Some("00:00:15.000".to_string()),
            rotation: "90".to_string(),
            flip_horizontal: true,
            flip_vertical: true,
            crop: Some(CropSettings {
                enabled: true,
                x: 10,
                y: 20,
                width: 300,
                height: 200,
                source_width: Some(1920),
                source_height: Some(1080),
                aspect_ratio: Some("16:9".to_string()),
            }),
            selected_audio_tracks: vec![1, 2],
            selected_subtitle_tracks: vec![3],
        };

        let core = core_config_from_gpui(&config);

        assert_eq!(core.processing_mode, "copy");
        assert_eq!(core.container, "mov");
        assert_eq!(core.start_time.as_deref(), Some("00:00:05.000"));
        assert_eq!(core.end_time.as_deref(), Some("00:00:15.000"));
        assert_eq!(core.rotation, "90");
        assert!(core.flip_horizontal);
        assert!(core.flip_vertical);
        assert_eq!(core.selected_audio_tracks, [1, 2]);
        assert_eq!(core.selected_subtitle_tracks, [3]);
        assert_eq!(core.crop.as_ref().map(|crop| crop.width), Some(300.0));
    }

    #[test]
    fn conversion_task_from_file_sanitizes_output_name() {
        let mut file = FileItem::from_path("file-1", "/tmp/source.mov", 1);
        file.output_name = "/tmp/export/final cut.mp4".to_string();

        let task = conversion_task_from_file(&file);

        assert_eq!(task.output_name.as_deref(), Some("final cut.mp4"));
        assert_eq!(task.file_path, "/tmp/source.mov");
    }

    #[test]
    fn ffmpeg_progress_uses_duration_line_before_time_line() {
        let mut duration = None;

        assert_eq!(
            ffmpeg_progress_from_line("Duration: 00:00:20.00, start: 0.000000", 0.0, &mut duration),
            None
        );

        let progress =
            ffmpeg_progress_from_line("frame=12 time=00:00:05.00 speed=1x", 0.0, &mut duration);

        assert_eq!(progress, Some(25.0));
    }

    #[test]
    fn ffmpeg_progress_prefers_trim_expected_duration() {
        let mut duration = Some(100.0);

        let progress =
            ffmpeg_progress_from_line("frame=12 time=00:00:05.00 speed=1x", 10.0, &mut duration);

        assert_eq!(progress, Some(50.0));
    }

    #[test]
    fn controller_tracks_registered_process_pid() {
        let controller = ConversionProcessController::default();

        controller
            .register_started_process("task-1", 0)
            .expect("pid registration should succeed");

        assert_eq!(controller.active_pid("task-1"), Some(0));
    }

    #[test]
    fn controller_uses_shared_default_max_concurrency() {
        let controller = ConversionProcessController::default();

        assert_eq!(
            controller
                .current_max_concurrency()
                .expect("default max concurrency should be readable"),
            DEFAULT_MAX_CONCURRENCY
        );
    }

    #[test]
    fn controller_update_max_concurrency_rejects_zero() {
        let controller = ConversionProcessController::default();

        let error = controller
            .update_max_concurrency(0)
            .expect_err("zero concurrency should be rejected");

        assert!(error.to_string().contains("at least 1"));
    }

    #[test]
    fn controller_update_max_concurrency_stores_live_limit() {
        let controller = ConversionProcessController::default();

        controller
            .update_max_concurrency(4)
            .expect("valid max concurrency should be stored");

        assert_eq!(
            controller
                .current_max_concurrency()
                .expect("max concurrency should be readable"),
            4
        );
    }

    #[test]
    fn controller_finish_task_reports_cancelled_state() {
        let controller = ConversionProcessController::default();
        controller
            .register_started_process("task-1", 0)
            .expect("pid registration should succeed");
        controller
            .cancel_task("task-1")
            .expect("cancelling pid zero should not signal an OS process");

        let was_cancelled = controller
            .finish_task("task-1")
            .expect("finishing task should succeed");

        assert!(was_cancelled);
    }

    #[test]
    fn controller_register_started_process_reports_pre_cancelled_task() {
        let controller = ConversionProcessController::default();
        controller
            .cancel_task("task-1")
            .expect("pre-cancel should succeed without an active process");

        let was_cancelled = controller
            .register_started_process("task-1", 0)
            .expect("pid registration should succeed");

        assert!(was_cancelled);
        assert!(
            controller
                .finish_task("task-1")
                .expect("finishing task should clean process state")
        );
        assert_eq!(controller.active_pid("task-1"), None);
    }

    #[test]
    fn run_conversion_task_with_control_emits_cancelled_when_cancelled_before_validation() {
        let controller = ConversionProcessController::default();
        controller
            .cancel_task("task-1")
            .expect("pre-cancel should succeed without an active process");
        let task = ConversionTask {
            id: "task-1".to_string(),
            file_path: "/definitely/missing.mov".to_string(),
            output_name: None,
            config: core_config_from_gpui(&GpuiConversionConfig::default()),
        };
        let mut events = Vec::new();

        let result = run_conversion_task_with_control(task, &controller, &mut |event| {
            events.push(event);
        });

        assert!(result.is_ok());
        assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
    }

    #[test]
    fn run_conversion_batch_with_control_accepts_empty_batches() {
        let controller = ConversionProcessController::default();
        let mut events = Vec::new();

        let result = run_conversion_batch_with_control(Vec::new(), controller, |event| {
            events.push(event);
        });

        assert!(result.is_ok());
        assert!(events.is_empty());
    }

    #[test]
    fn next_batch_launch_count_respects_live_concurrency_limit() {
        assert_eq!(next_batch_launch_count(5, 1, 2), 1);
        assert_eq!(next_batch_launch_count(5, 2, 2), 0);
        assert_eq!(next_batch_launch_count(1, 0, 4), 1);
    }
}
