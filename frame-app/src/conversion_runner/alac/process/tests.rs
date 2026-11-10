use super::*;
use crate::bounded_process::{CapturedOutput, ProgressProof};

fn output() -> ProcessOutput {
    ProcessOutput {
        success: true,
        code: Some(0),
        stdin_written: 0,
        stdin_error: None,
        stdout: CapturedOutput::default(),
        stderr: CapturedOutput::default(),
        progress: ProgressProof::default(),
    }
}

#[test]
fn successful_exit_does_not_hide_probe_errors_or_partial_captures() {
    let valid = output();
    validate_output(&valid).unwrap();
    let mut invalid = output();
    invalid.stderr.bytes = b"decode error\n".to_vec();
    assert!(matches!(
        validate_output(&invalid),
        Err(Failure::Invalid(_))
    ));
    invalid.stderr.bytes = b" \t\n".to_vec();
    validate_output(&invalid).unwrap();
    invalid.stdout.truncated = true;
    assert!(matches!(validate_output(&invalid), Err(Failure::Limit(_))));
    invalid.stdout.truncated = false;
    invalid.stderr.truncated = true;
    assert!(matches!(validate_output(&invalid), Err(Failure::Limit(_))));
    invalid.stderr.truncated = false;
    invalid.stdin_error = Some("short write".into());
    assert!(validate_output(&invalid).is_err());
    invalid.stdin_error = None;
    invalid.code = Some(1);
    assert!(validate_output(&invalid).is_err());
}

#[cfg(unix)]
fn shell(script: &str) -> Runtime {
    let mut runtime = Runtime::resolve("/bin/sh").unwrap();
    // Per-command args, without changing global process environment. Added
    // -progress/path are $1/$2 after the script's explicit argv[0].
    runtime.command.args = ["-c", script, "alac-helper-test"].map(Into::into).into();
    runtime
}

#[cfg(unix)]
#[test]
fn observer_requires_original_handle_completion_and_complete_success() {
    let _serial = crate::bounded_process::serial_test_processes();
    let scripts = [
        ("printf payload; printf 'progress=end\\n' > \"$2\"", true),
        ("printf payload", false),
        ("printf payload; printf 'progress=end' > \"$2\"", false),
        (
            "printf payload; printf 'progress=end\\n' > \"$2\"; exit 2",
            false,
        ),
        (
            "printf payload; printf 'progress=end\\n' > \"$2\"; printf error >&2",
            false,
        ),
        // A replacement pathname is not evidence written to the retained file.
        (
            "rm \"$2\"; printf 'progress=end\\n' > \"$2\"; printf payload",
            false,
        ),
    ];
    for (script, expected) in scripts {
        let mut helpers = Helpers::new(0);
        let result = helpers.execute(&shell(script), [], true, &|| false);
        assert_eq!(result.is_ok(), expected, "{script}: {result:?}");
        assert!(helpers.diagnostics.is_empty(), "{:?}", helpers.diagnostics);
        if let Ok(bytes) = result {
            assert_eq!(bytes, b"payload");
        }
    }
}

#[cfg(unix)]
#[test]
fn helper_budget_is_shared_and_cancellation_is_checked_before_spawn() {
    let _serial = crate::bounded_process::serial_test_processes();
    let runtime = shell("printf abc");
    let mut helpers = Helpers::new(TASK_CAPTURE_LIMIT - 3);
    let initial_time = helpers.remaining;
    assert_eq!(
        helpers.execute(&runtime, [], false, &|| false).unwrap(),
        b"abc"
    );
    assert!(helpers.remaining < initial_time);
    assert_eq!(helpers.captured_bytes, TASK_CAPTURE_LIMIT);
    assert!(matches!(
        helpers.execute(&runtime, [], false, &|| false),
        Err(Failure::Limit(_))
    ));
    let mut helpers = Helpers::new(0);
    assert!(matches!(
        helpers.execute(&runtime, [], false, &|| true),
        Err(Failure::Cancelled)
    ));
    assert_eq!(helpers.remaining, PHASE_TIME);
    assert_eq!(helpers.captured_bytes, 0);
    helpers.remaining = Duration::ZERO;
    assert!(matches!(
        helpers.execute(&runtime, [], false, &|| false),
        Err(Failure::Limit(_))
    ));
    assert!(helpers.diagnostics.is_empty());
}
