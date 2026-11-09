//! Bounded proof helpers. A complete parser result alone is never EOF evidence.

use crate::bounded_process::{
    CleanupOutcome, Limits, ProcessError, ProcessOutput, StopReason, run,
};
use crate::capabilities::runtime::Runtime;
use crate::conversion_runner::completion::CompletionFile;
use frame_core::alac_copy::ObservationError;
use frame_core::error::ConversionError;
use std::ffi::OsString;
use std::time::{Duration, Instant};

const PHASE_TIME: Duration = Duration::from_secs(30);
const STDOUT_LIMIT: usize = 32 * 1024 * 1024;
const STDERR_LIMIT: usize = 64 * 1024;
const TASK_CAPTURE_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub(super) enum Failure {
    Cancelled,
    Pending(String),
    Limit(String),
    Invalid(String),
}

impl Failure {
    pub fn invalid(detail: &(impl ToString + ?Sized)) -> Self {
        Self::Invalid(detail.to_string())
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::Cancelled => "audio verification cancelled",
            Self::Pending(detail) | Self::Limit(detail) | Self::Invalid(detail) => detail,
        }
    }

    pub fn user_error(&self) -> ConversionError {
        let message = match self {
            Self::Limit(_) => {
                "Audio verification reached its limit. Try a shorter segment or choose Re-encode."
            }
            Self::Pending(_) => {
                "Audio verification is still stopping. Wait a moment and try again."
            }
            Self::Invalid(_) => {
                "Could not verify the copied ALAC audio. Try again or choose Re-encode."
            }
            Self::Cancelled => "Audio verification was cancelled.",
        };
        ConversionError::Worker(message.into())
    }
}

impl From<ObservationError> for Failure {
    fn from(error: ObservationError) -> Self {
        if matches!(error, ObservationError::Limit(_)) {
            Self::Limit(error.to_string())
        } else {
            Self::invalid(&error)
        }
    }
}

pub(super) struct Helpers {
    remaining: Duration,
    pub captured_bytes: usize,
    pub diagnostics: Vec<String>,
}

impl Helpers {
    pub const fn new(captured_bytes: usize) -> Self {
        Self {
            remaining: PHASE_TIME,
            captured_bytes,
            diagnostics: Vec::new(),
        }
    }

    pub fn execute(
        &mut self,
        runtime: &Runtime,
        args: impl IntoIterator<Item = OsString>,
        require_completion: bool,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, Failure> {
        if cancelled() {
            return Err(Failure::Cancelled);
        }
        let available = TASK_CAPTURE_LIMIT.saturating_sub(self.captured_bytes);
        if self.remaining.is_zero() || available == 0 {
            return Err(Failure::Limit(
                "analysis phase or serialized capture budget exhausted".into(),
            ));
        }
        let mut completion = if require_completion {
            match CompletionFile::create() {
                Ok(owner) => Some(owner),
                Err(error) => {
                    self.diagnostics.extend(error.diagnostics);
                    return Err(Failure::invalid(&error.error));
                }
            }
        } else {
            None
        };
        let mut command = runtime.command.clone();
        if let Some(owner) = &completion {
            command.args.push("-progress".into());
            command.args.push(owner.path().as_os_str().to_owned());
        }
        command.args.extend(args);
        let cleanup_timeout = Duration::from_secs(1).min(self.remaining / 2);
        let timeout = self.remaining.saturating_sub(cleanup_timeout);
        let started = Instant::now();
        let result = run(
            &command,
            &[],
            Limits {
                timeout,
                cleanup_timeout,
                stdout_bytes: STDOUT_LIMIT.min(available),
                stderr_bytes: STDERR_LIMIT,
            },
            cancelled,
        );
        let elapsed = started.elapsed(); // Includes permit wait and owned cleanup.
        let within_budget = elapsed <= self.remaining;
        self.remaining = self.remaining.saturating_sub(elapsed);
        let completion_result = if result
            .as_ref()
            .is_ok_and(|output| output.success && output.code == Some(0))
        {
            completion.as_mut().map_or(Ok(()), |owner| {
                owner.verify().map_err(|error| Failure::invalid(&error))
            })
        } else {
            Ok(())
        };
        if let Some(owner) = completion {
            // On Pending this abandons only a failed private protocol, not the
            // supervised process tree. No proof is accepted and its permit stays
            // owned by the supervisor. A denied unlink remains a diagnostic.
            self.diagnostics.extend(owner.cleanup());
        }
        // Pending ownership dominates apparent success, budget, and cancellation.
        if let Err(error) = &result
            && error.cleanup == CleanupOutcome::Pending
        {
            return Err(Failure::Pending(error.to_string()));
        }
        if cancelled() {
            return Err(Failure::Cancelled);
        }
        if !within_budget {
            return Err(Failure::Limit(
                "analysis phase time budget exhausted".into(),
            ));
        }
        let output = result.map_err(|error| process_failure(&error))?;
        validate_output(&output)?;
        completion_result?;
        self.captured_bytes = self
            .captured_bytes
            .checked_add(output.stdout.bytes.len())
            .filter(|total| *total <= TASK_CAPTURE_LIMIT)
            .ok_or_else(|| Failure::Limit("serialized capture budget exhausted".into()))?;
        Ok(output.stdout.bytes)
    }
}

fn validate_output(output: &ProcessOutput) -> Result<(), Failure> {
    if output.stdout.truncated || output.stderr.truncated {
        return Err(Failure::Limit("helper capture limit reached".into()));
    }
    if !output.success
        || output.code != Some(0)
        || output.stdin_error.is_some()
        || !output.stderr.bytes.iter().all(u8::is_ascii_whitespace)
    {
        return Err(Failure::Invalid(format!(
            "helper exit={:?}, stdin={:?}; {}",
            output.code,
            output.stdin_error,
            String::from_utf8_lossy(&output.stderr.bytes).trim()
        )));
    }
    Ok(())
}

fn process_failure(error: &ProcessError) -> Failure {
    let detail = format!(
        "{error}; {}",
        String::from_utf8_lossy(&error.stderr.bytes).trim()
    );
    match error.reason {
        StopReason::Cancelled => Failure::Cancelled,
        StopReason::TimedOut => Failure::Limit(detail),
        StopReason::CleanupInProgress => Failure::Pending(detail),
        StopReason::Io(_) => Failure::Invalid(detail),
    }
}

#[cfg(test)]
mod tests;
