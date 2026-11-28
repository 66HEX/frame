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
    export_pair_with_reference_options(tools, sandbox, name, container, plan, &[])
}

fn export_pair_with_reference_options(
    tools: &Toolchain,
    sandbox: &Sandbox,
    name: &str,
    container: &str,
    plan: &[String],
    reference_options: &[&str],
) -> TestResult<[PathBuf; 2]> {
    let reference = sandbox.path(&format!("{name}-reference.{container}"));
    let native = sandbox.path(&format!("{name}-native.{container}"));
    let mut reference_args = plan.to_vec();
    reference_args.extend(args(reference_options));
    reference_args.extend(["-n".into(), path_arg(&reference)]);
    run_tool(&tools.ffmpeg, &reference_args)?;
    let mut native_args = plan.to_vec();
    native_args.extend(["-n".into(), path_arg(&native)]);
    tools.native("convert", &native_args)?;
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
        .skip(stream.len() / (2 * 188))
        .find(|(_, ts)| ts[0] == 0x47 && ts[1] & 0x5f == 1 && ts[2] == 0 && ts[3] & 0x10 != 0)
        .map(|(index, _)| index)
        .ok_or("missing video payload packet")?;
    stream[index * 188 + 1] |= 0x80; // MPEG-TS transport error indicator
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

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn chapter_tags_follow_metadata_policy_and_keep_the_chapter_interval() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("chapter_metadata")?;
    let source = sandbox.path("source.mkv");
    let chapter = sandbox.path("chapter.txt");
    fs::write(
        &chapter,
        b";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=500\ntitle=Private chapter\n",
    )
    .map_err(|e| e.to_string())?;
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=64x48:r=4:d=1",
            "-f",
            "ffmetadata",
            "-i",
            &path_arg(&chapter),
            "-map",
            "0:v:0",
            "-map_chapters",
            "1",
            "-c:v",
            "libx264",
            &path_arg(&source),
        ]),
    )?;
    for mode in ["preserve", "replace", "clean"] {
        let mut plan = args(&["-i", &path_arg(&source), "-map", "0:0", "-c", "copy"]);
        if mode != "preserve" {
            plan.extend(args(&["-map_metadata", "-1"]));
        }
        if mode == "replace" {
            plan.extend(args(&["-metadata", "title=Public"]));
        }
        let [reference, native] = export_pair(&tools, &sandbox, mode, "mkv", &plan)?;
        let inspect = |p: &Path| {
            run_tool_output(
                &tools.ffprobe,
                &args(&["-v", "error", "-show_chapters", "-of", "json", &path_arg(p)]),
            )
        };
        let expected = inspect(&reference)?;
        assert!(String::from_utf8_lossy(&expected).contains("0.500000"));
        assert_eq!(
            String::from_utf8_lossy(&expected).contains("Private chapter"),
            mode == "preserve"
        );
        assert_eq!(inspect(&native)?, expected);
    }
    Ok(())
}

#[test]
#[ignore = "requires HEVC/AV1 fixture tools; run with --ignored"]
fn heic_heif_and_avif_stills_match_reference_rgb_pixels() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("heif_family_import")?;
    let still = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/input.heic");
    let alternate_extension = sandbox.path("input.heif");
    fs::copy(&still, &alternate_extension).map_err(|e| e.to_string())?;
    let avif = sandbox.path("input.avif");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=128x96:r=1:d=1",
            "-frames:v",
            "1",
            "-c:v",
            "libsvtav1",
            "-preset",
            "10",
            "-crf",
            "24",
            "-f",
            "avif",
            &path_arg(&avif),
        ]),
    )?;
    for path in [&still, &alternate_extension, &avif] {
        let output = sandbox.path(&format!(
            "{}.png",
            path.file_name().unwrap().to_string_lossy()
        ));
        let mut config = image_config("png", "png");
        config.pixel_format = "rgb24".into();
        convert(&tools, path, &output, &config)?;
        let pixels = |p: &Path| {
            run_tool_output(
                &tools.ffmpeg,
                &args(&[
                    "-v",
                    "error",
                    "-i",
                    &path_arg(p),
                    "-map",
                    "0:v:0",
                    "-frames:v",
                    "1",
                    "-pix_fmt",
                    "rgb24",
                    "-f",
                    "rawvideo",
                    "pipe:1",
                ]),
            )
        };
        let expected = pixels(path)?;
        assert_eq!(expected.len(), 128 * 96 * 3);
        assert_bytes_match(&pixels(&output)?, &expected, &path_arg(path))?;
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn default_audio_track_follows_the_map_order_when_source_has_no_default() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("default_audio_disposition")?;
    let source = sandbox.path("source.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=32x32:r=4:d=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=d=0.5",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "1:a",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-disposition:a:0",
            "0",
            "-disposition:a:1",
            "0",
            &path_arg(&source),
        ]),
    )?;
    let plan = args(&[
        "-i",
        &path_arg(&source),
        "-map",
        "0:0",
        "-map",
        "0:2",
        "-map",
        "0:1",
        "-c",
        "copy",
    ]);
    let [reference, native] = export_pair(&tools, &sandbox, "tracks", "mkv", &plan)?;
    let inspect = |p: &Path| {
        run_tool_output(
            &tools.ffprobe,
            &args(&[
                "-v",
                "error",
                "-select_streams",
                "a",
                "-show_entries",
                "stream_disposition=default",
                "-of",
                "csv=p=0",
                &path_arg(p),
            ]),
        )
    };
    assert_eq!(inspect(&reference)?, b"1\n0\n");
    assert_eq!(inspect(&native)?, inspect(&reference)?);
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn many_mapped_streams_share_input_descriptors() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("mapped_descriptors")?;
    let source = sandbox.path("80-tracks.mkv");
    let mut generate = args(&["-f", "lavfi", "-i", "sine=d=0.03"]);
    for _ in 0..80 {
        generate.extend(args(&["-map", "0:a:0"]));
    }
    generate.extend(args(&["-c:a", "aac", &path_arg(&source)]));
    run_tool(&tools.ffmpeg, &generate)?;
    let wave = sandbox.path("tone.wav");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "sine=d=0.03",
            "-c:a",
            "pcm_s16le",
            &path_arg(&wave),
        ]),
    )?;
    let worker = tools.worker.as_ref().unwrap();
    for codec in ["copy", "flac"] {
        for (input, duplicate, input_codec) in [
            (&source, false, "aac"),
            (&source, true, "aac"),
            (&wave, true, "pcm_s16le"),
        ] {
            let output = sandbox.path(&format!("{input_codec}-{codec}-{duplicate}.mkv"));
            #[cfg(unix)]
            let mut command = {
                let mut cmd = Command::new("sh");
                cmd.args(["-c", r#"ulimit -n 64 && exec "$@""#, "frame-test"])
                    .arg(worker);
                cmd
            };
            #[cfg(not(unix))]
            let mut command = Command::new(worker);
            command
                .args(["convert", "-loglevel", "error", "-i"])
                .arg(input);
            for i in 0..80 {
                command
                    .arg("-map")
                    .arg(format!("0:{}", if duplicate { 0 } else { i }));
            }
            let result = command
                .args(["-c:a", codec, "-n"])
                .arg(&output)
                .output()
                .map_err(|e| e.to_string())?;
            assert!(
                result.status.success(),
                "{codec} duplicate={duplicate}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let streams = probe_media(&tools, &output)?.audio_tracks;
            assert_eq!(streams.len(), 80);
            assert!(
                streams
                    .iter()
                    .all(|s| s.codec == if codec == "copy" { input_codec } else { "flac" })
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn changing_audio_format_keeps_samples_in_partial_encoder_blocks() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("dynamic_audio_blocks")?;
    let mut adts = Vec::new();
    for (rate, channels, hz) in [(44100, 1, 440), (48000, 2, 880)] {
        let part = sandbox.path(&format!("{rate}.aac"));
        run_tool(
            &tools.ffmpeg,
            &args(&[
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={hz}:duration=1.6:sample_rate={rate}"),
                "-ac",
                &channels.to_string(),
                "-c:a",
                "aac",
                "-f",
                "adts",
                &path_arg(&part),
            ]),
        )?;
        adts.extend(fs::read(part).map_err(|e| e.to_string())?);
    }
    let elementary = sandbox.path("changes.aac");
    fs::write(&elementary, adts).map_err(|e| e.to_string())?;
    let source = sandbox.path("changes.m2t");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-i",
            &path_arg(&elementary),
            "-c:a",
            "copy",
            "-f",
            "mpegts",
            &path_arg(&source),
        ]),
    )?;
    for (name, filter) in [
        ("plain", "anull"),
        ("limited", "alimiter"),
        ("normalized", "loudnorm=I=-16:TP=-1.5:LRA=11"),
    ] {
        let [reference, native] = export_pair_with_reference_options(
            &tools,
            &sandbox,
            name,
            "m4a",
            &args(&[
                "-i",
                &path_arg(&source),
                "-map",
                "0:0",
                "-af",
                filter,
                "-ar",
                "48000",
                "-ac",
                "2",
                "-c:a",
                "aac",
            ]),
            // Older MOV muxers round the entire track start to milliseconds.
            // Use sample resolution to compare the actual audio clock.
            &["-movie_timescale", "48000"],
        )?;
        let decode = |p: &Path| {
            run_tool_output(
                &tools.ffmpeg,
                &args(&["-v", "error", "-i", &path_arg(p), "-f", "s16le", "pipe:1"]),
            )
        };
        let actual = decode(&native)?;
        let expected = decode(&reference)?;
        assert_eq!(actual.len(), expected.len(), "{filter} lost samples");
        // Each distribution builds its own floating-point AAC encoder. Allow
        // rounding differences while checking the waveform at every sample.
        let (mut error_energy, mut signal_energy) = (0_i64, 0_i64);
        for (sample, reference) in actual.chunks_exact(2).zip(expected.chunks_exact(2)) {
            let sample = i64::from(i16::from_le_bytes([sample[0], sample[1]]));
            let reference = i64::from(i16::from_le_bytes([reference[0], reference[1]]));
            error_energy += (sample - reference).pow(2);
            signal_energy += reference.pow(2);
        }
        assert!(
            error_energy <= signal_energy / 100,
            "{filter} waveform changed"
        );
        assert_bytes_match(
            &packet_timing(&tools, &native)?,
            &packet_timing(&tools, &reference)?,
            &format!("{filter} packet timing"),
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn export_start_past_eof_writes_a_valid_empty_container() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("seek_past_eof")?;
    let source = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &source, 0.6, 96, 64)?;
    let [reference, native] = export_pair(
        &tools,
        &sandbox,
        "empty",
        "mp4",
        &args(&[
            "-ss",
            "60",
            "-i",
            &path_arg(&source),
            "-map",
            "0:v:0",
            "-map",
            "0:a:0",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
        ]),
    )?;
    // MOV omits tracks without packets when writing its trailer. Parsing the
    // empty moov box distinguishes a completed export from an empty file.
    assert!(packet_timing(&tools, &reference)?.is_empty());
    assert!(packet_timing(&tools, &native)?.is_empty());
    Ok(())
}
