use super::*;
use frame_core::trim::subtitle_trim_plan;

fn source(tools: &Toolchain, sandbox: &Sandbox) -> TestResult<PathBuf> {
    let input = sandbox.path("source.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=10:duration=3",
            "-f",
            "lavfi",
            "-i",
            "sine=sample_rate=48000:duration=3",
            "-c:v",
            "ffv1",
            // Exact cue windows below assume keyframe-aligned stream-copy seek.
            // Long-GOP Copy retains pre-seek video and can shift the whole mux.
            "-g",
            "1",
            "-c:a",
            "pcm_s16le",
            &path_arg(&input),
        ]),
    )?;
    Ok(input)
}

fn text(tools: &Toolchain, path: &Path, ordinal: usize) -> TestResult<String> {
    text_with_demux_flags(tools, path, ordinal, None)
}

fn text_with_demux_flags(
    tools: &Toolchain,
    path: &Path,
    ordinal: usize,
    flags: Option<&str>,
) -> TestResult<String> {
    let mut input_options = args(&["-v", "error"]);
    if let Some(flags) = flags {
        input_options.extend(args(&["-fflags", flags]));
    }
    input_options.extend(args(&[
        "-i",
        &path_arg(path),
        "-map",
        &format!("0:s:{ordinal}"),
        "-c:s",
        "srt",
        "-f",
        "srt",
        "pipe:1",
    ]));
    let bytes = run_tool_output(&tools.ffmpeg, &input_options)?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

fn packets(tools: &Toolchain, path: &Path, selector: &str) -> TestResult<serde_json::Value> {
    let bytes = run_tool_output(
        &tools.ffprobe,
        &args(&[
            "-v",
            "error",
            "-select_streams",
            selector,
            "-show_packets",
            "-show_data_hash",
            "sha256",
            "-show_entries",
            "packet=pts,dts,duration,size,data_hash",
            "-of",
            "json",
            &path_arg(path),
        ]),
    )?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    Ok(json["packets"].clone())
}

fn text_formats(tools: &Toolchain, srt: &Path) -> TestResult<Vec<PathBuf>> {
    let mut paths = vec![srt.to_path_buf()];
    for extension in ["ass", "vtt"] {
        let output = srt.with_extension(extension);
        run_tool(
            &tools.ffmpeg,
            &args(&["-v", "error", "-i", &path_arg(srt), &path_arg(&output)]),
        )?;
        paths.push(output);
    }
    Ok(paths)
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn end_only_trim_should_keep_raw_microsecond_boundary_without_adding_a_frame() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("raw_end_microseconds")?;
    let input = sandbox.path("microseconds.mov");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=16x16:rate=1000000",
            "-vf",
            r"setpts=if(eq(N\,0)\,0\,123455+N)",
            "-frames:v",
            "3",
            "-c:v",
            "ffv1",
            "-g",
            "1",
            "-enc_time_base",
            "1:1000000",
            "-fps_mode",
            "passthrough",
            "-f",
            "nut",
            &path_arg(&input),
        ]),
    )?;
    let source_packets = packets(&tools, &input, "v:0")?;
    let source_pts: Vec<_> = source_packets
        .as_array()
        .unwrap()
        .iter()
        .map(|packet| packet["pts"].as_i64().unwrap())
        .collect();
    assert_eq!(source_pts, [0, 123_456, 123_457]);
    for (index, end) in ["0.1234569", "00:00:00.1234569"].into_iter().enumerate() {
        let mut config = video_config("mkv", "libx264", "flac");
        config.processing_mode = "copy".into();
        config.end_time = Some(end.into());
        let output = sandbox.path(&format!("trimmed-{index}.mkv"));
        convert(&tools, &input, &output, &config)?;
        assert_eq!(
            packets(&tools, &output, "v:0")?.as_array().unwrap().len(),
            1
        );
        let decoded = run_tool_output(
            &tools.ffmpeg,
            &args(&[
                "-v",
                "error",
                "-i",
                &path_arg(&output),
                "-pix_fmt",
                "rgb24",
                "-f",
                "rawvideo",
                "pipe:1",
            ]),
        )?;
        assert_eq!(
            decoded.len(),
            16 * 16 * 3,
            "raw end must decode exactly one frame"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn selectable_text_windows_should_clip_cues_without_shifting_video_or_audio() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("selectable_text_window")?;
    let input = source(&tools, &sandbox)?;
    let srt = sandbox.path("window.srt");
    fs::write(&srt, "1\n00:00:00,000 --> 00:00:00,500\nEXPIRED\n\n2\n00:00:00,750 --> 00:00:01,250\nSTART\n\n3\n00:00:01,250 --> 00:00:01,750\nINSIDE\n\n4\n00:00:01,750 --> 00:00:02,500\nEND\n\n5\n00:00:02,000 --> 00:00:02,750\nOUTSIDE\n").map_err(|error| error.to_string())?;
    let formats = text_formats(&tools, &srt)?;
    let expected = "1\n00:00:00,000 --> 00:00:00,250\nSTART\n\n2\n00:00:00,250 --> 00:00:00,750\nINSIDE\n\n3\n00:00:00,750 --> 00:00:01,000\nEND\n\n";
    for mode in ["copy", "reencode"] {
        let mut config = video_config("mkv", "libx264", "flac");
        config.processing_mode = mode.into();
        config.start_time = Some("1".into());
        config.end_time = Some("2".into());
        config.selected_audio_tracks = vec![1];
        let baseline = sandbox.path(&format!("baseline-{mode}.mkv"));
        convert(&tools, &input, &baseline, &config)?;
        for (index, path) in formats.iter().enumerate() {
            config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
                path: path_arg(path),
                ..ExternalSubtitleTrack::default()
            }];
            let output = sandbox.path(&format!("{mode}-{index}.mkv"));
            convert(&tools, &input, &output, &config)?;
            assert_eq!(text(&tools, &output, 0)?, expected);
            for selector in ["v:0", "a:0"] {
                assert!(
                    packets(&tools, &output, selector)? == packets(&tools, &baseline, selector)?,
                    "{mode} {selector} packet timing or payload changed"
                );
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn zero_duration_subtitle_before_trim_should_survive_without_shifting_output() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("zero_duration_subtitle")?;
    let input = source(&tools, &sandbox)?;
    let srt = sandbox.path("zero.srt");
    fs::write(&srt, "1\n00:00:00,500 --> 00:00:00,500\nZERO_BEFORE\n\n2\n00:00:01,250 --> 00:00:01,750\nVISIBLE\n").map_err(|error| error.to_string())?;
    for mode in ["copy", "reencode"] {
        let mut config = video_config("mkv", "libx264", "flac");
        config.processing_mode = mode.into();
        config.start_time = Some("1".into());
        config.end_time = Some("2".into());
        let baseline = sandbox.path(&format!("baseline-{mode}.mkv"));
        convert(&tools, &input, &baseline, &config)?;
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: path_arg(&srt),
            ..ExternalSubtitleTrack::default()
        }];
        let output = sandbox.path(&format!("zero-{mode}.mkv"));
        convert(&tools, &input, &output, &config)?;
        // Generic demux probing can infer a duration for a stored zero-duration
        // subtitle. Read the authored packet without that inferred replacement.
        assert_eq!(
            text_with_demux_flags(&tools, &output, 0, Some("+nofillin"))?,
            "1\n00:00:00,000 --> 00:00:00,000\nZERO_BEFORE\n\n2\n00:00:00,250 --> 00:00:00,750\nVISIBLE\n\n"
        );
        assert!(packets(&tools, &output, "v:0")? == packets(&tools, &baseline, "v:0")?);
        assert_eq!(
            probe_media(&tools, &output)?.duration,
            probe_media(&tools, &baseline)?.duration
        );
    }
    Ok(())
}

fn write_pgs(path: &Path) -> TestResult {
    // One 2x2 white bitmap on a 320x180 canvas, followed by a clear at 2 seconds.
    let segments: &[(u32, u8, &[u8])] = &[
        (
            0,
            0x16,
            &[
                1, 64, 0, 180, 0x10, 0, 0, 0x80, 0, 0, 1, 0, 0, 0, 0, 0, 100, 0, 100,
            ],
        ),
        (0, 0x17, &[1, 0, 0, 100, 0, 100, 0, 2, 0, 2]),
        (0, 0x14, &[0, 0, 1, 235, 128, 128, 255]),
        (
            0,
            0x15,
            &[0, 0, 0, 0xc0, 0, 0, 12, 0, 2, 0, 2, 1, 1, 0, 0, 1, 1, 0, 0],
        ),
        (0, 0x80, &[]),
        (180_000, 0x16, &[1, 64, 0, 180, 0x10, 0, 1, 0, 0, 0, 0]),
        (180_000, 0x80, &[]),
    ];
    let mut bytes = Vec::new();
    for &(pts, tag, payload) in segments {
        bytes.extend(b"PG");
        bytes.extend(pts.to_be_bytes());
        bytes.extend(pts.to_be_bytes());
        bytes.push(tag);
        bytes.extend(u16::try_from(payload.len()).unwrap().to_be_bytes());
        bytes.extend(payload);
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn without_subtitle_bsfs(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut iter = args.iter();
    while let Some(value) = iter.next() {
        if value.starts_with("-bsf:s:") {
            iter.next();
        } else {
            result.push(value.clone());
        }
    }
    result
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn mixed_subtitle_ordinals_should_clip_only_imported_text_and_preserve_bitmap_packets() -> TestResult
{
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("mixed_subtitle_ordinals")?;
    let video = source(&tools, &sandbox)?;
    let embedded = sandbox.path("embedded.srt");
    fs::write(&embedded, "1\n00:00:00,250 --> 00:00:00,750\nEMBEDDED\n")
        .map_err(|error| error.to_string())?;
    let embedded_formats = text_formats(&tools, &embedded)?;
    let external = sandbox.path("external.srt");
    fs::write(&external, "1\n00:00:00,200 --> 00:00:02,000\nEXTERNAL\n")
        .map_err(|error| error.to_string())?;
    let external_formats = text_formats(&tools, &external)?;
    let pgs = sandbox.path("bitmap.sup");
    write_pgs(&pgs)?;
    let input = sandbox.path("mixed.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(&video),
            "-i",
            &path_arg(&embedded),
            "-i",
            &path_arg(&pgs),
            "-i",
            &path_arg(&embedded_formats[1]),
            "-map",
            "0:v:0",
            "-map",
            "1:s:0",
            "-map",
            "2:s:0",
            "-map",
            "3:s:0",
            "-c",
            "copy",
            &path_arg(&input),
        ]),
    )?;
    let metadata = probe_media(&tools, &input)?;
    for mode in ["copy", "reencode"] {
        let mut config = video_config("mkv", "libx264", "flac");
        config.processing_mode = mode.into();
        config.end_time = Some("1".into());
        config.selected_subtitle_tracks = vec![3, 2, 1];
        config.external_subtitle_tracks = [
            &external_formats[0],
            &pgs,
            &external_formats[1],
            &external_formats[2],
        ]
        .map(|path| ExternalSubtitleTrack {
            path: path_arg(path),
            ..ExternalSubtitleTrack::default()
        })
        .to_vec();
        let output = sandbox.path(&format!("fixed-{mode}.mkv"));
        let argv = build_ffmpeg_args(&path_arg(&input), &path_arg(&output), &config, &metadata)
            .map_err(|error| error.to_string())?;
        run_tool(&tools.ffmpeg, &argv)?;
        let baseline = sandbox.path(&format!("baseline-{mode}.mkv"));
        let mut baseline_args = without_subtitle_bsfs(&argv);
        *baseline_args.last_mut().unwrap() = path_arg(&baseline);
        run_tool(&tools.ffmpeg, &baseline_args)?;
        for ordinal in [0, 1, 2, 4] {
            let selector = format!("s:{ordinal}");
            let actual = packets(&tools, &output, &selector)?;
            assert!(actual.as_array().is_some_and(|packets| !packets.is_empty()));
            assert!(
                actual == packets(&tools, &baseline, &selector)?,
                "{mode} {selector} changed"
            );
        }
        for ordinal in [3, 5, 6] {
            assert_eq!(
                text(&tools, &output, ordinal)?,
                "1\n00:00:00,200 --> 00:00:01,000\nEXTERNAL\n\n"
            );
        }
        assert!(packets(&tools, &output, "v:0")? == packets(&tools, &baseline, "v:0")?);
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct PacketRecord {
    pts: String,
    dts: String,
    duration: String,
    size: String,
    payload: String,
}

fn packet_records(stderr: &[u8]) -> Vec<Vec<PacketRecord>> {
    let pattern = regex::Regex::new(r"\[showinfo @ ([^\]]+)\] n:\s*\d+ size:\s*(\d+) pts:(\S+) pt:\S+ dts:(\S+) dt:\S+ ds:(\S+) d:\S+ adler32:(\S+)").unwrap();
    let mut groups: Vec<(String, Vec<PacketRecord>)> = Vec::new();
    for capture in pattern.captures_iter(&String::from_utf8_lossy(stderr)) {
        let address = &capture[1];
        let index = groups
            .iter()
            .position(|(key, _)| key == address)
            .unwrap_or_else(|| {
                groups.push((address.to_string(), Vec::new()));
                groups.len() - 1
            });
        groups[index].1.push(PacketRecord {
            pts: capture[3].into(),
            dts: capture[4].into(),
            duration: capture[5].into(),
            size: capture[2].into(),
            payload: capture[6].into(),
        });
    }
    groups.into_iter().map(|(_, records)| records).collect()
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn subtitle_packet_guards_should_preserve_unknown_times_zero_duration_payloads_and_eof()
-> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("subtitle_packet_guards")?;
    let input = sandbox.path("points.srt");
    fs::write(
        &input,
        "1\n00:00:00,200 --> 00:00:00,400\nFIRST\n\n2\n00:00:00,600 --> 00:00:00,900\nEOF\n",
    )
    .map_err(|error| error.to_string())?;
    let mut config = video_config("mkv", "libx264", "flac");
    config.end_time = Some("1".into());
    config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
        path: path_arg(&input),
        ..ExternalSubtitleTrack::default()
    }];
    let plan = subtitle_trim_plan(&config)
        .map_err(|error| error.to_string())?
        .unwrap();
    for inject in [
        "setts=duration=0",
        "setts=pts=PTS-1000:dts=DTS-1000:duration=0",
        "setts=pts=NOPTS:dts=NOPTS",
        "setts=pts=NOPTS",
        "setts=dts=NOPTS",
        "setts=duration=-1",
    ] {
        let output = Command::new(&tools.ffmpeg)
            .args(args(&[
                "-hide_banner",
                "-v",
                "info",
                "-i",
                &path_arg(&input),
                "-map",
                "0:s:0",
                "-c:s",
                "copy",
                "-bsf:s:0",
                &format!("{inject},showinfo,{},showinfo", plan.bitstream_filter),
                "-f",
                "null",
                "-",
            ]))
            .output()
            .map_err(|error| error.to_string())?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut groups = packet_records(&output.stderr);
        assert_eq!(groups.len(), 2, "{inject}");
        assert_eq!(groups[0].len(), 2, "source packet count");
        assert_eq!(groups[1].len(), 2, "EOF packet was not flushed");
        if inject.contains("PTS-1000") {
            for expected in &mut groups[0] {
                expected.pts = "0".into();
                expected.dts = "0".into();
            }
        }
        assert_eq!(groups[0], groups[1], "{inject}");
    }
    Ok(())
}
