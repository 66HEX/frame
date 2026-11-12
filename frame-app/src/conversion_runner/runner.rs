use super::completion::CompletionFile;
use super::controller::ConversionProcessController;
use super::output_paths::disambiguate_output_paths;
use super::{PublicationDecision, StagedOutput, publish_staged_output};
use crate::capabilities::{HardwareEncoder, HardwareProbePolicy, probe_hardware_encoder};
use crate::runtime_binaries::{ffmpeg_executable, ffprobe_executable};
use frame_core::args::{
    GifFramePolicy, build_alac_copy_observer, build_ffmpeg_args, build_ffmpeg_args_with_gif_policy,
    build_output_path, effective_video_encoder, validate_task_input,
};
use frame_core::error::ConversionError;
use frame_core::events::ConversionEvent;
use frame_core::types::{ConversionConfig as CoreConversionConfig, ConversionTask, ProbeMetadata};
use frame_core::utils::{DURATION_REGEX, TIME_REGEX, parse_time};
use std::collections::VecDeque;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;

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
    controller
        .begin_queued_attempts(&tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>())?;
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
    controller.begin_attempt(&task.id)?;
    disambiguate_output_paths(std::slice::from_mut(&mut task));
    run_prepared_conversion_task_with_control(&task, controller, emit)
}

fn run_prepared_conversion_task_with_control(
    task: &ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    controller.begin_attempt(&task.id)?;
    if controller.take_cancelled(&task.id)? {
        emit_cancelled_task(&task.id, emit);
        return Ok(());
    }

    validate_task_input(&task.file_path, &task.config)?;
    let Some(probe) = probe_conversion_source(
        &task.file_path,
        &ffprobe_executable(),
        &task.id,
        controller,
        emit,
    )?
    else {
        return Ok(());
    };

    let output_path = build_output_path(
        &task.output_directory,
        &task.config.container,
        task.output_name.as_deref(),
    );
    let ordinary_args = build_ffmpeg_args(&task.file_path, &output_path, &task.config, &probe)?;
    let executable = ffmpeg_executable();
    if let Some(observer) =
        build_alac_copy_observer(&task.file_path, &output_path, &task.config, &probe)?
    {
        // This scoped branch is full Copy to MOV/MP4/M4A; GIF and hardware
        // encoding checks are dormant, but selectable subtitle BSFs still apply.
        if !super::subtitles::check_subtitle_trim_with_control(
            &task.config,
            &executable,
            &task.id,
            controller,
            emit,
        )? {
            return Ok(());
        }
        return super::alac::run(
            task,
            &output_path,
            &observer,
            &executable,
            &ffprobe_executable(),
            controller,
            emit,
        );
    }
    let Some(prepared) = prepare_gif_export(
        task,
        &output_path,
        &probe,
        &executable,
        controller,
        emit,
        &ordinary_args,
    )?
    else {
        return Ok(());
    };

    if !super::subtitles::check_subtitle_trim_with_control(
        &task.config,
        &executable,
        &task.id,
        controller,
        emit,
    )? {
        return Ok(());
    }

    if !check_hardware_encoder(task, &probe, &executable, controller, emit)? {
        return Ok(());
    }

    run_final_conversion(
        task,
        &output_path,
        &executable,
        ordinary_args,
        prepared,
        controller,
        emit,
    )
}

struct FinalOutcome {
    result: Result<(), ConversionError>,
    was_cancelled: bool,
    diagnostics: Vec<String>,
}

fn run_final_conversion(
    task: &ConversionTask,
    output_path: &str,
    executable: &str,
    ordinary_args: Vec<String>,
    prepared: PreparedGifExport,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let publication =
        OutputPublication::for_environment(output_path, crate::runtime_environment::is_flatpak());
    run_final_conversion_with_publication(
        task,
        publication,
        executable,
        ordinary_args,
        prepared,
        controller,
        emit,
    )
}

#[derive(Clone, Copy)]
enum OutputPublication<'a> {
    Direct(&'a str),
    Staged(&'a str),
}

impl<'a> OutputPublication<'a> {
    const fn for_environment(path: &'a str, flatpak: bool) -> Self {
        if flatpak {
            Self::Direct(path)
        } else {
            Self::Staged(path)
        }
    }

    const fn path(self) -> &'a str {
        match self {
            Self::Direct(path) | Self::Staged(path) => path,
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Keep completion, staged file, child ownership and cleanup in one ordered lifecycle."
)]
fn run_final_conversion_with_publication(
    task: &ConversionTask,
    publication: OutputPublication<'_>,
    executable: &str,
    ordinary_args: Vec<String>,
    prepared: PreparedGifExport,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let output_path = publication.path();
    let mut completion = match CompletionFile::create() {
        Ok(owner) => owner,
        Err(error) => {
            return finish_final_conversion(
                task,
                output_path,
                controller,
                emit,
                FinalOutcome {
                    result: Err(ConversionError::Worker(format!(
                        "Could not prepare the export completion check: {}",
                        error.error
                    ))),
                    was_cancelled: false,
                    diagnostics: error.diagnostics,
                },
            );
        }
    };
    let mut stage = if matches!(publication, OutputPublication::Staged(_)) {
        match StagedOutput::create(std::path::Path::new(output_path)) {
            Ok(stage) => Some(stage),
            Err(error) => {
                let mut diagnostics = error.diagnostics;
                diagnostics.extend(completion.cleanup());
                return finish_final_conversion(
                    task,
                    output_path,
                    controller,
                    emit,
                    FinalOutcome {
                        result: Err(ConversionError::Worker(format!(
                            "Could not prepare a private export: {}",
                            error.error
                        ))),
                        was_cancelled: false,
                        diagnostics,
                    },
                );
            }
        }
    } else {
        None
    };
    let PreparedGifExport {
        mut args,
        proof: recovery_proof,
    } = prepared;
    let mut was_cancelled = false;
    let mut started = false;
    let result = (|| {
        emit(ConversionEvent::log(
            task.id.clone(),
            format!(
                "[INFO] Running {executable} -progress {} {}",
                completion.path().display(),
                logged_export_args(&args, stage.as_ref())
            ),
        ));
        // The log callback may replace source/runtime. Only the base media args
        // change; the OS-path transport is attached to the final Command below.
        if recovery_proof
            .as_ref()
            .is_some_and(|proof| !proof.unchanged())
        {
            args = ordinary_args;
            emit(ConversionEvent::log(
                task.id.clone(),
                format!(
                    "[INFO] GIF proof changed before launch; using ordinary export: {executable} -progress {} {}",
                    completion.path().display(),
                    logged_export_args(&args, stage.as_ref())
                ),
            ));
        }
        was_cancelled = controller.take_cancelled(&task.id)?;
        if was_cancelled {
            return Ok(());
        }
        let mut command = if let Some(stage) = &stage {
            stage.ensure_unwritten().map_err(ConversionError::Io)?;
            if args.last().map(String::as_str) != Some(output_path) {
                return Err(ConversionError::Worker(
                    "Export arguments do not identify the expected output file.".into(),
                ));
            }
            final_command_to_path(executable, &args, completion.path(), stage.path())
        } else {
            final_command(executable, &args, completion.path())
        };
        run_final_child_observed(
            &mut command,
            task,
            controller,
            emit,
            &mut was_cancelled,
            &mut started,
        )?;
        completion.verify().map_err(|reason| {
            ConversionError::Worker(format!(
                "FFmpeg did not confirm that the export finished: {reason}. Check the conversion log."
            ))
        })
    })();
    let diagnostics = completion.cleanup();
    if let Some(mut stage) = stage.take() {
        let sealed = if started {
            stage.seal().map_err(ConversionError::Io)
        } else {
            Ok(())
        };
        return finish_staged_conversion(
            task,
            output_path,
            stage,
            controller,
            emit,
            FinalOutcome {
                result: result.and(sealed),
                was_cancelled,
                diagnostics,
            },
        );
    }
    finish_final_conversion(
        task,
        output_path,
        controller,
        emit,
        FinalOutcome {
            result,
            was_cancelled,
            diagnostics,
        },
    )
}

fn logged_export_args(args: &[String], stage: Option<&StagedOutput>) -> String {
    stage.map_or_else(
        || args.join(" "),
        |stage| {
            format!(
                "{} {}",
                args[..args.len().saturating_sub(1)].join(" "),
                stage.path().display()
            )
        },
    )
}

fn final_command_to_path(
    executable: &str,
    args: &[String],
    progress_path: &std::path::Path,
    output_path: &std::path::Path,
) -> Command {
    let mut command = final_command(
        executable,
        &args[..args.len().saturating_sub(1)],
        progress_path,
    );
    command.arg(output_path);
    command
}

fn finish_staged_conversion(
    task: &ConversionTask,
    output_path: &str,
    stage: StagedOutput,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    mut outcome: FinalOutcome,
) -> Result<(), ConversionError> {
    if outcome.result.is_err() || outcome.was_cancelled {
        outcome.diagnostics.extend(stage.cleanup());
        return finish_final_conversion(task, output_path, controller, emit, outcome);
    }
    for diagnostic in outcome.diagnostics {
        emit(ConversionEvent::log(
            task.id.clone(),
            format!("[WARN] Export cleanup: {diagnostic}"),
        ));
    }
    emit(ConversionEvent::log(
        task.id.clone(),
        "[INFO] Export verified; preparing publication",
    ));
    let publication = publish_staged_output(stage, &task.id, controller);
    match publication.result {
        Ok(PublicationDecision::Published) => {}
        result => {
            return finish_final_conversion(
                task,
                output_path,
                controller,
                emit,
                FinalOutcome {
                    was_cancelled: matches!(result, Ok(PublicationDecision::Cancelled)),
                    result: result.map(|_| ()),
                    diagnostics: publication.diagnostics,
                },
            );
        }
    }
    for diagnostic in publication.diagnostics {
        emit(ConversionEvent::log(
            task.id.clone(),
            format!("[WARN] Export cleanup: {diagnostic}"),
        ));
    }
    emit(ConversionEvent::completed(task.id.clone(), output_path));
    Ok(())
}

fn final_command(executable: &str, args: &[String], progress_path: &std::path::Path) -> Command {
    let mut command = Command::new(executable);
    command
        .arg("-progress")
        .arg(progress_path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

pub(super) fn run_final_child(
    command: &mut Command,
    task: &ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    was_cancelled: &mut bool,
) -> Result<(), ConversionError> {
    let mut started = false;
    run_final_child_observed(command, task, controller, emit, was_cancelled, &mut started)
}

pub(super) fn run_final_child_observed(
    command: &mut Command,
    task: &ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    was_cancelled: &mut bool,
    started: &mut bool,
) -> Result<(), ConversionError> {
    let mut child = crate::process_launch::spawn(command).map_err(ConversionError::Io)?;
    *started = true;
    let started_cancelled = match controller.register_started_process(&task.id, child.id()) {
        Ok(cancelled) => cancelled,
        Err(error) => {
            // Keep the file owner alive until this already-started child settles,
            // even if process registration itself fails. Retain the primary error.
            let _ = child.kill();
            let _ = child.wait();
            if let Ok(cancelled) = controller.finish_task(&task.id) {
                *was_cancelled |= cancelled;
            }
            return Err(error);
        }
    };
    *was_cancelled |= started_cancelled;
    let stream_result = if started_cancelled {
        Ok(())
    } else {
        emit(ConversionEvent::started(task.id.clone()));
        emit(ConversionEvent::progress(task.id.clone(), 0.0));
        child.stderr.take().map_or_else(
            || {
                Err(ConversionError::Worker(
                    "ffmpeg stderr was not captured".to_string(),
                ))
            },
            |mut stderr| stream_ffmpeg_stderr(&mut stderr, task, emit),
        )
    };
    let status = child.wait().map_err(ConversionError::Io);
    // Unregister before proof IO, cleanup or any further callback: the PID must
    // not remain actionable after wait, even if subsequent verification fails.
    let finished = controller.finish_task(&task.id);
    *was_cancelled |= finished?;
    stream_result?;
    let status = status?;
    if status.success() {
        Ok(())
    } else {
        Err(ConversionError::Worker(format!(
            "ffmpeg exited with status {status}"
        )))
    }
}

fn finish_final_conversion(
    task: &ConversionTask,
    output_path: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    outcome: FinalOutcome,
) -> Result<(), ConversionError> {
    for diagnostic in outcome.diagnostics {
        emit(ConversionEvent::log(
            task.id.clone(),
            format!("[WARN] Completion cleanup: {diagnostic}"),
        ));
    }
    if outcome.was_cancelled || controller.is_cancelled(&task.id) {
        emit(ConversionEvent::log(
            task.id.clone(),
            "[INFO] Task cancelled",
        ));
    }
    // Always consume again, including when finish_task already consumed a marker.
    // Diagnostics can synchronously cancel. No callback follows this consume
    // before the terminal event/result (emit_cancelled_task would add such a log).
    let late_cancelled = controller.take_cancelled(&task.id)?;
    if outcome.was_cancelled || late_cancelled {
        emit(ConversionEvent::cancelled(task.id.clone()));
        return Ok(());
    }
    outcome.result?;
    emit(ConversionEvent::completed(task.id.clone(), output_path));
    Ok(())
}

fn check_hardware_encoder(
    task: &ConversionTask,
    probe: &ProbeMetadata,
    executable: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<bool, ConversionError> {
    if let Some(encoder) =
        effective_video_encoder(&task.config, probe).and_then(HardwareEncoder::from_name)
    {
        let policy = HardwareProbePolicy::new(encoder, task.config.videotoolbox_allow_sw);
        let report =
            probe_hardware_encoder(executable, policy, || controller.is_cancelled(&task.id));
        if controller.take_cancelled(&task.id)? {
            emit_cancelled_task(&task.id, emit);
            return Ok(false);
        }
        if !report.available() {
            if !report.diagnostics.is_empty() {
                emit(ConversionEvent::log(
                    task.id.clone(),
                    format!("[ERROR] Encoder check: {}", report.diagnostics),
                ));
            }
            return Err(ConversionError::InvalidInput(format!(
                "Cannot use {}: {}. Check encoder availability again or choose another encoder.",
                encoder.name(),
                report
                    .reason
                    .map_or("encoder is unavailable", |reason| reason.caption())
            )));
        }
    }
    Ok(true)
}

struct PreparedGifExport {
    args: Vec<String>,
    proof: Option<Box<super::gif::GifProof>>,
}

fn prepare_gif_export(
    task: &ConversionTask,
    output_path: &str,
    probe: &ProbeMetadata,
    executable: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    ordinary_args: &[String],
) -> Result<Option<PreparedGifExport>, ConversionError> {
    let check = super::gif::check_gif_minimum_frame(
        &task.file_path,
        &task.config,
        probe,
        executable,
        || controller.is_cancelled(&task.id),
    );
    // The callback is non-consuming until the owned helper has returned.
    if controller.take_cancelled(&task.id)?
        || matches!(check.decision, super::gif::GifDecision::Cancelled)
    {
        emit_cancelled_task(&task.id, emit);
        return Ok(None);
    }
    for diagnostic in check.diagnostics {
        emit(ConversionEvent::log(
            task.id.clone(),
            format!("[INFO] {diagnostic}"),
        ));
    }
    // Logging callbacks may cancel the task after the helper has settled.
    if controller.take_cancelled(&task.id)? {
        emit_cancelled_task(&task.id, emit);
        return Ok(None);
    }
    let mut args = ordinary_args.to_vec();
    let proof = match check.decision {
        super::gif::GifDecision::Recover(proof) => {
            match build_ffmpeg_args_with_gif_policy(
                &task.file_path,
                output_path,
                &task.config,
                probe,
                GifFramePolicy::ConfirmedMinimumFrame,
            ) {
                Ok(recovery) => {
                    args = recovery;
                    Some(proof)
                }
                Err(error) => {
                    emit(ConversionEvent::log(
                        task.id.clone(),
                        format!(
                            "[INFO] Could not build GIF recovery: {error}; using ordinary export."
                        ),
                    ));
                    None
                }
            }
        }
        super::gif::GifDecision::Empty => {
            return Err(ConversionError::InvalidInput(
                "No video frames were found in the selected interval. Adjust the trim range."
                    .into(),
            ));
        }
        super::gif::GifDecision::CleanupPending => {
            return Err(ConversionError::Worker(
                "The GIF frame check could not finish cleaning up. Try again before exporting."
                    .into(),
            ));
        }
        super::gif::GifDecision::Ordinary | super::gif::GifDecision::Cancelled => None,
    };
    Ok(Some(PreparedGifExport { args, proof }))
}

fn spawn_batch_worker(
    task: ConversionTask,
    controller: ConversionProcessController,
    event_tx: mpsc::Sender<ConversionEvent>,
    done_tx: mpsc::Sender<(String, Result<(), ConversionError>)>,
) {
    let task_id = task.id.clone();
    thread::spawn(move || {
        let result = run_prepared_conversion_task_with_control(&task, &controller, &mut |event| {
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

pub(super) fn emit_cancelled_task(id: &str, emit: &mut impl FnMut(ConversionEvent)) {
    emit(ConversionEvent::log(
        id.to_string(),
        "[INFO] Task cancelled",
    ));
    emit(ConversionEvent::cancelled(id.to_string()));
}

fn probe_conversion_source(
    file_path: &str,
    executable: &str,
    id: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<Option<ProbeMetadata>, ConversionError> {
    let result = crate::source_probe::probe(executable, std::path::Path::new(file_path), || {
        controller.is_cancelled(id)
    });
    if controller.take_cancelled(id)? {
        emit(ConversionEvent::log(id, "[INFO] Task cancelled"));
        // A diagnostic callback may repeat cancellation before the terminal event.
        controller.take_cancelled(id)?;
        emit(ConversionEvent::cancelled(id));
        return Ok(None);
    }
    result.map(Some)
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

pub(super) fn expected_duration_seconds(config: &CoreConversionConfig) -> f64 {
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

#[cfg(test)]
mod completion_tests;
#[cfg(all(test, unix))]
mod probe_tests;
