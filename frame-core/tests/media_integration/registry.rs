//! Expand the shipped compatibility table into real exports. The reference
//! decoder checks the resulting streams independently of the worker.
use super::*;
use frame_core::media_rules;
use serde_json::Value;

fn expected_codec(name: &str) -> &str {
    match name {
        "libx264" | "h264_nvenc" | "h264_videotoolbox" => "h264",
        "libx265" | "hevc_nvenc" | "hevc_videotoolbox" => "hevc",
        "libsvtav1" | "av1_nvenc" => "av1",
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
    let mut available = Vec::new();
    for name in [
        "h264_videotoolbox",
        "hevc_videotoolbox",
        "h264_nvenc",
        "hevc_nvenc",
        "av1_nvenc",
    ] {
        if hardware_encoder_available(&tools, name)? {
            available.push(name);
        }
    }
    if let Ok(required) = env::var("FRAME_TEST_REQUIRED_HW_ENCODERS") {
        for name in required.split(',') {
            assert!(
                available.contains(&name),
                "required device encoder {name} unavailable"
            );
        }
    }
    eprintln!("Testing working device encoders {available:?}");
    check_video_registry(Some(&available))
}

pub fn hardware_encoder_available(tools: &Toolchain, name: &str) -> TestResult<bool> {
    let trial_frame = vec![0; 128 * 128 * 3 / 2];
    let mut child = Command::new(tools.worker.as_ref().unwrap())
        .args(["capabilities", "hardware", name, "0"])
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
    let mut trial = args(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=128x128:r=1:d=1",
        "-frames:v",
        "1",
        "-c:v",
        name,
    ]);
    if name.ends_with("_videotoolbox") {
        trial.extend(args(&["-allow_sw", "0"]));
    }
    trial.extend(args(&["-f", "null", "-"]));
    let diagnostic = String::from_utf8_lossy(&result.stderr);
    if run_tool(&tools.ffmpeg, &trial).is_ok() {
        return Err(format!(
            "reference encoder {name} works but the worker probe failed: {diagnostic}"
        ));
    }
    eprintln!("{name} unavailable on this device: {diagnostic}");
    Ok(false)
}

fn check_video_registry(hardware: Option<&[&str]>) -> TestResult {
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
                || !encoder.ends_with("_nvenc") && !encoder.ends_with("_videotoolbox"),
                |available| available.contains(&encoder.as_str()),
            );
            if !use_encoder {
                continue;
            }
            let formats = rules["containerEncoderPixelFormatCompatibility"]
                [frame_core::container::media_rules_key(container)][encoder]
                .as_array()
                .into_iter()
                .flatten()
                .map(|v| v.as_str().unwrap());
            for format in std::iter::once("auto").chain(formats) {
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
                let metadata = probe_media(&tools, &output)?;
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
                decode_media(&tools, &output)?;
                if hardware.is_some() && container == "mp4" && format == "auto" {
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
    for container in media_rules::all_containers() {
        let Some(codecs) = rules["containerAudioCodecCompatibility"]
            [frame_core::container::media_rules_key(container)]
        .as_array() else {
            continue;
        };
        for entry in codecs {
            let audio = entry.as_str().unwrap();
            if audio == "libfdk_aac"
                && !encoders
                    .lines()
                    .any(|line| line.split_whitespace().nth(1) == Some(audio))
            {
                continue;
            }
            let video = if container == "webm" {
                "vp9"
            } else {
                "libx264"
            };
            let mut config = video_config(container, video, audio);
            config.audio_bitrate = "192".into();
            config.selected_audio_tracks = vec![1];
            config.audio_channels = "stereo".into();
            let output = sandbox.path(&format!("{audio}.{container}"));
            convert(&tools, &input, &output, &config)
                .map_err(|e| format!("{container}/{audio}: {e}"))?;
            let metadata = probe_media(&tools, &output)?;
            assert_eq!(
                metadata.audio_codec.as_deref(),
                Some(expected_codec(audio)),
                "{}",
                output.display()
            );
            assert_eq!(metadata.audio_tracks.len(), 1);
            decode_media(&tools, &output)?;
        }
    }
    Ok(())
}
