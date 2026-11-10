use super::*;
use crate::bounded_process::{CapturedOutput, ProgressProof};
use std::{
    cell::Cell,
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

fn config() -> ConversionConfig {
    let mut config = crate::conversion_runner::core_config_from_gpui(
        &crate::settings::ConversionConfig::default(),
    );
    config.container = "gif".into();
    config.video_codec = "gif".into();
    config.fps = "8".into();
    config
}

fn output(frames: Option<u64>) -> ProcessOutput {
    ProcessOutput {
        success: true,
        code: Some(0),
        stdin_written: 0,
        stdin_error: None,
        stdout: CapturedOutput::default(),
        stderr: CapturedOutput::default(),
        progress: ProgressProof {
            frames,
            ended: true,
        },
    }
}

struct Fixture {
    root: PathBuf,
    input: PathBuf,
    runtime: String,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "frame-gif-proof-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let input = root.join("input.mkv");
        fs::write(&input, b"initial source").unwrap();
        Self {
            root,
            input,
            runtime: std::env::current_exe().unwrap().to_str().unwrap().into(),
        }
    }
    fn check(
        &self,
        config: &ConversionConfig,
        budget: Duration,
        cancelled: &dyn Fn() -> bool,
        run: &mut ProbeRunner<'_>,
    ) -> GifPreflight {
        check_with_runner(
            &GifRequest {
                input: self.input.to_str().unwrap(),
                config,
                probe: &ProbeMetadata::default(),
            },
            &self.runtime,
            budget,
            cancelled,
            run,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn skipped_modes_never_resolve_or_run_a_gif_probe() {
    for (container, mode, fps) in [
        ("mp4", "reencode", "8"),
        ("mp3", "reencode", "8"),
        ("gif", "copy", "8"),
        ("gif", "reencode", "original"),
    ] {
        let mut config = config();
        config.container = container.into();
        config.processing_mode = mode.into();
        config.fps = fps.into();
        let result = check_with_runner(
            &GifRequest {
                input: "/missing/source",
                config: &config,
                probe: &ProbeMetadata::default(),
            },
            "/missing/runtime",
            Duration::ZERO,
            &|| panic!("skipped cancellation predicate"),
            &mut |_, _, _| panic!("skipped probe"),
        );
        assert!(matches!(result.decision, GifDecision::Ordinary));
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn only_complete_zero_then_one_authorizes_request_local_recovery() {
    let fixture = Fixture::new();
    for counts in [vec![1], vec![0, 1], vec![0, 0]] {
        let mut calls = 0;
        let mut timeouts = Vec::new();
        let result = fixture.check(
            &config(),
            Duration::from_secs(1),
            &|| false,
            &mut |command, limits, _| {
                let graph = command
                    .args
                    .windows(2)
                    .find(|pair| pair[0] == "-filter_complex")
                    .unwrap()[1]
                    .to_str()
                    .unwrap();
                assert_eq!(graph.contains("eof_action=pass"), calls == 1);
                assert!(!graph.contains("palettegen"));
                assert_eq!(limits.cleanup_timeout, Duration::from_secs(1));
                assert_eq!(
                    (limits.stdout_bytes, limits.stderr_bytes),
                    (16 * 1024, 16 * 1024)
                );
                timeouts.push(limits.timeout);
                let frames = counts[calls];
                calls += 1;
                Ok(output(Some(frames)))
            },
        );
        assert_eq!(calls, counts.len());
        assert!(timeouts.windows(2).all(|pair| pair[1] <= pair[0]));
        match counts.as_slice() {
            [1] => assert!(matches!(result.decision, GifDecision::Ordinary)),
            [0, 0] => assert!(matches!(result.decision, GifDecision::Empty)),
            [0, 1] => {
                let GifDecision::Recover(proof) = result.decision else {
                    panic!("missing proof")
                };
                assert!(proof.unchanged());
                fs::write(&fixture.input, b"replacement source after preflight").unwrap();
                assert!(!proof.unchanged());
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn ambiguous_progress_and_process_failures_are_never_zero_proofs() {
    for variant in 0..8 {
        let mut result = output(Some(0));
        match variant {
            0 => result.success = false,
            1 => result.code = Some(1),
            2 => result.code = None,
            3 => result.stdin_error = Some("write failed".into()),
            4 => result.stdout.truncated = true,
            5 => result.progress.ended = false,
            6 => result.progress.frames = None,
            7 => result.progress.frames = Some(2),
            _ => unreachable!(),
        }
        assert!(
            matches!(classify_probe(Ok(result)), ProbeOutcome::Unknown(_)),
            "variant {variant}"
        );
    }
    let mut drained = output(Some(1));
    drained.stderr.truncated = true;
    assert_eq!(classify_probe(Ok(drained)), ProbeOutcome::Frames(1));
    for reason in [
        StopReason::Cancelled,
        StopReason::TimedOut,
        StopReason::Io("read failed".into()),
        StopReason::CleanupInProgress,
    ] {
        for cleanup in [CleanupOutcome::Complete, CleanupOutcome::Pending] {
            let outcome = classify_probe(Err(ProcessError {
                reason: reason.clone(),
                cleanup,
                stdout: CapturedOutput::default(),
                stderr: CapturedOutput::default(),
            }));
            match (cleanup, &reason) {
                (CleanupOutcome::Pending, _) => {
                    assert!(matches!(outcome, ProbeOutcome::CleanupPending(_)));
                }
                (_, StopReason::Cancelled) => assert_eq!(outcome, ProbeOutcome::Cancelled),
                _ => assert!(matches!(outcome, ProbeOutcome::Unknown(_))),
            }
        }
    }
}

#[test]
fn identity_change_after_either_phase_never_grants_a_cap() {
    for change_at in [0, 1] {
        let fixture = Fixture::new();
        let initial_size = fs::metadata(&fixture.input).unwrap().len();
        let mut calls = 0;
        let result = fixture.check(
            &config(),
            Duration::from_secs(1),
            &|| false,
            &mut |_, _, _| {
                if calls == change_at {
                    // Same-size writes can share a filesystem timestamp tick.
                    fs::write(&fixture.input, b"changed source after GIF probe").unwrap();
                    assert_ne!(fs::metadata(&fixture.input).unwrap().len(), initial_size);
                }
                let frames = u64::from(calls != 0);
                calls += 1;
                Ok(output(Some(frames)))
            },
        );
        assert!(matches!(result.decision, GifDecision::Ordinary));
        assert_eq!(calls, change_at + 1);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|line| line.contains("changed"))
        );
    }
}

#[test]
fn cancellation_after_zero_stops_before_pass_and_pending_dominates() {
    let fixture = Fixture::new();
    for pending in [false, true] {
        let cancelled = Cell::new(false);
        let mut calls = 0;
        let result = fixture.check(
            &config(),
            Duration::from_secs(1),
            &|| cancelled.get(),
            &mut |_, _, _| {
                calls += 1;
                cancelled.set(true);
                if pending {
                    Err(ProcessError {
                        reason: StopReason::Cancelled,
                        cleanup: CleanupOutcome::Pending,
                        stdout: CapturedOutput::default(),
                        stderr: CapturedOutput::default(),
                    })
                } else {
                    Ok(output(Some(0)))
                }
            },
        );
        assert_eq!(calls, 1);
        if pending {
            assert!(matches!(result.decision, GifDecision::CleanupPending));
        } else {
            assert!(matches!(result.decision, GifDecision::Cancelled));
        }
    }
}

#[test]
fn second_probe_cannot_receive_a_new_operation_budget() {
    let fixture = Fixture::new();
    let mut calls = 0;
    let result = fixture.check(
        &config(),
        Duration::from_millis(100),
        &|| false,
        &mut |_, limits, _| {
            calls += 1;
            assert!(limits.timeout <= Duration::from_millis(100));
            std::thread::sleep(Duration::from_millis(110));
            Ok(output(Some(0)))
        },
    );
    assert_eq!(calls, 1);
    assert!(matches!(result.decision, GifDecision::Ordinary));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|line| line.contains("deadline"))
    );
}

#[test]
fn unobserved_or_script_runtime_never_authorizes_a_probe() {
    let mut fixture = Fixture::new();
    #[cfg(unix)]
    {
        fixture.runtime = "ffmpeg".into();
        let result = fixture.check(
            &config(),
            Duration::from_secs(1),
            &|| false,
            &mut |_, _, _| panic!("bare name must not run"),
        );
        assert!(matches!(result.decision, GifDecision::Ordinary));
    }
    let script = fixture.root.join("wrapper");
    fs::write(&script, b"#!/bin/sh\nexit 0\n").unwrap();
    for executable in [script.to_str().unwrap(), "/missing/runtime"] {
        fixture.runtime = executable.into();
        let result = fixture.check(
            &config(),
            Duration::from_secs(1),
            &|| false,
            &mut |_, _, _| panic!("unobserved runtime must not run"),
        );
        assert!(matches!(result.decision, GifDecision::Ordinary));
        assert!(!result.diagnostics.is_empty());
    }
}

#[test]
fn raw_helper_burn_dependency_is_bound_without_expanding_product_support() {
    let fixture = Fixture::new();
    let subtitle = fixture.root.join("captions.srt");
    fs::write(&subtitle, b"initial captions").unwrap();
    let mut config = config();
    config.subtitle_burn_path = Some(subtitle.to_str().unwrap().into());
    assert!(
        frame_core::args::validate_task_input(fixture.input.to_str().unwrap(), &config).is_err()
    );
    let mut calls = 0;
    let result = fixture.check(
        &config,
        Duration::from_secs(1),
        &|| false,
        &mut |_, _, _| {
            fs::write(&subtitle, b"new captions").unwrap();
            calls += 1;
            Ok(output(Some(0)))
        },
    );
    assert_eq!(calls, 1);
    assert!(matches!(result.decision, GifDecision::Ordinary));
}

#[test]
fn inconclusive_phase_preserves_ordinary_export_and_never_retries() {
    let fixture = Fixture::new();
    for fail_at in [0, 1] {
        let mut calls = 0;
        let result = fixture.check(
            &config(),
            Duration::from_secs(1),
            &|| false,
            &mut |_, _, _| {
                let frames = if calls == fail_at { None } else { Some(0) };
                calls += 1;
                Ok(output(frames))
            },
        );
        assert!(matches!(result.decision, GifDecision::Ordinary));
        assert_eq!(calls, fail_at + 1);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|line| line.contains("inconclusive"))
        );
    }
    let result = check_with_runner(
        &GifRequest {
            input: "/missing/source",
            config: &config(),
            probe: &ProbeMetadata::default(),
        },
        "/missing/runtime",
        Duration::from_secs(1),
        &|| true,
        &mut |_, _, _| panic!("cancelled request must not launch"),
    );
    assert!(matches!(result.decision, GifDecision::Cancelled));
    assert!(result.diagnostics.is_empty());
}
