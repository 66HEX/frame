//! Runtime encoder capability detection for the native app.

mod hardware;
mod runtime;

pub use hardware::{
    EncoderInventory, HardwareCapabilities, HardwareEncoder, HardwareProbePolicy,
    HardwareProbeReport, HardwareReason, HardwareStatus, detect_hardware_capabilities,
    invalidate_hardware_cache, probe_hardware_encoder,
};

use crate::bounded_process::{self, Limits};
use crate::runtime_binaries::ffmpeg_executable;
use frame_core::capabilities::{
    AvailableEncoders, AvailableFilters, ffmpeg_encoder_list_args, ffmpeg_filter_list_args,
    parse_available_encoders, parse_available_filters,
};
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityDetectionError {
    #[error("failed to run ffmpeg encoder detection: {0}")]
    Io(#[from] io::Error),
    #[error("ffmpeg encoder detection failed: {0}")]
    Ffmpeg(String),
    #[error("ffmpeg capability check did not finish: {0}")]
    Incomplete(String),
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
    let (stdout, stderr) = bounded_listing(executable, &ffmpeg_encoder_list_args(), "Encoders:")?;
    available_encoders_from_output(true, &stdout, &stderr)
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
    let (stdout, stderr) = bounded_listing(executable, &ffmpeg_filter_list_args(), "Filters:")?;
    available_filters_from_output(true, &stdout, &stderr)
}

fn bounded_listing(
    executable: &str,
    args: &[&str],
    header: &str,
) -> Result<(Vec<u8>, Vec<u8>), CapabilityDetectionError> {
    let mut runtime = runtime::Runtime::resolve(executable)?;
    runtime.command.args = args.iter().map(std::ffi::OsString::from).collect();
    let output = bounded_process::run(
        &runtime.command,
        &[],
        Limits {
            stdout_bytes: 256 * 1024,
            ..Limits::default()
        },
        || false,
    )
    .map_err(|error| {
        CapabilityDetectionError::Incomplete(format!(
            "{error}: {}",
            String::from_utf8_lossy(&error.stderr.bytes).trim()
        ))
    })?;
    if !output.success {
        return Err(CapabilityDetectionError::Ffmpeg(format!(
            "exit {:?}: {}",
            output.code,
            String::from_utf8_lossy(&output.stderr.bytes).trim()
        )));
    }
    if output.stdout.truncated {
        return Err(CapabilityDetectionError::Incomplete(
            "capability table exceeds the output limit".into(),
        ));
    }
    let text = std::str::from_utf8(&output.stdout.bytes).map_err(|_| {
        CapabilityDetectionError::Incomplete("capability table is not valid UTF-8".into())
    })?;
    validate_listing_table(text, header)?;
    if !runtime.identity_unchanged() {
        return Err(CapabilityDetectionError::Incomplete(
            "FFmpeg changed during the check".into(),
        ));
    }
    Ok((output.stdout.bytes, output.stderr.bytes))
}

fn validate_listing_table(text: &str, header: &str) -> Result<(), CapabilityDetectionError> {
    let mut found_header = false;
    let mut rows = 0;
    for raw in text.split_inclusive('\n') {
        let line = raw.trim();
        if line == header {
            found_header = true;
            continue;
        }
        if !found_header || line.is_empty() || line.bytes().all(|byte| byte == b'-') {
            continue;
        }
        let fields: Vec<_> = line.split_ascii_whitespace().collect();
        if fields.get(1) == Some(&"=") {
            continue;
        }
        let flags = fields.first().copied().unwrap_or("");
        let valid_flags = flags
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || matches!(byte, b'.' | b'|'));
        let valid = if header == "Encoders:" {
            valid_flags
                && flags.len() == 6
                && fields.len() >= 2
                && (fields.len() > 2 || raw.ends_with(char::is_whitespace))
        } else {
            valid_flags
                && matches!(flags.len(), 2 | 3)
                && fields.len() >= 3
                && fields[2].split_once("->").is_some_and(|(input, output)| {
                    !input.is_empty()
                        && !output.is_empty()
                        && input
                            .bytes()
                            .chain(output.bytes())
                            .all(|byte| matches!(byte, b'A' | b'V' | b'N' | b'|'))
                })
        };
        if !valid {
            return Err(CapabilityDetectionError::Incomplete(
                "capability table contains an incomplete row".into(),
            ));
        }
        rows += 1;
    }
    if !found_header || rows == 0 {
        return Err(CapabilityDetectionError::Incomplete(
            "capability table is missing".into(),
        ));
    }
    Ok(())
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
    fn listing_requires_complete_rows_but_allows_empty_descriptions_and_flag_versions() {
        for valid in [
            "Encoders:\n V..... h264_nvenc \n",
            "Encoders:\n V..... h264_nvenc NVIDIA",
            "Encoders:\n V..... h264_nvenc ",
        ] {
            validate_listing_table(valid, "Encoders:").unwrap();
        }
        for valid in [
            "Filters:\n TS lutyuv V->V Color",
            "Filters:\n TSC lutyuv V->V\n",
            "Filters:\n .. color |->V ",
        ] {
            validate_listing_table(valid, "Filters:").unwrap();
        }
        for invalid in [
            "Encoders:\n V..... libx264 Encoder\n V....",
            "Encoders:\n V.....\n",
            "Encoders:\n V..... h264_nve",
            "Encoders:\n",
        ] {
            assert!(
                validate_listing_table(invalid, "Encoders:").is_err(),
                "{invalid}"
            );
        }
        for invalid in [
            "Filters:\n TS lutyuv\n",
            "Filters:\n TS lutyuv V->",
            "Filters:\n TSC valid V->V Filter\n TSC",
        ] {
            assert!(
                validate_listing_table(invalid, "Filters:").is_err(),
                "{invalid}"
            );
        }
    }

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
        let stdout = b"Filters:\n TSC lutyuv V->V Adjust brightness\n ... deesser A->A De-ess\n";

        let actual = available_filters_from_output(true, stdout, b"")
            .expect("successful ffmpeg filter output should parse");

        assert!(actual.lutyuv);
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
