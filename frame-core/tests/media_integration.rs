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
        ConversionConfig, CropConfig, MetadataConfig, MetadataMode, OverlayConfig, ProbeMetadata,
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
fn h264_mp4_reencode_should_write_h264_video_and_aac_audio() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("h264_mp4_reencode")?;
    let input = sandbox.path("source.mp4");
    let output = sandbox.path("output.mp4");

    generate_h264_aac_source(&tools, &input, 1.0, 64, 48)?;
    let config = video_config("mp4", "libx264", "aac");
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
    let config = video_config("webm", "vp9", "libopus");
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
        let config = audio_config(case.0, case.1);
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
