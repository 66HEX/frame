use super::*;

const COMPRESSIONS: [(&str, u16); 4] =
    [("packbits", 32_773), ("raw", 1), ("lzw", 5), ("deflate", 8)];

fn raw_png(
    tools: &Toolchain,
    sandbox: &Sandbox,
    name: &str,
    format: &str,
    pixels: &[u8],
) -> TestResult<PathBuf> {
    let raw = sandbox.path(&format!("{name}.raw"));
    let png = sandbox.path(&format!("{name}.png"));
    fs::write(&raw, pixels).map_err(|error| error.to_string())?;
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
            "8x4",
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

fn palette_png(tools: &Toolchain, sandbox: &Sandbox, name: &str) -> TestResult<(PathBuf, Vec<u8>)> {
    let mut raw: Vec<u8> = (0..32).collect();
    // FFmpeg's PAL8 palette is native-endian ARGB; supported targets are LE.
    for index in 0..256_u32 {
        for component in [index * 13, index * 29, index * 47, index] {
            raw.push(u8::try_from(component % 256).unwrap());
        }
    }
    let expected = (0..32_u32)
        .flat_map(|index| {
            [index * 47, index * 29, index * 13, index]
                .map(|value| u8::try_from(value % 256).unwrap())
        })
        .collect();
    let png = raw_png(tools, sandbox, name, "pal8", &raw)?;
    assert_eq!(
        probe_media(tools, &png)?.pixel_format.as_deref(),
        Some("pal8")
    );
    Ok((png, expected))
}

fn rgba16_pixels() -> Vec<u8> {
    (0..128_u32)
        .flat_map(|value| {
            u16::try_from((997 * value + 17) % 65_536)
                .unwrap()
                .to_le_bytes()
        })
        .collect()
}

fn decoded_pixels(tools: &Toolchain, path: &Path, format: &str) -> TestResult<Vec<u8>> {
    run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-xerror",
            "-i",
            &path_arg(path),
            "-frames:v",
            "1",
            "-pix_fmt",
            format,
            "-f",
            "rawvideo",
            "-",
        ]),
    )
}

/// Reads the actual TIFF Compression SHORT tag, independently of the encoder args.
fn compression_tag(path: &Path) -> TestResult<u16> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let little = match bytes.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Err("not a TIFF header".into()),
    };
    let word = |offset: usize| -> TestResult<u16> {
        let pair: [u8; 2] = bytes
            .get(offset..offset + 2)
            .ok_or("truncated TIFF word")?
            .try_into()
            .unwrap();
        Ok(if little {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        })
    };
    let dword = |offset: usize| -> TestResult<u32> {
        let value: [u8; 4] = bytes
            .get(offset..offset + 4)
            .ok_or("truncated TIFF dword")?
            .try_into()
            .unwrap();
        Ok(if little {
            u32::from_le_bytes(value)
        } else {
            u32::from_be_bytes(value)
        })
    };
    assert_eq!(word(2)?, 42);
    let directory = usize::try_from(dword(4)?).map_err(|error| error.to_string())?;
    for index in 0..usize::from(word(directory)?) {
        let entry = directory + 2 + index * 12;
        if word(entry)? == 259 {
            assert_eq!((word(entry + 2)?, dword(entry + 4)?), (3, 1));
            return word(entry + 8);
        }
    }
    Err("TIFF Compression tag is missing".into())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn tiff_compression_should_preserve_native_gray16_and_rgba16_samples() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("tiff_native_depth")?;
    let gray: Vec<u8> = (0..32_u16)
        .flat_map(|value| (value * 2003).to_le_bytes())
        .collect();
    for (format, pixels) in [("gray16le", gray), ("rgba64le", rgba16_pixels())] {
        let input = raw_png(&tools, &sandbox, format, format, &pixels)?;
        for (compression, expected_tag) in COMPRESSIONS {
            let mut config = image_config("tiff", "tiff");
            config.image_tiff_compression = compression.into();
            let output = sandbox.path(&format!("{format}-{compression}.tiff"));
            convert(&tools, &input, &output, &config)?;
            assert_eq!(compression_tag(&output)?, expected_tag, "{compression}");
            assert_eq!(
                probe_media(&tools, &output)?.pixel_format.as_deref(),
                Some(format)
            );
            assert_bytes_match(
                &decoded_pixels(&tools, &output, format)?,
                &pixels,
                &format!("{format}/{compression}"),
            )?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn palette_tiff_should_match_true_color_alpha_through_geometry_and_color_controls() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("tiff_palette_alpha")?;
    let (input, rgba) = palette_png(&tools, &sandbox, "palette")?;
    let reference = raw_png(&tools, &sandbox, "reference", "rgba", &rgba)?;
    for effect in ["none", "geometry", "color"] {
        let mut config = image_config("tiff", "tiff");
        if effect == "geometry" {
            config.crop = Some(CropConfig {
                enabled: true,
                x: 1.0,
                y: 1.0,
                width: 6.0,
                height: 2.0,
                ..CropConfig::default()
            });
            config.resolution = "custom".into();
            config.custom_width = Some("12".into());
            config.custom_height = Some("4".into());
        } else if effect == "color" {
            config.video_filters.color.brightness.enabled = true;
            config.video_filters.color.brightness.value = 10;
            config.video_filters.color.saturation.enabled = true;
            config.video_filters.color.saturation.value = 120;
        }
        // A true-color source is an independent alpha reference for palette
        // expansion. Active 8-bit RGB/YUV rounding remains the known R9 behavior.
        let graph = build_visual_filter_chain_with_context(
            &config,
            VisualFilterProfile::ExportImage,
            VisualFilterContext::new(Some(8), Some(4)),
        )
        .join(",");
        let expected = run_tool_output(
            &tools.ffmpeg,
            &args(&[
                "-v",
                "error",
                "-i",
                &path_arg(&reference),
                "-vf",
                &graph,
                "-frames:v",
                "1",
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "-",
            ]),
        )?;
        assert!(!expected.is_empty());
        if effect == "none" {
            assert_bytes_match(&expected, &rgba, "palette source reference")?;
        }
        for (compression, _) in COMPRESSIONS {
            config.image_tiff_compression = compression.into();
            let output = sandbox.path(&format!("{effect}-{compression}.tiff"));
            convert(&tools, &input, &output, &config)?;
            assert_eq!(
                probe_media(&tools, &output)?.pixel_format.as_deref(),
                Some("rgba")
            );
            assert_bytes_match(
                &decoded_pixels(&tools, &output, "rgba")?,
                &expected,
                &format!("palette {effect}/{compression}"),
            )?;
        }
    }
    Ok(())
}

fn one_preview_frame(
    tools: &Toolchain,
    path: &Path,
    config: &ConversionConfig,
    options: &PreviewFfmpegOptions,
) -> TestResult<Vec<u8>> {
    let plan = build_ffmpeg_preview_args(&path_arg(path), config, options)
        .map_err(|error| error.to_string())?;
    let mut args = plan.args;
    let index = args.len() - 1;
    args.insert(index, "-frames:v".into());
    args.insert(index + 1, "1".into());
    let pixels = run_tool_output(&tools.ffmpeg, &args)?;
    assert_eq!(pixels.len(), plan.frame_bytes);
    Ok(pixels)
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn tiff_palette_stream_seek_should_preserve_a_later_rgba16_frame_and_preview() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("tiff_dynamic_palette")?;
    palette_png(&tools, &sandbox, "frame-0")?;
    let expected = rgba16_pixels();
    let later_frame = raw_png(&tools, &sandbox, "frame-1", "rgba64le", &expected)?;
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
        Some("pal8")
    );
    let mut config = image_config("tiff", "tiff");
    config.start_time = Some("0.04".into());
    for compression in ["raw", "deflate"] {
        config.image_tiff_compression = compression.into();
        let output = sandbox.path(&format!("later-{compression}.tiff"));
        convert(&tools, &input, &output, &config)?;
        assert_eq!(
            probe_media(&tools, &output)?.pixel_format.as_deref(),
            Some("rgba64le")
        );
        assert_bytes_match(
            &decoded_pixels(&tools, &output, "rgba64le")?,
            &expected,
            "selected later 16-bit samples",
        )?;
    }
    let options = PreviewFfmpegOptions {
        start_seconds: 0.04,
        end_seconds: None,
        source_width: Some(8),
        source_height: Some(4),
        source_is_palette: true,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        max_width: 8,
        max_height: 4,
        fps: 25,
        realtime: false,
        precise_seek: true,
        source_is_image: false,
    };
    let preview = one_preview_frame(&tools, &input, &config, &options)?;
    let reference = one_preview_frame(
        &tools,
        &later_frame,
        &config,
        &PreviewFfmpegOptions {
            start_seconds: 0.0,
            source_is_palette: false,
            source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
            ..options
        },
    )?;
    assert_bytes_match(&preview, &reference, "selected later preview frame")?;
    Ok(())
}
