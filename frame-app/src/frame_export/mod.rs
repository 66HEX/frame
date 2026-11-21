use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits, StopReason};
use crate::conversion_runner::ConversionProcessController;
use frame_core::error::ConversionError;
use frame_core::frame_export::{SourceFrameOptions, build_source_frame_args};
use std::path::PathBuf;
use std::time::Duration;

mod publication;
mod request;
mod source_identity;
mod validation;
pub use request::SourceFrameExportRequest;

const ENCODED_LIMIT: usize = 256 * 1024 * 1024;
const DECODED_LIMIT: u64 = 512 * 1024 * 1024;
static EXPORTS: ExecutionDomain = ExecutionDomain::new();

#[derive(Debug, Eq, PartialEq)]
pub enum SourceFrameExportOutcome {
    Published { path: PathBuf },
    Cancelled,
}

fn check_environment(flatpak: bool) -> Result<(), ConversionError> {
    if flatpak {
        return Err(ConversionError::InvalidInput(
            "Source frame export is unavailable in this installation".into(),
        ));
    }
    Ok(())
}

pub fn export_source_frame(
    request: &SourceFrameExportRequest,
    controller: &ConversionProcessController,
) -> Result<SourceFrameExportOutcome, ConversionError> {
    if controller.is_cancelled(&request.id) {
        return finish_result(request, controller, Ok(SourceFrameExportOutcome::Cancelled));
    }
    let result = (|| {
        check_environment(crate::runtime_environment::is_flatpak())?;
        let args = build_source_frame_args(
            &request.source_path.to_string_lossy(),
            SourceFrameOptions {
                position_microseconds: request.position_microseconds,
                video_stream_index: request.video_stream_index,
            },
        )?;
        let executable = crate::runtime_binaries::ffmpeg_executable();
        let mut command = CommandSpec::new(&executable);
        #[cfg(windows)]
        {
            let (program, interpreter) =
                crate::bounded_process::resolve_windows_program(std::ffi::OsStr::new(&executable))?;
            command.program = program;
            command.interpreter = interpreter;
        }
        command.args = args.into_iter().map(Into::into).collect();
        if let Some(index) = command.args.iter().position(|arg| arg == "-i") {
            request
                .source_path
                .as_os_str()
                .clone_into(&mut command.args[index + 1]);
        }
        run_export(
            request,
            controller,
            &command,
            Limits {
                timeout: Duration::from_secs(30),
                cleanup_timeout: Duration::from_secs(1),
                stdout_bytes: ENCODED_LIMIT,
                stderr_bytes: 64 * 1024,
            },
        )
    })();
    finish_result(request, controller, result)
}

#[cfg(test)]
fn export_with_command(
    request: &SourceFrameExportRequest,
    controller: &ConversionProcessController,
    command: &CommandSpec,
    limits: Limits,
) -> Result<SourceFrameExportOutcome, ConversionError> {
    let result = run_export(request, controller, command, limits);
    finish_result(request, controller, result)
}

fn run_export(
    request: &SourceFrameExportRequest,
    controller: &ConversionProcessController,
    command: &CommandSpec,
    limits: Limits,
) -> Result<SourceFrameExportOutcome, ConversionError> {
    check_environment(crate::runtime_environment::is_flatpak())?;
    request.ensure_source_unchanged()?;
    let output = EXPORTS
        .run(command, &[], limits, || {
            controller.is_cancelled(&request.id)
        })
        .map_err(|error| {
            let reason = match error.reason {
                StopReason::TimedOut => "Source frame export timed out".to_owned(),
                StopReason::Cancelled => "Source frame export cancelled".to_owned(),
                StopReason::CleanupInProgress => {
                    "Source frame cleanup is still in progress; retry shortly".to_owned()
                }
                StopReason::Io(message) => format!("Could not run source frame export: {message}"),
            };
            ConversionError::Worker(format!("{reason} (cleanup: {:?})", error.cleanup))
        })?;
    if controller.is_cancelled(&request.id) {
        return Ok(SourceFrameExportOutcome::Cancelled);
    }
    if !output.success {
        return Err(ConversionError::Worker(format!(
            "Source frame export failed (code {:?}): {}",
            output.code,
            String::from_utf8_lossy(&output.stderr.bytes).trim()
        )));
    }
    if output.stdout.truncated {
        return Err(ConversionError::Worker(format!(
            "Source frame exceeds the PNG output limit of {} bytes",
            limits.stdout_bytes
        )));
    }
    if output.stdout.bytes.is_empty() {
        return Err(ConversionError::Worker(
            "No source frame exists at or after this position".into(),
        ));
    }
    validation::validate_png(&output.stdout.bytes, DECODED_LIMIT)?;
    publication::publish_png(request, controller, &output.stdout.bytes, || {})
}

fn finish_result(
    request: &SourceFrameExportRequest,
    controller: &ConversionProcessController,
    result: Result<SourceFrameExportOutcome, ConversionError>,
) -> Result<SourceFrameExportOutcome, ConversionError> {
    if matches!(&result, Ok(SourceFrameExportOutcome::Published { .. })) {
        return result;
    }
    if controller.finish_task(&request.id)? {
        return Ok(SourceFrameExportOutcome::Cancelled);
    }
    result
}

#[cfg(test)]
mod tests;
