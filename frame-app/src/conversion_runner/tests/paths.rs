use super::*;

#[test]
fn conversion_task_from_file_sanitizes_output_name() {
    let mut file = FileItem::from_path("file-1", "/tmp/source.mov", 1);
    file.output_name = "/tmp/export/final cut.mp4".to_string();

    let task = conversion_task_from_file(&file, "/tmp/frame-output");

    assert_eq!(task.output_name.as_deref(), Some("final cut.mp4"));
    assert_eq!(task.file_path, "/tmp/source.mov");
    assert_eq!(task.output_directory, "/tmp/frame-output");
}

#[test]
fn disambiguate_output_paths_suffixes_same_stem_files_from_different_directories() {
    let sandbox = ConversionRunnerSandbox::new("duplicate-output-names");
    let output_directory = sandbox.root.to_string_lossy();
    let first = FileItem::from_path("mov", "/A/clip.mov", 1);
    let second = FileItem::from_path("mkv", "/B/clip.mkv", 1);
    let mut tasks = vec![
        conversion_task_from_file(&first, &output_directory),
        conversion_task_from_file(&second, &output_directory),
    ];

    disambiguate_output_paths(&mut tasks);

    assert_eq!(
        (
            tasks[0].output_name.as_deref(),
            tasks[1].output_name.as_deref()
        ),
        (Some("clip_converted"), Some("clip_converted_2"))
    );
}

#[test]
fn disambiguate_output_paths_preserves_single_non_conflicting_name() {
    let sandbox = ConversionRunnerSandbox::new("single-output-name");
    let file = FileItem::from_path("mov", "/A/clip.mov", 1);
    let mut tasks = vec![conversion_task_from_file(
        &file,
        &sandbox.root.to_string_lossy(),
    )];

    disambiguate_output_paths(&mut tasks);

    assert_eq!(tasks[0].output_name.as_deref(), Some("clip_converted"));
}

#[test]
fn disambiguate_output_paths_skips_existing_files() {
    let sandbox = ConversionRunnerSandbox::new("existing-output-name");
    fs::write(sandbox.path("clip_converted.mp4"), b"keep")
        .expect("existing output fixture should be written");
    let file = FileItem::from_path("mov", "/A/clip.mov", 1);
    let mut tasks = vec![conversion_task_from_file(
        &file,
        &sandbox.root.to_string_lossy(),
    )];

    disambiguate_output_paths(&mut tasks);

    assert_eq!(tasks[0].output_name.as_deref(), Some("clip_converted_2"));
}

#[test]
fn disambiguate_output_paths_keeps_transport_stream_public_suffix() {
    let sandbox = ConversionRunnerSandbox::new("existing-m2ts-output-name");
    fs::write(sandbox.path("clip_converted.m2ts"), b"keep")
        .expect("existing M2TS output fixture should be written");
    let mut file = FileItem::from_path("mts", "/A/clip.MTS", 1);
    file.config.container = "m2ts".to_string();
    let mut tasks = vec![conversion_task_from_file(
        &file,
        &sandbox.root.to_string_lossy(),
    )];

    disambiguate_output_paths(&mut tasks);

    assert_eq!(tasks[0].output_name.as_deref(), Some("clip_converted_2"));
    assert_eq!(tasks[0].config.container, "m2ts");
}

#[test]
fn disambiguate_output_paths_uses_next_free_suffix_deterministically() {
    let sandbox = ConversionRunnerSandbox::new("occupied-output-suffixes");
    fs::write(sandbox.path("clip_converted.mp4"), b"keep")
        .expect("base output fixture should be written");
    fs::write(sandbox.path("clip_converted_2.mp4"), b"keep")
        .expect("suffixed output fixture should be written");
    let first = FileItem::from_path("mov", "/A/clip.mov", 1);
    let second = FileItem::from_path("mkv", "/B/clip.mkv", 1);
    let output_directory = sandbox.root.to_string_lossy();
    let mut tasks = vec![
        conversion_task_from_file(&first, &output_directory),
        conversion_task_from_file(&second, &output_directory),
    ];

    disambiguate_output_paths(&mut tasks);

    assert_eq!(
        (
            tasks[0].output_name.as_deref(),
            tasks[1].output_name.as_deref()
        ),
        (Some("clip_converted_3"), Some("clip_converted_4"))
    );
}
