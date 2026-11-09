use super::*;
use frame_core::{
    args::{GifFramePolicy, build_ffmpeg_args_with_gif_policy, build_gif_probe_args},
    filters::FrameRateEofAction,
    preview::build_ffmpeg_snapshot_preview_args,
};

fn probe_frames(
    tools: &Toolchain,
    input: &Path,
    config: &ConversionConfig,
    mode: FrameRateEofAction,
) -> TestResult<u64> {
    let metadata = probe_media(tools, input)?;
    let argv = build_gif_probe_args(&path_arg(input), config, &metadata, mode)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "expected GIF frame probe".to_string())?;
    let stdout = String::from_utf8(run_tool_output(&tools.ffmpeg, &argv)?)
        .map_err(|error| error.to_string())?;
    assert!(
        stdout.lines().last() == Some("progress=end"),
        "incomplete progress: {stdout}"
    );
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("frame="))
        .next_back()
        .ok_or_else(|| "missing frame proof".to_string())?
        .trim()
        .parse()
        .map_err(|error| format!("invalid frame count: {error}"))
}

fn png_source(tools: &Toolchain, sandbox: &Sandbox) -> TestResult<PathBuf> {
    let ppm = sandbox.path("source.ppm");
    let png = sandbox.path("source.png");
    write_solid_ppm(&ppm, 64, 48, Rgb::RED)?;
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(&ppm),
            "-frames:v",
            "1",
            "-update",
            "1",
            &path_arg(&png),
        ]),
    )?;
    Ok(png)
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn gif_minimum_frame_proof_should_recover_a_still_and_distinguish_empty_trim() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("gif_minimum_frame")?;
    let input = png_source(&tools, &sandbox)?;
    let mut config = image_config("gif", "gif");
    config.fps = "8".into();
    config.crop = Some(CropConfig {
        enabled: true,
        x: 3.0,
        y: 5.0,
        width: 31.0,
        height: 33.0,
        ..CropConfig::default()
    });
    validate_task_input(&path_arg(&input), &config).map_err(|error| error.to_string())?;
    assert_eq!(
        probe_frames(&tools, &input, &config, FrameRateEofAction::Round)?,
        0
    );
    assert_eq!(
        probe_frames(&tools, &input, &config, FrameRateEofAction::Pass)?,
        1
    );
    let output = sandbox.path("recovered.gif");
    let argv = build_ffmpeg_args_with_gif_policy(
        &path_arg(&input),
        &path_arg(&output),
        &config,
        &probe_media(&tools, &input)?,
        GifFramePolicy::ConfirmedMinimumFrame,
    )
    .map_err(|error| error.to_string())?;
    run_tool(&tools.ffmpeg, &argv)?;
    let metadata = probe_media(&tools, &output)?;
    assert_eq!((metadata.width, metadata.height), (Some(31), Some(33)));
    let pixels = run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(&output),
            "-fps_mode",
            "passthrough",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "pipe:1",
        ]),
    )?;
    let expected = [255, 0, 0].repeat(31 * 33);
    assert_bytes_match(&pixels, &expected, "single recovered GIF frame")?;
    config.start_time = Some("2".into());
    assert_eq!(
        probe_frames(&tools, &input, &config, FrameRateEofAction::Round)?,
        0
    );
    assert_eq!(
        probe_frames(&tools, &input, &config, FrameRateEofAction::Pass)?,
        0
    );
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn gif_probe_should_preserve_normal_animation_and_one_frame_video_duration() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("gif_ordinary_frames")?;
    for source_frames in [1, 4] {
        let input = sandbox.path(&format!("video-{source_frames}.mkv"));
        run_tool(
            &tools.ffmpeg,
            &args(&[
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=64x48:r=4",
                "-frames:v",
                &source_frames.to_string(),
                "-c:v",
                "ffv1",
                &path_arg(&input),
            ]),
        )?;
        let mut config = image_config("gif", "gif");
        config.fps = "8".into();
        assert_eq!(
            probe_frames(&tools, &input, &config, FrameRateEofAction::Round)?,
            1
        );
        let output = sandbox.path(&format!("ordinary-{source_frames}.gif"));
        convert(&tools, &input, &output, &config)?;
        let pixels = run_tool_output(
            &tools.ffmpeg,
            &args(&[
                "-v",
                "error",
                "-i",
                &path_arg(&output),
                "-fps_mode",
                "passthrough",
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "pipe:1",
            ]),
        )?;
        assert_eq!(
            pixels.len(),
            source_frames * 2 * 64 * 48 * 4,
            "one source frame at 4 fps retains its 0.25-second duration"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn snapshot_without_pacing_should_keep_one_frame_without_changing_the_stream_plan() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("snapshot_eof_frame")?;
    let input = png_source(&tools, &sandbox)?;
    let mut config = image_config("gif", "gif");
    config.fps = "8".into();
    let options = PreviewFfmpegOptions {
        start_seconds: 0.0,
        end_seconds: None,
        source_width: Some(64),
        source_height: Some(48),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        max_width: 64,
        max_height: 48,
        fps: 8,
        realtime: false,
        precise_seek: true,
        source_is_image: true,
    };
    let ordinary = build_ffmpeg_preview_args(&path_arg(&input), &config, &options)
        .map_err(|error| error.to_string())?;
    let snapshot = build_ffmpeg_snapshot_preview_args(&path_arg(&input), &config, &options)
        .map_err(|error| error.to_string())?;
    assert_eq!(snapshot.pixel_aspect_ratio, ordinary.pixel_aspect_ratio);
    assert_eq!(
        snapshot.output_raster_aspect_ratio,
        ordinary.output_raster_aspect_ratio
    );
    assert_eq!(snapshot.frame_bytes, ordinary.frame_bytes);
    let decode = |mut argv: Vec<String>| {
        let index = argv.len() - 1;
        argv.splice(index..index, ["-frames:v".into(), "1".into()]);
        run_tool_output(&tools.ffmpeg, &argv)
    };
    assert!(decode(ordinary.args)?.is_empty());
    let pixels = decode(snapshot.args)?;
    assert_bytes_match(
        &pixels,
        &[0, 0, 255, 255].repeat(64 * 48),
        "snapshot BGRA at EOF",
    )?;
    Ok(())
}
