use super::*;

fn first_frame(tools: &Toolchain, path: &Path) -> TestResult<Vec<u8>> {
    run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(path),
            "-frames:v",
            "1",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "pipe:1",
        ]),
    )
}

fn first_video_pts(tools: &Toolchain, path: &Path) -> TestResult<f64> {
    let bytes = run_tool_output(
        &tools.ffprobe,
        &args(&[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_packets",
            "-show_entries",
            "packet=pts_time",
            "-of",
            "json",
            &path_arg(path),
        ]),
    )?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    json["packets"][0]["pts_time"]
        .as_str()
        .ok_or_else(|| "missing first video PTS".to_string())?
        .parse::<f64>()
        .map_err(|error| error.to_string())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn negative_input_seek_should_burn_source_zero_and_restore_output_timestamps() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("negative_subtitle_clock")?;
    let input = sandbox.path("source.mp4");
    // RGB avoids unrelated YUV/RGB conversion differences in the overlay graph.
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:size=320x180:rate=12:duration=2,format=rgb24",
            "-c:v",
            "libx264rgb",
            "-crf",
            "0",
            "-preset",
            "ultrafast",
            &path_arg(&input),
        ]),
    )?;
    let subtitle = sandbox.path("source-clock.srt");
    fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:00,300\nSOURCE ZERO\n\n2\n00:00:00,400 --> 00:00:01,900\nLATER FRAME\n").map_err(|error| error.to_string())?;
    let overlay = sandbox.path("black.ppm");
    write_solid_ppm(&overlay, 16, 16, Rgb::BLACK)?;
    for with_overlay in [false, true] {
        let mut config = video_config("mkv", "libx264", "flac");
        config.crf = 0;
        config.subtitle_burn_path = Some(path_arg(&subtitle));
        config.subtitle_font_size = Some("24".into());
        if with_overlay {
            config.overlay = Some(OverlayConfig {
                enabled: true,
                path: path_arg(&overlay),
                x: 0.05,
                y: 0.05,
                width: 0.05,
                opacity: 1.0,
                anchor: "center".into(),
            });
        }
        let reference = sandbox.path(&format!("reference-{with_overlay}.mkv"));
        convert(&tools, &input, &reference, &config)?;
        let expected = first_frame(&tools, &reference)?;
        assert_eq!(expected.len(), 320 * 180 * 3);
        assert!(expected.iter().any(|byte| *byte > 35));
        for (index, start) in ["-.5", "-00:00:00.5", "-1.5"].into_iter().enumerate() {
            config.start_time = Some(start.into());
            config.end_time = Some("1.5".into());
            let output = sandbox.path(&format!("negative-{with_overlay}-{index}.mkv"));
            convert(&tools, &input, &output, &config)?;
            assert_bytes_match(
                &first_frame(&tools, &output)?,
                &expected,
                "negative seek source-zero subtitle",
            )?;
            let expected_pts = if index == 2 { 1.5 } else { 0.5 };
            assert!(
                (first_video_pts(&tools, &output)? - expected_pts).abs() < 0.000_001,
                "subtitle clock must restore the existing negative-seek timestamp offset"
            );
        }
    }
    Ok(())
}

#[derive(Debug)]
struct DiagnosticFrames {
    time_base: String,
    pts: Vec<i64>,
    hashes: Vec<String>,
}

fn diagnostic_frames(
    tools: &Toolchain,
    input: &Path,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
) -> TestResult<DiagnosticFrames> {
    let production = build_ffmpeg_args(&path_arg(input), "unused.mkv", config, probe)
        .map_err(|error| error.to_string())?;
    let input_position = production
        .iter()
        .position(|value| value == "-i")
        .ok_or_else(|| "missing actual input".to_string())?;
    // Keep the actual input seek and verbatim production filter chain. Only the
    // diagnostic sink changes, so encoder/muxer FPS rounding cannot hide drift.
    let filter = production
        .windows(2)
        .find(|pair| pair[0] == "-vf")
        .map_or("null", |pair| pair[1].as_str());
    let mut command = args(&["-v", "error"]);
    command.extend_from_slice(&production[..input_position + 2]);
    command.extend(args(&[
        "-an",
        "-sn",
        "-dn",
        "-vf",
        filter,
        "-c:v",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-fps_mode",
        "passthrough",
        "-enc_time_base",
        "filter",
        "-f",
        "framehash",
        "pipe:1",
    ]));
    let output = run_tool_output(&tools.ffmpeg, &command)?;
    let output = String::from_utf8(output).map_err(|error| error.to_string())?;
    let mut result = DiagnosticFrames {
        time_base: String::new(),
        pts: Vec::new(),
        hashes: Vec::new(),
    };
    for line in output.lines() {
        if line.starts_with("#tb ") {
            result.time_base = line.to_string();
        }
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err(format!("unexpected framehash row: {line}"));
        }
        result.pts.push(
            fields[2]
                .parse::<i64>()
                .map_err(|error| error.to_string())?,
        );
        result.hashes.push(fields[5].to_string());
    }
    if result.time_base.is_empty() || result.hashes.len() < 20 {
        return Err(format!(
            "incomplete framehash diagnostic: {} frames",
            result.hashes.len()
        ));
    }
    Ok(result)
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn fractional_burn_clock_should_match_every_source_frame_and_preserve_filter_pts() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("fractional_source_clock")?;
    let input = sandbox.path("source.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            // Isolate subtitle clock boundaries from bwdif's missing temporal
            // history after accurate seek; changing captions still expose drift.
            "color=black:size=320x180:rate=20:duration=3",
            "-c:v",
            "ffv1",
            "-g",
            "1",
            &path_arg(&input),
        ]),
    )?;
    let probe = probe_media(&tools, &input)?;
    let subtitle = sandbox.path("boundaries.srt");
    fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:00,300\nZERO\n\n2\n00:00:00,500 --> 00:00:00,800\nHALF\n\n3\n00:00:01,000 --> 00:00:01,300\nONE\n\n4\n00:00:02,000 --> 00:00:02,300\nTWO\n")
        .map_err(|error| error.to_string())?;
    for deinterlace in [
        frame_core::types::DeinterlaceMode::Off,
        frame_core::types::DeinterlaceMode::On,
    ] {
        let mut config = video_config("mkv", "libx264", "flac");
        config.video_filters.deinterlace = deinterlace;
        config.subtitle_burn_path = Some(path_arg(&subtitle));
        let reference = diagnostic_frames(&tools, &input, &config, &probe)?;
        assert_eq!(reference.hashes.len(), 60);
        assert_eq!(
            reference.time_base,
            if deinterlace == frame_core::types::DeinterlaceMode::Off {
                "#tb 0: 1/1000"
            } else {
                "#tb 0: 1/2000"
            },
            "the active temporal filter must change the time base"
        );
        for start in ["-.123456789", ".123456789", "-.123500", ".123500"] {
            config.start_time = Some(start.into());
            let actual = diagnostic_frames(&tools, &input, &config, &probe)?;
            assert_eq!(
                actual.hashes.len(),
                if start.starts_with('-') { 60 } else { 57 },
                "exact decoded frame count at {start}, {deinterlace:?}"
            );
            let first = reference.hashes.len() - actual.hashes.len();
            let mismatch = actual
                .hashes
                .iter()
                .zip(&reference.hashes[first..])
                .position(|(actual, expected)| actual != expected);
            assert!(
                mismatch.is_none(),
                "source-clock frame {mismatch:?} differs at {start}, {deinterlace:?} ({} frames)",
                actual.hashes.len()
            );
            let mut no_burn = config.clone();
            no_burn.subtitle_burn_path = None;
            let unchanged = diagnostic_frames(&tools, &input, &no_burn, &probe)?;
            assert_eq!(actual.time_base, unchanged.time_base);
            assert_eq!(
                actual.pts, unchanged.pts,
                "pre-encode PTS at {start}, {deinterlace:?}"
            );
        }
    }
    Ok(())
}
