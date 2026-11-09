//! Optional, request-local minimum-frame recovery for explicit-rate GIF exports.

use crate::bounded_process::{
    self, CleanupOutcome, CommandSpec, Limits, ProcessError, ProcessOutput, StopReason,
};
use crate::capabilities::runtime::{Runtime, RuntimeIdentity};
use frame_core::args::{build_gif_probe_args, is_gif_minimum_frame_candidate};
use frame_core::filters::FrameRateEofAction;
use frame_core::types::{ConversionConfig, ProbeMetadata};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(super) enum GifDecision {
    Ordinary,
    Recover(Box<GifProof>),
    Empty,
    Cancelled,
    CleanupPending,
}

pub(super) struct GifPreflight {
    pub decision: GifDecision,
    pub diagnostics: Vec<String>,
}

/// A proof is useful only while the observed source and native runtime match.
pub(super) struct GifProof {
    runtime: Runtime,
    dependencies: Vec<PathBuf>,
    identity: RuntimeIdentity,
}

impl GifProof {
    pub fn unchanged(&self) -> bool {
        self.runtime.identity.is_some()
            && self.runtime.identity_unchanged()
            && self.identity.matches_paths(&self.dependencies)
    }
}

impl GifPreflight {
    fn ordinary(reason: impl Into<String>) -> Self {
        Self {
            decision: GifDecision::Ordinary,
            diagnostics: vec![reason.into()],
        }
    }
}

/// Unknown checks preserve the already-built ordinary plan; they never grant a cap.
pub(super) fn check_gif_minimum_frame(
    input: &str,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
    executable: &str,
    cancelled: impl Fn() -> bool,
) -> GifPreflight {
    check_with_runner(
        &GifRequest {
            input,
            config,
            probe,
        },
        executable,
        Duration::from_secs(5),
        &cancelled,
        &mut |command, limits, cancelled| bounded_process::run(command, &[], limits, cancelled),
    )
}

struct GifRequest<'a> {
    input: &'a str,
    config: &'a ConversionConfig,
    probe: &'a ProbeMetadata,
}

type ProbeRunner<'a> =
    dyn FnMut(&CommandSpec, Limits, &dyn Fn() -> bool) -> Result<ProcessOutput, ProcessError> + 'a;

fn prepare_proof(request: &GifRequest<'_>, executable: &str) -> Result<GifProof, String> {
    let runtime = Runtime::resolve(executable)
        .map_err(|error| format!("GIF runtime check failed: {error}"))?;
    if runtime.identity.is_none() {
        return Err("GIF minimum-frame recovery needs an observed native runtime identity".into());
    }
    let mut dependencies = vec![PathBuf::from(request.input)];
    // Valid application GIF tasks reject burned subtitles. Preserve binding for
    // raw-core/helper callers without claiming that product feature is supported.
    if let Some(path) = request
        .config
        .subtitle_burn_path
        .as_deref()
        .filter(|path| !path.is_empty())
    {
        dependencies.push(PathBuf::from(path));
    }
    let identity = RuntimeIdentity::capture_paths(&dependencies)
        .ok_or_else(|| "Could not observe GIF input identity".to_string())?;
    Ok(GifProof {
        runtime,
        dependencies,
        identity,
    })
}

fn check_with_runner(
    request: &GifRequest<'_>,
    executable: &str,
    budget: Duration,
    cancelled: &dyn Fn() -> bool,
    run: &mut ProbeRunner<'_>,
) -> GifPreflight {
    if !is_gif_minimum_frame_candidate(request.config) {
        return GifPreflight {
            decision: GifDecision::Ordinary,
            diagnostics: Vec::new(),
        };
    }
    if cancelled() {
        return GifPreflight {
            decision: GifDecision::Cancelled,
            diagnostics: Vec::new(),
        };
    }
    let Some(deadline) = Instant::now().checked_add(budget) else {
        return GifPreflight::ordinary("Invalid GIF check deadline; using ordinary export.");
    };
    let proof = match prepare_proof(request, executable) {
        Ok(proof) => proof,
        Err(reason) => return GifPreflight::ordinary(format!("{reason}; using ordinary export.")),
    };
    let mut diagnostics = Vec::new();
    for mode in [FrameRateEofAction::Round, FrameRateEofAction::Pass] {
        let decision = match run_probe(request, &proof, mode, deadline, cancelled, run) {
            ProbeOutcome::Frames(frames) => {
                diagnostics.push(format!("GIF frame check {mode:?}: {frames} frame(s)."));
                match (mode, frames) {
                    (FrameRateEofAction::Round, 0) => continue,
                    (FrameRateEofAction::Round, _) => GifDecision::Ordinary,
                    (FrameRateEofAction::Pass, 0) => GifDecision::Empty,
                    (FrameRateEofAction::Pass, _) => GifDecision::Recover(Box::new(proof)),
                }
            }
            ProbeOutcome::Unknown(reason) => {
                diagnostics.push(format!(
                    "GIF frame check was inconclusive: {reason}; using ordinary export."
                ));
                GifDecision::Ordinary
            }
            ProbeOutcome::Cancelled => GifDecision::Cancelled,
            ProbeOutcome::CleanupPending(reason) => {
                diagnostics.push(reason);
                GifDecision::CleanupPending
            }
        };
        return GifPreflight {
            decision,
            diagnostics,
        };
    }
    unreachable!("a pass probe always returns a decision")
}

fn run_probe(
    request: &GifRequest<'_>,
    proof: &GifProof,
    mode: FrameRateEofAction,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
    run: &mut ProbeRunner<'_>,
) -> ProbeOutcome {
    if cancelled() {
        return ProbeOutcome::Cancelled;
    }
    if !proof.unchanged() {
        return ProbeOutcome::Unknown("GIF input or runtime changed".into());
    }
    let args = match build_gif_probe_args(request.input, request.config, request.probe, mode) {
        Ok(Some(args)) => args,
        Ok(None) => return ProbeOutcome::Unknown("GIF frame check no longer applies".into()),
        Err(error) => {
            return ProbeOutcome::Unknown(format!("Could not build GIF frame check: {error}"));
        }
    };
    let Some(timeout) = deadline
        .checked_duration_since(Instant::now())
        .filter(|time| !time.is_zero())
    else {
        return ProbeOutcome::Unknown("GIF frame check exceeded its shared deadline".into());
    };
    let mut command = proof.runtime.command.clone();
    command.args = args.into_iter().map(Into::into).collect();
    let outcome = classify_probe(run(
        &command,
        Limits {
            timeout,
            ..Limits::default()
        },
        cancelled,
    ));
    // Pending cleanup dominates cancellation and every apparent progress proof.
    if matches!(outcome, ProbeOutcome::CleanupPending(_)) {
        return outcome;
    }
    if cancelled() {
        return ProbeOutcome::Cancelled;
    }
    if matches!(outcome, ProbeOutcome::Frames(_)) && !proof.unchanged() {
        return ProbeOutcome::Unknown("GIF input or runtime changed".into());
    }
    outcome
}

#[derive(Debug, PartialEq)]
enum ProbeOutcome {
    Frames(u64),
    Unknown(String),
    Cancelled,
    CleanupPending(String),
}

fn classify_probe(result: Result<ProcessOutput, ProcessError>) -> ProbeOutcome {
    match result {
        Ok(output)
            if output.success
                && output.code == Some(0)
                && output.stdin_error.is_none()
                && !output.stdout.truncated
                && output.progress.ended
                && matches!(output.progress.frames, Some(0 | 1)) =>
        {
            ProbeOutcome::Frames(output.progress.frames.unwrap())
        }
        Ok(output) => ProbeOutcome::Unknown(format!(
            "exit={:?}, final progress={:?}, stdout truncated={}, stdin error={:?}; {}",
            output.code,
            output.progress,
            output.stdout.truncated,
            output.stdin_error,
            String::from_utf8_lossy(&output.stderr.bytes).trim()
        )),
        Err(error) if error.cleanup == CleanupOutcome::Pending => {
            ProbeOutcome::CleanupPending(format!("GIF frame check cleanup is pending: {error}"))
        }
        Err(error) if error.reason == StopReason::Cancelled => ProbeOutcome::Cancelled,
        Err(error) => ProbeOutcome::Unknown(format!(
            "{error}; {}",
            String::from_utf8_lossy(&error.stderr.bytes).trim()
        )),
    }
}

#[cfg(test)]
mod tests;
