use super::*;

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn run_conversion_task_should_emit_completed_for_real_ffmpeg_job() {
    let sandbox = ConversionRunnerSandbox::new("real-ffmpeg-job");
    let input = sandbox.path("source.mp4");
    let output_name = "runner-output.mp4";
    let output = sandbox.path(output_name);
    generate_runner_source(&input);
    let task = ConversionTask {
        id: "task-real".to_string(),
        file_path: input.to_string_lossy().into_owned(),
        output_directory: sandbox.root.to_string_lossy().into_owned(),
        output_name: Some(output_name.to_string()),
        config: core_config_from_gpui(&GpuiConversionConfig::default()),
    };
    let mut events = Vec::new();

    run_conversion_task(task, |event| events.push(event))
        .expect("real ffmpeg conversion should succeed");

    assert!(output.is_file(), "{} should be created", output.display());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Started(_))),
        "runner should emit a Started event"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Completed(payload) if payload.output_path == output.to_string_lossy())),
        "runner should emit a Completed event for {}",
        output.display()
    );
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn run_conversion_task_should_emit_completed_for_real_image_encoding_job() {
    let sandbox = ConversionRunnerSandbox::new("real-image-encoding-job");
    let input = sandbox.path("source.png");
    let output_name = "runner-output.webp";
    let output = sandbox.path(output_name);
    generate_runner_image_source(&input);
    let config = GpuiConversionConfig {
        container: "webp".to_string(),
        video_codec: "libwebp".to_string(),
        image_webp_lossless: true,
        image_webp_quality: 90,
        image_webp_compression: 6,
        image_webp_preset: "photo".to_string(),
        ..GpuiConversionConfig::default()
    };
    let task = ConversionTask {
        id: "task-image-real".to_string(),
        file_path: input.to_string_lossy().into_owned(),
        output_directory: sandbox.root.to_string_lossy().into_owned(),
        output_name: Some(output_name.to_string()),
        config: core_config_from_gpui(&config),
    };
    let mut events = Vec::new();

    run_conversion_task(task, |event| events.push(event))
        .expect("real ffmpeg image conversion should succeed");

    assert!(output.is_file(), "{} should be created", output.display());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Completed(payload) if payload.output_path == output.to_string_lossy())),
        "runner should emit a Completed event for {}",
        output.display()
    );
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn run_conversion_batch_should_create_distinct_outputs_for_same_stem_sources() {
    let sandbox = ConversionRunnerSandbox::new("duplicate-real-outputs");
    let output_directory = sandbox.path("exports");
    let first_input = sandbox.path("A/clip.mov");
    let second_input = sandbox.path("B/clip.mkv");
    fs::create_dir_all(
        first_input
            .parent()
            .expect("first source should have a parent directory"),
    )
    .expect("first source directory should be created");
    fs::create_dir_all(
        second_input
            .parent()
            .expect("second source should have a parent directory"),
    )
    .expect("second source directory should be created");
    fs::create_dir_all(&output_directory).expect("output directory should be created");
    generate_runner_source(&first_input);
    generate_runner_source(&second_input);
    let first = FileItem::from_os_path("mov", &first_input);
    let second = FileItem::from_os_path("mkv", &second_input);
    let output_directory_string = output_directory.to_string_lossy();
    let tasks = vec![
        conversion_task_from_file(&first, &output_directory_string),
        conversion_task_from_file(&second, &output_directory_string),
    ];
    let controller = ConversionProcessController::default();
    controller
        .update_max_concurrency(2)
        .expect("concurrency should be updated");
    let mut events = Vec::new();

    run_conversion_batch_with_control(tasks, &controller, |event| events.push(event))
        .expect("duplicate-name batch should finish");

    let first_output = output_directory.join("clip_converted.mp4");
    let second_output = output_directory.join("clip_converted_2.mp4");
    let mut completed_paths = events
        .iter()
        .filter_map(|event| match event {
            ConversionEvent::Completed(payload) => Some(payload.output_path.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    completed_paths.sort();
    let mut expected_paths = vec![
        first_output.to_string_lossy().into_owned(),
        second_output.to_string_lossy().into_owned(),
    ];
    expected_paths.sort();

    assert_eq!(
        (
            first_output.is_file(),
            second_output.is_file(),
            completed_paths
        ),
        (true, true, expected_paths)
    );
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn run_conversion_task_should_preserve_existing_output_and_use_suffix() {
    let sandbox = ConversionRunnerSandbox::new("preserve-existing-output");
    let input = sandbox.path("source.mov");
    let output_directory = sandbox.path("exports");
    fs::create_dir_all(&output_directory).expect("output directory should be created");
    generate_runner_source(&input);
    let existing_output = output_directory.join("protected.mp4");
    let suffixed_output = output_directory.join("protected_2.mp4");
    fs::write(&existing_output, b"keep this data")
        .expect("protected output fixture should be written");
    let mut file = FileItem::from_os_path("protected", &input);
    file.output_name = "protected".to_string();
    let task = conversion_task_from_file(&file, &output_directory.to_string_lossy());
    let mut events = Vec::new();

    run_conversion_task(task, |event| events.push(event))
        .expect("conversion beside an existing output should succeed");

    let completed_path = events.iter().find_map(|event| match event {
        ConversionEvent::Completed(payload) => Some(payload.output_path.as_str()),
        _ => None,
    });
    assert_eq!(
        (
            fs::read(&existing_output).expect("protected output should remain readable"),
            suffixed_output.is_file(),
            completed_path
        ),
        (
            b"keep this data".to_vec(),
            true,
            Some(suffixed_output.to_string_lossy().as_ref())
        )
    );
}
