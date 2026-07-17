use std::{
    collections::VecDeque,
    io::Read,
    process::{Command, Stdio},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use frame_core::{
    args::{build_ffmpeg_args, build_output_path, validate_task_input},
    error::ConversionError,
    events::ConversionEvent,
    probe::{ffprobe_json_args, parse_ffprobe_stdout},
    types::{ConversionConfig as CoreConversionConfig, ConversionTask, ProbeMetadata},
    utils::{DURATION_REGEX, TIME_REGEX, parse_time},
};

use crate::runtime_binaries::{ffmpeg_executable, ffprobe_executable};

use super::{controller::ConversionProcessController, output_paths::disambiguate_output_paths};

/// Runs a single conversion task with a default process controller.
///
/// # Errors
///
/// Returns an error when task validation, probing, process spawning, log
/// reading, cancellation handling, or `FFmpeg` execution fails.
pub fn run_conversion_task(
    task: ConversionTask,
    mut emit: impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    run_conversion_task_with_control(task, &ConversionProcessController::default(), &mut emit)
}

/// Runs conversion tasks with shared process control and concurrency limits.
///
/// # Errors
///
/// Returns an error when the controller state cannot be read or the worker
/// channel disconnects before all tasks complete.
pub fn run_conversion_batch_with_control(
    mut tasks: Vec<ConversionTask>,
    controller: &ConversionProcessController,
    mut emit: impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    disambiguate_output_paths(&mut tasks);
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

/// Runs one conversion task with an explicit process controller.
///
/// # Errors
///
/// Returns an error when task validation, probing, process spawning, process
/// registration, log reading, cancellation handling, or `FFmpeg` execution fails.
pub fn run_conversion_task_with_control(
    mut task: ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    disambiguate_output_paths(std::slice::from_mut(&mut task));
    run_prepared_conversion_task_with_control(task, controller, emit)
}

fn run_prepared_conversion_task_with_control(
    task: ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    if controller.take_cancelled(&task.id)? {
        emit_cancelled_task(&task.id, emit);
        return Ok(());
    }

    validate_task_input(&task.file_path, &task.config)?;
    let probe = probe_media_file(&task.file_path)?;

    let output_path = build_output_path(
        &task.output_directory,
        &task.config.container,
        task.output_name.as_deref(),
    );
    let args = build_ffmpeg_args(&task.file_path, &output_path, &task.config, &probe)?;
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
        let result = run_prepared_conversion_task_with_control(task, &controller, &mut |event| {
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

pub(super) fn next_batch_launch_count(
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

pub(super) fn ffmpeg_progress_from_line(
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
