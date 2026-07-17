#![expect(
    clippy::float_cmp,
    reason = "Conversion runner tests compare exact parsed metadata fixture values."
)]

use super::*;
use crate::settings::{
    AudioFiltersConfig, CropSettings, DeinterlaceMode, FilterStrength, FilterValue, MetadataConfig,
    MetadataMode, ProcessingMode, VideoColorFiltersConfig, VideoFiltersConfig,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This regression test intentionally keeps a full conversion config fixture in one assertion flow."
)]
fn core_config_from_gpui_preserves_active_conversion_fields() {
    let config = GpuiConversionConfig {
        processing_mode: ProcessingMode::Copy,
        container: "mov".to_string(),
        audio_codec: "aac".to_string(),
        audio_bitrate: "192".to_string(),
        audio_bitrate_mode: "bitrate".to_string(),
        audio_quality: "4".to_string(),
        audio_channels: "stereo".to_string(),
        audio_volume: 125,
        audio_normalize: true,
        video_filters: VideoFiltersConfig {
            color: VideoColorFiltersConfig {
                brightness: FilterValue {
                    enabled: true,
                    value: 15,
                },
                contrast: FilterValue {
                    enabled: true,
                    value: 120,
                },
                saturation: FilterValue {
                    enabled: true,
                    value: 80,
                },
                gamma: FilterValue {
                    enabled: true,
                    value: 110,
                },
            },
            hue: FilterValue {
                enabled: true,
                value: -30,
            },
            temperature: FilterValue {
                enabled: true,
                value: 4200,
            },
            sharpen: FilterValue {
                enabled: true,
                value: 40,
            },
            gaussian_blur: FilterValue {
                enabled: true,
                value: 10,
            },
            denoise_enabled: true,
            denoise_strength: FilterStrength::High,
            deband: FilterValue {
                enabled: true,
                value: 50,
            },
            vignette: FilterValue {
                enabled: true,
                value: 35,
            },
            grayscale: true,
            deinterlace: DeinterlaceMode::Auto,
        },
        audio_filters: AudioFiltersConfig {
            compressor_enabled: true,
            compressor_strength: FilterStrength::Low,
            limiter: FilterValue {
                enabled: true,
                value: -3,
            },
            bass: FilterValue {
                enabled: true,
                value: 6,
            },
            treble: FilterValue {
                enabled: true,
                value: -4,
            },
            high_pass: FilterValue {
                enabled: true,
                value: 120,
            },
            low_pass: FilterValue {
                enabled: true,
                value: 15_000,
            },
            noise_reduction: FilterValue {
                enabled: true,
                value: 10,
            },
            de_esser: FilterValue {
                enabled: true,
                value: 45,
            },
            stereo_width: FilterValue {
                enabled: true,
                value: 125,
            },
        },
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:15.000".to_string()),
        metadata: MetadataConfig {
            mode: MetadataMode::Replace,
            title: Some("Render Title".to_string()),
            artist: Some("Frame".to_string()),
            ..MetadataConfig::default()
        },
        subtitle_burn_path: Some("/tmp/dialogue.srt".to_string()),
        subtitle_font_name: Some("Arial".to_string()),
        subtitle_font_size: Some("24".to_string()),
        subtitle_font_color: Some("#ffffff".to_string()),
        subtitle_outline_color: Some("#000000".to_string()),
        subtitle_position: Some("bottom".to_string()),
        rotation: "90".to_string(),
        flip_horizontal: true,
        flip_vertical: true,
        crop: Some(CropSettings {
            enabled: true,
            x: 10,
            y: 20,
            width: 300,
            height: 200,
            source_width: Some(1920),
            source_height: Some(1080),
            aspect_ratio: Some("16:9".to_string()),
        }),
        overlay: None,
        selected_audio_tracks: vec![1, 2],
        selected_subtitle_tracks: vec![3],
        video_codec: "libx265".to_string(),
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: "9000".to_string(),
        resolution: "custom".to_string(),
        custom_width: Some("1920".to_string()),
        custom_height: Some("1080".to_string()),
        scaling_algorithm: "lanczos".to_string(),
        fps: "30".to_string(),
        crf: 18,
        quality: 60,
        preset: "slow".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        image_jpeg_quality: 92,
        image_jpeg_huffman: "optimal".to_string(),
        image_webp_lossless: true,
        image_webp_quality: 88,
        image_webp_compression: 6,
        image_webp_preset: "photo".to_string(),
        image_png_compression: 8,
        image_png_prediction: "mixed".to_string(),
        image_tiff_compression: "deflate".to_string(),
        gif_colors: 128,
        gif_dither: "floyd_steinberg".to_string(),
        gif_loop: 3,
        nvenc_spatial_aq: false,
        nvenc_temporal_aq: false,
        videotoolbox_allow_sw: false,
        hw_decode: false,
    };

    let core = core_config_from_gpui(&config);

    assert_eq!(core.processing_mode, "copy");
    assert_eq!(core.container, "mov");
    assert_eq!(core.audio_bitrate, "192");
    assert_eq!(core.audio_channels, "stereo");
    assert_eq!(core.audio_volume, 125.0);
    assert!(core.audio_normalize);
    assert_eq!(core.video_codec, "libx265");
    assert_eq!(core.video_bitrate_mode, "bitrate");
    assert_eq!(core.video_bitrate, "9000");
    assert_eq!(core.resolution, "custom");
    assert_eq!(core.custom_width.as_deref(), Some("1920"));
    assert_eq!(core.custom_height.as_deref(), Some("1080"));
    assert_eq!(core.scaling_algorithm, "lanczos");
    assert_eq!(core.fps, "30");
    assert_eq!(core.crf, 18);
    assert_eq!(core.quality, 60);
    assert_eq!(core.preset, "slow");
    assert_eq!(core.pixel_format, "yuv420p10le");
    assert_eq!(core.image_jpeg_quality, 92);
    assert_eq!(core.image_jpeg_huffman, "optimal");
    assert!(core.image_webp_lossless);
    assert_eq!(core.image_webp_quality, 88);
    assert_eq!(core.image_webp_compression, 6);
    assert_eq!(core.image_webp_preset, "photo");
    assert_eq!(core.image_png_compression, 8);
    assert_eq!(core.image_png_prediction, "mixed");
    assert_eq!(core.image_tiff_compression, "deflate");
    assert_eq!(core.gif_colors, 128);
    assert_eq!(core.gif_dither, "floyd_steinberg");
    assert_eq!(core.gif_loop, 3);
    assert_eq!(core.start_time.as_deref(), Some("00:00:05.000"));
    assert_eq!(core.end_time.as_deref(), Some("00:00:15.000"));
    assert_eq!(core.rotation, "90");
    assert!(core.flip_horizontal);
    assert!(core.flip_vertical);
    assert_eq!(core.selected_audio_tracks, [1, 2]);
    assert_eq!(core.selected_subtitle_tracks, [3]);
    assert_eq!(
        core.subtitle_burn_path.as_deref(),
        Some("/tmp/dialogue.srt")
    );
    assert_eq!(core.subtitle_font_name.as_deref(), Some("Arial"));
    assert_eq!(core.subtitle_font_size.as_deref(), Some("24"));
    assert_eq!(core.subtitle_font_color.as_deref(), Some("#ffffff"));
    assert_eq!(core.subtitle_outline_color.as_deref(), Some("#000000"));
    assert_eq!(core.subtitle_position.as_deref(), Some("bottom"));
    assert_eq!(core.crop.as_ref().map(|crop| crop.width), Some(300.0));
    assert_eq!(core.metadata.mode, frame_core::types::MetadataMode::Replace);
    assert_eq!(core.metadata.title.as_deref(), Some("Render Title"));
    assert_eq!(core.metadata.artist.as_deref(), Some("Frame"));
}

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

#[test]
fn ffmpeg_progress_uses_duration_line_before_time_line() {
    let mut duration = None;

    assert_eq!(
        ffmpeg_progress_from_line("Duration: 00:00:20.00, start: 0.000000", 0.0, &mut duration),
        None
    );

    let progress =
        ffmpeg_progress_from_line("frame=12 time=00:00:05.00 speed=1x", 0.0, &mut duration);

    assert_eq!(progress, Some(25.0));
}

#[test]
fn ffmpeg_progress_prefers_trim_expected_duration() {
    let mut duration = Some(100.0);

    let progress =
        ffmpeg_progress_from_line("frame=12 time=00:00:05.00 speed=1x", 10.0, &mut duration);

    assert_eq!(progress, Some(50.0));
}

#[test]
fn controller_tracks_registered_process_pid() {
    let controller = ConversionProcessController::default();

    controller
        .register_started_process("task-1", 0)
        .expect("pid registration should succeed");

    assert_eq!(controller.active_pid("task-1"), Some(0));
}

#[test]
fn controller_tracks_registered_process_start_time() {
    let controller = ConversionProcessController::default();
    let pid = std::process::id();

    controller
        .register_started_process("task-1", pid)
        .expect("pid registration should succeed");

    assert!(
        controller
            .active_start_time("task-1")
            .is_some_and(|time| time > 0),
        "registered process should keep a start-time guard"
    );
}

#[test]
fn controller_uses_shared_default_max_concurrency() {
    let controller = ConversionProcessController::default();

    assert_eq!(
        controller
            .current_max_concurrency()
            .expect("default max concurrency should be readable"),
        DEFAULT_MAX_CONCURRENCY
    );
}

#[test]
fn controller_update_max_concurrency_rejects_zero() {
    let controller = ConversionProcessController::default();

    let error = controller
        .update_max_concurrency(0)
        .expect_err("zero concurrency should be rejected");

    assert!(error.to_string().contains("at least 1"));
}

#[test]
fn controller_update_max_concurrency_stores_live_limit() {
    let controller = ConversionProcessController::default();

    controller
        .update_max_concurrency(4)
        .expect("valid max concurrency should be stored");

    assert_eq!(
        controller
            .current_max_concurrency()
            .expect("max concurrency should be readable"),
        4
    );
}

#[test]
fn controller_finish_task_reports_cancelled_state() {
    let controller = ConversionProcessController::default();
    controller
        .register_started_process("task-1", 0)
        .expect("pid registration should succeed");
    controller
        .cancel_task("task-1")
        .expect("cancelling pid zero should not signal an OS process");

    let was_cancelled = controller
        .finish_task("task-1")
        .expect("finishing task should succeed");

    assert!(was_cancelled);
}

#[test]
fn controller_register_started_process_reports_pre_cancelled_task() {
    let controller = ConversionProcessController::default();
    controller
        .cancel_task("task-1")
        .expect("pre-cancel should succeed without an active process");

    let was_cancelled = controller
        .register_started_process("task-1", 0)
        .expect("pid registration should succeed");

    assert!(was_cancelled);
    assert!(
        controller
            .finish_task("task-1")
            .expect("finishing task should clean process state")
    );
    assert_eq!(controller.active_pid("task-1"), None);
}

#[test]
fn run_conversion_task_with_control_emits_cancelled_when_cancelled_before_validation() {
    let controller = ConversionProcessController::default();
    controller
        .cancel_task("task-1")
        .expect("pre-cancel should succeed without an active process");
    let task = ConversionTask {
        id: "task-1".to_string(),
        file_path: "/definitely/missing.mov".to_string(),
        output_directory: "/tmp/frame-output".to_string(),
        output_name: None,
        config: core_config_from_gpui(&GpuiConversionConfig::default()),
    };
    let mut events = Vec::new();

    let result = run_conversion_task_with_control(task, &controller, &mut |event| {
        events.push(event);
    });

    assert!(result.is_ok());
    assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
}

#[test]
fn run_conversion_batch_with_control_accepts_empty_batches() {
    let controller = ConversionProcessController::default();
    let mut events = Vec::new();

    let result = run_conversion_batch_with_control(Vec::new(), &controller, |event| {
        events.push(event);
    });

    assert!(result.is_ok());
    assert!(events.is_empty());
}

#[test]
fn next_batch_launch_count_respects_live_concurrency_limit() {
    assert_eq!(next_batch_launch_count(5, 1, 2), 1);
    assert_eq!(next_batch_launch_count(5, 2, 2), 0);
    assert_eq!(next_batch_launch_count(1, 0, 4), 1);
}

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

struct ConversionRunnerSandbox {
    root: PathBuf,
    keep: bool,
}

impl ConversionRunnerSandbox {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "frame-runner-test-{}-{}-{now}",
            std::process::id(),
            name
        ));
        fs::create_dir_all(&root).expect("runner temp directory should be created");
        Self {
            root,
            keep: std::env::var_os("FRAME_KEEP_MEDIA_TESTS").is_some(),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for ConversionRunnerSandbox {
    fn drop(&mut self) {
        if self.keep {
            eprintln!(
                "keeping conversion runner test artifacts in {}",
                self.root.display()
            );
            return;
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn generate_runner_source(output: &Path) {
    let status = Command::new(crate::runtime_binaries::ffmpeg_executable())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=12:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=0.5",
            "-shortest",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
            "-y",
        ])
        .arg(output)
        .status()
        .expect("ffmpeg should start to generate runner fixture");

    assert!(
        status.success(),
        "ffmpeg fixture generation failed with {status}"
    );
}

fn generate_runner_image_source(output: &Path) {
    let status = Command::new(crate::runtime_binaries::ffmpeg_executable())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=1",
            "-frames:v",
            "1",
            "-update",
            "1",
            "-y",
        ])
        .arg(output)
        .status()
        .expect("ffmpeg should start to generate image fixture");

    assert!(
        status.success(),
        "ffmpeg image fixture generation failed with {status}"
    );
}
