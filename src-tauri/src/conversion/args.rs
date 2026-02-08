use std::path::{Path, PathBuf};

use crate::conversion::codec::{add_audio_codec_args, add_fps_args, add_subtitle_codec_args, add_video_codec_args};
use crate::conversion::error::ConversionError;
use crate::conversion::filters::{build_audio_filters, build_video_filters};
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

    let audio_filters = build_audio_filters(config);
    if !audio_filters.is_empty() {
        args.push("-af".to_string());
        args.push(audio_filters.join(","));
    }

    args.push("-y".to_string());
    args.push(output.to_string());

    args
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

pub fn build_output_path(file_path: &str, container: &str, output_name: Option<String>) -> String {
    if let Some(custom) = output_name.and_then(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }) {
        let input_path = Path::new(file_path);
        let mut output: PathBuf = match input_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::new(),
        };
        output.push(custom);
        if output.extension().is_none() {
            output.set_extension(container);
        }
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

    if config.video_bitrate_mode == "bitrate" && !is_audio_only_container(&config.container) {
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
    if !is_audio_only && !is_video_codec_allowed(&config.container, &config.video_codec) {
        return Err(ConversionError::InvalidInput(format!(
            "Video codec '{}' is not compatible with container '{}'",
            config.video_codec, config.container
        )));
    }

    if !is_audio_codec_allowed(&config.container, &config.audio_codec) {
        return Err(ConversionError::InvalidInput(format!(
            "Audio codec '{}' is not compatible with container '{}'",
            config.audio_codec, config.container
        )));
    }

    let has_ml_upscale = config
        .ml_upscale
        .as_ref()
        .is_some_and(|mode| !mode.is_empty() && mode != "none");

    if is_audio_only && has_ml_upscale {
        return Err(ConversionError::InvalidInput(
            "ML upscaling requires a video container".to_string(),
        ));
    }

    if is_audio_only
        && (!config.selected_subtitle_tracks.is_empty()
            || config
                .subtitle_burn_path
                .as_ref()
                .is_some_and(|path| !path.trim().is_empty()))
    {
        return Err(ConversionError::InvalidInput(
            "Subtitle options are not available for audio-only containers".to_string(),
        ));
    }

    Ok(())
}

fn is_video_codec_allowed(container: &str, codec: &str) -> bool {
    match container {
        "mp4" => matches!(
            codec,
            "libx264"
                | "libx265"
                | "vp9"
                | "libsvtav1"
                | "h264_videotoolbox"
                | "h264_nvenc"
                | "hevc_videotoolbox"
                | "hevc_nvenc"
                | "av1_nvenc"
        ),
        "mkv" => matches!(
            codec,
            "libx264"
                | "libx265"
                | "vp9"
                | "prores"
                | "libsvtav1"
                | "h264_videotoolbox"
                | "h264_nvenc"
                | "hevc_videotoolbox"
                | "hevc_nvenc"
                | "av1_nvenc"
        ),
        "webm" => codec == "vp9",
        "mov" => matches!(
            codec,
            "libx264"
                | "libx265"
                | "prores"
                | "h264_videotoolbox"
                | "h264_nvenc"
                | "hevc_videotoolbox"
                | "hevc_nvenc"
        ),
        _ => true,
    }
}

fn is_audio_codec_allowed(container: &str, codec: &str) -> bool {
    match container {
        "mp3" => codec == "mp3",
        "wav" => codec == "pcm_s16le",
        "flac" => codec == "flac",
        "m4a" => matches!(codec, "aac" | "alac"),
        "mp4" => matches!(codec, "aac" | "ac3" | "libopus" | "mp3" | "alac"),
        "webm" => matches!(codec, "libopus" | "vorbis"),
        "mov" | "mkv" => true,
        _ => true,
    }
}
