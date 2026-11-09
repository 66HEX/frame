use super::*;

fn completion_task(
    sandbox: &ConversionRunnerSandbox,
    input: &Path,
    output_name: &str,
    config: &GpuiConversionConfig,
) -> ConversionTask {
    ConversionTask {
        id: "completion-real".into(),
        file_path: input.to_string_lossy().into_owned(),
        output_directory: sandbox.root.to_string_lossy().into_owned(),
        output_name: Some(output_name.into()),
        config: core_config_from_gpui(config),
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn final_completion_should_reject_png_and_recovered_gif_overwrite_races() {
    #[cfg(unix)]
    let _serial = crate::bounded_process::serial_test_processes();
    let sandbox = ConversionRunnerSandbox::new("completion-output-race");
    let input = sandbox.path("source.png");
    generate_runner_image_source(&input);
    for container in ["png", "gif"] {
        let output_name = format!("protected.{container}");
        let output = sandbox.path(&output_name);
        let task = completion_task(
            &sandbox,
            &input,
            &output_name,
            &GpuiConversionConfig {
                container: container.into(),
                video_codec: container.into(),
                fps: if container == "gif" { "8" } else { "original" }.into(),
                ..GpuiConversionConfig::default()
            },
        );
        let controller = ConversionProcessController::default();
        let mut injected = false;
        let mut events = Vec::new();
        let error = run_conversion_task_with_control(task, &controller, &mut |event| {
            if matches!(&event, ConversionEvent::Log(payload) if payload.line.starts_with("[INFO] Running ")) {
                // The exact manifest path is known; never execute a path parsed
                // from the human-readable command diagnostic.
                fs::write(&output, b"sentinel must survive").unwrap();
                injected = true;
            }
            events.push(event);
        }).expect_err("exit zero without terminal progress must not complete");
        assert!(injected);
        assert!(
            error
                .to_string()
                .contains("did not confirm that the export finished"),
            "{error}"
        );
        assert_eq!(fs::read(&output).unwrap(), b"sentinel must survive");
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ConversionEvent::Completed(_)))
        );
        assert_eq!(controller.active_process_count().unwrap(), 0);
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn final_completion_should_accept_a_legal_zero_packet_wav() {
    let sandbox = ConversionRunnerSandbox::new("completion-empty-wav");
    let input = sandbox.path("source.mp4");
    generate_runner_source(&input);
    let output = sandbox.path("empty.wav");
    let task = completion_task(
        &sandbox,
        &input,
        "empty.wav",
        &GpuiConversionConfig {
            container: "wav".into(),
            audio_codec: "pcm_s16le".into(),
            selected_audio_tracks: vec![1],
            start_time: Some("0".into()),
            // Existing generated -t precision makes this legal positive interval
            // zero microseconds. Completion cannot require a positive packet count.
            end_time: Some("0.0000001".into()),
            ..GpuiConversionConfig::default()
        },
    );
    let mut events = Vec::new();
    run_conversion_task(task, |event| events.push(event)).expect("legal empty WAV should complete");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Completed(_)))
    );
    let probe = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffprobe_executable())
            .args([
                "-v",
                "error",
                "-show_packets",
                "-show_streams",
                "-of",
                "json",
            ])
            .arg(&output),
    )
    .unwrap();
    assert!(
        probe.status.success(),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(value["packets"].as_array().unwrap().len(), 0);
    assert_eq!(value["streams"][0]["codec_name"], "pcm_s16le");
    assert!(fs::metadata(output).unwrap().len() > 0);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn final_completion_should_cancel_a_started_child_and_allow_same_id_retry() {
    let sandbox = ConversionRunnerSandbox::new("completion-cancel-retry");
    let input = sandbox.path("source.mp4");
    generate_runner_source(&input);
    let task = completion_task(
        &sandbox,
        &input,
        "cancelled.mp4",
        &GpuiConversionConfig {
            resolution: "custom".into(),
            custom_width: Some("1920".into()),
            custom_height: Some("1080".into()),
            preset: "veryslow".into(),
            ..GpuiConversionConfig::default()
        },
    );
    let task_id = task.id.clone();
    let controller = ConversionProcessController::default();
    let mut events = Vec::new();
    run_conversion_task_with_control(task, &controller, &mut |event| {
        if matches!(event, ConversionEvent::Started(_)) {
            controller
                .cancel_task(&task_id)
                .expect("started child should be cancellable");
        }
        events.push(event);
    })
    .expect("cancellation is a terminal event, not a worker error");
    assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Completed(_)))
    );
    assert_eq!(controller.active_process_count().unwrap(), 0);
    assert!(!controller.take_cancelled(&task_id).unwrap());

    let retry = completion_task(
        &sandbox,
        &input,
        "retry.mp4",
        &GpuiConversionConfig::default(),
    );
    run_conversion_task_with_control(retry, &controller, &mut |event| events.push(event))
        .expect("same-id retry should complete");
    assert!(matches!(events.last(), Some(ConversionEvent::Completed(_))));
    assert_eq!(controller.active_process_count().unwrap(), 0);
}
