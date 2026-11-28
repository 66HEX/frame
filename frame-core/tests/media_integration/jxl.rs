use super::*;

#[test]
#[ignore = "requires a JPEG XL-enabled worker and FFmpeg reference tools"]
fn jpeg_xl_should_preserve_lossless_pixels_and_encode_lossy_stills() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("jxl")?;
    let reference_decoders = run_tool_output(&tools.ffmpeg, &args(&["-hide_banner", "-decoders"]))?;
    let reference_decodes_jxl = String::from_utf8_lossy(&reference_decoders)
        .lines()
        .any(|line| matches!(line.split_whitespace().nth(1), Some("jpegxl" | "libjxl")));

    for (format, bytes_per_pixel) in [("rgb24", 3), ("rgba", 4), ("rgba64le", 8)] {
        let mut pixels: Vec<u8> = (0..17 * 9 * bytes_per_pixel)
            .map(|index| u8::try_from((index * 47 + 53) % 256).unwrap())
            .collect();
        if bytes_per_pixel == 4 {
            for (i, pixel) in pixels.chunks_exact_mut(4).enumerate() {
                pixel[3] = [0, 127, 255][i % 3];
            }
        }
        let raw = sandbox.path(&format!("{format}.raw"));
        fs::write(&raw, &pixels).map_err(|error| error.to_string())?;
        let input = sandbox.path(&format!("{format}.png"));
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
                "17x9",
                "-i",
                &path_arg(&raw),
                "-frames:v",
                "1",
                "-c:v",
                "png",
                "-update",
                "1",
                &path_arg(&input),
            ]),
        )?;
        for lossless in [true, false] {
            let mut config = image_config("jxl", "libjxl");
            config.image_jxl_lossless = lossless;
            let output = sandbox.path(&format!("{format}-{lossless}.jxl"));
            convert(&tools, &input, &output, &config)?;
            let metadata = tools.native("probe", &["metadata".into(), path_arg(&output)])?;
            let metadata: serde_json::Value =
                serde_json::from_slice(&metadata).map_err(|e| e.to_string())?;
            let streams = metadata["streams"].as_array().unwrap();
            assert_eq!(streams.len(), 1);
            assert_eq!(streams[0]["codec_name"], "jpegxl");
            assert_eq!(
                (streams[0]["width"].as_u64(), streams[0]["height"].as_u64()),
                (Some(17), Some(9))
            );

            // Some historical macOS reference binaries cannot decode JPEG XL. In
            // that case pass a lossless PNG from the worker to the reference decoder.
            let decoded = if reference_decodes_jxl {
                output
            } else {
                let png = sandbox.path(&format!("{format}-{lossless}.png"));
                convert(&tools, &output, &png, &image_config("png", "png"))?;
                png
            };
            let result = run_tool_output(
                &tools.ffmpeg,
                &args(&[
                    "-v",
                    "error",
                    "-xerror",
                    "-i",
                    &path_arg(&decoded),
                    "-pix_fmt",
                    format,
                    "-f",
                    "rawvideo",
                    "-",
                ]),
            )?;
            assert_eq!(result.len(), pixels.len(), "one decoded picture");
            if lossless {
                assert_bytes_match(&result, &pixels, format)?;
            } else {
                assert_ne!(result, pixels, "{format} lossy export");
                if format == "rgba" {
                    assert!(
                        result
                            .chunks_exact(4)
                            .zip(pixels.chunks_exact(4))
                            .all(|(a, b)| a[3] == b[3])
                    );
                }
            }
        }
    }
    Ok(())
}
