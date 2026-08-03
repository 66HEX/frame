use std::{
    env,
    ffi::OsStr,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use frame_core::{
    args::{build_ffmpeg_args, validate_task_input},
    preview::{PreviewFfmpegOptions, build_ffmpeg_preview_args},
    probe::{ffprobe_json_args, parse_ffprobe_stdout},
    types::{
        ConversionConfig, CropConfig, ExternalSubtitleTrack, MetadataConfig, MetadataMode,
        OverlayConfig, ProbeMetadata,
    },
};

type TestResult<T = ()> = Result<T, String>;

#[derive(Clone, Debug)]
struct Toolchain {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Debug)]
struct RgbFrame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[derive(Debug)]
struct Sandbox {
    root: PathBuf,
    keep: bool,
}

impl Toolchain {
    fn discover() -> TestResult<Self> {
        let ffmpeg = discover_tool("ffmpeg", "FRAME_TEST_FFMPEG")?;
        let ffprobe = discover_tool("ffprobe", "FRAME_TEST_FFPROBE")?;
        Ok(Self { ffmpeg, ffprobe })
    }
}

impl Rgb {
    const BLACK: Self = Self::new(0, 0, 0);
    const BLUE: Self = Self::new(0, 0, 255);
    const GREEN: Self = Self::new(0, 255, 0);
    const RED: Self = Self::new(255, 0, 0);
    const YELLOW: Self = Self::new(255, 255, 0);

    const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

impl RgbFrame {
    fn pixel(&self, x: u32, y: u32) -> TestResult<Rgb> {
        if x >= self.width || y >= self.height {
            return Err(format!(
                "pixel coordinate {x},{y} is outside {}x{} frame",
                self.width, self.height
            ));
        }

        let offset = usize::try_from((y * self.width + x) * 3)
            .map_err(|error| format!("pixel offset overflow: {error}"))?;
        Ok(Rgb {
            red: self.pixels[offset],
            green: self.pixels[offset + 1],
            blue: self.pixels[offset + 2],
        })
    }
}

impl Sandbox {
    fn new(name: &str) -> TestResult<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before unix epoch: {error}"))?
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "frame-media-test-{}-{}-{now}",
            std::process::id(),
            sanitize_name(name)
        ));
        fs::create_dir_all(&root)
            .map_err(|error| format!("failed to create {}: {error}", root.display()))?;
        Ok(Self {
            root,
            keep: env::var_os("FRAME_KEEP_MEDIA_TESTS").is_some(),
        })
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.keep {
            eprintln!("keeping media test artifacts in {}", self.root.display());
            return;
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_reencode_matrix_should_write_packets_streams_and_service_metadata() -> TestResult
{
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_reencode_matrix")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 1.0, 160, 90)?;

    for (container, video_codec, audio_codec, packet_size) in [
        ("m2t", "mpeg2video", "mp2", 188_u16),
        ("mts", "libx264", "ac3", 192_u16),
        ("m2ts", "libx264", "ac3", 192_u16),
    ] {
        let output = sandbox.path(&format!("reencoded.{container}"));
        let mut config = video_config(container, video_codec, audio_codec);
        config.video_bitrate_mode = "bitrate".to_string();
        config.video_bitrate = "2500".to_string();
        config.audio_bitrate = "192".to_string();
        config.metadata.mode = MetadataMode::Replace;
        config.metadata.service_name = Some(format!("Frame {container}"));
        config.metadata.service_provider = Some("Frame Integration".to_string());
        convert(&tools, &input, &output, &config)?;

        let metadata = probe_media(&tools, &output)?;
        let transport = metadata
            .transport_stream
            .ok_or_else(|| format!("{container} did not probe as a transport stream"))?;
        assert_eq!(transport.packet_size, Some(packet_size));
        assert_eq!(
            transport.service_name.as_deref(),
            Some(format!("Frame {container}").as_str())
        );
        assert_eq!(
            transport.service_provider.as_deref(),
            Some("Frame Integration")
        );
        assert_eq!(metadata.subtitle_tracks.len(), 0);
        let file_size = fs::metadata(&output)
            .map_err(|error| format!("failed to stat {}: {error}", output.display()))?
            .len();
        assert_eq!(file_size % u64::from(packet_size), 0);
        decode_media(&tools, &output)?;
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_audio_encoder_matrix_should_write_standard_stream_types() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_audio_encoder_matrix")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 0.5, 160, 90)?;

    for (container, audio_encoder, expected_codec) in [
        ("m2t", "mp2", "mp2"),
        ("m2t", "aac", "aac"),
        ("m2t", "ac3", "ac3"),
        ("m2t", "mp3", "mp3"),
        ("m2t", "libopus", "opus"),
        ("m2ts", "ac3", "ac3"),
        ("m2ts", "pcm_bluray", "pcm_bluray"),
    ] {
        let capability_encoder = if audio_encoder == "mp3" {
            "libmp3lame"
        } else {
            audio_encoder
        };
        if !encoder_available(&tools, capability_encoder)? {
            eprintln!("skipping unavailable encoder {audio_encoder}");
            continue;
        }
        let output = sandbox.path(&format!("audio-{audio_encoder}.{container}"));
        let mut config = video_config(container, "libx264", audio_encoder);
        config.selected_audio_tracks = vec![1];
        config.video_bitrate_mode = "bitrate".to_string();
        config.video_bitrate = "1500".to_string();
        config.audio_bitrate = "192".to_string();
        convert(&tools, &input, &output, &config)?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata.audio_codec.as_deref(),
            Some(expected_codec),
            "unexpected audio codec for {audio_encoder} in {container}"
        );
        decode_media(&tools, &output)?;
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_video_encoder_matrix_should_write_standard_stream_types() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_video_encoder_matrix")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 0.25, 160, 90)?;

    for (container, video_encoder, expected_codec, audio_encoder) in [
        ("m2t", "mpeg2video", "mpeg2video", "mp2"),
        ("m2t", "libx264", "h264", "aac"),
        ("m2t", "libx265", "hevc", "aac"),
        ("m2ts", "mpeg2video", "mpeg2video", "ac3"),
        ("m2ts", "libx264", "h264", "ac3"),
        ("m2ts", "libx265", "hevc", "ac3"),
    ] {
        if !encoder_available(&tools, video_encoder)? {
            eprintln!("skipping unavailable encoder {video_encoder}");
            continue;
        }
        let output = sandbox.path(&format!("video-{video_encoder}.{container}"));
        let mut config = video_config(container, video_encoder, audio_encoder);
        config.audio_bitrate = "192".to_string();
        if video_encoder == "mpeg2video" {
            config.video_bitrate_mode = "bitrate".to_string();
            config.video_bitrate = "2500".to_string();
        }
        convert(&tools, &input, &output, &config)?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata.video_codec.as_deref(),
            Some(expected_codec),
            "unexpected video codec for {video_encoder} in {container}"
        );
        decode_media(&tools, &output)?;
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_copy_should_preserve_compatible_video_and_audio() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_copy")?;
    let standard_input = sandbox.path("h264-aac.mp4");
    generate_h264_aac_source(&tools, &standard_input, 1.0, 160, 90)?;

    let m2t_output = sandbox.path("copied.m2t");
    let mut m2t_config = video_config("m2t", "libx264", "aac");
    m2t_config.processing_mode = "copy".to_string();
    m2t_config.selected_audio_tracks = vec![1];
    convert(&tools, &standard_input, &m2t_output, &m2t_config)?;
    let m2t_metadata = probe_media(&tools, &m2t_output)?;
    assert_eq!(m2t_metadata.video_codec.as_deref(), Some("h264"));
    assert_eq!(m2t_metadata.audio_codec.as_deref(), Some("aac"));
    assert_eq!(
        m2t_metadata.transport_stream.unwrap().packet_size,
        Some(188)
    );

    let bluray_input = sandbox.path("h264-ac3.mkv");
    generate_h264_ac3_source(&tools, &bluray_input)?;
    for container in ["mts", "m2ts"] {
        let output = sandbox.path(&format!("copied.{container}"));
        let mut config = video_config(container, "libx264", "ac3");
        config.processing_mode = "copy".to_string();
        config.selected_audio_tracks = vec![1];
        convert(&tools, &bluray_input, &output, &config)?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(metadata.video_codec.as_deref(), Some("h264"));
        assert_eq!(metadata.audio_codec.as_deref(), Some("ac3"));
        assert_eq!(metadata.transport_stream.unwrap().packet_size, Some(192));
        decode_media(&tools, &output)?;
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_metadata_modes_should_round_trip_program_tags() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_metadata_modes")?;
    let input = sandbox.path("source.mp4");
    let tagged_source = sandbox.path("tagged-source.m2t");
    generate_h264_aac_source(&tools, &input, 1.0, 160, 90)?;

    let mut source_config = video_config("m2t", "mpeg2video", "mp2");
    source_config.video_bitrate_mode = "bitrate".to_string();
    source_config.video_bitrate = "2500".to_string();
    source_config.audio_bitrate = "192".to_string();
    source_config.metadata.mode = MetadataMode::Replace;
    source_config.metadata.service_name = Some("Original Service".to_string());
    source_config.metadata.service_provider = Some("Original Provider".to_string());
    convert(&tools, &input, &tagged_source, &source_config)?;

    for (mode, expected_name, expected_provider) in [
        (
            MetadataMode::Preserve,
            "Original Service",
            "Original Provider",
        ),
        (MetadataMode::Clean, "Service01", "Frame"),
        (MetadataMode::Replace, "Replacement", "Replacement Provider"),
    ] {
        let output = sandbox.path(&format!(
            "metadata-{}.m2t",
            match mode {
                MetadataMode::Preserve => "preserve",
                MetadataMode::Clean => "clean",
                MetadataMode::Replace => "replace",
            }
        ));
        let mut config = video_config("m2t", "mpeg2video", "mp2");
        config.processing_mode = "copy".to_string();
        config.metadata.mode = mode.clone();
        if mode == MetadataMode::Replace {
            config.metadata.service_name = Some("Replacement".to_string());
            config.metadata.service_provider = Some("Replacement Provider".to_string());
        }
        convert(&tools, &tagged_source, &output, &config)?;
        let transport = probe_media(&tools, &output)?
            .transport_stream
            .ok_or_else(|| "metadata output did not expose program tags".to_string())?;
        assert_eq!(transport.service_name.as_deref(), Some(expected_name));
        assert_eq!(
            transport.service_provider.as_deref(),
            Some(expected_provider)
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_burn_in_should_support_srt_ass_and_vtt_for_every_suffix() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_burn_in")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 1.0, 160, 90)?;
    let subtitle_cases = [
        ("srt", "1\n00:00:00,000 --> 00:00:00,900\nFrame SRT\n"),
        ("vtt", "WEBVTT\n\n00:00.000 --> 00:00.900\nFrame VTT\n"),
        (
            "ass",
            "[Script Info]\nScriptType: v4.00+\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:00.90,Default,,0,0,0,,Frame ASS\n",
        ),
    ];
    for (extension, contents) in subtitle_cases {
        let subtitle = sandbox.path(&format!("captions.{extension}"));
        fs::write(&subtitle, contents)
            .map_err(|error| format!("failed to write {}: {error}", subtitle.display()))?;
        for container in ["m2t", "mts", "m2ts"] {
            let output = sandbox.path(&format!("burned-{extension}.{container}"));
            let mut config = video_config(
                container,
                if container == "m2t" {
                    "mpeg2video"
                } else {
                    "libx264"
                },
                if container == "m2t" { "mp2" } else { "ac3" },
            );
            config.video_bitrate_mode = "bitrate".to_string();
            config.video_bitrate = "2500".to_string();
            config.audio_bitrate = "192".to_string();
            config.subtitle_burn_path = Some(path_arg(&subtitle));
            convert(&tools, &input, &output, &config)?;
            let metadata = probe_media(&tools, &output)?;
            assert!(metadata.video_codec.is_some());
            assert!(metadata.subtitle_tracks.is_empty());
            decode_media(&tools, &output)?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn transport_stream_combined_processing_graph_should_preserve_existing_features() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("transport_stream_combined_graph")?;
    let input = sandbox.path("source.mp4");
    let overlay = sandbox.path("overlay.ppm");
    let subtitle = sandbox.path("burn.srt");
    generate_h264_aac_source(&tools, &input, 1.0, 160, 90)?;
    write_solid_ppm(&overlay, 24, 16, Rgb::RED)?;
    write_srt(&subtitle)?;

    for container in ["m2t", "m2ts"] {
        let output = sandbox.path(&format!("combined.{container}"));
        let mut config = video_config(
            container,
            if container == "m2t" {
                "mpeg2video"
            } else {
                "libx264"
            },
            if container == "m2t" { "mp2" } else { "ac3" },
        );
        config.video_bitrate_mode = "bitrate".to_string();
        config.video_bitrate = "2500".to_string();
        config.audio_bitrate = "192".to_string();
        config.start_time = Some("00:00:00.100".to_string());
        config.end_time = Some("00:00:00.800".to_string());
        config.crop = Some(CropConfig {
            enabled: true,
            x: 8.0,
            y: 4.0,
            width: 144.0,
            height: 80.0,
            source_width: Some(160.0),
            source_height: Some(90.0),
            aspect_ratio: None,
        });
        config.resolution = "custom".to_string();
        config.custom_width = Some("128".to_string());
        config.custom_height = Some("72".to_string());
        config.rotation = "180".to_string();
        config.flip_horizontal = true;
        config.video_filters.color.brightness = frame_core::types::FilterValue {
            enabled: true,
            value: 5,
        };
        config.video_filters.sharpen = frame_core::types::FilterValue {
            enabled: true,
            value: 15,
        };
        config.audio_filters.bass = frame_core::types::FilterValue {
            enabled: true,
            value: 2,
        };
        config.audio_filters.limiter = frame_core::types::FilterValue {
            enabled: true,
            value: -1,
        };
        config.overlay = Some(OverlayConfig {
            enabled: true,
            path: path_arg(&overlay),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 0.75,
            anchor: "center".to_string(),
        });
        config.subtitle_burn_path = Some(path_arg(&subtitle));
        config.subtitle_font_size = Some("18".to_string());
        config.subtitle_position = Some("bottom".to_string());

        convert(&tools, &input, &output, &config)?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!((metadata.width, metadata.height), (Some(128), Some(72)));
        let duration = duration_seconds(&metadata)?;
        assert!((0.4..=1.0).contains(&duration));
        decode_media(&tools, &output)?;
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe and FRAME_TEST_PGS_FIXTURE pointing to a .sup file"]
fn pgs_sidecar_should_copy_to_m2ts_and_transcode_to_dvb_for_m2t() -> TestResult {
    let Some(pgs_fixture) = env::var_os("FRAME_TEST_PGS_FIXTURE").map(PathBuf::from) else {
        eprintln!("skipping PGS test: FRAME_TEST_PGS_FIXTURE is not set");
        return Ok(());
    };
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("pgs_transport_stream")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 2.0, 160, 90)?;

    for (container, expected_codec) in [("m2t", "dvb_subtitle"), ("m2ts", "hdmv_pgs_subtitle")] {
        let output = sandbox.path(&format!("subtitled.{container}"));
        let mut config = video_config(
            container,
            if container == "m2t" {
                "mpeg2video"
            } else {
                "libx264"
            },
            if container == "m2t" { "mp2" } else { "ac3" },
        );
        config.video_bitrate_mode = "bitrate".to_string();
        config.video_bitrate = "2500".to_string();
        config.audio_bitrate = "192".to_string();
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: path_arg(&pgs_fixture),
            language: (container == "m2t").then(|| "eng".to_string()),
            ..ExternalSubtitleTrack::default()
        }];
        convert(&tools, &input, &output, &config)?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(metadata.subtitle_tracks.len(), 1);
        assert_eq!(metadata.subtitle_tracks[0].codec, expected_codec);
        assert_eq!(
            metadata.subtitle_tracks[0].language.as_deref(),
            (container == "m2t").then_some("eng")
        );
    }

    let pgs_source = sandbox.path("embedded-pgs.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            &path_arg(&input),
            "-i",
            &path_arg(&pgs_fixture),
            "-map",
            "0:v:0",
            "-map",
            "0:a:0",
            "-map",
            "1:s:0",
            "-c",
            "copy",
            "-metadata:s:s:0",
            "language=eng",
            "-y",
            &path_arg(&pgs_source),
        ]),
    )?;
    let subtitle_index = probe_media(&tools, &pgs_source)?
        .subtitle_tracks
        .first()
        .map(|track| track.index)
        .ok_or_else(|| "generated PGS source has no subtitle stream".to_string())?;

    let dvb_output = sandbox.path("mixed-copy-pgs-to-dvb.m2t");
    let mut dvb_config = video_config("m2t", "libx264", "aac");
    dvb_config.processing_mode = "copy".to_string();
    dvb_config.selected_audio_tracks = vec![1];
    dvb_config.selected_subtitle_tracks = vec![subtitle_index];
    convert(&tools, &pgs_source, &dvb_output, &dvb_config)?;
    let dvb_probe = probe_media(&tools, &dvb_output)?;
    assert_eq!(dvb_probe.video_codec.as_deref(), Some("h264"));
    assert_eq!(dvb_probe.audio_codec.as_deref(), Some("aac"));
    assert_eq!(dvb_probe.subtitle_tracks[0].codec, "dvb_subtitle");
    assert_eq!(
        dvb_probe.subtitle_tracks[0].language.as_deref(),
        Some("eng")
    );

    let pgs_output = sandbox.path("mixed-reencode-pgs-copy.m2ts");
    let mut pgs_config = video_config("m2ts", "libx264", "ac3");
    pgs_config.selected_audio_tracks = vec![1];
    pgs_config.selected_subtitle_tracks = vec![subtitle_index];
    pgs_config.audio_bitrate = "192".to_string();
    convert(&tools, &pgs_source, &pgs_output, &pgs_config)?;
    let pgs_probe = probe_media(&tools, &pgs_output)?;
    assert_eq!(pgs_probe.video_codec.as_deref(), Some("h264"));
    assert_eq!(pgs_probe.audio_codec.as_deref(), Some("ac3"));
    assert_eq!(pgs_probe.subtitle_tracks[0].codec, "hdmv_pgs_subtitle");
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn h264_mp4_reencode_should_write_h264_video_and_aac_audio() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("h264_mp4_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.mp4");

    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.selected_audio_tracks = vec![1];
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("h264"));
    assert_eq!(metadata.audio_codec.as_deref(), Some("aac"));
    assert_eq!(metadata.width, Some(64));
    assert_eq!(metadata.height, Some(48));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn hevc_mkv_reencode_should_write_hevc_video() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("hevc_mkv_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.mkv");

    generate_h264_aac_source(&tools, &input, 0.5, 48, 32)?;
    let config = video_config("mkv", "libx265", "aac");
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("hevc"));
    assert_eq!(metadata.width, Some(48));
    assert_eq!(metadata.height, Some(32));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn vp9_webm_reencode_should_write_vp9_video_and_opus_audio() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("vp9_webm_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.webm");

    generate_h264_aac_source(&tools, &input, 0.5, 48, 32)?;
    let mut config = video_config("webm", "vp9", "libopus");
    config.selected_audio_tracks = vec![1];
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("vp9"));
    assert_eq!(metadata.audio_codec.as_deref(), Some("opus"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn svt_av1_mp4_reencode_should_accept_frame_preset_and_write_av1_video() -> TestResult {
    let tools = Toolchain::discover()?;
    if !encoder_available(&tools, "libsvtav1")? {
        eprintln!("skipping libsvtav1 media integration test: encoder is unavailable");
        return Ok(());
    }

    let sandbox = Sandbox::new("svt_av1_mp4_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.mp4");

    generate_h264_aac_source(&tools, &input, 0.25, 32, 24)?;
    let mut config = video_config("mp4", "libsvtav1", "aac");
    config.preset = "ultrafast".to_string();
    config.crf = 38;
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("av1"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn prores_mov_reencode_should_write_prores_422_10bit_video() -> TestResult {
    let tools = Toolchain::discover()?;
    if !encoder_available(&tools, "prores")? {
        eprintln!("skipping prores media integration test: encoder is unavailable");
        return Ok(());
    }

    let sandbox = Sandbox::new("prores_mov_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.mov");

    generate_h264_aac_source(&tools, &input, 0.5, 48, 32)?;
    let config = video_config("mov", "prores", "aac");
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("prores"));
    assert_eq!(metadata.pixel_format.as_deref(), Some("yuv422p10le"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn x264_pixel_format_matrix_should_write_requested_formats() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("x264_pixel_format_matrix")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 0.5, 48, 32)?;

    for pixel_format in ["yuv420p", "yuv422p", "yuv444p", "yuv420p10le"] {
        let output = sandbox.path(&format!("output-{pixel_format}.mp4"));
        let mut config = video_config("mp4", "libx264", "aac");
        config.pixel_format = pixel_format.to_string();
        convert(&tools, &input, &output, &config)
            .map_err(|error| format!("x264 {pixel_format} output failed: {error}"))?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata.pixel_format.as_deref(),
            Some(pixel_format),
            "x264 should write requested {pixel_format} pixel format"
        );
    }

    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn audio_container_matrix_should_write_supported_audio_outputs() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio_container_matrix")?;
    let input = sandbox.path("source.wav");
    generate_audio_source(&tools, &input)?;

    for case in [
        ("mp3", "mp3", "mp3"),
        ("m4a", "aac", "aac"),
        ("wav", "pcm_s16le", "pcm_s16le"),
        ("flac", "flac", "flac"),
    ] {
        let output = sandbox.path(&format!("output.{}", case.0));
        let mut config = audio_config(case.0, case.1);
        config.selected_audio_tracks = vec![0];
        convert(&tools, &input, &output, &config)
            .map_err(|error| format!("{} audio output failed: {error}", case.0))?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata.audio_codec.as_deref(),
            Some(case.2),
            "{} should produce {} audio",
            case.0,
            case.2
        );
    }

    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn image_container_matrix_should_write_single_frame_outputs() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("image_container_matrix")?;
    let input = sandbox.path("quadrants.ppm");
    write_quadrant_ppm(&input, 64, 48)?;

    for case in [
        ("png", "png", "png"),
        ("jpg", "mjpeg", "mjpeg"),
        ("webp", "libwebp", "webp"),
        ("bmp", "bmp", "bmp"),
        ("tiff", "tiff", "tiff"),
    ] {
        let output = sandbox.path(&format!("output.{}", case.0));
        let config = image_config(case.0, case.1);
        convert(&tools, &input, &output, &config)
            .map_err(|error| format!("{} image output failed: {error}", case.0))?;
        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata.media_kind, "image",
            "{} should probe as image",
            case.0
        );
        assert_eq!(
            metadata.video_codec.as_deref(),
            Some(case.2),
            "{} should produce {} image codec",
            case.0,
            case.2
        );
    }

    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn image_sequence_container_matrix_should_write_numbered_frames() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("image_sequence_container_matrix")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 0.5, 64, 48)?;

    for (container, codec) in [
        ("png", "png"),
        ("jpg", "mjpeg"),
        ("webp", "libwebp"),
        ("bmp", "bmp"),
        ("tiff", "tiff"),
    ] {
        if !encoder_available(&tools, codec)? {
            eprintln!("skipping unavailable image encoder {codec}");
            continue;
        }
        let result = sandbox.path(&format!("{container}_frames"));
        fs::create_dir(&result).map_err(|error| error.to_string())?;
        let output = result.join(format!("frame_%06d.{container}"));
        let mut config = image_config(container, codec);
        config.image_output_mode = "sequence".to_string();
        let input_arg = path_arg(&input);
        let output_arg = path_arg(&output);
        validate_task_input(&input_arg, &config).map_err(|error| error.to_string())?;
        let probe = probe_media(&tools, &input)?;
        let args = build_ffmpeg_args(&input_arg, &output_arg, &config, &probe)
            .map_err(|error| error.to_string())?;
        run_tool(&tools.ffmpeg, &args)
            .map_err(|error| format!("{container} image sequence failed: {error}"))?;

        let mut frames = fs::read_dir(&result)
            .map_err(|error| error.to_string())?
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        frames.sort();
        assert!(!frames.is_empty(), "{container} should emit frames");
        assert_eq!(frames[0], format!("frame_000001.{container}"));
    }

    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn gif_output_should_write_palette_gif_video() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("gif_output")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.gif");

    generate_h264_aac_source(&tools, &input, 0.75, 48, 32)?;
    let mut config = video_config("gif", "gif", "aac");
    config.fps = "6".to_string();
    config.gif_colors = 32;
    config.gif_dither = "bayer".to_string();
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("gif"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn odd_yuv420p_reencode_should_pad_to_even_dimensions() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("odd_yuv420p_reencode")?;
    let input = sandbox.path("odd.mov");
    let output = sandbox.path("output.mp4");

    generate_odd_h264_source(&tools, &input)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.pixel_format = "yuv420p".to_string();
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.width, Some(854));
    assert_eq!(metadata.height, Some(480));
    assert_eq!(metadata.pixel_format.as_deref(), Some("yuv420p"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn rotate_90_image_output_should_swap_dimensions_and_move_quadrants() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("rotate_90_image_output")?;
    let input = sandbox.path("quadrants.ppm");
    let output = sandbox.path("rotated.png");

    write_quadrant_ppm(&input, 64, 48)?;
    let mut config = image_config("png", "png");
    config.rotation = "90".to_string();
    convert(&tools, &input, &output, &config)?;

    let frame = read_rgb_frame(&tools, &output)?;
    assert_eq!((frame.width, frame.height), (48, 64));
    assert_color_near(frame.pixel(2, 2)?, Rgb::BLUE, 2, "top-left")?;
    assert_color_near(frame.pixel(frame.width - 3, 2)?, Rgb::RED, 2, "top-right")?;
    assert_color_near(
        frame.pixel(2, frame.height - 3)?,
        Rgb::YELLOW,
        2,
        "bottom-left",
    )?;
    assert_color_near(
        frame.pixel(frame.width - 3, frame.height - 3)?,
        Rgb::GREEN,
        2,
        "bottom-right",
    )?;
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn flip_horizontal_image_output_should_mirror_quadrants() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("flip_horizontal_image_output")?;
    let input = sandbox.path("quadrants.ppm");
    let output = sandbox.path("flipped.png");

    write_quadrant_ppm(&input, 64, 48)?;
    let mut config = image_config("png", "png");
    config.flip_horizontal = true;
    convert(&tools, &input, &output, &config)?;

    let frame = read_rgb_frame(&tools, &output)?;
    assert_color_near(frame.pixel(2, 2)?, Rgb::GREEN, 2, "top-left")?;
    assert_color_near(frame.pixel(frame.width - 3, 2)?, Rgb::RED, 2, "top-right")?;
    assert_color_near(
        frame.pixel(2, frame.height - 3)?,
        Rgb::YELLOW,
        2,
        "bottom-left",
    )?;
    assert_color_near(
        frame.pixel(frame.width - 3, frame.height - 3)?,
        Rgb::BLUE,
        2,
        "bottom-right",
    )?;
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn flip_vertical_image_output_should_mirror_quadrants() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("flip_vertical_image_output")?;
    let input = sandbox.path("quadrants.ppm");
    let output = sandbox.path("flipped.png");

    write_quadrant_ppm(&input, 64, 48)?;
    let mut config = image_config("png", "png");
    config.flip_vertical = true;
    convert(&tools, &input, &output, &config)?;

    let frame = read_rgb_frame(&tools, &output)?;
    assert_color_near(frame.pixel(2, 2)?, Rgb::BLUE, 2, "top-left")?;
    assert_color_near(
        frame.pixel(frame.width - 3, 2)?,
        Rgb::YELLOW,
        2,
        "top-right",
    )?;
    assert_color_near(
        frame.pixel(2, frame.height - 3)?,
        Rgb::RED,
        2,
        "bottom-left",
    )?;
    assert_color_near(
        frame.pixel(frame.width - 3, frame.height - 3)?,
        Rgb::GREEN,
        2,
        "bottom-right",
    )?;
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn crop_image_output_should_emit_selected_region() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("crop_image_output")?;
    let input = sandbox.path("quadrants.ppm");
    let output = sandbox.path("cropped.png");

    write_quadrant_ppm(&input, 64, 48)?;
    let mut config = image_config("png", "png");
    config.crop = Some(CropConfig {
        enabled: true,
        x: 32.0,
        y: 0.0,
        width: 32.0,
        height: 24.0,
        source_width: Some(64.0),
        source_height: Some(48.0),
        aspect_ratio: None,
    });
    convert(&tools, &input, &output, &config)?;

    let frame = read_rgb_frame(&tools, &output)?;
    assert_eq!((frame.width, frame.height), (32, 24));
    assert_color_near(frame.pixel(2, 2)?, Rgb::GREEN, 2, "cropped top-left")?;
    assert_color_near(
        frame.pixel(frame.width - 3, frame.height - 3)?,
        Rgb::GREEN,
        2,
        "cropped bottom-right",
    )?;
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn overlay_image_output_should_composite_overlay_at_center() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("overlay_image_output")?;
    let input = sandbox.path("base.ppm");
    let overlay = sandbox.path("overlay.ppm");
    let output = sandbox.path("overlayed.png");

    write_solid_ppm(&input, 64, 48, Rgb::BLACK)?;
    write_solid_ppm(&overlay, 16, 16, Rgb::RED)?;
    let mut config = image_config("png", "png");
    config.overlay = Some(OverlayConfig {
        enabled: true,
        path: path_arg(&overlay),
        x: 0.5,
        y: 0.5,
        width: 0.25,
        opacity: 1.0,
        anchor: "center".to_string(),
    });
    convert(&tools, &input, &output, &config)?;

    let frame = read_rgb_frame(&tools, &output)?;
    assert_color_near(
        frame.pixel(frame.width / 2, frame.height / 2)?,
        Rgb::RED,
        2,
        "center",
    )?;
    assert_color_near(frame.pixel(2, 2)?, Rgb::BLACK, 2, "corner")?;
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn custom_resolution_should_pad_to_requested_canvas() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("custom_resolution")?;
    let input = sandbox.path("quadrants.ppm");
    let output = sandbox.path("scaled.png");

    write_quadrant_ppm(&input, 64, 32)?;
    let mut config = image_config("png", "png");
    config.resolution = "custom".to_string();
    config.custom_width = Some("80".to_string());
    config.custom_height = Some("80".to_string());
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.width, Some(80));
    assert_eq!(metadata.height, Some(80));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn trimmed_reencode_should_shorten_duration() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("trimmed_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("trimmed.mp4");

    generate_h264_aac_source(&tools, &input, 2.0, 64, 48)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.start_time = Some("00:00:00.250".to_string());
    config.end_time = Some("00:00:00.750".to_string());
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    let duration = duration_seconds(&metadata)?;
    assert!(
        (0.30..=0.80).contains(&duration),
        "trimmed duration should be close to 0.5s, got {duration}"
    );
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn stream_copy_should_preserve_h264_aac_streams() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("stream_copy")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("copied.mp4");

    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.processing_mode = "copy".to_string();
    config.selected_audio_tracks = vec![1];
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("h264"));
    assert_eq!(metadata.audio_codec.as_deref(), Some("aac"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn selected_audio_track_should_emit_only_requested_track() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("selected_audio_track")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("selected.mp4");

    generate_two_audio_track_source(&tools, &input)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.selected_audio_tracks = vec![2];
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.audio_tracks.len(), 1);
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn subtitle_stream_should_transcode_to_mov_text_in_mp4() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("subtitle_stream")?;
    let input = sandbox.path("source.mp4");
    let subtitle = sandbox.path("subtitle.srt");
    let output = sandbox.path("subtitled.mp4");

    write_srt(&subtitle)?;
    generate_subtitled_source(&tools, &input, &subtitle)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.selected_subtitle_tracks = vec![2];
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.subtitle_tracks.len(), 1);
    assert_eq!(
        metadata
            .subtitle_tracks
            .first()
            .map(|track| track.codec.as_str()),
        Some("mov_text")
    );
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn external_selectable_subtitle_should_survive_stream_copy_with_metadata() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("external_selectable_subtitle")?;
    let input = sandbox.path("source.mp4");
    let subtitle = sandbox.path("english.srt");
    let output = sandbox.path("subtitled.mp4");

    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;
    write_srt(&subtitle)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.processing_mode = "copy".to_string();
    config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
        path: path_arg(&subtitle),
        language: Some("eng".to_string()),
        title: Some("English".to_string()),
        is_default: true,
        is_forced: true,
    }];

    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.subtitle_tracks.len(), 1);
    assert_eq!(metadata.subtitle_tracks[0].codec, "mov_text");
    assert_eq!(metadata.subtitle_tracks[0].language.as_deref(), Some("eng"));
    assert_eq!(
        metadata.subtitle_tracks[0].label.as_deref(),
        Some("English")
    );

    let probe_args = args(&[
        "-v",
        "error",
        "-select_streams",
        "s:0",
        "-show_entries",
        "stream_disposition=default,forced",
        "-of",
        "json",
        &path_arg(&output),
    ]);
    let output_json: serde_json::Value =
        serde_json::from_slice(&run_tool_output(&tools.ffprobe, &probe_args)?)
            .map_err(|error| format!("failed to parse subtitle disposition: {error}"))?;
    let disposition = &output_json["streams"][0]["disposition"];
    assert_eq!(disposition["default"], 1);
    assert_eq!(disposition["forced"], 1);
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn external_selectable_subtitle_should_use_each_container_codec_contract() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("external_subtitle_container_matrix")?;
    let input = sandbox.path("source.mp4");
    let subtitle = sandbox.path("captions.srt");
    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;
    write_srt(&subtitle)?;

    for (container, video_codec, audio_codec, expected_subtitle_codec) in [
        ("mp4", "libx264", "aac", "mov_text"),
        ("mov", "libx264", "aac", "mov_text"),
        ("mkv", "libx264", "aac", "subrip"),
        ("webm", "vp9", "libopus", "webvtt"),
    ] {
        let output = sandbox.path(&format!("subtitled.{container}"));
        let mut config = video_config(container, video_codec, audio_codec);
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: path_arg(&subtitle),
            ..ExternalSubtitleTrack::default()
        }];

        convert(&tools, &input, &output, &config)?;

        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata
                .subtitle_tracks
                .first()
                .map(|track| track.codec.as_str()),
            Some(expected_subtitle_codec),
            "unexpected external subtitle codec for {container}"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn every_supported_external_subtitle_format_should_embed_in_mkv() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("external_subtitle_format_matrix")?;
    let input = sandbox.path("source.mp4");
    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;

    for (extension, contents, expected_codec) in [
        (
            "srt",
            "1\n00:00:00,000 --> 00:00:00,900\nSubRip subtitle\n",
            "subrip",
        ),
        (
            "ass",
            "[Script Info]\nScriptType: v4.00+\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:00.90,Default,,0,0,0,,ASS subtitle\n",
            "ass",
        ),
        (
            "vtt",
            "WEBVTT\n\n00:00.000 --> 00:00.900\nWebVTT subtitle\n",
            "webvtt",
        ),
    ] {
        let subtitle = sandbox.path(&format!("captions.{extension}"));
        let output = sandbox.path(&format!("subtitled-{extension}.mkv"));
        fs::write(&subtitle, contents)
            .map_err(|error| format!("failed to write {}: {error}", subtitle.display()))?;
        let mut config = video_config("mkv", "libx264", "aac");
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: path_arg(&subtitle),
            ..ExternalSubtitleTrack::default()
        }];

        convert(&tools, &input, &output, &config)?;

        let metadata = probe_media(&tools, &output)?;
        assert_eq!(
            metadata
                .subtitle_tracks
                .first()
                .map(|track| track.codec.as_str()),
            Some(expected_codec),
            "unexpected MKV subtitle codec for .{extension}"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn trimmed_external_selectable_subtitle_should_keep_source_relative_timing() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("trimmed_external_subtitle")?;
    let input = sandbox.path("source.mp4");
    let subtitle = sandbox.path("captions.srt");
    let output = sandbox.path("trimmed.mp4");
    generate_h264_aac_source(&tools, &input, 3.0, 64, 48)?;
    fs::write(
        &subtitle,
        "1\n00:00:01,200 --> 00:00:02,200\nTrimmed subtitle\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", subtitle.display()))?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.start_time = Some("00:00:01.000".to_string());
    config.end_time = Some("00:00:02.500".to_string());
    config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
        path: path_arg(&subtitle),
        ..ExternalSubtitleTrack::default()
    }];

    convert(&tools, &input, &output, &config)?;

    let packet_args = args(&[
        "-v",
        "error",
        "-select_streams",
        "s:0",
        "-show_entries",
        "packet=pts_time,duration_time,size",
        "-of",
        "json",
        &path_arg(&output),
    ]);
    let packets: serde_json::Value =
        serde_json::from_slice(&run_tool_output(&tools.ffprobe, &packet_args)?)
            .map_err(|error| format!("failed to parse subtitle packets: {error}"))?;
    let timed_packet = packets["packets"]
        .as_array()
        .and_then(|packets| {
            packets.iter().find(|packet| {
                packet["size"]
                    .as_str()
                    .and_then(|size| size.parse::<u64>().ok())
                    .is_some_and(|size| size > 2)
            })
        })
        .ok_or_else(|| "trimmed output has no subtitle payload packet".to_string())?;
    let pts = timed_packet["pts_time"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .ok_or_else(|| "subtitle payload packet has no timestamp".to_string())?;
    assert!((pts - 0.2).abs() < 0.05, "unexpected subtitle PTS: {pts}");
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn metadata_replace_should_write_requested_title() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("metadata_replace")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("metadata.mp4");

    generate_h264_aac_source(&tools, &input, 0.5, 48, 32)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.metadata = MetadataConfig {
        mode: MetadataMode::Replace,
        title: Some("Frame Media Integration".to_string()),
        artist: Some("Frame".to_string()),
        album: None,
        genre: None,
        date: None,
        comment: None,
        ..MetadataConfig::default()
    };
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(
        metadata.tags.and_then(|tags| tags.title).as_deref(),
        Some("Frame Media Integration")
    );
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn audio_normalize_and_mono_wav_should_emit_mono_pcm() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio_normalize_mono")?;
    let input = sandbox.path("source.wav");
    let output = sandbox.path("mono.wav");

    generate_audio_source(&tools, &input)?;
    let mut config = audio_config("wav", "pcm_s16le");
    config.selected_audio_tracks = vec![0];
    config.audio_normalize = true;
    config.audio_channels = "mono".to_string();
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.audio_codec.as_deref(), Some("pcm_s16le"));
    assert_eq!(
        metadata
            .audio_tracks
            .first()
            .map(|track| track.channels.as_str()),
        Some("1")
    );
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn subtitle_burn_should_encode_video_when_srt_is_present() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("subtitle_burn")?;
    let input = sandbox.path("source.mp4");
    let subtitle = sandbox.path("subtitle.srt");
    let output = sandbox.path("burned.mp4");

    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;
    write_srt(&subtitle)?;
    let mut config = video_config("mp4", "libx264", "aac");
    config.subtitle_burn_path = Some(path_arg(&subtitle));
    config.subtitle_font_size = Some("16".to_string());
    config.subtitle_font_color = Some("#ffffff".to_string());
    config.subtitle_outline_color = Some("#000000".to_string());
    config.subtitle_position = Some("bottom".to_string());
    convert(&tools, &input, &output, &config)?;

    let metadata = probe_media(&tools, &output)?;
    assert_eq!(metadata.video_codec.as_deref(), Some("h264"));
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn preview_subtitle_burn_should_use_source_time_after_seek() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("preview_subtitle_seek")?;
    let input = sandbox.path("source.mp4");
    let subtitle = sandbox.path("delayed.srt");

    generate_black_h264_source(&tools, &input, 2.0, 160, 90)?;
    write_delayed_srt(&subtitle)?;

    let mut config = video_config("mp4", "libx264", "aac");
    config.subtitle_burn_path = Some(path_arg(&subtitle));
    config.subtitle_font_size = Some("24".to_string());
    config.subtitle_font_color = Some("#ffffff".to_string());
    config.subtitle_outline_color = Some("#000000".to_string());
    config.subtitle_position = Some("middle".to_string());

    let plan = build_ffmpeg_preview_args(
        &path_arg(&input),
        &config,
        &PreviewFfmpegOptions {
            start_seconds: 1.0,
            end_seconds: Some(1.2),
            source_width: Some(160),
            source_height: Some(90),
            max_width: 160,
            max_height: 90,
            fps: 1,
            realtime: false,
            precise_seek: true,
            source_is_image: false,
        },
    )
    .map_err(|error| error.to_string())?;

    let mut args = plan.args.clone();
    let insert_at = args.len().saturating_sub(1);
    args.insert(insert_at, "-frames:v".to_string());
    args.insert(insert_at + 1, "1".to_string());
    let output = run_tool_output(&tools.ffmpeg, &args)?;
    if output.len() < plan.frame_bytes {
        return Err(format!(
            "preview frame was too short: got {}, expected at least {}",
            output.len(),
            plan.frame_bytes
        ));
    }

    let visible_pixels = count_visible_bgra_pixels(&output[..plan.frame_bytes]);
    if visible_pixels < 50 {
        return Err(format!(
            "seeked preview did not render delayed subtitle; visible pixel count was {visible_pixels}"
        ));
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn display_matrix_rotation_should_produce_portrait_preview_frame() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("display_matrix_rotation")?;
    let base = sandbox.path("base.mp4");
    let rotated = sandbox.path("rotated.mov");

    generate_h264_aac_source(&tools, &base, 0.25, 160, 90)?;
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-display_rotation:v:0",
            "90",
            "-i",
            &path_arg(&base),
            "-map",
            "0:v:0",
            "-c",
            "copy",
            "-y",
            &path_arg(&rotated),
        ]),
    )?;

    let metadata = probe_media(&tools, &rotated)?;
    let plan = build_ffmpeg_preview_args(
        &path_arg(&rotated),
        &video_config("mov", "libx264", "aac"),
        &PreviewFfmpegOptions {
            start_seconds: 0.0,
            end_seconds: Some(0.2),
            source_width: metadata.width,
            source_height: metadata.height,
            max_width: 160,
            max_height: 160,
            fps: 12,
            realtime: false,
            precise_seek: true,
            source_is_image: false,
        },
    )
    .map_err(|error| error.to_string())?;
    let mut preview_args = plan.args.clone();
    let output_index = preview_args.len().saturating_sub(1);
    preview_args.insert(output_index, "-frames:v".to_string());
    preview_args.insert(output_index + 1, "1".to_string());
    let frame = run_tool_output(&tools.ffmpeg, &preview_args)?;

    assert_eq!(
        (
            metadata.width,
            metadata.height,
            plan.width,
            plan.height,
            frame.len()
        ),
        (Some(90), Some(160), 90, 160, plan.frame_bytes),
        "preview args: {:?}",
        plan.args
    );
    Ok(())
}

fn convert(
    tools: &Toolchain,
    input: &Path,
    output: &Path,
    config: &ConversionConfig,
) -> TestResult {
    let input = path_arg(input);
    let output = path_arg(output);
    validate_task_input(&input, config).map_err(|error| error.to_string())?;
    let probe = probe_media(tools, Path::new(&input))?;
    let args =
        build_ffmpeg_args(&input, &output, config, &probe).map_err(|error| error.to_string())?;
    run_tool(&tools.ffmpeg, &args)?;

    let output_path = Path::new(&output);
    if !output_path.is_file() {
        return Err(format!(
            "conversion did not create {}",
            output_path.display()
        ));
    }
    Ok(())
}

fn video_config(container: &str, video_codec: &str, audio_codec: &str) -> ConversionConfig {
    let mut config = base_config(container, video_codec);
    config.audio_codec = audio_codec.to_string();
    if audio_codec == "libopus" {
        config.audio_bitrate = "96".to_string();
    }
    config
}

fn image_config(container: &str, video_codec: &str) -> ConversionConfig {
    let mut config = base_config(container, video_codec);
    config.image_jpeg_quality = 85;
    config.image_webp_quality = 85;
    config
}

fn audio_config(container: &str, audio_codec: &str) -> ConversionConfig {
    let mut config = base_config(container, "libx264");
    config.audio_codec = audio_codec.to_string();
    config
}

fn base_config(container: &str, video_codec: &str) -> ConversionConfig {
    ConversionConfig {
        processing_mode: "reencode".to_string(),
        container: container.to_string(),
        video_codec: video_codec.to_string(),
        video_bitrate_mode: "crf".to_string(),
        video_bitrate: "5000".to_string(),
        audio_codec: "aac".to_string(),
        audio_bitrate: "96".to_string(),
        audio_bitrate_mode: "bitrate".to_string(),
        audio_quality: "4".to_string(),
        audio_channels: "original".to_string(),
        audio_volume: 100.0,
        audio_normalize: false,
        video_filters: frame_core::types::VideoFiltersConfig::default(),
        audio_filters: frame_core::types::AudioFiltersConfig::default(),
        selected_audio_tracks: Vec::new(),
        selected_subtitle_tracks: Vec::new(),
        external_subtitle_tracks: Vec::new(),
        subtitle_burn_path: None,
        subtitle_font_name: None,
        subtitle_font_size: None,
        subtitle_font_color: None,
        subtitle_outline_color: None,
        subtitle_position: None,
        resolution: "original".to_string(),
        custom_width: None,
        custom_height: None,
        scaling_algorithm: "bicubic".to_string(),
        fps: "original".to_string(),
        crf: 28,
        quality: 60,
        preset: "ultrafast".to_string(),
        start_time: None,
        end_time: None,
        metadata: MetadataConfig::default(),
        rotation: "0".to_string(),
        flip_horizontal: false,
        flip_vertical: false,
        crop: None,
        overlay: None,
        nvenc_spatial_aq: false,
        nvenc_temporal_aq: false,
        videotoolbox_allow_sw: false,
        hw_decode: false,
        pixel_format: "auto".to_string(),
        image_output_mode: "single".to_string(),
        image_jpeg_quality: 85,
        image_jpeg_huffman: "optimal".to_string(),
        image_webp_lossless: false,
        image_webp_quality: 75,
        image_webp_compression: 4,
        image_webp_preset: "default".to_string(),
        image_png_compression: 9,
        image_png_prediction: "paeth".to_string(),
        image_tiff_compression: "packbits".to_string(),
        gif_colors: 256,
        gif_dither: "sierra2_4a".to_string(),
        gif_loop: 0,
    }
}

fn generate_h264_aac_source(
    tools: &Toolchain,
    output: &Path,
    duration: f64,
    width: u32,
    height: u32,
) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc2=size={width}x{height}:rate=12:duration={duration:.3}"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:sample_rate=48000:duration={duration:.3}"),
            "-shortest",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn generate_h264_ac3_source(tools: &Toolchain, output: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x90:rate=12:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=1",
            "-shortest",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "ac3",
            "-b:a",
            "192k",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn decode_media(tools: &Toolchain, input: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            &path_arg(input),
            "-map",
            "0:v?",
            "-map",
            "0:a?",
            "-f",
            "null",
            "-",
        ]),
    )
}

fn generate_two_audio_track_source(tools: &Toolchain, output: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=12:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:duration=1",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-map",
            "2:a:0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn generate_subtitled_source(tools: &Toolchain, output: &Path, subtitle: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=12:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=1",
            "-i",
            &path_arg(subtitle),
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-map",
            "2:s:0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
            "-c:s",
            "mov_text",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn generate_black_h264_source(
    tools: &Toolchain,
    output: &Path,
    duration: f64,
    width: u32,
    height: u32,
) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black:size={width}x{height}:rate=12:duration={duration:.3}"),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-an",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn generate_odd_h264_source(tools: &Toolchain, output: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=853x480:rate=1:duration=1",
            "-vf",
            "format=yuv444p",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv444p",
            "-an",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn generate_audio_source(tools: &Toolchain, output: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=1",
            "-c:a",
            "pcm_s16le",
            "-y",
            &path_arg(output),
        ]),
    )
}

fn probe_media(tools: &Toolchain, path: &Path) -> Result<ProbeMetadata, String> {
    let probe_args = ffprobe_json_args(&path_arg(path));
    let stdout = run_tool_output(&tools.ffprobe, &probe_args)?;
    let stdout = String::from_utf8(stdout)
        .map_err(|error| format!("ffprobe stdout was not utf8: {error}"))?;
    parse_ffprobe_stdout(&path_arg(path), stdout).map_err(|error| error.to_string())
}

fn read_rgb_frame(tools: &Toolchain, path: &Path) -> Result<RgbFrame, String> {
    let metadata = probe_media(tools, path)?;
    let width = metadata
        .width
        .ok_or_else(|| format!("{} has no probed width", path.display()))?;
    let height = metadata
        .height
        .ok_or_else(|| format!("{} has no probed height", path.display()))?;
    let output = run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(path),
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ]),
    )?;
    let expected_len = usize::try_from(width)
        .and_then(|w| usize::try_from(height).map(|h| w * h * 3))
        .map_err(|error| format!("frame size overflow: {error}"))?;
    if output.len() != expected_len {
        return Err(format!(
            "raw frame length mismatch for {}: got {}, expected {expected_len}",
            path.display(),
            output.len()
        ));
    }
    Ok(RgbFrame {
        width,
        height,
        pixels: output,
    })
}

fn duration_seconds(metadata: &ProbeMetadata) -> Result<f64, String> {
    metadata
        .duration
        .as_deref()
        .ok_or_else(|| "metadata has no duration".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("duration was not numeric: {error}"))
}

fn write_quadrant_ppm(path: &Path, width: u32, height: u32) -> TestResult {
    let mut file = create_ppm(path, width, height)?;
    for y in 0..height {
        for x in 0..width {
            let color = match (x < width / 2, y < height / 2) {
                (true, true) => Rgb::RED,
                (false, true) => Rgb::GREEN,
                (true, false) => Rgb::BLUE,
                (false, false) => Rgb::YELLOW,
            };
            file.write_all(&[color.red, color.green, color.blue])
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn write_solid_ppm(path: &Path, width: u32, height: u32, color: Rgb) -> TestResult {
    let mut file = create_ppm(path, width, height)?;
    for _ in 0..width * height {
        file.write_all(&[color.red, color.green, color.blue])
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn create_ppm(path: &Path, width: u32, height: u32) -> Result<File, String> {
    let mut file = File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    write!(file, "P6\n{width} {height}\n255\n")
        .map_err(|error| format!("failed to write {} header: {error}", path.display()))?;
    Ok(file)
}

fn write_srt(path: &Path) -> TestResult {
    fs::write(
        path,
        "1\n00:00:00,000 --> 00:00:00,900\nFrame subtitle integration\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn write_delayed_srt(path: &Path) -> TestResult {
    fs::write(path, "1\n00:00:01,000 --> 00:00:01,900\nVISIBLE\n")
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn count_visible_bgra_pixels(bytes: &[u8]) -> usize {
    bytes
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 35 || pixel[1] > 35 || pixel[2] > 35)
        .count()
}

fn assert_color_near(actual: Rgb, expected: Rgb, tolerance: u8, label: &str) -> TestResult {
    let tolerance = i16::from(tolerance);
    for (channel, actual, expected) in [
        ("red", actual.red, expected.red),
        ("green", actual.green, expected.green),
        ("blue", actual.blue, expected.blue),
    ] {
        let delta = (i16::from(actual) - i16::from(expected)).abs();
        if delta > tolerance {
            return Err(format!(
                "{label} {channel} channel mismatch: got {actual}, expected {expected} +/- {tolerance}"
            ));
        }
    }
    Ok(())
}

fn discover_tool(tool: &str, env_var: &str) -> Result<PathBuf, String> {
    if let Some(value) = env::var_os(env_var)
        && !value.is_empty()
    {
        let path = PathBuf::from(value);
        verify_tool(&path)?;
        return Ok(path);
    }

    for candidate in bundled_tool_candidates(tool) {
        if candidate.is_file() {
            verify_tool(&candidate)?;
            return Ok(candidate);
        }
    }

    if let Some(path) = find_on_path(tool) {
        verify_tool(&path)?;
        return Ok(path);
    }

    Err(format!(
        "{tool} was not found. Set {env_var} or install {tool} on PATH."
    ))
}

fn bundled_tool_candidates(tool: &str) -> Vec<PathBuf> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from(".."), Path::to_path_buf);
    let binaries = workspace_root
        .join("frame-app")
        .join("resources")
        .join("binaries");
    let suffixes: &[&str] = match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => &["aarch64-apple-darwin"],
        ("macos", "x86_64") => &["x86_64-apple-darwin"],
        ("linux", "x86_64") => &["x86_64-unknown-linux-gnu"],
        ("windows", "x86_64") => &["x86_64-pc-windows-msvc.exe"],
        _ => &[],
    };

    suffixes
        .iter()
        .map(|suffix| binaries.join(format!("{tool}-{suffix}")))
        .collect()
}

fn find_on_path(tool: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let tool_names = path_tool_names(tool);
    env::split_paths(&path)
        .flat_map(|dir| tool_names.iter().map(move |tool_name| dir.join(tool_name)))
        .find(|candidate| candidate.is_file())
}

fn path_tool_names(tool: &str) -> Vec<String> {
    let has_exe_extension = Path::new(tool)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    if cfg!(windows) && !has_exe_extension {
        vec![format!("{tool}.exe"), tool.to_string()]
    } else {
        vec![tool.to_string()]
    }
}

fn verify_tool(path: &Path) -> TestResult {
    let status = Command::new(path)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("failed to run {} -version: {error}", path.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} -version exited with {status}", path.display()))
    }
}

fn encoder_available(tools: &Toolchain, encoder: &str) -> Result<bool, String> {
    let output = run_tool_output(&tools.ffmpeg, &args(&["-hide_banner", "-encoders"]))?;
    let output = String::from_utf8(output)
        .map_err(|error| format!("ffmpeg -encoders output was not utf8: {error}"))?;
    Ok(output.lines().any(|line| {
        line.split_whitespace()
            .nth(1)
            .is_some_and(|name| name == encoder)
    }))
}

fn run_tool(tool: &Path, args: &[String]) -> TestResult {
    run_tool_output(tool, args).map(|_| ())
}

fn run_tool_output(tool: &Path, args: &[String]) -> Result<Vec<u8>, String> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", command_label(tool, args)))?;
    if output.status.success() {
        return Ok(output.stdout);
    }

    Err(format!(
        "{} exited with {}\nstdout:\n{}\nstderr:\n{}",
        command_label(tool, args),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn command_label(tool: &Path, args: &[String]) -> String {
    std::iter::once(tool.as_os_str())
        .chain(args.iter().map(OsStr::new))
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}
