use super::*;
use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits};
use std::ffi::OsString;
use std::time::Duration;

fn run(executable: String, args: Vec<OsString>) -> Vec<u8> {
    static DOMAIN: ExecutionDomain = ExecutionDomain::new();
    let mut command = CommandSpec::new(executable);
    command.args.extend(
        crate::runtime_binaries::worker_subcommand(&command.program, "preview")
            .map(Into::<OsString>::into),
    );
    command.args.extend(args);
    let output = DOMAIN
        .run(
            &command,
            &[],
            Limits {
                timeout: Duration::from_secs(15),
                cleanup_timeout: Duration::from_secs(1),
                stdout_bytes: 32 * 1024 * 1024,
                stderr_bytes: 64 * 1024,
            },
            || false,
        )
        .unwrap();
    assert!(
        output.success,
        "{}",
        String::from_utf8_lossy(&output.stderr.bytes)
    );
    output.stdout.bytes
}

fn variation(bytes: &[u8], width: usize, height: usize) -> u64 {
    let frames: Vec<_> = bytes.chunks_exact(width * height * 4).collect();
    frames[8..]
        .windows(2)
        .map(|pair| {
            (32..height - 32)
                .flat_map(|y| (32..width - 32).map(move |x| (y * width + x) * 4))
                .map(|i| u64::from(pair[0][i].abs_diff(pair[1][i])))
                .sum::<u64>()
        })
        .sum()
}

#[test]
#[ignore = "requires the bundled FFmpeg runtime"]
#[expect(
    clippy::too_many_lines,
    reason = "The independent pixel oracle exercises preview and published export on the same translated source."
)]
fn stabilization_runtime_reduces_translation_jitter_with_production_preview_plan() {
    #[cfg(unix)]
    let _serial = crate::bounded_process::serial_test_processes();
    let directory =
        std::env::temp_dir().join(format!("frame-stabilization-pixels-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let (width, height) = (320_u32, 240_u32);
    for index in 0..32 {
        let offset = [0_u32, 6, 2, 8, 0, 4, 8, 2][index % 8];
        image::RgbImage::from_fn(width, height, |x, y| {
            let x = x + offset;
            let value = u8::try_from((x.wrapping_mul(31) ^ y.wrapping_mul(47) ^ (x * y / 7)) % 256)
                .unwrap();
            image::Rgb([value, value, value])
        })
        .save(directory.join(format!("{index:02}.png")))
        .unwrap();
    }
    let input = directory.join("shake.mkv");
    run(
        crate::runtime_binaries::ffmpeg_executable(),
        ["-v", "error", "-framerate", "25", "-i"]
            .map(OsString::from)
            .into_iter()
            .chain([directory.join("%02d.png").into_os_string()])
            .chain(["-c:v", "ffv1"].map(Into::into))
            .chain([input.clone().into_os_string()])
            .collect(),
    );
    let options = frame_core::preview::PreviewFfmpegOptions {
        start_seconds: 0.0,
        end_seconds: None,
        source_width: Some(width),
        source_height: Some(height),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        max_width: width,
        max_height: height,
        fps: 25,
        realtime: false,
        precise_seek: false,
        source_is_image: false,
    };
    let mut config = config();
    let mut outputs = Vec::new();
    for enabled in [false, true] {
        config.video_filters.stabilization_enabled = enabled;
        check_support(
            &config,
            &crate::runtime_binaries::media_worker_executable(),
            || false,
        )
        .unwrap();
        let plan = frame_core::preview::build_ffmpeg_preview_args(
            input.to_str().unwrap(),
            &config,
            &options,
        )
        .unwrap();
        assert_eq!((plan.width, plan.height), (width, height));
        let bytes = run(
            crate::runtime_binaries::media_worker_executable(),
            plan.args.into_iter().map(Into::into).collect(),
        );
        assert_eq!(bytes.len(), 32 * plan.frame_bytes);
        outputs.push(bytes);
    }
    let original = variation(&outputs[0], width as usize, height as usize);
    let stabilized = variation(&outputs[1], width as usize, height as usize);
    eprintln!(
        "stabilization central-region temporal absolute difference: off={original}, on={stabilized}"
    );
    let mut exported = Vec::new();
    for enabled in [false, true] {
        config.video_filters.stabilization_enabled = enabled;
        let name = if enabled { "stabilized" } else { "original" };
        let task = frame_core::types::ConversionTask {
            id: name.into(),
            file_path: input.to_str().unwrap().into(),
            output_directory: directory.to_str().unwrap().into(),
            output_name: Some(name.into()),
            config: config.clone(),
        };
        let output = frame_core::args::build_output_path(
            &task.output_directory,
            &task.config.container,
            task.output_name.as_deref(),
        );
        let mut completed = false;
        crate::conversion_runner::run_conversion_task(task, |event| {
            if matches!(event, ConversionEvent::Completed(_)) {
                completed = true;
            }
        })
        .unwrap();
        assert!(completed, "export must reach published completion");
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
        config.video_filters.stabilization_enabled = false;
        let plan =
            frame_core::preview::build_ffmpeg_preview_args(&output, &config, &options).unwrap();
        let pixels = run(
            crate::runtime_binaries::media_worker_executable(),
            plan.args.into_iter().map(Into::into).collect(),
        );
        assert_eq!(pixels.len(), 32 * plan.frame_bytes);
        exported.push(variation(&pixels, width as usize, height as usize));
    }
    eprintln!(
        "stabilization published export central-region temporal absolute difference: off={}, on={}",
        exported[0], exported[1]
    );
    assert!(
        exported[1] * 5 < exported[0] * 4,
        "published export must reduce jitter by at least 20%"
    );
    std::fs::remove_dir_all(directory).unwrap();
    assert!(original > 0);
    assert!(
        stabilized * 5 < original * 4,
        "expected at least 20% less jitter: off={original}, on={stabilized}"
    );
}
