//! Final conversion uses the existing process runner. Only repaired ALAC output
//! is staged; publication has one controller-serialized cancellation cutoff.

use super::output;
use super::prepare::{self, Bindings, Preparation, PreparedRepair};
use super::process::Failure;
use super::staging::StagedOutput;
use crate::capabilities::runtime::Runtime;
use crate::conversion_runner::completion::CompletionFile;
use crate::conversion_runner::controller::ConversionProcessController;
use crate::conversion_runner::runner::{run_final_child, run_final_child_observed};
use frame_core::args::AlacCopyObserver;
use frame_core::error::ConversionError;
use frame_core::events::ConversionEvent;
use frame_core::types::ConversionTask;
use std::path::Path;
use std::process::{Command, Stdio};

pub(in crate::conversion_runner) fn run(
    task: &ConversionTask,
    output_path: &str,
    observer: &AlacCopyObserver,
    executable: &str,
    probe_executable: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let report = prepare::prepare(
        observer,
        Path::new(&task.file_path),
        executable,
        probe_executable,
        &|| controller.is_cancelled(&task.id),
        &mut |line| emit(ConversionEvent::log(task.id.clone(), line)),
    );
    diagnostics(&task.id, report.diagnostics, emit);
    let prepared = match report.result {
        Ok(prepared) => prepared,
        Err(failure) => return failed(task, controller, emit, &failure, false, Vec::new()),
    };
    // Cleanup diagnostics above may synchronously cancel or mutate a binding.
    match prepared {
        Preparation::Ordinary(bindings) => {
            ordinary(task, output_path, observer, &bindings, controller, emit)
        }
        Preparation::Repair(prepared) => repaired(task, output_path, &prepared, controller, emit),
    }
}

fn command(
    runtime: &Runtime,
    args: &[String],
    progress: &Path,
    stage: Option<&Path>,
) -> Result<Command, Failure> {
    // Use the resolved program from the observation, not a second PATH lookup.
    // std Command retains its platform batch-wrapper semantics on Windows.
    let mut command = Command::new(&runtime.command.program);
    command
        .args(&runtime.command.args)
        .envs(runtime.command.env.iter().cloned());
    #[cfg(unix)]
    if let Some(argv0) = &runtime.command.argv0 {
        use std::os::unix::process::CommandExt;
        command.arg0(argv0);
    }
    command.arg("-progress").arg(progress);
    if let Some(stage) = stage {
        if args.len() < 2 || args[args.len() - 2] != "-n" {
            return Err(Failure::invalid("changed prepared output sink"));
        }
        command.args(&args[..args.len() - 1]).arg(stage);
    } else {
        command.args(args);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    Ok(command)
}

fn log_command(task: &ConversionTask, command: &Command, emit: &mut impl FnMut(ConversionEvent)) {
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    emit(ConversionEvent::log(
        task.id.clone(),
        format!(
            "[INFO] Running {} {args}",
            command.get_program().to_string_lossy()
        ),
    ));
}

fn ordinary(
    task: &ConversionTask,
    output_path: &str,
    observer: &AlacCopyObserver,
    bindings: &Bindings,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let mut completion = match CompletionFile::create() {
        Ok(owner) => owner,
        Err(error) => {
            return failed(
                task,
                controller,
                emit,
                &Failure::invalid(&error.error),
                false,
                error.diagnostics,
            );
        }
    };
    let mut was_cancelled = false;
    let result = (|| {
        bindings.check(&|| controller.is_cancelled(&task.id))?;
        let args = observer.export_args(&[])?;
        let mut command = command(&bindings.ffmpeg, &args, completion.path(), None)?;
        log_command(task, &command, emit);
        bindings.check(&|| controller.is_cancelled(&task.id))?;
        was_cancelled = controller
            .take_cancelled(&task.id)
            .map_err(|error| Failure::invalid(&error))?;
        if was_cancelled {
            return Err(Failure::Cancelled);
        }
        run_final_child(&mut command, task, controller, emit, &mut was_cancelled)
            .map_err(|error| Failure::invalid(&error))?;
        completion
            .verify()
            .map_err(|error| Failure::invalid(&error))
    })();
    let cleanup = completion.cleanup();
    if let Err(failure) = result {
        failed(task, controller, emit, &failure, was_cancelled, cleanup)
    } else {
        diagnostics(&task.id, cleanup, emit);
        if consume_cancellation(task, controller, emit, was_cancelled)? {
            return Ok(());
        }
        emit(ConversionEvent::completed(task.id.clone(), output_path));
        Ok(())
    }
}

fn repaired(
    task: &ConversionTask,
    output_path: &str,
    prepared: &PreparedRepair,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<(), ConversionError> {
    let mut stage = match StagedOutput::create(Path::new(output_path)) {
        Ok(stage) => stage,
        Err(error) => {
            return failed(
                task,
                controller,
                emit,
                &Failure::invalid(&error.error),
                false,
                error.diagnostics,
            );
        }
    };
    let mut completion = match CompletionFile::create() {
        Ok(owner) => owner,
        Err(error) => {
            let mut cleanup = error.diagnostics;
            cleanup.extend(stage.cleanup());
            return failed(
                task,
                controller,
                emit,
                &Failure::invalid(&error.error),
                false,
                cleanup,
            );
        }
    };
    let mut lifecycle = ChildState::default();
    let child = staged_child(
        task,
        prepared,
        &stage,
        &mut completion,
        controller,
        emit,
        &mut lifecycle,
    );
    // Only a successfully spawned child can create an output we may adopt.
    // A no-spawn callback replacement remains unowned and cleanup leaves it.
    let sealed = if lifecycle.started {
        stage.seal().map_err(|error| Failure::invalid(&error))
    } else {
        Ok(())
    };
    let mut cleanup = completion.cleanup();
    let mut result = child.and(sealed);
    if result.is_ok() && !lifecycle.cancelled {
        let report = output::verify(
            prepared,
            &stage,
            &|| controller.is_cancelled(&task.id),
            &mut |line| emit(ConversionEvent::log(task.id.clone(), line)),
        );
        result = report.result;
        cleanup.extend(report.diagnostics);
    }
    if let Err(failure) = result {
        // Pending abandons only our unpublished owned file. The probe tree and
        // permit remain supervised; a denied unlink is reported, never hidden.
        cleanup.extend(stage.cleanup());
        return failed(
            task,
            controller,
            emit,
            &failure,
            lifecycle.cancelled,
            cleanup,
        );
    }
    diagnostics(&task.id, cleanup, emit);
    let ready = (|| {
        prepared
            .bindings
            .check(&|| controller.is_cancelled(&task.id))?;
        prepared.bindings.checkpoint(
            "output verified; preparing publication",
            &|| controller.is_cancelled(&task.id),
            &mut |line| emit(ConversionEvent::log(task.id.clone(), line)),
        )?;
        stage
            .ensure_unchanged()
            .map_err(|error| Failure::invalid(&error))
    })();
    if let Err(failure) = ready {
        return failed(
            task,
            controller,
            emit,
            &failure,
            lifecycle.cancelled,
            stage.cleanup(),
        );
    }
    publish(
        task,
        output_path,
        stage,
        controller,
        emit,
        lifecycle.cancelled,
    )
}

#[derive(Default)]
struct ChildState {
    started: bool,
    cancelled: bool,
}

fn staged_child(
    task: &ConversionTask,
    prepared: &PreparedRepair,
    stage: &StagedOutput,
    completion: &mut CompletionFile,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    lifecycle: &mut ChildState,
) -> Result<(), Failure> {
    prepared
        .bindings
        .check(&|| controller.is_cancelled(&task.id))?;
    let mut command = command(
        &prepared.bindings.ffmpeg,
        &prepared.args,
        completion.path(),
        Some(stage.path()),
    )?;
    log_command(task, &command, emit);
    prepared
        .bindings
        .check(&|| controller.is_cancelled(&task.id))?;
    stage
        .ensure_unwritten()
        .map_err(|error| Failure::invalid(&error))?;
    lifecycle.cancelled = controller
        .take_cancelled(&task.id)
        .map_err(|error| Failure::invalid(&error))?;
    if lifecycle.cancelled {
        return Err(Failure::Cancelled);
    }
    run_final_child_observed(
        &mut command,
        task,
        controller,
        emit,
        &mut lifecycle.cancelled,
        &mut lifecycle.started,
    )
    .map_err(|error| Failure::invalid(&error))?;
    if lifecycle.cancelled {
        return Err(Failure::Cancelled);
    }
    completion
        .verify()
        .map_err(|error| Failure::invalid(&error))
}

fn publish(
    task: &ConversionTask,
    output_path: &str,
    stage: StagedOutput,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    was_cancelled: bool,
) -> Result<(), ConversionError> {
    let cutoff = match controller.begin_publication(&task.id, was_cancelled) {
        Ok(Some(cutoff)) => cutoff,
        Ok(None) => {
            return failed(
                task,
                controller,
                emit,
                &Failure::Cancelled,
                true,
                stage.cleanup(),
            );
        }
        Err(error) => {
            return failed(
                task,
                controller,
                emit,
                &Failure::invalid(&error),
                was_cancelled,
                stage.cleanup(),
            );
        }
    };
    // No callback between the cutoff and its recorded success/failure decision.
    let result = stage.publish();
    let recorded = cutoff.finish();
    diagnostics(&task.id, stage.cleanup(), emit);
    recorded?;
    result.map_err(|error| {
        ConversionError::Worker(format!(
            "Could not save the verified export: {error}. Choose another output name or folder."
        ))
    })?;
    emit(ConversionEvent::completed(task.id.clone(), output_path));
    Ok(())
}

fn diagnostics(id: &str, diagnostics: Vec<String>, emit: &mut impl FnMut(ConversionEvent)) {
    for diagnostic in diagnostics {
        emit(ConversionEvent::log(
            id.to_owned(),
            format!("[WARN] ALAC verification cleanup: {diagnostic}"),
        ));
    }
}

fn failed(
    task: &ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    failure: &Failure,
    was_cancelled: bool,
    cleanup: Vec<String>,
) -> Result<(), ConversionError> {
    diagnostics(&task.id, cleanup, emit);
    if !matches!(failure, Failure::Cancelled) {
        emit(ConversionEvent::log(
            task.id.clone(),
            format!("[ERROR] ALAC verification: {}", failure.detail()),
        ));
    }
    if consume_cancellation(
        task,
        controller,
        emit,
        was_cancelled || matches!(failure, Failure::Cancelled),
    )? {
        return Ok(());
    }
    Err(failure.user_error())
}

fn consume_cancellation(
    task: &ConversionTask,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
    was_cancelled: bool,
) -> Result<bool, ConversionError> {
    if was_cancelled || controller.is_cancelled(&task.id) {
        emit(ConversionEvent::log(
            task.id.clone(),
            "[INFO] Task cancelled",
        ));
    }
    // Always consume, including a saved cancellation. No diagnostic callback may
    // follow this point before the terminal event/result.
    let late = controller.take_cancelled(&task.id)?;
    if was_cancelled || late {
        emit(ConversionEvent::cancelled(task.id.clone()));
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests;
