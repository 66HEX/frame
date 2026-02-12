use std::path::{Path, PathBuf};

use crate::conversion::codec::{
    add_audio_codec_args, add_fps_args, add_subtitle_codec_args, add_video_codec_args,
};
use crate::conversion::error::ConversionError;
use crate::conversion::filters::{build_audio_filters, build_video_filters};
use crate::conversion::media_rules::{
    container_supports_audio, container_supports_subtitles, is_audio_codec_allowed,
    is_video_codec_allowed, is_video_only_container,
};
use crate::conversion::types::{ConversionConfig, MetadataConfig, MetadataMode};
use crate::conversion::utils::{get_hwaccel_args, is_audio_only_container, parse_time};

pub fn build_ffmpeg_args(input: &str, output: &str, config: &ConversionConfig) -> Vec<String> {
    let mut args = Vec::new();

    // Hardware decode acceleration (must be before -i)
    if config.hw_decode {
        args.extend(get_hwaccel_args(&config.video_codec));
    }

    if let Some(start) = &config.start_time {
        if !start.is_empty() {
            args.push("-ss".to_string());
            args.push(start.clone());
        }
    }

    args.push("-i".to_string());
    args.push(input.to_string());

    if let Some(end_str) = &config.end_time {
        if !end_str.is_empty() {
            if let Some(start_str) = &config.start_time {
                if !start_str.is_empty() {
                    if let (Some(start_t), Some(end_t)) =
                        (parse_time(start_str), parse_time(end_str))
                    {
                        let duration = end_t - start_t;
                        if duration > 0.0 {
                            args.push("-t".to_string());
                            args.push(format!("{:.3}", duration));
                        }
                    }
                } else {
                    args.push("-to".to_string());
                    args.push(end_str.clone());
                }
            } else {
                args.push("-to".to_string());
                args.push(end_str.clone());
            }
        }
    }

    match config.metadata.mode {
        MetadataMode::Clean => {
            args.push("-map_metadata".to_string());
            args.push("-1".to_string());
        }
        MetadataMode::Replace => {
            args.push("-map_metadata".to_string());
            args.push("-1".to_string());
            add_metadata_flags(&mut args, &config.metadata);
        }
        MetadataMode::Preserve => {
            add_metadata_flags(&mut args, &config.metadata);
        }
    }

    let is_audio_only = is_audio_only_container(&config.container);
    let is_video_only = is_video_only_container(&config.container);
    let has_burn_subtitles = config
        .subtitle_burn_path
        .as_ref()
        .is_some_and(|path| !path.trim().is_empty());

    if is_audio_only {
        args.push("-vn".to_string());

        if !config.selected_audio_tracks.is_empty() {
            for track_index in &config.selected_audio_tracks {
                args.push("-map".to_string());
                args.push(format!("0:{}", track_index));
            }
        } else {
            args.push("-map".to_string());
            args.push("0:a?".to_string());
        }

        add_audio_codec_args(&mut args, config);
    } else if is_video_only {
        args.push("-filter_complex".to_string());
        args.push(build_gif_filter_complex(config));

        args.push("-map".to_string());
        args.push("[gif_out]".to_string());
        args.push("-an".to_string());

        args.push("-c:v".to_string());
        args.push("gif".to_string());

        args.push("-loop".to_string());
        args.push(config.gif_loop.to_string());
        args.push("-f".to_string());
        args.push("gif".to_string());
    } else {
        add_video_codec_args(&mut args, config);

        let video_filters = build_video_filters(config, true);
        if !video_filters.is_empty() {
            args.push("-vf".to_string());
            args.push(video_filters.join(","));
        }

        add_fps_args(&mut args, config);
        args.push("-map".to_string());
        args.push("0:v:0".to_string());

        if !config.selected_audio_tracks.is_empty() {
            for track_index in &config.selected_audio_tracks {
                args.push("-map".to_string());
                args.push(format!("0:{}", track_index));
            }
        } else {
            args.push("-map".to_string());
            args.push("0:a?".to_string());
        }

        add_audio_codec_args(&mut args, config);

        if !config.selected_subtitle_tracks.is_empty() {
            for track_index in &config.selected_subtitle_tracks {
                args.push("-map".to_string());
                args.push(format!("0:{}", track_index));
            }
            add_subtitle_codec_args(&mut args, config);
        } else if !has_burn_subtitles {
            args.push("-map".to_string());
            args.push("0:s?".to_string());
            add_subtitle_codec_args(&mut args, config);
        }
    }

    if !is_video_only {
        let audio_filters = build_audio_filters(config);
        if !audio_filters.is_empty() {
            args.push("-af".to_string());
            args.push(audio_filters.join(","));
        }
    }

    args.push("-y".to_string());
    args.push(output.to_string());

    args
}

fn normalize_gif_dither(dither: &str) -> &'static str {
    match dither {
        "none" => "none",
        "bayer" => "bayer",
        "floyd_steinberg" => "floyd_steinberg",
        "sierra2_4a" => "sierra2_4a",
        _ => "sierra2_4a",
    }
}

fn build_gif_filter_complex(config: &ConversionConfig) -> String {
    let mut filters = build_video_filters(config, true);
    if config.fps != "original" {
        filters.push(format!("fps={}", config.fps));
    }

    let chain = if filters.is_empty() {
        "split[gif_src][gif_palette_src]".to_string()
    } else {
        format!("{},split[gif_src][gif_palette_src]", filters.join(","))
    };

    let colors = config.gif_colors.clamp(2, 256);
    let dither = normalize_gif_dither(&config.gif_dither);

    format!(
        "[0:v:0]{chain};[gif_palette_src]palettegen=max_colors={colors}:stats_mode=single[gif_palette];[gif_src][gif_palette]paletteuse=dither={dither}:new=1[gif_out]"
    )
}

pub fn add_metadata_flags(args: &mut Vec<String>, metadata: &MetadataConfig) {
    if let Some(v) = &metadata.title {
        if !v.is_empty() {
            args.push("-metadata".to_string());
            args.push(format!("title={}", v));
        }
    }
    if let Some(v) = &metadata.artist {
        if !v.is_empty() {
            args.push("-metadata".to_string());
            args.push(format!("artist={}", v));
        }
    }
    if let Some(v) = &metadata.album {
        if !v.is_empty() {
            args.push("-metadata".to_string());
            args.push(format!("album={}", v));
        }
    }
    if let Some(v) = &metadata.genre {
        if !v.is_empty() {
            args.push("-metadata".to_string());
            args.push(format!("genre={}", v));
        }
    }
    if let Some(v) = &metadata.date {
        if !v.is_empty() {
            args.push("-metadata".to_string());
            args.push(format!("date={}", v));
        }
    }
    if let Some(v) = &metadata.comment {
        if !v.is_empty() {
            args.push("-metadata".to_string());
            args.push(format!("comment={}", v));
        }
    }
}

fn sanitize_output_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed
        .rsplit(|ch| ch == '/' || ch == '\\')
        .next()
        .map(str::trim)
        .unwrap_or("");

    if candidate.is_empty() || candidate == "." || candidate == ".." {
        return None;
    }

    Some(candidate.to_string())
}

pub fn build_output_path(file_path: &str, container: &str, output_name: Option<String>) -> String {
    if let Some(custom) = output_name.as_deref().and_then(sanitize_output_name) {
        let input_path = Path::new(file_path);
        let mut output: PathBuf = match input_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::new(),
        };
        output.push(custom);
        output.set_extension(container);
        output.to_string_lossy().to_string()
    } else {
        format!("{}_converted.{}", file_path, container)
    }
}

pub fn validate_task_input(
    file_path: &str,
    config: &ConversionConfig,
) -> Result<(), ConversionError> {
    let input_path = Path::new(file_path);
    if !input_path.exists() {
        return Err(ConversionError::InvalidInput(format!(
            "Input file does not exist: {}",
            file_path
        )));
    }
    if !input_path.is_file() {
        return Err(ConversionError::InvalidInput(format!(
            "Input path is not a file: {}",
            file_path
        )));
    }

    let start_time = config
        .start_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let end_time = config
        .end_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if let Some(start) = start_time {
        if parse_time(start).is_none() {
            return Err(ConversionError::InvalidInput(format!(
                "Invalid start time: {}",
                start
            )));
        }
    }

    if let Some(end) = end_time {
        if parse_time(end).is_none() {
            return Err(ConversionError::InvalidInput(format!(
                "Invalid end time: {}",
                end
            )));
        }
    }

    if let (Some(start), Some(end)) = (start_time, end_time) {
        if let (Some(start_t), Some(end_t)) = (parse_time(start), parse_time(end)) {
            if end_t <= start_t {
                return Err(ConversionError::InvalidInput(
                    "End time must be greater than start time".to_string(),
                ));
            }
        }
    }

    if config.resolution == "custom" {
        let w_str = config.custom_width.as_deref().unwrap_or("-1");
        let h_str = config.custom_height.as_deref().unwrap_or("-1");

        let w = w_str.parse::<i32>().map_err(|_| {
            ConversionError::InvalidInput(format!("Invalid custom width: {}", w_str))
        })?;
        let h = h_str.parse::<i32>().map_err(|_| {
            ConversionError::InvalidInput(format!("Invalid custom height: {}", h_str))
        })?;

        if w == 0 || h == 0 {
            return Err(ConversionError::InvalidInput(
                "Resolution dimensions cannot be zero".to_string(),
            ));
        }
        if w < -1 || h < -1 {
            return Err(ConversionError::InvalidInput(
                "Resolution dimensions cannot be negative (except -1 for auto)".to_string(),
            ));
        }
    }

    if config.video_bitrate_mode == "bitrate"
        && !is_audio_only_container(&config.container)
        && !is_video_only_container(&config.container)
    {
        let bitrate = config.video_bitrate.parse::<f64>().map_err(|_| {
            ConversionError::InvalidInput(format!(
                "Invalid video bitrate: {}",
                config.video_bitrate
            ))
        })?;
        if bitrate <= 0.0 {
            return Err(ConversionError::InvalidInput(
                "Video bitrate must be positive".to_string(),
            ));
        }
    }

    let is_audio_only = is_audio_only_container(&config.container);
    let is_video_only = is_video_only_container(&config.container);
    let supports_audio = container_supports_audio(&config.container);
    let supports_subtitles = container_supports_subtitles(&config.container);
    if !is_audio_only && !is_video_codec_allowed(&config.container, &config.video_codec) {
        return Err(ConversionError::InvalidInput(format!(
            "Video codec '{}' is not compatible with container '{}'",
            config.video_codec, config.container
        )));
    }

    if supports_audio && !is_audio_codec_allowed(&config.container, &config.audio_codec) {
        return Err(ConversionError::InvalidInput(format!(
            "Audio codec '{}' is not compatible with container '{}'",
            config.audio_codec, config.container
        )));
    }

    let has_ml_upscale = config
        .ml_upscale
        .as_ref()
        .is_some_and(|mode| !mode.is_empty() && mode != "none");

    if let Some(mode) = config.ml_upscale.as_deref() {
        if !mode.is_empty() && mode != "none" && mode != "esrgan-2x" && mode != "esrgan-4x" {
            return Err(ConversionError::InvalidInput(format!(
                "Invalid ML upscale mode: {}",
                mode
            )));
        }
    }

    if (is_audio_only || is_video_only) && has_ml_upscale {
        return Err(ConversionError::InvalidInput(
            "ML upscaling requires an audio-capable video container".to_string(),
        ));
    }

    if !supports_audio && !config.selected_audio_tracks.is_empty() {
        return Err(ConversionError::InvalidInput(
            "Audio track selection is not available for this container".to_string(),
        ));
    }

    if !supports_subtitles
        && (!config.selected_subtitle_tracks.is_empty()
            || config
                .subtitle_burn_path
                .as_ref()
                .is_some_and(|path| !path.trim().is_empty()))
    {
        return Err(ConversionError::InvalidInput(
            "Subtitle options are not available for this container".to_string(),
        ));
    }

    if is_video_only {
        if !(2..=256).contains(&config.gif_colors) {
            return Err(ConversionError::InvalidInput(format!(
                "GIF palette size must be between 2 and 256 colors: {}",
                config.gif_colors
            )));
        }

        if !matches!(
            config.gif_dither.as_str(),
            "none" | "bayer" | "floyd_steinberg" | "sierra2_4a"
        ) {
            return Err(ConversionError::InvalidInput(format!(
                "Invalid GIF dither mode: {}",
                config.gif_dither
            )));
        }
    }

    Ok(())
}
