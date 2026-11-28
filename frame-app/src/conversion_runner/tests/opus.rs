use super::*;
use frame_core::error::ConversionError;

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn native_opus_trim_rejects_incomplete_selected_packet_timing_before_export() {
    let sandbox = ConversionRunnerSandbox::new("opus-copy-timing");
    for origin in ["0", "5"] {
        let input = sandbox.path(&format!("source-{origin}.mov"));
        generate_opus_source(&input, "nut", origin);
        let original = fs::read(&input).unwrap();
        for (label, start, end) in [
            ("end", None, "0.2130009"),
            ("late", Some("1.4570009"), "1.8000009"),
        ] {
            for (processing_mode, selected) in [
                (ProcessingMode::Copy, vec![0]),
                (ProcessingMode::Copy, vec![1, 0, 1]),
                (ProcessingMode::CopyAudio, vec![0]),
            ] {
                let name = format!(
                    "{origin}-{label}-{}-{}.opus",
                    processing_mode.id(),
                    selected.len()
                );
                let output = sandbox.path(&name);
                let config = GpuiConversionConfig {
                    container: "opus".into(),
                    processing_mode,
                    audio_codec: "copy".into(),
                    selected_audio_tracks: selected,
                    start_time: start.map(Into::into),
                    end_time: Some(end.into()),
                    ..GpuiConversionConfig::default()
                };
                let task = opus_task(&sandbox, &input, &name, &config);
                let controller = ConversionProcessController::default();
                let mut events = Vec::new();
                let result = run_conversion_task_with_control(task, &controller, &mut |event| {
                    events.push(event);
                });
                assert!(
                    matches!(&result, Err(ConversionError::InvalidInput(message)) if message.contains("incomplete packet timing")),
                    "native trimmed Opus copy must fail before losing source audio: {result:?}"
                );
                assert!(!output.exists());
                assert!(!events.iter().any(|event| matches!(
                    event,
                    ConversionEvent::Started(_) | ConversionEvent::Completed(_)
                )));
                assert_eq!(controller.active_process_count().unwrap(), 0);
                assert_eq!(fs::read(&input).unwrap(), original);

                let full = GpuiConversionConfig {
                    start_time: None,
                    end_time: None,
                    ..config
                };
                run_conversion_task_with_control(
                    opus_task(&sandbox, &input, &name, &full),
                    &controller,
                    &mut |_| {},
                )
                .expect("full Copy retry must remain supported");
                assert!(output.is_file());
            }
        }
    }
}

fn opus_task(
    sandbox: &ConversionRunnerSandbox,
    input: &Path,
    output_name: &str,
    config: &GpuiConversionConfig,
) -> ConversionTask {
    ConversionTask {
        id: "opus-copy-timing".into(),
        file_path: input.to_str().unwrap().into(),
        output_directory: sandbox.root.to_str().unwrap().into(),
        output_name: Some(output_name.into()),
        config: core_config_from_gpui(config),
    }
}

fn generate_opus_source(output: &Path, format: &str, origin: &str) {
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=2.2",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=880:sample_rate=48000:duration=1.32",
                "-map",
                "0:a",
                "-map",
                "1:a",
                "-c:a",
                "libopus",
                "-frame_duration:a:0",
                "20",
                "-frame_duration:a:1",
                "60",
                "-output_ts_offset",
                origin,
                "-f",
                format,
                "-n",
            ])
            .arg(output),
    )
    .unwrap();
    assert!(status.success(), "Opus source generation failed: {status}");
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn opus_preflight_preserves_healthy_sources_reencode_and_other_remuxes() {
    let sandbox = ConversionRunnerSandbox::new("opus-copy-controls");
    for (format, extension) in [
        ("ogg", "opus"),
        ("matroska", "mkv"),
        ("webm", "webm"),
        ("nut", "mov"),
    ] {
        let input = sandbox.path(&format!("source.{extension}"));
        generate_opus_source(&input, format, "0");
        for (label, processing_mode, container, audio_codec, start) in [
            ("end", ProcessingMode::Copy, "opus", "copy", None),
            (
                "seek",
                ProcessingMode::Copy,
                "opus",
                "copy",
                Some("0.4570009"),
            ),
            (
                "encode",
                ProcessingMode::Reencode,
                "opus",
                "libopus",
                Some("0.4570009"),
            ),
        ] {
            if format == "nut" && matches!(label, "end" | "seek") {
                continue;
            }
            let name = format!("{format}-{label}.{container}");
            let config = GpuiConversionConfig {
                container: container.into(),
                processing_mode,
                audio_codec: audio_codec.into(),
                selected_audio_tracks: vec![1, 0, 1],
                start_time: start.map(Into::into),
                end_time: Some("0.8130009".into()),
                ..GpuiConversionConfig::default()
            };
            let mut completed = false;
            run_conversion_task(opus_task(&sandbox, &input, &name, &config), |event| {
                completed |= matches!(event, ConversionEvent::Completed(_));
            })
            .unwrap_or_else(|error| panic!("{format}/{label} should remain supported: {error}"));
            assert!(completed);
            assert!(sandbox.path(&name).is_file());
        }
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn opus_preflight_preserves_existing_video_remuxes() {
    let sandbox = ConversionRunnerSandbox::new("opus-video-remux");
    let audio = sandbox.path("audio.opus");
    generate_opus_source(&audio, "ogg", "0");
    let input = sandbox.path("source.mkv");
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=size=32x32:rate=5:duration=2.2",
                "-i",
            ])
            .arg(&audio)
            .args([
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:a",
                "copy",
                "-n",
            ])
            .arg(&input),
    )
    .unwrap();
    assert!(status.success());
    let config = GpuiConversionConfig {
        container: "mkv".into(),
        processing_mode: ProcessingMode::Copy,
        audio_codec: "copy".into(),
        selected_audio_tracks: vec![2, 1, 2],
        start_time: Some("0.4570009".into()),
        end_time: Some("0.8130009".into()),
        ..GpuiConversionConfig::default()
    };
    let mut completed = false;
    run_conversion_task(opus_task(&sandbox, &input, "remux.mkv", &config), |event| {
        completed |= matches!(event, ConversionEvent::Completed(_));
    })
    .expect("existing video remux must remain supported");
    assert!(completed);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn opus_timing_proof_cannot_be_reused_after_the_source_changes() {
    let sandbox = ConversionRunnerSandbox::new("opus-copy-changed");
    let input = sandbox.path("source.mkv");
    generate_opus_source(&input, "matroska", "0");
    let config = GpuiConversionConfig {
        container: "opus".into(),
        processing_mode: ProcessingMode::Copy,
        audio_codec: "copy".into(),
        selected_audio_tracks: vec![0],
        end_time: Some("0.8130009".into()),
        ..GpuiConversionConfig::default()
    };
    let mut changed = false;
    let mut events = Vec::new();
    let result = run_conversion_task(
        opus_task(&sandbox, &input, "changed.opus", &config),
        |event| {
            if matches!(&event, ConversionEvent::Log(log) if log.line.starts_with("[INFO] Running "))
            {
                use std::io::Write;
                fs::OpenOptions::new()
                    .append(true)
                    .open(&input)
                    .unwrap()
                    .write_all(b"changed")
                    .unwrap();
                changed = true;
            }
            events.push(event);
        },
    );
    assert!(changed);
    assert!(
        matches!(result, Err(ConversionError::InvalidInput(message)) if message.contains("changed"))
    );
    assert!(!sandbox.path("changed.opus").exists());
    assert!(!events.iter().any(|event| matches!(
        event,
        ConversionEvent::Started(_) | ConversionEvent::Completed(_)
    )));
    assert_eq!(fs::read_dir(&sandbox.root).unwrap().count(), 1);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; generates one hour of compressed silence"]
fn a_small_opus_trim_near_the_end_of_a_long_source_fits_the_scan_budget() {
    let sandbox = ConversionRunnerSandbox::new("opus-long-small-window");
    let input = sandbox.path("hour.opus");
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=48000:cl=mono",
                "-t",
                "3600",
                "-c:a",
                "libopus",
                "-b:a",
                "12k",
                "-n",
            ])
            .arg(&input),
    )
    .unwrap();
    assert!(status.success());
    let config = GpuiConversionConfig {
        container: "opus".into(),
        processing_mode: ProcessingMode::Copy,
        audio_codec: "copy".into(),
        selected_audio_tracks: vec![0],
        start_time: Some("3599.4570009".into()),
        end_time: Some("3599.8000009".into()),
        ..GpuiConversionConfig::default()
    };
    let mut completed = false;
    run_conversion_task(
        opus_task(&sandbox, &input, "short.opus", &config),
        |event| {
            completed |= matches!(event, ConversionEvent::Completed(_));
        },
    )
    .expect("a short trim must not scan the entire hour of packets");
    assert!(completed);
}
