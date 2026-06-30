#![expect(
    clippy::float_cmp,
    reason = "Conversion runner tests compare exact parsed metadata fixture values."
)]

use super::*;
use crate::settings::{CropSettings, MetadataConfig, MetadataMode, ProcessingMode};
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

    let task = conversion_task_from_file(&file);

    assert_eq!(task.output_name.as_deref(), Some("final cut.mp4"));
    assert_eq!(task.file_path, "/tmp/source.mov");
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
