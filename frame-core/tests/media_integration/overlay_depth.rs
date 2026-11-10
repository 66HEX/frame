use super::*;

fn rgba_bytes(pixels: &[[u16; 4]]) -> Vec<u8> {
    pixels
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn write_png(
    tools: &Toolchain,
    sandbox: &Sandbox,
    name: &str,
    size: (u32, u32),
    format: &str,
    bytes: &[u8],
) -> TestResult<PathBuf> {
    let raw = sandbox.path(&format!("{name}.raw"));
    let png = sandbox.path(&format!("{name}.png"));
    fs::write(&raw, bytes).map_err(|error| error.to_string())?;
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "rawvideo",
            "-pixel_format",
            format,
            "-video_size",
            &format!("{}x{}", size.0, size.1),
            "-framerate",
            "25",
            "-i",
            &path_arg(&raw),
            "-frames:v",
            "1",
            "-c:v",
            "png",
            "-update",
            "1",
            &path_arg(&png),
        ]),
    )?;
    Ok(png)
}

fn decode(tools: &Toolchain, path: &Path, format: &str) -> TestResult<Vec<u8>> {
    run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-xerror",
            "-i",
            &path_arg(path),
            "-fps_mode",
            "passthrough",
            "-pix_fmt",
            format,
            "-f",
            "rawvideo",
            "-",
        ]),
    )
}

fn overlay_config(path: &Path, opacity: f64, width: f64) -> OverlayConfig {
    OverlayConfig {
        enabled: true,
        path: path_arg(path),
        x: 0.5,
        y: 0.5,
        width,
        opacity,
        anchor: "center".into(),
    }
}

fn round_ratio(numerator: u64, denominator: u64) -> u16 {
    u16::try_from((2 * numerator + denominator) / (2 * denominator)).unwrap()
}

fn source_over(background: [u16; 4], source: [u16; 4], opacity: u16) -> [u16; 4] {
    const MAX: u64 = 65_535;
    let alpha = (u64::from(source[3]) * u64::from(opacity) + 500) / 1000;
    if alpha == 0 {
        return background;
    }
    let back_alpha = u64::from(background[3]);
    let denominator = alpha * MAX + back_alpha * (MAX - alpha);
    let mut result = [0; 4];
    for channel in 0..3 {
        let numerator = u64::from(source[channel]) * alpha * MAX
            + u64::from(background[channel]) * back_alpha * (MAX - alpha);
        result[channel] = round_ratio(numerator, denominator);
    }
    result[3] = round_ratio(denominator, MAX);
    result
}

fn patterned(count: u32) -> Vec<[u16; 4]> {
    (0..count)
        .map(|index| {
            [
                u16::try_from((index * 997 + 17) % 65_536).unwrap(),
                u16::try_from((index * 313 + 123) % 65_536).unwrap(),
                u16::try_from((index * 197 + 29) % 65_536).unwrap(),
                [0, 1, 32_768, 65_535][usize::try_from(index % 4).unwrap()],
            ]
        })
        .collect()
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn explicit_rgb16_overlay_should_match_integer_source_over_for_every_output_sample() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("overlay_rgb16_exact")?;
    let main = patterned(16 * 8);
    let logo: Vec<_> = (0..16)
        .map(|index| [43_210, 12_345, 54_321, [100, 0, 32_768, 65_535][index % 4]])
        .collect();
    let input = write_png(
        &tools,
        &sandbox,
        "main",
        (16, 8),
        "rgba64le",
        &rgba_bytes(&main),
    )?;
    let overlay = write_png(
        &tools,
        &sandbox,
        "logo",
        (4, 4),
        "rgba64le",
        &rgba_bytes(&logo),
    )?;
    for (container, format) in [
        ("png", "rgba64be"),
        ("png", "rgb48be"),
        ("tiff", "rgba64le"),
        ("tiff", "rgb48le"),
    ] {
        for (opacity, numerator) in [(0.0, 0), (0.145, 145), (1.0, 1000)] {
            let mut expected = main.clone();
            for y in 0..4 {
                for x in 0..4 {
                    let index = (y + 2) * 16 + x + 6;
                    expected[index] = source_over(main[index], logo[y * 4 + x], numerator);
                }
            }
            if format.starts_with("rgb48") {
                for pixel in &mut expected {
                    pixel[3] = u16::MAX;
                }
            }
            let mut config = image_config(container, container);
            config.pixel_format = format.into();
            config.overlay = Some(overlay_config(&overlay, opacity, 0.25));
            let output = sandbox.path(&format!("{format}-{numerator}.{container}"));
            convert(&tools, &input, &output, &config)?;
            assert_bytes_match(
                &decode(&tools, &output, "rgba64le")?,
                &rgba_bytes(&expected),
                &format!("{container}/{format}/{numerator}"),
            )?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn explicit_rgb16_overlay_should_scale_constant_planes_exactly_at_tiny_aspect_limits() -> TestResult
{
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("overlay_rgb16_tiny")?;
    let back = [1111, 2222, 3333, 21_011];
    let front = [43_210, 12_345, 54_321, 32_768];
    for (index, (main_size, logo_size)) in
        [((1, 17), (100, 1)), ((17, 1), (1, 100)), ((3, 5), (5, 7))]
            .into_iter()
            .enumerate()
    {
        let main = vec![back; usize::try_from(main_size.0 * main_size.1).unwrap()];
        let logo = vec![front; usize::try_from(logo_size.0 * logo_size.1).unwrap()];
        let input = write_png(
            &tools,
            &sandbox,
            &format!("main{index}"),
            main_size,
            "rgba64le",
            &rgba_bytes(&main),
        )?;
        let overlay = write_png(
            &tools,
            &sandbox,
            &format!("logo{index}"),
            logo_size,
            "rgba64le",
            &rgba_bytes(&logo),
        )?;
        let mut config = image_config("tiff", "tiff");
        config.pixel_format = "rgba64le".into();
        config.image_tiff_compression = "deflate".into();
        config.overlay = Some(overlay_config(&overlay, 0.145, 0.25));
        let output = sandbox.path(&format!("tiny{index}.tiff"));
        convert(&tools, &input, &output, &config)?;
        // All three discrete aspect limits require exactly a 1x1 logo. The
        // source is constant, so this oracle does not use FFmpeg's scaler.
        let mut expected = main;
        let position = ((main_size.1 - 1) / 2) * main_size.0 + (main_size.0 - 1) / 2;
        expected[usize::try_from(position).unwrap()] = source_over(back, front, 145);
        assert_bytes_match(
            &decode(&tools, &output, "rgba64le")?,
            &rgba_bytes(&expected),
            &format!("tiny plane scale {index}"),
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn selected_rgb16_frames_should_use_precise_overlay_in_export_and_snapshot() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("overlay_dynamic_rgb16")?;
    write_png(
        &tools,
        &sandbox,
        "frame-0",
        (8, 4),
        "rgba",
        &[7, 11, 19, 255].repeat(32),
    )?;
    let main = patterned(32);
    write_png(
        &tools,
        &sandbox,
        "frame-1",
        (8, 4),
        "rgba64le",
        &rgba_bytes(&main),
    )?;
    let input = sandbox.path("dynamic.mov");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-framerate",
            "25",
            "-i",
            &path_arg(&sandbox.path("frame-%d.png")),
            "-c:v",
            "copy",
            &path_arg(&input),
        ]),
    )?;
    assert_eq!(
        probe_media(&tools, &input)?.pixel_format.as_deref(),
        Some("rgba")
    );
    let logo = [[43_210, 12_345, 54_321, 100]; 4];
    let overlay = write_png(
        &tools,
        &sandbox,
        "logo",
        (2, 2),
        "rgba64le",
        &rgba_bytes(&logo),
    )?;
    let mut config = image_config("png", "png");
    config.pixel_format = "rgba64be".into();
    config.start_time = Some("0.04".into());
    config.overlay = Some(overlay_config(&overlay, 0.145, 0.25));
    let output = sandbox.path("selected.png");
    convert(&tools, &input, &output, &config)?;
    let mut expected = main.clone();
    for y in 1..3 {
        for x in 3..5 {
            let index = y * 8 + x;
            expected[index] = source_over(main[index], logo[0], 145);
        }
    }
    assert_bytes_match(
        &decode(&tools, &output, "rgba64le")?,
        &rgba_bytes(&expected),
        "selected dynamic frame",
    )?;

    let options = PreviewFfmpegOptions {
        start_seconds: 0.04,
        end_seconds: None,
        source_width: Some(8),
        source_height: Some(4),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        max_width: 8,
        max_height: 4,
        fps: 25,
        realtime: false,
        precise_seek: true,
        source_is_image: false,
    };
    let plan = frame_core::preview::build_ffmpeg_snapshot_preview_args(
        &path_arg(&input),
        &config,
        &options,
    )
    .map_err(|error| error.to_string())?;
    let mut command = plan.args;
    let sink = command.len() - 1;
    command.splice(sink..sink, ["-frames:v".into(), "1".into()]);
    let preview = run_tool_output(&tools.ffmpeg, &command)?;
    assert_eq!(preview.len(), 8 * 4 * 4);
    assert_bytes_match(
        &preview,
        &decode(&tools, &output, "bgra")?,
        "dynamic snapshot/export pixels",
    )?;

    check_first_dynamic_snapshot(&tools, &sandbox, &input, &overlay, logo[0], &options)
}

fn check_first_dynamic_snapshot(
    tools: &Toolchain,
    sandbox: &Sandbox,
    input: &Path,
    overlay: &Path,
    logo: [u16; 4],
    options: &PreviewFfmpegOptions,
) -> TestResult {
    let mut config = image_config("png", "png");
    config.pixel_format = "rgba64be".into();
    // Frame zero must reach the snapshot before the next decoded PNG changes
    // pixel format. A terminal playback FPS filter buffers and loses it during
    // reinitialization, even when its EOF policy is `pass`.
    let first_options = PreviewFfmpegOptions {
        start_seconds: 0.0,
        ..options.clone()
    };
    let first = [7 * 257, 11 * 257, 19 * 257, 65_535];
    let mut without_overlay = None;
    for (name, opacity) in [("none", None), ("zero", Some(0.0)), ("active", Some(0.145))] {
        config.overlay = opacity.map(|opacity| overlay_config(overlay, opacity, 0.25));
        let mut expected = vec![first; 32];
        if opacity == Some(0.145) {
            for y in 1..3 {
                for x in 3..5 {
                    expected[y * 8 + x] = source_over(first, logo, 145);
                }
            }
        }
        let oracle = write_png(
            tools,
            sandbox,
            &format!("first-oracle-{name}"),
            (8, 4),
            "rgba64le",
            &rgba_bytes(&expected),
        )?;
        let plan = frame_core::preview::build_ffmpeg_snapshot_preview_args(
            &path_arg(input),
            &config,
            &first_options,
        )
        .map_err(|error| error.to_string())?;
        let mut command = plan.args;
        let sink = command.len() - 1;
        command.splice(sink..sink, ["-frames:v".into(), "1".into()]);
        let pixels = run_tool_output(&tools.ffmpeg, &command)?;
        assert_bytes_match(
            &pixels,
            &decode(tools, &oracle, "bgra")?,
            &format!("first dynamic snapshot {name}"),
        )?;
        if name == "none" {
            without_overlay = Some(pixels);
        } else if name == "zero" {
            assert_eq!(Some(pixels), without_overlay);
        }
    }
    Ok(())
}
