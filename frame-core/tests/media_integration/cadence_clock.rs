//! Compare actual frame selection and transport timestamps against the fixture
//! toolchain. Matching only duration would miss a repeated or dropped picture.
use super::*;

fn export_pair(
    tools: &Toolchain,
    sandbox: &Sandbox,
    name: &str,
    container: &str,
    plan: &[String],
) -> TestResult<[PathBuf; 2]> {
    let reference = sandbox.path(&format!("{name}-reference.{container}"));
    let native = sandbox.path(&format!("{name}-native.{container}"));
    let mut args = plan.to_vec();
    args.extend(["-n".into(), path_arg(&reference)]);
    run_tool(&tools.ffmpeg, &args)?;
    *args.last_mut().unwrap() = path_arg(&native);
    tools.native("convert", &args)?;
    Ok([reference, native])
}

fn packet_timing(tools: &Toolchain, path: &Path) -> TestResult<Vec<u8>> {
    run_tool_output(
        &tools.ffprobe,
        &args(&[
            "-v",
            "error",
            "-show_entries",
            "packet=stream_index,pts,dts,duration",
            "-of",
            "csv=p=0",
            &path_arg(path),
        ]),
    )
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn variable_rate_exports_match_decoded_pictures_and_packet_timing() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("export_cadence")?;
    let source = sandbox.path("bursts.mp4");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=96x64:r=100:d=0.3",
            "-vf",
            "setpts='if(lt(N,12),N,50+(N-12)*6)'",
            "-fps_mode",
            "passthrough",
            "-c:v",
            "libx264",
            "-bf",
            "0",
            &path_arg(&source),
        ]),
    )?;
    for (rate, container) in [
        ("original", "mp4"),
        ("24", "mkv"),
        ("30", "mkv"),
        ("60", "mkv"),
    ] {
        let mut plan = args(&[
            "-i",
            &path_arg(&source),
            "-map",
            "0:0",
            "-c:v",
            "libx264",
            "-crf",
            "0",
        ]);
        if rate != "original" {
            plan.extend(args(&["-r", rate]));
        }
        let [reference, native] = export_pair(&tools, &sandbox, rate, container, &plan)?;
        let decode = |path: &Path| {
            run_tool_output(
                &tools.ffmpeg,
                &args(&[
                    "-v",
                    "error",
                    "-i",
                    &path_arg(path),
                    "-map",
                    "0:v:0",
                    "-fps_mode",
                    "passthrough",
                    "-pix_fmt",
                    "yuv420p",
                    "-f",
                    "rawvideo",
                    "pipe:1",
                ]),
            )
        };
        let expected = decode(&reference)?;
        assert!(!expected.is_empty());
        assert_bytes_match(
            &decode(&native)?,
            &expected,
            &format!("{rate} fps pictures"),
        )?;
        assert_eq!(
            packet_timing(&tools, &native)?,
            packet_timing(&tools, &reference)?,
            "{rate} packet timing"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn discontinuous_transport_streams_use_the_clock_of_selected_tracks() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("discontinuous_clock")?;
    let offset = sandbox.path("offset.ts");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-itsoffset",
            "2",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=96x64:r=5:d=3",
            "-f",
            "lavfi",
            "-i",
            "sine=d=4",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "mpeg2video",
            "-bf",
            "0",
            "-c:a",
            "mp2",
            "-muxdelay",
            "0",
            &path_arg(&offset),
        ]),
    )?;
    let gap = sandbox.path("splice.ts");
    let mut bytes = Vec::new();
    for offset in ["0", "22"] {
        let segment = sandbox.path(&format!("part-{offset}.ts"));
        run_tool(
            &tools.ffmpeg,
            &args(&[
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=96x64:r=10:d=1",
                "-c:v",
                "mpeg2video",
                "-bf",
                "0",
                "-muxdelay",
                "0",
                "-output_ts_offset",
                offset,
                &path_arg(&segment),
            ]),
        )?;
        bytes.extend(fs::read(segment).map_err(|e| e.to_string())?);
    }
    fs::write(&gap, bytes).map_err(|e| e.to_string())?;
    for (name, input, start) in [
        ("late", &offset, None),
        ("trim", &offset, Some("0.5")),
        ("splice", &gap, None),
    ] {
        for codec in ["copy", "libx264"] {
            let mut plan = Vec::new();
            if let Some(start) = start {
                plan.extend(args(&["-ss", start]));
            }
            plan.extend(args(&["-i", &path_arg(input)]));
            if input == &offset {
                plan.extend(args(&["-t", "1"]));
            }
            plan.extend(args(&["-map", "0:0", "-c:v", codec]));
            let [reference, native] =
                export_pair(&tools, &sandbox, &format!("{name}-{codec}"), "mp4", &plan)?;
            let expected = packet_timing(&tools, &reference)?;
            assert!(!expected.is_empty());
            assert_eq!(
                packet_timing(&tools, &native)?,
                expected,
                "{name} {codec} timestamps"
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn original_audio_channels_negotiate_the_encoder_layout() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("surround_layouts")?;
    let source = sandbox.path("surround.wav");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.3|0.2|0.1|0.05|0.025|0.0125:c=5.1:s=48000:d=0.3",
            "-c:a",
            "pcm_s16le",
            &path_arg(&source),
        ]),
    )?;
    for (encoder, container, channels) in [
        ("libmp3lame", "mp3", 2),
        ("mp2", "mp2", 2),
        ("libopus", "opus", 6),
    ] {
        let plan = args(&[
            "-i",
            &path_arg(&source),
            "-vn",
            "-map",
            "0:0",
            "-c:a",
            encoder,
            "-b:a",
            "192k",
        ]);
        let [reference, native] = export_pair(&tools, &sandbox, encoder, container, &plan)?;
        for file in [&native, &reference] {
            let facts = probe_media(&tools, file)?;
            assert_eq!(
                facts.audio_tracks[0].channels,
                channels.to_string(),
                "{encoder}"
            );
        }
        let decode = |file: &Path| {
            run_tool_output(
                &tools.ffmpeg,
                &args(&[
                    "-v",
                    "error",
                    "-i",
                    &path_arg(file),
                    "-f",
                    "s16le",
                    "pipe:1",
                ]),
            )
        };
        // Compare encoded layout and decoded length across toolchain versions.
        assert_eq!(decode(&native)?.len(), decode(&reference)?.len());
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn strict_packet_observer_rejects_incomplete_input_without_issuing_completion() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("corrupt_transport_packet")?;
    let original = sandbox.path("original.ts");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=160x96:r=24:d=1",
            "-c:v",
            "mpeg2video",
            "-b:v",
            "2M",
            "-streamid",
            "0:256",
            "-f",
            "mpegts",
            &path_arg(&original),
        ]),
    )?;
    let mut stream = fs::read(&original).map_err(|e| e.to_string())?;
    let index = stream
        .chunks_exact(188)
        .enumerate()
        .skip(30)
        .find(|(_, ts)| ts[0] == 0x47 && ts[1] & 0x5f == 1 && ts[2] == 0 && ts[3] & 0x10 != 0)
        .map(|(index, _)| index)
        .ok_or("missing video payload packet")?;
    stream.drain(index * 188..(index + 1) * 188);
    let corrupt = sandbox.path("corrupt.ts");
    fs::write(&corrupt, stream).map_err(|e| e.to_string())?;
    let marker = sandbox.path("progress");
    for codec in ["copy", "rawvideo"] {
        let plan = args(&[
            "-loglevel",
            "error",
            "-xerror",
            "-progress",
            &path_arg(&marker),
            "-i",
            &path_arg(&corrupt),
            "-map",
            "0:0",
            "-c:v",
            codec,
            "-f",
            "framehash",
            "pipe:1",
        ]);
        assert!(
            run_tool(&tools.ffmpeg, &plan).is_err(),
            "fixture must fail strict inspection"
        );
        assert!(
            tools.native("convert", &plan).is_err(),
            "strict {codec} accepted a corrupt packet"
        );
        assert!(
            !fs::read_to_string(&marker)
                .map_err(|e| e.to_string())?
                .contains("progress=end")
        );
    }
    Ok(())
}
