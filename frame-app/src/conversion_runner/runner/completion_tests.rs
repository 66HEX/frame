use super::*;

fn task() -> ConversionTask {
    ConversionTask {
        id: "completion-test".into(),
        file_path: "unused.mp4".into(),
        output_directory: "unused".into(),
        output_name: None,
        config: super::super::config::core_config_from_gpui(
            &crate::settings::ConversionConfig::default(),
        ),
    }
}

#[test]
fn finalizer_consumes_cleanup_and_cancel_log_markers_even_after_initial_cancellation() {
    for initially_cancelled in [false, true] {
        let task = task();
        let controller = ConversionProcessController::default();
        controller.register_started_process(&task.id, 0).unwrap();
        if initially_cancelled {
            controller.cancel_task(&task.id).unwrap();
        }
        let saved = controller.finish_task(&task.id).unwrap();
        assert_eq!(saved, initially_cancelled);
        let owner = CompletionFile::create().unwrap();
        let parent = owner.path().parent().unwrap().to_path_buf();
        let extra = parent.join("keep");
        std::fs::write(&extra, b"not recursively removed").unwrap();
        let diagnostics = owner.cleanup();
        assert_eq!(diagnostics.len(), 1);
        let mut events = Vec::new();
        finish_final_conversion(
            &task,
            "output",
            &controller,
            &mut |event| {
                if let ConversionEvent::Log(payload) = &event {
                    assert_eq!(controller.active_process_count().unwrap(), 0);
                    if payload.line.starts_with("[WARN] Completion cleanup: ")
                        || payload.line == "[INFO] Task cancelled"
                    {
                        controller.cancel_task(&task.id).unwrap();
                    }
                }
                events.push(event);
            },
            FinalOutcome {
                result: Err(ConversionError::Worker("original proof failure".into())),
                was_cancelled: saved,
                diagnostics,
            },
        )
        .unwrap();
        assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ConversionEvent::Completed(_)))
        );
        assert!(!controller.take_cancelled(&task.id).unwrap());
        std::fs::remove_file(extra).unwrap();
        std::fs::remove_dir(parent).unwrap();

        // Reusing the same task id must not inherit either consumed marker.
        finish_final_conversion(
            &task,
            "retry",
            &controller,
            &mut |event| events.push(event),
            FinalOutcome {
                result: Ok(()),
                was_cancelled: false,
                diagnostics: Vec::new(),
            },
        )
        .unwrap();
        assert!(
            matches!(events.last(), Some(ConversionEvent::Completed(payload)) if payload.output_path == "retry")
        );
    }
}

#[test]
fn setup_and_process_errors_keep_their_primary_result_after_cleanup_diagnostics() {
    for primary in [
        "setup failed",
        "ffmpeg exited with status 1",
        "missing terminal record",
    ] {
        let task = task();
        let controller = ConversionProcessController::default();
        let mut events = Vec::new();
        let error = finish_final_conversion(
            &task,
            "output",
            &controller,
            &mut |event| events.push(event),
            FinalOutcome {
                result: Err(ConversionError::Worker(primary.into())),
                was_cancelled: false,
                diagnostics: vec!["cleanup failed".into()],
            },
        )
        .expect_err("cleanup warning must not hide the primary error");
        assert!(error.to_string().contains(primary));
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], ConversionEvent::Log(payload) if payload.line.starts_with("[WARN] Completion cleanup: "))
        );
    }
}

#[test]
fn setup_error_diagnostic_cancellation_wins_and_leaves_no_marker() {
    let task = task();
    let controller = ConversionProcessController::default();
    let mut events = Vec::new();
    finish_final_conversion(
        &task,
        "output",
        &controller,
        &mut |event| {
            if matches!(&event, ConversionEvent::Log(payload) if payload.line.starts_with("[WARN]"))
            {
                controller.cancel_task(&task.id).unwrap();
            }
            events.push(event);
        },
        FinalOutcome {
            result: Err(ConversionError::Worker(
                "could not create completion file".into(),
            )),
            was_cancelled: false,
            diagnostics: vec!["partial setup cleanup failed".into()],
        },
    )
    .unwrap();
    assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
    assert!(!controller.take_cancelled(&task.id).unwrap());
}

#[test]
fn command_attaches_progress_to_the_final_base_args_without_shell_reconstruction() {
    let args = vec![
        "-n".into(),
        "-i".into(),
        "source".into(),
        "output.png".into(),
    ];
    let path = std::path::Path::new("/tmp/Zażółć spaces ; $literal/progress");
    let command = final_command("ffmpeg", &args, path);
    let actual = command.get_args().collect::<Vec<_>>();
    assert_eq!(actual[0], "-progress");
    assert_eq!(actual[1], path.as_os_str());
    assert_eq!(
        actual[2..],
        args.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>()
    );
}

#[cfg(unix)]
#[test]
fn command_preserves_non_utf8_progress_path_bytes_without_requiring_filesystem_support() {
    use std::os::unix::ffi::OsStringExt;
    let raw = b"/tmp/frame-\xff/progress".to_vec();
    let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(raw));
    let command = final_command("ffmpeg", &["-n".into(), "output".into()], &path);
    assert_eq!(command.get_args().nth(1), Some(path.as_os_str()));
}

#[test]
fn final_spawn_failure_keeps_its_error_and_does_not_report_completion() {
    let task = task();
    let controller = ConversionProcessController::default();
    let mut events = Vec::new();
    let missing_executable = format!("./frame-missing-runtime-{}/ffmpeg", std::process::id());
    assert!(!std::path::Path::new(&missing_executable).exists());
    let error = run_final_conversion(
        &task,
        "output",
        &missing_executable,
        vec!["-n".into(), "output".into()],
        PreparedGifExport {
            args: vec!["-n".into(), "output".into()],
            proof: None,
        },
        &controller,
        &mut |event| events.push(event),
    )
    .expect_err("failed spawn cannot pass final completion");
    assert!(matches!(error, ConversionError::Io(_)));
    assert_eq!(controller.active_process_count().unwrap(), 0);
    assert!(!events.iter().any(|event| matches!(
        event,
        ConversionEvent::Completed(_) | ConversionEvent::Started(_)
    )));
}
