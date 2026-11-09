use crate::{
    dimensions::{OutputRasterPolicy, output_dimensions, output_raster_policy},
    error::ConversionError,
    filters::{
        PREVIEW_OUTPUT_LABEL, VisualFilterBase, VisualFilterContext, VisualFilterProfile,
        build_audio_filters, build_visual_filter_complex_with_context, has_overlay,
    },
    types::ConversionConfig,
    utils::format_ffmpeg_seconds as format_seconds,
};

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Playback pacing, seek accuracy, source kind and palette storage are independent preview facts."
)]
pub struct PreviewFfmpegOptions {
    pub start_seconds: f64,
    pub end_seconds: Option<f64>,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub source_is_palette: bool,
    pub max_width: u32,
    pub max_height: u32,
    pub fps: u32,
    pub realtime: bool,
    pub precise_seek: bool,
    pub source_is_image: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewAudioFfmpegOptions {
    pub start_seconds: f64,
    pub end_seconds: Option<f64>,
    pub sample_rate: u32,
    pub channels: u16,
    pub realtime: bool,
    pub precise_seek: bool,
    pub selected_track: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFfmpegPlan {
    pub args: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub frame_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewAudioFfmpegPlan {
    pub args: Vec<String>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreviewGeometry {
    preview_width: u32,
    preview_height: u32,
}

/// Builds an `FFmpeg` command plan that streams low-resolution BGRA preview
/// frames to stdout.
///
/// # Errors
///
/// Returns an error when the requested preview bounds, FPS, seek position, or
/// frame byte size are invalid.
pub fn build_ffmpeg_preview_args(
    input: &str,
    config: &ConversionConfig,
    options: &PreviewFfmpegOptions,
) -> Result<PreviewFfmpegPlan, ConversionError> {
    validate_preview_options(input, options)?;

    let fps = effective_preview_fps(config, options.fps);
    let geometry = preview_geometry(config, options)?;
    let frame_bytes = frame_bytes(geometry.preview_width, geometry.preview_height)?;
    let base = match output_raster_policy(&config.container, config.processing_mode == "copy") {
        OutputRasterPolicy::Source => VisualFilterBase::Source,
        OutputRasterPolicy::Exact => VisualFilterBase::Image,
        OutputRasterPolicy::Even => VisualFilterBase::Video,
    };
    let graph = build_visual_filter_complex_with_context(
        config,
        VisualFilterProfile::PreviewLowRes {
            base,
            width: geometry.preview_width,
            height: geometry.preview_height,
            fps,
            source_time_seconds: options.start_seconds,
        },
        VisualFilterContext {
            source_is_palette: options.source_is_palette,
            ..VisualFilterContext::new(options.source_width, options.source_height)
        },
    );

    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "warning".to_string(),
    ];

    if options.start_seconds > 0.0 {
        args.push("-ss".to_string());
        args.push(format_seconds(options.start_seconds));
        if !options.precise_seek && options.end_seconds.is_none() {
            args.push("-noaccurate_seek".to_string());
        }
    }
    if options.realtime {
        args.push("-readrate".to_string());
        args.push("1".to_string());
    }
    args.push("-i".to_string());
    args.push(input.to_string());

    if base != VisualFilterBase::Source
        && has_overlay(config)
        && let Some(overlay) = &config.overlay
    {
        args.push("-i".to_string());
        args.push(overlay.path.clone());
    }

    if let Some(duration) = preview_duration(options) {
        args.push("-t".to_string());
        args.push(format_seconds(duration));
    }

    args.extend([
        "-filter_complex".to_string(),
        graph,
        "-map".to_string(),
        format!("[{PREVIEW_OUTPUT_LABEL}]"),
        "-an".to_string(),
        "-sn".to_string(),
        "-dn".to_string(),
        "-pix_fmt".to_string(),
        "bgra".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "pipe:1".to_string(),
    ]);

    Ok(PreviewFfmpegPlan {
        args,
        width: geometry.preview_width,
        height: geometry.preview_height,
        fps,
        frame_bytes,
    })
}

/// Builds an `FFmpeg` command plan that streams decoded preview audio as PCM
/// samples to stdout.
///
/// # Errors
///
/// Returns an error when the requested sample format, seek position, or input
/// path is invalid.
pub fn build_ffmpeg_preview_audio_args(
    input: &str,
    config: &ConversionConfig,
    options: &PreviewAudioFfmpegOptions,
) -> Result<PreviewAudioFfmpegPlan, ConversionError> {
    validate_preview_audio_options(input, options)?;

    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "warning".to_string(),
    ];

    if options.start_seconds > 0.0 {
        args.push("-ss".to_string());
        args.push(format_seconds(options.start_seconds));
        if !options.precise_seek && options.end_seconds.is_none() {
            args.push("-noaccurate_seek".to_string());
        }
    }
    if options.realtime {
        args.push("-readrate".to_string());
        args.push("1".to_string());
    }
    args.push("-i".to_string());
    args.push(input.to_string());

    if let Some(duration) = audio_preview_duration(options) {
        args.push("-t".to_string());
        args.push(format_seconds(duration));
    }

    args.push("-vn".to_string());
    args.push("-sn".to_string());
    args.push("-dn".to_string());
    args.push("-map".to_string());
    args.push(format!("0:{}", options.selected_track));

    let audio_filters = build_audio_filters(config);
    if !audio_filters.is_empty() {
        args.push("-af".to_string());
        args.push(audio_filters.join(","));
    }

    args.extend([
        "-ac".to_string(),
        options.channels.to_string(),
        "-ar".to_string(),
        options.sample_rate.to_string(),
        "-f".to_string(),
        "f32le".to_string(),
        "pipe:1".to_string(),
    ]);

    Ok(PreviewAudioFfmpegPlan {
        args,
        sample_rate: options.sample_rate,
        channels: options.channels,
    })
}

fn validate_preview_options(
    input: &str,
    options: &PreviewFfmpegOptions,
) -> Result<(), ConversionError> {
    if input.trim().is_empty() {
        return Err(ConversionError::InvalidInput(
            "Preview input path cannot be empty".to_string(),
        ));
    }
    if !options.start_seconds.is_finite() || options.start_seconds < 0.0 {
        return Err(ConversionError::InvalidInput(
            "Preview start position must be a positive finite number".to_string(),
        ));
    }
    if let Some(end_seconds) = options.end_seconds
        && (!end_seconds.is_finite() || end_seconds <= options.start_seconds)
    {
        return Err(ConversionError::InvalidInput(
            "Preview end position must be greater than start position".to_string(),
        ));
    }
    if options.max_width == 0 || options.max_height == 0 {
        return Err(ConversionError::InvalidInput(
            "Preview maximum dimensions must be non-zero".to_string(),
        ));
    }
    if options.fps == 0 {
        return Err(ConversionError::InvalidInput(
            "Preview FPS must be non-zero".to_string(),
        ));
    }
    Ok(())
}

fn preview_duration(options: &PreviewFfmpegOptions) -> Option<f64> {
    options
        .end_seconds
        .map(|end_seconds| end_seconds - options.start_seconds)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
}

fn validate_preview_audio_options(
    input: &str,
    options: &PreviewAudioFfmpegOptions,
) -> Result<(), ConversionError> {
    if input.trim().is_empty() {
        return Err(ConversionError::InvalidInput(
            "Preview input path cannot be empty".to_string(),
        ));
    }
    if !options.start_seconds.is_finite() || options.start_seconds < 0.0 {
        return Err(ConversionError::InvalidInput(
            "Preview start position must be a positive finite number".to_string(),
        ));
    }
    if let Some(end_seconds) = options.end_seconds
        && (!end_seconds.is_finite() || end_seconds <= options.start_seconds)
    {
        return Err(ConversionError::InvalidInput(
            "Preview end position must be greater than start position".to_string(),
        ));
    }
    if options.sample_rate == 0 {
        return Err(ConversionError::InvalidInput(
            "Preview audio sample rate must be non-zero".to_string(),
        ));
    }
    if options.channels == 0 {
        return Err(ConversionError::InvalidInput(
            "Preview audio channel count must be non-zero".to_string(),
        ));
    }
    Ok(())
}

fn audio_preview_duration(options: &PreviewAudioFfmpegOptions) -> Option<f64> {
    options
        .end_seconds
        .map(|end_seconds| end_seconds - options.start_seconds)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
}

fn effective_preview_fps(config: &ConversionConfig, preview_fps: u32) -> u32 {
    if config.processing_mode == "copy" {
        return preview_fps;
    }
    config
        .fps
        .parse::<u32>()
        .ok()
        .filter(|fps| *fps > 0)
        .map_or(preview_fps, |export_fps| export_fps.min(preview_fps).max(1))
}

fn preview_geometry(
    config: &ConversionConfig,
    options: &PreviewFfmpegOptions,
) -> Result<PreviewGeometry, ConversionError> {
    let export = export_dimensions(config, options)?;

    let preview = fit_dimensions(
        export.width,
        export.height,
        options.max_width,
        options.max_height,
    );

    Ok(PreviewGeometry {
        preview_width: preview.width,
        preview_height: preview.height,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Dimensions {
    width: u32,
    height: u32,
}

fn export_dimensions(
    config: &ConversionConfig,
    options: &PreviewFfmpegOptions,
) -> Result<Dimensions, ConversionError> {
    let (source_width, source_height) = match (options.source_width, options.source_height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => (width, height),
        (Some(_), Some(_)) => {
            return Err(ConversionError::InvalidInput(
                "Preview source dimensions must be non-zero".to_string(),
            ));
        }
        _ => source_dimensions_from_custom_resolution(config).ok_or_else(|| {
            ConversionError::InvalidInput(
                "Preview source dimensions are required for dynamic output geometry".to_string(),
            )
        })?,
    };

    if output_raster_policy(&config.container, config.processing_mode == "copy")
        == OutputRasterPolicy::Source
    {
        return Ok(Dimensions {
            width: source_width,
            height: source_height,
        });
    }

    let (width, height) =
        output_dimensions(config, source_width, source_height, None).ok_or_else(|| {
            ConversionError::InvalidInput("Preview output dimensions are invalid".to_string())
        })?;
    Ok(Dimensions { width, height })
}

fn source_dimensions_from_custom_resolution(config: &ConversionConfig) -> Option<(u32, u32)> {
    if config.resolution != "custom" {
        return None;
    }
    let width = parse_dimension(config.custom_width.as_deref())?;
    let height = parse_dimension(config.custom_height.as_deref())?;
    Some((width, height))
}

fn fit_dimensions(
    source_width: u32,
    source_height: u32,
    max_width: u32,
    max_height: u32,
) -> Dimensions {
    let width_scale = f64::from(max_width) / f64::from(source_width);
    let height_scale = f64::from(max_height) / f64::from(source_height);
    let scale = width_scale.min(height_scale).min(1.0);

    Dimensions {
        width: round_to_u32(f64::from(source_width) * scale),
        height: round_to_u32(f64::from(source_height) * scale),
    }
}

fn parse_dimension(value: Option<&str>) -> Option<u32> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-1")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
}

fn round_to_u32(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 1;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "preview dimensions are finite and clamped into u32 range"
    )]
    let converted = value.round().min(f64::from(u32::MAX)) as u32;
    converted.max(1)
}

fn frame_bytes(width: u32, height: u32) -> Result<usize, ConversionError> {
    let pixels = width.checked_mul(height).ok_or_else(|| {
        ConversionError::InvalidInput("Preview frame dimensions are too large".to_string())
    })?;
    let bytes = pixels.checked_mul(4).ok_or_else(|| {
        ConversionError::InvalidInput("Preview frame byte size is too large".to_string())
    })?;
    usize::try_from(bytes).map_err(|_| {
        ConversionError::InvalidInput("Preview frame byte size is too large".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CropConfig, MetadataConfig, OverlayConfig};

    fn default_config() -> ConversionConfig {
        ConversionConfig {
            processing_mode: "reencode".to_string(),
            container: "mp4".to_string(),
            video_codec: "libx264".to_string(),
            video_bitrate_mode: "crf".to_string(),
            video_bitrate: "5000".to_string(),
            audio_codec: "aac".to_string(),
            audio_bitrate: "192".to_string(),
            audio_bitrate_mode: "bitrate".to_string(),
            opus_vbr: true,
            audio_quality: "4".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            video_filters: crate::types::VideoFiltersConfig::default(),
            audio_filters: crate::types::AudioFiltersConfig::default(),
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
            external_subtitle_tracks: vec![],
            subtitle_burn_path: None,
            subtitle_font_name: None,
            subtitle_font_size: None,
            subtitle_font_color: None,
            subtitle_outline_color: None,
            subtitle_position: None,
            resolution: "original".to_string(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "lanczos".to_string(),
            fps: "original".to_string(),
            crf: 23,
            quality: 50,
            preset: "medium".to_string(),
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

    fn default_options() -> PreviewFfmpegOptions {
        PreviewFfmpegOptions {
            start_seconds: 12.345,
            end_seconds: Some(15.0),
            source_width: Some(1920),
            source_height: Some(1080),
            source_is_palette: false,
            max_width: 1280,
            max_height: 720,
            fps: 30,
            realtime: true,
            precise_seek: true,
            source_is_image: false,
        }
    }

    fn default_audio_options() -> PreviewAudioFfmpegOptions {
        PreviewAudioFfmpegOptions {
            start_seconds: 1.0,
            end_seconds: Some(3.5),
            sample_rate: 48_000,
            channels: 2,
            realtime: true,
            precise_seek: true,
            selected_track: 0,
        }
    }

    #[test]
    fn bounded_video_seek_uses_accurate_decoding_without_changing_other_arguments() {
        for start_seconds in [0.0, 7.2] {
            for end_seconds in [None, Some(10.0)] {
                for precise_seek in [false, true] {
                    let options = PreviewFfmpegOptions {
                        start_seconds,
                        end_seconds,
                        precise_seek,
                        ..default_options()
                    };
                    let plan = build_ffmpeg_preview_args("input.mp4", &default_config(), &options)
                        .unwrap();
                    assert_eq!(
                        plan.args.iter().any(|arg| arg == "-noaccurate_seek"),
                        start_seconds > 0.0 && end_seconds.is_none() && !precise_seek
                    );
                    let precise = build_ffmpeg_preview_args(
                        "input.mp4",
                        &default_config(),
                        &PreviewFfmpegOptions {
                            precise_seek: true,
                            ..options
                        },
                    )
                    .unwrap();
                    let mut args = plan.args;
                    args.retain(|arg| arg != "-noaccurate_seek");
                    assert_eq!(args, precise.args);
                }
            }
        }
    }

    #[test]
    fn bounded_audio_seek_uses_accurate_decoding_without_changing_other_arguments() {
        for start_seconds in [0.0, 7.2] {
            for end_seconds in [None, Some(10.0)] {
                for precise_seek in [false, true] {
                    let options = PreviewAudioFfmpegOptions {
                        start_seconds,
                        end_seconds,
                        precise_seek,
                        ..default_audio_options()
                    };
                    let plan =
                        build_ffmpeg_preview_audio_args("input.mp4", &default_config(), &options)
                            .unwrap();
                    assert_eq!(
                        plan.args.iter().any(|arg| arg == "-noaccurate_seek"),
                        start_seconds > 0.0 && end_seconds.is_none() && !precise_seek
                    );
                    let precise = build_ffmpeg_preview_audio_args(
                        "input.mp4",
                        &default_config(),
                        &PreviewAudioFfmpegOptions {
                            precise_seek: true,
                            ..options
                        },
                    )
                    .unwrap();
                    let mut args = plan.args;
                    args.retain(|arg| arg != "-noaccurate_seek");
                    assert_eq!(args, precise.args);
                }
            }
        }
    }

    #[test]
    fn build_ffmpeg_preview_args_streams_raw_bgra_to_stdout() {
        let plan = build_ffmpeg_preview_args("input.mp4", &default_config(), &default_options())
            .expect("preview args");

        assert!(plan.args.windows(2).any(|args| args == ["-f", "rawvideo"]));
        assert_eq!(plan.args.last(), Some(&"pipe:1".to_string()));
    }

    #[test]
    fn build_ffmpeg_preview_args_uses_bgra_filter_and_frame_size() {
        let plan = build_ffmpeg_preview_args("input.mp4", &default_config(), &default_options())
            .expect("preview args");

        assert!(plan.args.iter().any(|arg| arg.contains("format=bgra")));
        assert_eq!(plan.frame_bytes, 1280 * 720 * 4);
    }

    #[test]
    fn build_ffmpeg_preview_args_maps_overlay_to_preview_label() {
        let mut config = default_config();
        config.overlay = Some(OverlayConfig {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 0.75,
            anchor: "custom".to_string(),
        });

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert!(
            plan.args
                .iter()
                .any(|arg| arg.contains("[preview_export]fps=30,scale=1280:720"))
        );
    }

    #[test]
    fn build_ffmpeg_preview_args_keeps_subtitle_style_escaping() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("C:\\Media\\John's [cut],final.srt".to_string());

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert!(plan.args.iter().any(|arg| {
            arg.contains(r"subtitles=filename=C\\:\\\\Media\\\\John\\\'s\\\ \[cut\]\,final.srt")
        }));
    }

    #[test]
    fn build_ffmpeg_preview_args_rebases_subtitle_timebase_to_source_seek() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        let mut options = default_options();
        options.start_seconds = 10.0;

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &options).expect("preview args");

        assert!(plan.args.iter().any(|arg| {
            arg.contains(
                "setpts=PTS+10.000/TB,subtitles=filename=/tmp/sub.srt,setpts=PTS-10.000/TB",
            )
        }));
    }

    #[test]
    fn preview_subtitle_clock_matches_submillisecond_input_seek() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        config.start_time = Some("0.750".to_string());
        let mut options = default_options();
        options.start_seconds = 1.234_567;
        let plan = build_ffmpeg_preview_args("input.mp4", &config, &options).unwrap();
        assert!(plan.args.windows(2).any(|pair| pair == ["-ss", "1.234567"]));
        assert!(plan.args.iter().any(|arg| arg.contains(
            "setpts=PTS+1.234567/TB,subtitles=filename=/tmp/sub.srt,setpts=PTS-1.234567/TB"
        )));
    }

    #[test]
    fn build_ffmpeg_preview_args_omits_encoder_only_args() {
        let plan = build_ffmpeg_preview_args("input.mp4", &default_config(), &default_options())
            .expect("preview args");

        assert!(
            !plan
                .args
                .iter()
                .any(|arg| matches!(arg.as_str(), "-c:v" | "-crf" | "-preset" | "-y"))
        );
    }

    #[test]
    fn build_ffmpeg_preview_args_scales_portrait_export_without_outer_canvas() {
        let mut config = default_config();
        config.resolution = "custom".to_string();
        config.custom_width = Some("1080".to_string());
        config.custom_height = Some("1920".to_string());
        config.crop = Some(CropConfig {
            enabled: true,
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 600.0,
            source_width: None,
            source_height: None,
            aspect_ratio: None,
        });

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert_eq!(plan.width, 405);
        assert_eq!(plan.height, 720);
        assert!(plan.args.iter().any(|arg| {
            arg.contains("crop=300:600:10:20:exact=1")
                && arg.contains("scale=1080:1920:force_original_aspect_ratio=decrease")
                && arg.contains("scale=405:720")
                && !arg.contains("pad=405:720")
        }));
    }

    #[test]
    fn build_ffmpeg_preview_args_swaps_preview_dimensions_after_side_rotation() {
        let mut config = default_config();
        config.rotation = "90".to_string();

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert_eq!(plan.width, 405);
        assert_eq!(plan.height, 720);
        assert_eq!(plan.frame_bytes, 405 * 720 * 4);
    }

    #[test]
    fn build_ffmpeg_preview_args_preserves_display_oriented_portrait_dimensions() {
        let mut options = default_options();
        options.source_width = Some(2160);
        options.source_height = Some(3840);

        let plan = build_ffmpeg_preview_args("iphone.mov", &default_config(), &options)
            .expect("display-oriented preview args");

        assert_eq!(
            (plan.width, plan.height, plan.frame_bytes),
            (405, 720, 405 * 720 * 4)
        );
    }

    #[test]
    fn build_ffmpeg_preview_args_uses_cropped_aspect_for_preview_dimensions() {
        let mut config = default_config();
        config.crop = Some(CropConfig {
            enabled: true,
            x: 420.0,
            y: 0.0,
            width: 1080.0,
            height: 1080.0,
            source_width: None,
            source_height: None,
            aspect_ratio: Some("1:1".to_string()),
        });

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert_eq!(plan.width, 720);
        assert_eq!(plan.height, 720);
    }

    #[test]
    fn build_ffmpeg_preview_audio_args_streams_selected_track_as_pcm() {
        let mut options = default_audio_options();
        options.selected_track = 2;
        let mut config = default_config();
        config.audio_volume = 50.0;

        let plan = build_ffmpeg_preview_audio_args("input.mp4", &config, &options)
            .expect("audio preview args");

        assert!(plan.args.windows(2).any(|args| args == ["-map", "0:2"]));
        assert!(plan.args.windows(2).any(|args| args == ["-f", "f32le"]));
        assert!(plan.args.windows(2).any(|args| args == ["-t", "2.500"]));
        assert!(
            plan.args
                .windows(2)
                .any(|args| args == ["-af", "volume=0.500"])
        );
        assert_eq!(plan.args.last(), Some(&"pipe:1".to_string()));
    }

    #[test]
    fn effective_preview_fps_caps_to_export_fps_when_lower() {
        let mut config = default_config();
        config.fps = "15".to_string();

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert_eq!(plan.fps, 15);
    }

    #[test]
    fn preview_geometry_matches_odd_output_and_exact_crop_dimensions() {
        let mut options = default_options();
        options.source_width = Some(320);
        options.source_height = Some(180);
        for (container, fixed, height_only) in [
            ("mp4", (128, 130), (230, 130)),
            ("gif", (127, 129), (229, 129)),
            ("png", (127, 129), (229, 129)),
        ] {
            let mut config = default_config();
            config.container = container.to_string();
            config.resolution = "custom".to_string();
            config.custom_width = Some("127".to_string());
            config.custom_height = Some("129".to_string());
            let plan = build_ffmpeg_preview_args("input.mp4", &config, &options).unwrap();
            assert_eq!((plan.width, plan.height), fixed, "{container}");
            config.custom_width = Some("-1".to_string());
            let plan = build_ffmpeg_preview_args("input.mp4", &config, &options).unwrap();
            assert_eq!((plan.width, plan.height), height_only, "{container}");
        }

        let mut config = default_config();
        config.container = "png".to_string();
        config.crop = Some(CropConfig {
            enabled: true,
            x: 0.0,
            y: 0.0,
            width: 127.0,
            height: 129.0,
            source_width: None,
            source_height: None,
            aspect_ratio: None,
        });
        let plan = build_ffmpeg_preview_args("input.media", &config, &options).unwrap();
        assert_eq!((plan.width, plan.height), (127, 129));
        assert_eq!(plan.frame_bytes, 127 * 129 * 4);
    }

    #[test]
    fn copy_preview_ignores_dormant_export_transforms_and_overlay() {
        let mut config = default_config();
        config.processing_mode = "copy".to_string();
        config.rotation = "90".to_string();
        config.resolution = "custom".to_string();
        config.custom_width = Some("127".to_string());
        config.custom_height = Some("129".to_string());
        config.fps = "8".to_string();
        config.subtitle_burn_path = Some("/tmp/dormant.srt".to_string());
        config.overlay = Some(OverlayConfig {
            enabled: true,
            path: "/tmp/dormant.png".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 1.0,
            anchor: "center".to_string(),
        });
        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options()).unwrap();
        assert_eq!((plan.width, plan.height, plan.fps), (1280, 720, 30));
        assert_eq!(plan.args.iter().filter(|arg| *arg == "-i").count(), 1);
        let graph = plan
            .args
            .windows(2)
            .find(|pair| pair[0] == "-filter_complex")
            .unwrap();
        for inactive in ["transpose", "overlay", "subtitles", "127", "pad="] {
            assert!(!graph[1].contains(inactive), "{inactive}");
        }
    }

    #[test]
    fn build_ffmpeg_preview_args_keeps_exact_raster_for_gif_output() {
        let mut config = default_config();
        config.container = "gif".to_string();
        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert!(!plan.args.iter().any(|arg| arg.contains("ceil(iw/2)*2")));
    }
}
