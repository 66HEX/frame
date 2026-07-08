//! Runtime encoder capability detection for the native app.

use std::{
    io,
    process::{Command, Stdio},
};

use frame_core::capabilities::{
    AvailableEncoders, AvailableFilters, ffmpeg_encoder_list_args, ffmpeg_filter_list_args,
    parse_available_encoders, parse_available_filters,
};

use crate::runtime_binaries::ffmpeg_executable;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityDetectionError {
    #[error("failed to run ffmpeg encoder detection: {0}")]
    Io(#[from] io::Error),
    #[error("ffmpeg encoder detection failed: {0}")]
    Ffmpeg(String),
}

/// Detects `FFmpeg` encoders available to the bundled runtime.
///
/// # Errors
///
/// Returns an error when `FFmpeg` cannot be executed or reports a failed encoder
/// listing command.
pub fn detect_available_encoders() -> Result<AvailableEncoders, CapabilityDetectionError> {
    let executable = ffmpeg_executable();
    detect_available_encoders_with_executable(&executable)
}

/// Detects `FFmpeg` encoders using a specific executable path.
///
/// # Errors
///
/// Returns an error when the executable cannot be launched or exits with a
/// non-zero status while listing encoders.
pub fn detect_available_encoders_with_executable(
    executable: &str,
) -> Result<AvailableEncoders, CapabilityDetectionError> {
    let output = Command::new(executable)
        .args(ffmpeg_encoder_list_args())
        .stdin(Stdio::null())
        .output()?;

    available_encoders_from_output(output.status.success(), &output.stdout, &output.stderr)
}

/// Detects `FFmpeg` filters available to the bundled runtime.
///
/// # Errors
///
/// Returns an error when `FFmpeg` cannot be executed or reports a failed filter
/// listing command.
pub fn detect_available_filters() -> Result<AvailableFilters, CapabilityDetectionError> {
    let executable = ffmpeg_executable();
    detect_available_filters_with_executable(&executable)
}

/// Detects `FFmpeg` filters using a specific executable path.
///
/// # Errors
///
/// Returns an error when the executable cannot be launched or exits with a
/// non-zero status while listing filters.
pub fn detect_available_filters_with_executable(
    executable: &str,
) -> Result<AvailableFilters, CapabilityDetectionError> {
    let output = Command::new(executable)
        .args(ffmpeg_filter_list_args())
        .stdin(Stdio::null())
        .output()?;

    available_filters_from_output(output.status.success(), &output.stdout, &output.stderr)
}

fn available_encoders_from_output(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<AvailableEncoders, CapabilityDetectionError> {
    if !success {
        let message = String::from_utf8_lossy(stderr);
        let message = message.trim();
        return Err(CapabilityDetectionError::Ffmpeg(if message.is_empty() {
            "unknown ffmpeg encoder detection failure".to_string()
        } else {
            message.to_string()
        }));
    }

    Ok(parse_available_encoders(String::from_utf8_lossy(stdout)))
}

fn available_filters_from_output(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<AvailableFilters, CapabilityDetectionError> {
    if !success {
        let message = String::from_utf8_lossy(stderr);
        let message = message.trim();
        return Err(CapabilityDetectionError::Ffmpeg(if message.is_empty() {
            "unknown ffmpeg filter detection failure".to_string()
        } else {
            message.to_string()
        }));
    }

    Ok(parse_available_filters(String::from_utf8_lossy(stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_encoders_from_output_parses_successful_ffmpeg_stdout() {
        let stdout =
            b"Encoders:\n V..... h264_videotoolbox VideoToolbox H.264\n A..... libmp3lame MP3\n";

        let actual = available_encoders_from_output(true, stdout, b"")
            .expect("successful ffmpeg encoder output should parse");

        assert!(actual.h264_videotoolbox);
        assert!(actual.libmp3lame);
    }

    #[test]
    fn available_filters_from_output_parses_successful_ffmpeg_stdout() {
        let stdout = b"Filters:\n TSC eq V->V Adjust brightness\n ... deesser A->A De-ess\n";

        let actual = available_filters_from_output(true, stdout, b"")
            .expect("successful ffmpeg filter output should parse");

        assert!(actual.eq);
        assert!(actual.deesser);
    }

    #[test]
    fn available_encoders_from_output_reports_stderr_on_failed_ffmpeg() {
        let error = available_encoders_from_output(false, b"", b"ffmpeg missing codec table\n")
            .expect_err("failed ffmpeg output should surface stderr");

        assert_eq!(
            error.to_string(),
            "ffmpeg encoder detection failed: ffmpeg missing codec table"
        );
    }

    #[test]
    fn available_encoders_from_output_uses_fallback_message_without_stderr() {
        let error = available_encoders_from_output(false, b"", b"")
            .expect_err("failed ffmpeg output without stderr should still be meaningful");

        assert_eq!(
            error.to_string(),
            "ffmpeg encoder detection failed: unknown ffmpeg encoder detection failure"
        );
    }
}
