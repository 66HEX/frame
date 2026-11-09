use super::*;

fn generate_alac(
    sandbox: &ConversionRunnerSandbox,
    samples: u32,
    extension: &str,
) -> (PathBuf, Vec<u8>) {
    let pcm = (0..samples)
        .flat_map(|index| {
            let value = (i32::try_from((index * 7_919) % 65_536).unwrap() - 32_768) * 65_536;
            value.to_le_bytes()
        })
        .collect::<Vec<_>>();
    let raw = sandbox.path(&format!("source-{samples}-{extension}.s32le"));
    fs::write(&raw, &pcm).unwrap();
    let source = sandbox.path(&format!("source-{samples}.{extension}"));
    let result = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "s32le",
                "-ar",
                "48000",
                "-ac",
                "1",
                "-i",
            ])
            .arg(&raw)
            .args(["-map", "0:a:0", "-c:a", "alac", "-threads", "1", "-n"])
            .arg(&source),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    (source, pcm)
}

fn task(
    sandbox: &ConversionRunnerSandbox,
    source: &Path,
    name: &str,
    duplicate: bool,
) -> ConversionTask {
    ConversionTask {
        id: "alac-real".into(),
        file_path: source.to_str().unwrap().into(),
        output_directory: sandbox.root.to_str().unwrap().into(),
        output_name: Some(name.into()),
        config: core_config_from_gpui(&GpuiConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "m4a".into(),
            audio_codec: "copy".into(),
            selected_audio_tracks: if duplicate { vec![0, 0] } else { vec![0] },
            ..GpuiConversionConfig::default()
        }),
    }
}

fn decoded(path: &Path, ordinal: u32) -> Vec<u8> {
    let result = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args(["-v", "error", "-i"])
            .arg(path)
            .args([
                "-map",
                &format!("0:a:{ordinal}"),
                "-c:a",
                "pcm_s32le",
                "-f",
                "s32le",
                "pipe:1",
            ]),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

fn packets(path: &Path, ordinal: u32) -> Vec<serde_json::Value> {
    let result = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffprobe_executable())
            .args([
                "-v",
                "error",
                "-select_streams",
                &format!("a:{ordinal}"),
                "-show_packets",
                "-show_data_hash",
                "sha256",
                "-of",
                "json",
            ])
            .arg(path),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["packets"]
        .as_array()
        .unwrap()
        .clone()
}

fn assert_samples(actual: &[u8], expected: &[u8]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "all decoded samples must be retained"
    );
    assert!(
        actual == expected,
        "first different PCM byte: {:?}",
        actual
            .iter()
            .zip(expected)
            .position(|(actual, expected)| actual != expected)
    );
}

fn has_log(events: &[ConversionEvent], suffix: &str) -> bool {
    events.iter().any(
        |event| matches!(event, ConversionEvent::Log(payload) if payload.line.ends_with(suffix)),
    )
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn alac_copy_should_verify_zero_tails_and_preserve_duplicate_and_tiny_pcm() {
    #[cfg(unix)]
    let _serial = crate::bounded_process::serial_test_processes();
    let sandbox = ConversionRunnerSandbox::new("alac-tail");
    for (samples, extension, repair, duplicate) in [
        (5000, "mka", true, true),
        (1, "mka", true, false),
        (5000, "m4a", false, false),
    ] {
        let (source, expected) = generate_alac(&sandbox, samples, extension);
        assert_samples(&decoded(&source, 0), &expected);
        let source_packets = packets(&source, 0);
        assert_eq!(
            source_packets.last().unwrap()["duration"]
                .as_i64()
                .unwrap_or(0)
                == 0,
            repair
        );
        let output_name = format!("result-{samples}-{extension}.m4a");
        let output = sandbox.path(&output_name);
        let controller = ConversionProcessController::default();
        let mut events = Vec::new();
        run_conversion_task_with_control(
            task(&sandbox, &source, &output_name, duplicate),
            &controller,
            &mut |event| events.push(event),
        )
        .unwrap();
        assert!(matches!(events.last(), Some(ConversionEvent::Completed(_))));
        assert_eq!(
            has_log(&events, "output verified; preparing publication"),
            repair
        );
        for ordinal in 0..=u32::from(duplicate) {
            assert_samples(&decoded(&output, ordinal), &expected);
            let output_packets = packets(&output, ordinal);
            assert_eq!(output_packets.len(), source_packets.len());
            for (actual, expected) in output_packets.iter().zip(&source_packets) {
                assert_eq!(actual["data_hash"], expected["data_hash"]);
                assert_eq!(actual["size"], expected["size"]);
            }
            assert!(output_packets.last().unwrap()["duration"].as_i64().unwrap() > 0);
        }
        assert_eq!(controller.active_process_count().unwrap(), 0);
        assert!(!controller.take_cancelled("alac-real").unwrap());
        assert!(!fs::read_dir(&sandbox.root).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".frame-alac-")
        }));
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn alac_copy_should_cancel_before_publication_and_protect_a_racing_destination() {
    #[cfg(unix)]
    let _serial = crate::bounded_process::serial_test_processes();
    let sandbox = ConversionRunnerSandbox::new("alac-publication");
    let (source, expected) = generate_alac(&sandbox, 5000, "mka");
    let controller = ConversionProcessController::default();
    for cancel in [true, false] {
        let name = if cancel {
            "cancelled.m4a"
        } else {
            "sentinel.m4a"
        };
        let output = sandbox.path(name);
        let task = task(&sandbox, &source, name, false);
        let mut events = Vec::new();
        let result = run_conversion_task_with_control(task.clone(), &controller, &mut |event| {
            if matches!(&event, ConversionEvent::Log(payload) if payload.line.ends_with("output verified; preparing publication"))
            {
                assert_eq!(controller.active_process_count().unwrap(), 0);
                assert!(!output.exists());
                if cancel {
                    controller.cancel_task(&task.id).unwrap();
                } else {
                    fs::write(&output, b"racing destination sentinel").unwrap();
                }
            }
            events.push(event);
        });
        assert!(has_log(&events, "output verified; preparing publication"));
        assert_eq!(result.is_ok(), cancel);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ConversionEvent::Completed(_)))
        );
        assert!(!controller.take_cancelled(&task.id).unwrap());
        if cancel {
            assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
            assert!(!output.exists());
            run_conversion_task_with_control(task, &controller, &mut |_| {}).unwrap();
            assert_samples(&decoded(&output, 0), &expected);
        } else {
            assert_eq!(fs::read(output).unwrap(), b"racing destination sentinel");
        }
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn alac_published_task_should_remain_cancellable_while_its_batch_retry_is_queued() {
    #[cfg(unix)]
    let _serial = crate::bounded_process::serial_test_processes();
    let sandbox = ConversionRunnerSandbox::new("alac-queued-retry");
    let (repair_source, expected) = generate_alac(&sandbox, 5000, "mka");
    let (ordinary_source, _) = generate_alac(&sandbox, 5000, "m4a");
    let controller = ConversionProcessController::default();
    controller.update_max_concurrency(1).unwrap();
    let mut prior = task(&sandbox, &repair_source, "prior-b.m4a", false);
    prior.id = "queued-b".into();
    let mut prior_events = Vec::new();
    run_conversion_task_with_control(prior, &controller, &mut |event| prior_events.push(event))
        .unwrap();
    assert!(has_log(
        &prior_events,
        "output verified; preparing publication"
    ));
    assert!(matches!(
        prior_events.last(),
        Some(ConversionEvent::Completed(_))
    ));
    assert_samples(&decoded(&sandbox.path("prior-b.m4a"), 0), &expected);

    let mut first = task(&sandbox, &ordinary_source, "ordinary-a.m4a", false);
    first.id = "queued-a".into();
    let mut retry = task(&sandbox, &repair_source, "retry-b.m4a", false);
    retry.id = "queued-b".into();
    let mut events = Vec::new();
    run_conversion_batch_with_control(vec![first, retry.clone()], &controller, |event| {
        if matches!(&event, ConversionEvent::Completed(payload) if payload.id == "queued-a") {
            assert_eq!(controller.active_process_count().unwrap(), 0);
            controller.cancel_task("queued-b").unwrap();
        }
        events.push(event);
    })
    .unwrap();
    assert!(events.iter().any(
        |event| matches!(event, ConversionEvent::Completed(payload) if payload.id == "queued-a")
    ));
    assert_eq!(events.iter().filter(|event| matches!(event, ConversionEvent::Cancelled(payload) if payload.id == "queued-b")).count(), 1);
    assert!(!events.iter().any(
        |event| matches!(event, ConversionEvent::Started(payload) if payload.id == "queued-b")
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Error(_)))
    );
    assert!(!sandbox.path("retry-b.m4a").exists());
    assert!(!controller.take_cancelled("queued-b").unwrap());
    assert_samples(&decoded(&sandbox.path("ordinary-a.m4a"), 0), &expected);
    run_conversion_task_with_control(retry, &controller, &mut |_| {}).unwrap();
    assert_samples(&decoded(&sandbox.path("retry-b.m4a"), 0), &expected);
}
