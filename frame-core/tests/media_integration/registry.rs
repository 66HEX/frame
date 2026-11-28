//! Expand the shipped compatibility table into real exports. The reference
//! decoder checks the resulting streams independently of the worker.
use super::*;
use frame_core::media_rules;
use serde_json::Value;

const HARDWARE_ENCODERS: &[&str] = &[
    "h264_videotoolbox",
    "hevc_videotoolbox",
    "h264_nvenc",
    "hevc_nvenc",
    "av1_nvenc",
    "h264_vaapi",
    "hevc_vaapi",
    "av1_vaapi",
    "hevc_amf",
];

fn expected_codec(name: &str) -> &str {
    match name {
        "libx264" | "h264_nvenc" | "h264_videotoolbox" | "h264_vaapi" => "h264",
        "libx265" | "hevc_nvenc" | "hevc_videotoolbox" | "hevc_vaapi" | "hevc_amf" => "hevc",
        "libsvtav1" | "av1_nvenc" | "av1_vaapi" => "av1",
        "libjxl" => "jpegxl",
        "libwebp" => "webp",
        "libopus" => "opus",
        "libfdk_aac" => "aac",
        other => other,
    }
}

fn rules() -> Value {
    serde_json::from_str(include_str!("../../media-rules.json")).unwrap()
}

#[test]
#[ignore = "requires release codecs, the Frame worker and independent reference tools"]
fn software_video_codecs_and_explicit_pixel_formats_in_every_container() -> TestResult {
    check_video_registry(None)
}

#[test]
#[ignore = "runs a real encode for each hardware encoder available on this machine"]
fn hardware_video_codecs_and_explicit_pixel_formats_in_every_container() -> TestResult {
    let tools = Toolchain::discover()?;
    let mut available = std::collections::BTreeMap::new();
    for &name in HARDWARE_ENCODERS {
        if (name.ends_with("_vaapi") && !cfg!(target_os = "linux"))
            || (name.ends_with("_amf") && !cfg!(windows))
            || (name.ends_with("_videotoolbox") && !cfg!(target_os = "macos"))
            || (name.ends_with("_nvenc") && cfg!(target_os = "macos"))
        {
            continue;
        }
        if hardware_encoder_available(&tools, name)? {
            let main10 =
                !name.starts_with("h264_") && hardware_profile_available(&tools, name, "p010le")?;
            available.insert(name, main10);
        }
    }
    for (variable, needs_main10) in [
        ("FRAME_TEST_REQUIRED_HW_ENCODERS", false),
        ("FRAME_TEST_REQUIRED_10BIT_HW_ENCODERS", true),
    ] {
        if let Ok(required) = env::var(variable) {
            for name in required
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
            {
                assert!(
                    available
                        .get(name)
                        .is_some_and(|main10| !needs_main10 || *main10),
                    "{variable} requires a working {name} profile on this machine"
                );
            }
        }
    }
    eprintln!("Working device encoders and 10-bit support: {available:?}");
    check_video_registry(Some(&available))
}

pub fn hardware_encoder_available(tools: &Toolchain, name: &str) -> TestResult<bool> {
    hardware_profile_available(tools, name, "yuv420p")
}

fn hardware_profile_available(tools: &Toolchain, name: &str, pixel: &str) -> TestResult<bool> {
    let trial_frame = vec![0; 128 * 128 * 3 / 2];
    let mut child = Command::new(tools.worker.as_ref().unwrap())
        .args(["capabilities", "hardware", name, "0", pixel])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&trial_frame)
        .map_err(|e| e.to_string())?;
    let result = child.wait_with_output().map_err(|e| e.to_string())?;
    if result.status.success() {
        assert!(String::from_utf8_lossy(&result.stdout).contains("progress=end"));
        return Ok(true);
    }
    // An unavailable device is expected on hosted runners. Detect a broken
    // worker probe if the legacy executable can actually use this encoder.
    let diagnostic = String::from_utf8_lossy(&result.stderr);
    if reference_hardware_profile_works(tools, name, pixel) {
        return Err(format!(
            "reference encoder {name}/{pixel} works but the worker probe failed: {diagnostic}"
        ));
    }
    eprintln!("{name}/{pixel} unavailable on this device: {diagnostic}");
    Ok(false)
}

fn reference_hardware_profile_works(tools: &Toolchain, name: &str, pixel: &str) -> bool {
    let try_device = |render_node: Option<&Path>| {
        let mut trial = args(&["-v", "error"]);
        if let Some(node) = render_node {
            trial.extend(["-vaapi_device".into(), path_arg(node)]);
        }
        trial.extend(args(&[
            "-f",
            "lavfi",
            "-i",
            "color=black:s=320x256:r=1:d=1",
            "-frames:v",
            "1",
        ]));
        if render_node.is_some() {
            let samples = if pixel == "p010le" { "p010le" } else { "nv12" };
            trial.extend(["-vf".into(), format!("format={samples},hwupload")]);
        } else {
            trial.extend(args(&["-pix_fmt", pixel]));
        }
        trial.extend(args(&["-c:v", name, "-b:v", "1M"]));
        if name.ends_with("_videotoolbox") {
            trial.extend(args(&["-allow_sw", "0"]));
        }
        trial.extend(args(&["-f", "null", "-"]));
        run_tool(&tools.ffmpeg, &trial).is_ok()
    };
    if !name.ends_with("_vaapi") {
        return try_device(None);
    }
    // Match the worker's device search. The reference CLI needs device
    // surfaces even when this probe uses a software-generated input frame.
    std::fs::read_dir("/dev/dri").is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_name().to_string_lossy().starts_with("renderD")
                && try_device(Some(&entry.path()))
        })
    })
}

fn check_video_registry(hardware: Option<&std::collections::BTreeMap<&str, bool>>) -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("video-registry")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 0.25, 320, 256)?;
    let rules = rules();
    for container in media_rules::all_containers() {
        let Some(encoders) = media_rules::video_codecs_for_container(container) else {
            continue;
        };
        for encoder in encoders {
            let use_encoder = hardware.map_or_else(
                || !HARDWARE_ENCODERS.contains(&encoder.as_str()),
                |available| available.contains_key(encoder.as_str()),
            );
            if !use_encoder || (encoder == "libjxl" && !jxl::encoder_available(&tools)?) {
                continue;
            }
            let formats = rules["containerEncoderPixelFormatCompatibility"]
                [frame_core::container::media_rules_key(container)][encoder]
                .as_array()
                .into_iter()
                .flatten()
                .map(|v| v.as_str().unwrap());
            for format in std::iter::once("auto").chain(formats) {
                if format == "yuv420p10le"
                    && hardware.is_some_and(|available| !available[encoder.as_str()])
                {
                    continue;
                }
                let mut config = video_config(
                    container,
                    encoder,
                    media_rules::default_audio_codec_for_container(container),
                );
                config.pixel_format = format.into();
                config.fps = "30".into();
                config.hw_decode = hardware.is_some();
                if hardware.is_some() {
                    config.video_bitrate_mode = "bitrate".into();
                }
                if encoder.ends_with("_nvenc") {
                    config.nvenc_spatial_aq = true;
                    config.nvenc_temporal_aq = true;
                }
                let output = sandbox.path(&format!("{encoder}-{format}.{container}"));
                convert(&tools, &input, &output, &config)
                    .map_err(|e| format!("{container}/{encoder}/{format}: {e}"))?;
                let metadata = probe_and_decode_export(&tools, &output, encoder)?;
                assert_eq!(
                    metadata.video_codec.as_deref(),
                    Some(expected_codec(encoder)),
                    "{}",
                    output.display()
                );
                if format != "auto" {
                    assert_eq!(
                        metadata.pixel_format.as_deref(),
                        Some(if encoder == "prores" && format == "yuv444p10le" {
                            // The reference ProRes decoder exposes a 12-bit 4:4:4 raster.
                            "yuv444p12le"
                        } else {
                            format
                        }),
                        "{}",
                        output.display()
                    );
                }
                if hardware.is_some()
                    && container == "mp4"
                    && format == "auto"
                    && !frame_core::codec::requires_video_bitrate(encoder)
                {
                    // Quality mode is optional on Intel VideoToolbox. A capable
                    // legacy backend must have a working native counterpart.
                    config.video_bitrate_mode = "crf".into();
                    let reference = sandbox.path(&format!("{encoder}-reference.mp4"));
                    let candidate = sandbox.path(&format!("{encoder}-quality.mp4"));
                    let plan = build_ffmpeg_args(
                        &path_arg(&input),
                        &path_arg(&reference),
                        &config,
                        &probe_media(&tools, &input)?,
                    )
                    .map_err(|err| err.to_string())?;
                    let baseline = run_tool(&tools.ffmpeg, &plan);
                    match convert(&tools, &input, &candidate, &config) {
                        Ok(()) => decode_media(&tools, &candidate)?,
                        Err(error)
                            if baseline
                                .as_ref()
                                .is_err_and(|message| message.contains("qscale not available"))
                                && error.contains("qscale not available") =>
                        {
                            eprintln!("{encoder} quality mode unavailable on this device");
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }
    }
    Ok(())
}

fn probe_and_decode_export(
    tools: &Toolchain,
    output: &Path,
    encoder: &str,
) -> TestResult<ProbeMetadata> {
    if encoder == "libjxl" && !jxl::reference_decodes_jxl(tools)? {
        let png = jxl::decode_to_png(tools, output)?;
        decode_media(tools, &png)?;
        let metadata = tools.native("probe", &["metadata".into(), path_arg(output)])?;
        return parse_ffprobe_stdout(
            &path_arg(output),
            String::from_utf8(metadata).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string());
    }
    decode_media(tools, output)?;
    probe_media(tools, output)
}

fn audio_packet_hashes(tools: &Toolchain, output: &Path) -> TestResult<String> {
    let bytes = run_tool_output(
        &tools.ffprobe,
        &args(&[
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_packets",
            "-show_entries",
            "packet=data_hash:packet_side_data=",
            "-show_data_hash",
            "sha256",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &path_arg(output),
        ]),
    )?;
    let hashes = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    assert!(
        !hashes.is_empty(),
        "no audio packets in {}",
        output.display()
    );
    for line in hashes.lines() {
        assert!(
            line.strip_prefix("SHA256:")
                .is_some_and(|digest| digest.len() == 64
                    && digest.bytes().all(|byte| byte.is_ascii_hexdigit())),
            "invalid packet hash in {}: {line}",
            output.display(),
        );
    }
    Ok(hashes)
}

#[test]
#[ignore = "requires release codecs, the Frame worker and independent reference tools"]
fn audio_codec_container_registry_produces_decodable_selected_tracks() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio-registry")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 0.25, 128, 96)?;
    let rules = rules();
    let encoders = String::from_utf8(tools.native("capabilities", &["-encoders".into()])?)
        .map_err(|e| e.to_string())?;
    let fdk_available = encoders
        .lines()
        .any(|line| line.split_whitespace().nth(1) == Some("libfdk_aac"));
    let only_fdk = env::var("FRAME_TEST_ONLY_FDK_AAC").as_deref() == Ok("1");
    if only_fdk && !fdk_available {
        return Err(
            "The selected media SDK does not provide the required libfdk_aac encoder".into(),
        );
    }
    let mut fdk_outputs = Vec::new();
    for container in media_rules::all_containers() {
        let Some(codecs) = rules["containerAudioCodecCompatibility"]
            [frame_core::container::media_rules_key(container)]
        .as_array() else {
            continue;
        };
        for entry in codecs {
            let audio = entry.as_str().unwrap();
            if (audio == "libfdk_aac" && !fdk_available) || (only_fdk && audio != "libfdk_aac") {
                continue;
            }
            let video = if container == "webm" {
                "vp9"
            } else {
                "libx264"
            };
            let mut config = video_config(container, video, audio);
            config.selected_audio_tracks = vec![1];
            config.audio_channels = "stereo".into();
            let profiles: &[(&str, &str)] = if audio == "libfdk_aac" {
                &[("bitrate", "192"), ("vbr", "1"), ("vbr", "5")]
            } else {
                &[("bitrate", "192")]
            };
            let mut fingerprints = std::collections::HashSet::new();
            for &(mode, value) in profiles {
                config.audio_bitrate_mode = mode.into();
                if mode == "vbr" {
                    config.audio_quality = value.into();
                } else {
                    config.audio_bitrate = value.into();
                }
                let output = sandbox.path(&format!("{audio}-{mode}{value}.{container}"));
                convert(&tools, &input, &output, &config)
                    .map_err(|e| format!("{container}/{audio}/{mode}{value}: {e}"))?;
                let metadata = probe_media(&tools, &output)?;
                assert_eq!(
                    metadata.audio_codec.as_deref(),
                    Some(expected_codec(audio)),
                    "{}",
                    output.display()
                );
                assert_eq!(metadata.audio_tracks.len(), 1);
                decode_media(&tools, &output)?;
                if audio == "libfdk_aac" {
                    let track = &metadata.audio_tracks[0];
                    assert_eq!(track.channels, "2", "{}", output.display());
                    assert_eq!(
                        track.channel_layout.as_deref(),
                        Some("stereo"),
                        "{}",
                        output.display()
                    );
                    assert_eq!(
                        track.sample_rate.as_deref(),
                        Some("48000"),
                        "{}",
                        output.display()
                    );
                    assert!(
                        fingerprints.insert(audio_packet_hashes(&tools, &output)?),
                        "{container}/{audio}/{mode}{value} produced the same AAC packets as an earlier profile",
                    );
                }
            }
            if audio == "libfdk_aac" {
                fdk_outputs.push(container.as_str());
            }
        }
    }
    if only_fdk {
        for expected in ["mp4", "mkv", "mov", "m2t", "m4a"] {
            assert!(
                fdk_outputs.contains(&expected),
                "missing FDK AAC output {expected}"
            );
        }
        eprintln!("FDK AAC bitrate and VBR exports decoded for {fdk_outputs:?}");
    }
    Ok(())
}
