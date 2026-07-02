use crate::{
    error::ConversionError,
    filters::{
        PREVIEW_OUTPUT_LABEL, VisualFilterBase, VisualFilterProfile, build_visual_filter_complex,
        has_overlay,
    },
    types::ConversionConfig,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewFfmpegOptions {
    pub start_seconds: f64,
    pub end_seconds: Option<f64>,
    pub target_width: u32,
    pub target_height: u32,
    pub fps: u32,
    pub realtime: bool,
    pub precise_seek: bool,
    pub source_is_image: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFfmpegPlan {
    pub args: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub frame_bytes: usize,
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
    let frame_bytes = frame_bytes(options.target_width, options.target_height)?;
    let base = if options.source_is_image {
        VisualFilterBase::Image
    } else {
        VisualFilterBase::Video
    };
    let graph = build_visual_filter_complex(
        config,
        VisualFilterProfile::PreviewLowRes {
            base,
            width: options.target_width,
            height: options.target_height,
            fps,
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
        if !options.precise_seek {
            args.push("-noaccurate_seek".to_string());
        }
    }
    if options.realtime {
        args.push("-readrate".to_string());
        args.push("1".to_string());
    }
    args.push("-i".to_string());
    args.push(input.to_string());

    if has_overlay(config)
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
        width: options.target_width,
        height: options.target_height,
        fps,
        frame_bytes,
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
    if options.target_width == 0 || options.target_height == 0 {
        return Err(ConversionError::InvalidInput(
            "Preview target dimensions must be non-zero".to_string(),
        ));
    }
    if options.fps == 0 {
        return Err(ConversionError::InvalidInput(
            "Preview FPS must be non-zero".to_string(),
        ));
    }
    let _ = frame_bytes(options.target_width, options.target_height)?;
    Ok(())
}

fn preview_duration(options: &PreviewFfmpegOptions) -> Option<f64> {
    options
        .end_seconds
        .map(|end_seconds| end_seconds - options.start_seconds)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
}

fn effective_preview_fps(config: &ConversionConfig, preview_fps: u32) -> u32 {
    config
        .fps
        .parse::<u32>()
        .ok()
        .filter(|fps| *fps > 0)
        .map_or(preview_fps, |export_fps| export_fps.min(preview_fps).max(1))
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

fn format_seconds(seconds: f64) -> String {
    format!("{seconds:.3}")
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
            audio_quality: "4".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
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
            target_width: 1280,
            target_height: 720,
            fps: 30,
            realtime: true,
            precise_seek: true,
            source_is_image: false,
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
                .any(|arg| arg.contains("[preview_export]scale=1280:720"))
        );
    }

    #[test]
    fn build_ffmpeg_preview_args_keeps_subtitle_style_escaping() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("C:\\Media\\John's [cut],final.srt".to_string());

        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert!(
            plan.args
                .iter()
                .any(|arg| arg.contains("subtitles='C\\:/Media/John\\'s \\[cut\\]\\,final.srt'"))
        );
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
    fn build_ffmpeg_preview_args_preserves_portrait_export_inside_preview_canvas() {
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

        assert!(plan.args.iter().any(|arg| {
            arg.contains("crop=300:600:10:20,scale=1080:1920:force_original_aspect_ratio=decrease")
        }));
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
    fn build_ffmpeg_preview_args_keeps_video_base_for_gif_output() {
        let mut config = default_config();
        config.container = "gif".to_string();
        let plan = build_ffmpeg_preview_args("input.mp4", &config, &default_options())
            .expect("preview args");

        assert!(plan.args.iter().any(|arg| arg.contains("ceil(iw/2)*2")));
    }
}
