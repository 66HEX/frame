use regex::Regex;
use std::path::{Path, PathBuf};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc;

use crate::conversion::error::ConversionError;
use crate::conversion::manager::ManagerMessage;
use crate::conversion::types::{
    CompletedPayload, ConversionConfig, ConversionTask, ErrorPayload, LogPayload, MetadataConfig,
    MetadataMode, ProgressPayload, VOLUME_EPSILON,
};

pub(crate) fn parse_frame_rate_string(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("n/a") {
        return None;
    }

    if let Some((num, den)) = value.split_once('/') {
        let numerator: f64 = num.trim().parse().ok()?;
        let denominator: f64 = den.trim().parse().ok()?;
        if denominator == 0.0 {
            return None;
        }
        Some(numerator / denominator)
    } else {
        value.parse::<f64>().ok()
    }
}

pub(crate) fn parse_probe_bitrate(raw: Option<&str>) -> Option<f64> {
    let raw = raw?.trim();
    if raw.eq_ignore_ascii_case("n/a") || raw.is_empty() {
        return None;
    }
    let numeric = raw.parse::<f64>().ok()?;
    if numeric <= 0.0 {
        return None;
    }
    Some(numeric / 1000.0)
}

pub(crate) fn is_audio_only_container(container: &str) -> bool {
    matches!(
        container.to_lowercase().as_str(),
        "mp3" | "wav" | "flac" | "aac" | "m4a"
    )
}

fn is_nvenc_codec(codec: &str) -> bool {
    matches!(codec, "h264_nvenc" | "hevc_nvenc" | "av1_nvenc")
}

fn is_videotoolbox_codec(codec: &str) -> bool {
    matches!(codec, "h264_videotoolbox" | "hevc_videotoolbox")
}

fn map_nvenc_preset(preset: &str) -> String {
    match preset {
        "fast" | "medium" | "slow" => preset.to_string(),
        "default" => "default".to_string(),
        "p1" | "p2" | "p3" | "p4" | "p5" | "p6" | "p7" => preset.to_string(),
        "ultrafast" | "superfast" | "veryfast" | "faster" => "fast".to_string(),
        "slower" | "veryslow" => "slow".to_string(),
        _ => "medium".to_string(),
    }
}

fn parse_time(time_str: &str) -> Option<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

pub fn build_ffmpeg_args(input: &str, output: &str, config: &ConversionConfig) -> Vec<String> {
    let mut args = Vec::new();

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
    let is_nvenc = is_nvenc_codec(&config.video_codec);
    let is_videotoolbox = is_videotoolbox_codec(&config.video_codec);

    if is_audio_only {
        args.push("-vn".to_string());
    } else {
        args.push("-c:v".to_string());
        args.push(config.video_codec.clone());

        if config.video_bitrate_mode == "bitrate" {
            args.push("-b:v".to_string());
            args.push(format!("{}k", config.video_bitrate));
        } else if is_nvenc {
            let cq = (52.0 - (config.quality as f64 / 2.0))
                .round()
                .clamp(1.0, 51.0) as u32;
            args.push("-rc:v".to_string());
            args.push("vbr".to_string());
            args.push("-cq:v".to_string());
            args.push(cq.to_string());
        } else if is_videotoolbox {
            args.push("-q:v".to_string());
            args.push(config.quality.to_string());
        } else {
            args.push("-crf".to_string());
            args.push(config.crf.to_string());
        }

        if !is_videotoolbox {
            args.push("-preset".to_string());
            let preset_value = if is_nvenc {
                map_nvenc_preset(&config.preset)
            } else {
                config.preset.clone()
            };
            args.push(preset_value);
        }

        if is_nvenc {
            if config.nvenc_spatial_aq {
                args.push("-spatial_aq".to_string());
                args.push("1".to_string());
            }
            if config.nvenc_temporal_aq {
                args.push("-temporal_aq".to_string());
                args.push("1".to_string());
            }
        }

        if is_videotoolbox {
            if config.videotoolbox_allow_sw {
                args.push("-allow_sw".to_string());
                args.push("1".to_string());
            }
        }

        let mut video_filters = Vec::new();

        if config.flip_horizontal {
            video_filters.push("hflip".to_string());
        }

        if config.flip_vertical {
            video_filters.push("vflip".to_string());
        }

        match config.rotation.as_str() {
            "90" => video_filters.push("transpose=1".to_string()),
            "180" => video_filters.push("transpose=1,transpose=1".to_string()),
            "270" => video_filters.push("transpose=2".to_string()),
            _ => {}
        }

        if let Some(crop) = &config.crop {
            if crop.enabled {
                let crop_width = crop.width.max(1.0).round() as i32;
                let crop_height = crop.height.max(1.0).round() as i32;
                let crop_x = crop.x.max(0.0).round() as i32;
                let crop_y = crop.y.max(0.0).round() as i32;
                video_filters.push(format!(
                    "crop={}:{}:{}:{}",
                    crop_width, crop_height, crop_x, crop_y
                ));
            }
        }

        if let Some(burn_path) = &config.subtitle_burn_path {
            if !burn_path.is_empty() {
                let escaped_path = burn_path.replace('\\', "/").replace(':', "\\:");
                video_filters.push(format!("subtitles='{}'", escaped_path));
            }
        }

        if config.resolution != "original" || config.resolution == "custom" {
            let algorithm = match config.scaling_algorithm.as_str() {
                "lanczos" => ":flags=lanczos",
                "bilinear" => ":flags=bilinear",
                "nearest" => ":flags=neighbor",
                "bicubic" => ":flags=bicubic",
                _ => "",
            };

            let scale_filter = if config.resolution == "custom" {
                let w = config.custom_width.as_deref().unwrap_or("-1");
                let h = config.custom_height.as_deref().unwrap_or("-1");
                if w != "-1" && h != "-1" {
                    format!(
                        "scale={w}:{h}:force_original_aspect_ratio=decrease{algo},pad={w}:{h}:(ow-iw)/2:(oh-ih)/2",
                        w = w,
                        h = h,
                        algo = algorithm
                    )
                } else if w == "-1" && h == "-1" {
                    "scale=-1:-1".to_string()
                } else {
                    format!("scale={}:{}{}", w, h, algorithm)
                }
            } else {
                match config.resolution.as_str() {
                    "1080p" => format!("scale=-2:1080{}", algorithm),
                    "720p" => format!("scale=-2:720{}", algorithm),
                    "480p" => format!("scale=-2:480{}", algorithm),
                    _ => "scale=-1:-1".to_string(),
                }
            };

            video_filters.push(scale_filter);
        }

        if !video_filters.is_empty() {
            args.push("-vf".to_string());
            args.push(video_filters.join(","));
        }

        if config.fps != "original" {
            args.push("-r".to_string());
            args.push(config.fps.clone());
        }
    }

    if (!config.selected_audio_tracks.is_empty() || !config.selected_subtitle_tracks.is_empty())
        && !is_audio_only
    {
        args.push("-map".to_string());
        args.push("0:v:0".to_string());
    }

    if !config.selected_audio_tracks.is_empty() {
        for track_index in &config.selected_audio_tracks {
            args.push("-map".to_string());
            args.push(format!("0:{}", track_index));
        }
    }

    if !config.selected_audio_tracks.is_empty() {
        args.push("-c:a".to_string());
        args.push(config.audio_codec.clone());
    }

    if !config.selected_subtitle_tracks.is_empty() {
        for track_index in &config.selected_subtitle_tracks {
            args.push("-map".to_string());
            args.push(format!("0:{}", track_index));
        }
    } else if !is_audio_only {
        args.push("-map".to_string());
        args.push("0:s?".to_string());
    }

    if config.subtitle_burn_path.is_none()
        || config
            .subtitle_burn_path
            .as_ref()
            .map_or(true, |p| p.is_empty())
    {
        args.push("-c:s".to_string());
        args.push("copy".to_string());
    }

    let lossless_audio_codecs = ["flac", "alac", "pcm_s16le"];
    if !lossless_audio_codecs.contains(&config.audio_codec.as_str())
        && !config.selected_audio_tracks.is_empty()
    {
        args.push("-b:a".to_string());
        args.push(format!("{}k", config.audio_bitrate));
    }

    match config.audio_channels.as_str() {
        "stereo" => {
            args.push("-ac".to_string());
            args.push("2".to_string());
        }
        "mono" => {
            args.push("-ac".to_string());
            args.push("1".to_string());
        }
        _ => {}
    }

    let mut audio_filters: Vec<String> = Vec::new();

    if config.audio_normalize {
        audio_filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
    }

    if (config.audio_volume - 100.0).abs() > VOLUME_EPSILON {
        let volume_factor = config.audio_volume / 100.0;
        audio_filters.push(format!("volume={:.2}", volume_factor));
    }

    if !audio_filters.is_empty() {
        args.push("-af".to_string());
        args.push(audio_filters.join(","));
    }

    args.push("-y".to_string());
    args.push(output.to_string());

    args
}

fn add_metadata_flags(args: &mut Vec<String>, metadata: &MetadataConfig) {
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

    Ok(())
}

async fn run_upscale_worker(
    app: AppHandle,
    tx: mpsc::Sender<ManagerMessage>,
    task: ConversionTask,
) -> Result<(), ConversionError> {
    let (scale, model_name) = match task.config.ml_upscale.as_deref() {
        Some("esrgan-2x") => ("2", "realesr-animevideov3-x2"),
        Some("esrgan-4x") => ("4", "realesr-animevideov3-x4"),
        _ => return Err(ConversionError::InvalidInput("Invalid upscale mode".into())),
    };

    let output_path = build_output_path(&task.file_path, &task.config.container, task.output_name);

    let probe = crate::conversion::probe::probe_media_file(&app, &task.file_path)
        .await
        .map_err(|e| ConversionError::Worker(format!("Probe failed: {}", e)))?;

    let fps = probe.frame_rate.unwrap_or(30.0);
    let full_duration = probe
        .duration
        .as_deref()
        .and_then(parse_time)
        .unwrap_or(0.0);

    let start_t = task
        .config
        .start_time
        .as_deref()
        .and_then(parse_time)
        .unwrap_or(0.0);
    let end_t = task
        .config
        .end_time
        .as_deref()
        .and_then(parse_time)
        .unwrap_or(full_duration);
    let active_duration = (end_t - start_t).max(0.0);
    let total_frames = (active_duration * fps).ceil() as u32;

    let temp_dir = std::env::temp_dir().join(format!("frame_upscale_{}", task.id));
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).map_err(ConversionError::Io)?;
    let input_frames_dir = temp_dir.join("input");
    let output_frames_dir = temp_dir.join("output");
    std::fs::create_dir_all(&input_frames_dir).map_err(ConversionError::Io)?;
    std::fs::create_dir_all(&output_frames_dir).map_err(ConversionError::Io)?;

    let app_clone = app.clone();
    let id_clone = task.id.clone();

    let mut dec_args = vec!["-i".to_string(), task.file_path.clone()];
    if let Some(start) = &task.config.start_time {
        if !start.is_empty() {
            dec_args.insert(0, "-ss".to_string());
            dec_args.insert(1, start.clone());
        }
    }

    if let Some(end) = &task.config.end_time {
        if !end.is_empty() {
            if let Some(start) = &task.config.start_time {
                if !start.is_empty() {
                    if let (Some(s_t), Some(e_t)) = (parse_time(start), parse_time(end)) {
                        let duration = e_t - s_t;
                        if duration > 0.0 {
                            dec_args.push("-t".to_string());
                            dec_args.push(format!("{:.3}", duration));
                        }
                    }
                } else {
                    dec_args.push("-to".to_string());
                    dec_args.push(end.clone());
                }
            } else {
                dec_args.push("-to".to_string());
                dec_args.push(end.clone());
            }
        }
    }

    if let Some(crop) = &task.config.crop {
        if crop.enabled {
            let crop_width = crop.width.max(1.0).round() as i32;
            let crop_height = crop.height.max(1.0).round() as i32;
            let crop_x = crop.x.max(0.0).round() as i32;
            let crop_y = crop.y.max(0.0).round() as i32;
            dec_args.push("-vf".to_string());
            dec_args.push(format!(
                "crop={}:{}:{}:{}",
                crop_width, crop_height, crop_x, crop_y
            ));
        }
    }
    dec_args.push(
        input_frames_dir
            .join("frame_%08d.png")
            .to_string_lossy()
            .to_string(),
    );

    let (mut dec_rx, dec_child) = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(dec_args)
        .spawn()
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    let _ = tx
        .send(ManagerMessage::TaskStarted(
            task.id.clone(),
            dec_child.pid(),
        ))
        .await;

    let _ = app_clone.emit(
        "conversion-progress",
        ProgressPayload {
            id: id_clone.clone(),
            progress: 0.0,
        },
    );

    let frame_regex = Regex::new(r"frame=\s*(\d+)").unwrap();
    let mut decode_success = false;

    while let Some(event) = dec_rx.recv().await {
        match event {
            CommandEvent::Stderr(ref line_bytes) => {
                let line = String::from_utf8_lossy(line_bytes);
                let _ = app_clone.emit(
                    "conversion-log",
                    LogPayload {
                        id: id_clone.clone(),
                        line: format!("[DECODE] {}", line.trim()),
                    },
                );

                if total_frames > 0 {
                    if let Some(caps) = frame_regex.captures(&line) {
                        if let Some(frame_match) = caps.get(1) {
                            if let Ok(current_frame) = frame_match.as_str().parse::<u32>() {
                                let decode_progress =
                                    (current_frame as f64 / total_frames as f64) * 5.0;
                                let _ = app_clone.emit(
                                    "conversion-progress",
                                    ProgressPayload {
                                        id: id_clone.clone(),
                                        progress: decode_progress.min(5.0),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            CommandEvent::Terminated(payload) => {
                decode_success = payload.code == Some(0);
                break;
            }
            _ => {}
        }
    }

    if !decode_success {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(ConversionError::Worker("Frame extraction failed".into()));
    }

    let actual_frames = std::fs::read_dir(&input_frames_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "png")
                        .unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(total_frames);
    let total_frames = if actual_frames > 0 {
        actual_frames
    } else {
        total_frames
    };

    let models_path = app
        .path()
        .resolve("resources/models", BaseDirectory::Resource)
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    let upscaler_args = vec![
        "-v".to_string(),
        "-i".to_string(),
        input_frames_dir.to_string_lossy().to_string(),
        "-o".to_string(),
        output_frames_dir.to_string_lossy().to_string(),
        "-s".to_string(),
        scale.to_string(),
        "-f".to_string(),
        "png".to_string(),
        "-m".to_string(),
        models_path.to_string_lossy().to_string(),
        "-n".to_string(),
        model_name.to_string(),
        "-j".to_string(),
        "4:4:4".to_string(),
        "-g".to_string(),
        "0".to_string(),
        "-t".to_string(),
        "0".to_string(),
    ];

    let (mut upscale_rx, upscale_child) = app
        .shell()
        .sidecar("realesrgan-ncnn-vulkan")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(upscaler_args)
        .spawn()
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    let _ = tx
        .send(ManagerMessage::TaskStarted(
            task.id.clone(),
            upscale_child.pid(),
        ))
        .await;

    let mut upscale_success = false;
    let mut last_error = String::new();
    let mut completed_frames: u32 = 0;

    while let Some(event) = upscale_rx.recv().await {
        if let CommandEvent::Stderr(ref line_bytes) = event {
            let line = String::from_utf8_lossy(line_bytes);
            let trimmed = line.trim();
            last_error = line.to_string();

            let is_percentage_line = trimmed.ends_with('%')
                && trimmed
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false);
            if !is_percentage_line && !trimmed.is_empty() {
                let _ = app_clone.emit(
                    "conversion-log",
                    LogPayload {
                        id: id_clone.clone(),
                        line: format!("[UPSCALE] {}", trimmed),
                    },
                );
            }

            if line.contains("→") || line.contains("->") {
                completed_frames += 1;

                let progress = if total_frames > 0 {
                    5.0 + (completed_frames as f64 / total_frames as f64) * 85.0
                } else {
                    5.0 + (completed_frames as f64).min(85.0)
                };

                let _ = app_clone.emit(
                    "conversion-progress",
                    ProgressPayload {
                        id: id_clone.clone(),
                        progress: progress.min(90.0),
                    },
                );
            }
        }
        if let CommandEvent::Terminated(payload) = event {
            upscale_success = payload.code == Some(0);
            break;
        }
    }
    if !upscale_success {
        return Err(ConversionError::Worker(format!(
            "Upscaling failed: {}",
            last_error
        )));
    }

    let mut enc_args = vec![
        "-framerate".to_string(),
        fps.to_string(),
        "-start_number".to_string(),
        "1".to_string(),
        "-i".to_string(),
        output_frames_dir
            .join("frame_%08d.png")
            .to_string_lossy()
            .to_string(),
    ];

    if let Some(start) = &task.config.start_time {
        if !start.is_empty() {
            enc_args.push("-ss".to_string());
            enc_args.push(start.clone());
        }
    }

    enc_args.push("-i".to_string());
    enc_args.push(task.file_path.clone());

    enc_args.extend(vec![
        "-map".to_string(),
        "0:v:0".to_string(),
        "-map".to_string(),
        "1:a?".to_string(),
        "-c:v".to_string(),
        task.config.video_codec.clone(),
    ]);

    if task.config.video_bitrate_mode == "bitrate" {
        enc_args.push("-b:v".to_string());
        enc_args.push(format!("{}k", task.config.video_bitrate));
    } else {
        enc_args.push("-crf".to_string());
        enc_args.push(task.config.crf.to_string());
    }

    enc_args.push("-c:a".to_string());
    enc_args.push("copy".to_string());
    enc_args.push("-preset".to_string());
    enc_args.push(task.config.preset.clone());
    enc_args.push("-pix_fmt".to_string());
    enc_args.push("yuv420p".to_string());
    enc_args.push("-shortest".to_string());
    enc_args.push("-y".to_string());
    enc_args.push(output_path.clone());

    let (mut enc_rx, enc_child) = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(enc_args)
        .spawn()
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    let _ = tx
        .send(ManagerMessage::TaskStarted(
            task.id.clone(),
            enc_child.pid(),
        ))
        .await;

    let encode_frame_regex = Regex::new(r"frame=\s*(\d+)").unwrap();

    while let Some(event) = enc_rx.recv().await {
        match event {
            CommandEvent::Stderr(ref line_bytes) => {
                let line = String::from_utf8_lossy(line_bytes);
                let _ = app_clone.emit(
                    "conversion-log",
                    LogPayload {
                        id: id_clone.clone(),
                        line: format!("[ENCODE] {}", line.trim()),
                    },
                );

                if total_frames > 0 {
                    if let Some(caps) = encode_frame_regex.captures(&line) {
                        if let Some(frame_match) = caps.get(1) {
                            if let Ok(current_frame) = frame_match.as_str().parse::<u32>() {
                                let encode_progress =
                                    90.0 + (current_frame as f64 / total_frames as f64) * 10.0;
                                let _ = app_clone.emit(
                                    "conversion-progress",
                                    ProgressPayload {
                                        id: id_clone.clone(),
                                        progress: encode_progress.min(99.0),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            CommandEvent::Terminated(payload) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                if payload.code == Some(0) {
                    let _ = app.emit(
                        "conversion-completed",
                        CompletedPayload {
                            id: task.id.clone(),
                            output_path,
                        },
                    );
                    return Ok(());
                } else {
                    return Err(ConversionError::Worker(format!(
                        "Encoder failed with code {:?}",
                        payload.code
                    )));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub async fn run_ffmpeg_worker(
    app: AppHandle,
    tx: mpsc::Sender<ManagerMessage>,
    task: ConversionTask,
) -> Result<(), ConversionError> {
    if let Some(upscale_mode) = &task.config.ml_upscale {
        if upscale_mode != "none" && !upscale_mode.is_empty() {
            return run_upscale_worker(app, tx, task).await;
        }
    }

    let output_path = build_output_path(&task.file_path, &task.config.container, task.output_name);
    let args = build_ffmpeg_args(&task.file_path, &output_path, &task.config);

    let sidecar_command = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(args);

    let (mut rx, child) = sidecar_command
        .spawn()
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    let id = task.id;
    let app_clone = app.clone();

    let _ = tx
        .send(ManagerMessage::TaskStarted(id.clone(), child.pid()))
        .await;

    let duration_regex = Regex::new(r"Duration: (\d{2}:\d{2}:\d{2}\.\d{2})").unwrap();
    let time_regex = Regex::new(r"time=(\d{2}:\d{2}:\d{2}\.\d{2})").unwrap();

    let mut exit_code: Option<i32> = None;
    let mut total_duration: Option<f64> = None;

    let expected_duration = {
        let start_t = task
            .config
            .start_time
            .as_deref()
            .and_then(parse_time)
            .unwrap_or(0.0);
        let probe = crate::conversion::probe::probe_media_file(&app, &task.file_path)
            .await
            .ok();
        let full_duration = probe
            .and_then(|p| p.duration)
            .as_deref()
            .and_then(parse_time)
            .unwrap_or(0.0);
        let end_t = task
            .config
            .end_time
            .as_deref()
            .and_then(parse_time)
            .unwrap_or(full_duration);
        (end_t - start_t).max(0.0)
    };

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stderr(line_bytes) => {
                let line = String::from_utf8_lossy(&line_bytes).to_string();

                let _ = app_clone.emit(
                    "conversion-log",
                    LogPayload {
                        id: id.clone(),
                        line: line.clone(),
                    },
                );

                if let Some(caps) = time_regex.captures(&line) {
                    if let Some(match_str) = caps.get(1) {
                        if let Some(current_time) = parse_time(match_str.as_str()) {
                            let duration = if expected_duration > 0.0 {
                                expected_duration
                            } else if let Some(d) = total_duration {
                                d
                            } else if let Some(caps) = duration_regex.captures(&line) {
                                if let Some(m) = caps.get(1) {
                                    total_duration = parse_time(m.as_str());
                                    total_duration.unwrap_or(0.0)
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };

                            if duration > 0.0 {
                                let progress = (current_time / duration * 100.0).min(100.0);
                                let _ = app_clone.emit(
                                    "conversion-progress",
                                    ProgressPayload {
                                        id: id.clone(),
                                        progress,
                                    },
                                );
                            }
                        }
                    }
                }
            }
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
            }
            _ => {}
        }
    }

    if exit_code == Some(0) {
        let _ = app_clone.emit(
            "conversion-completed",
            CompletedPayload {
                id: id.clone(),
                output_path: output_path.clone(),
            },
        );
        Ok(())
    } else {
        let err_msg = format!("Process terminated with code {:?}", exit_code);
        let _ = app_clone.emit(
            "conversion-error",
            ErrorPayload {
                id: id.clone(),
                error: err_msg.clone(),
            },
        );
        Err(ConversionError::Worker(err_msg))
    }
}
