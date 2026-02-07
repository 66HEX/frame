use regex::Regex;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc;

use crate::conversion::args::build_output_path;
use crate::conversion::error::ConversionError;
use crate::conversion::manager::ManagerMessage;
use crate::conversion::types::{
    CompletedPayload, ConversionTask, LogPayload, MetadataMode, ProgressPayload, StartedPayload,
    VOLUME_EPSILON,
};
use crate::conversion::utils::{
    is_nvenc_codec, is_videotoolbox_codec, map_nvenc_preset, parse_time,
};

pub async fn run_upscale_worker(
    app: AppHandle,
    tx: mpsc::Sender<ManagerMessage>,
    task: ConversionTask,
) -> Result<(), ConversionError> {
    let (scale, model_name) = match task.config.ml_upscale.as_deref() {
        Some("esrgan-2x") => ("2", "realesr-animevideov3-x2"),
        Some("esrgan-4x") => ("4", "realesr-animevideov3-x4"),
        _ => return Err(ConversionError::InvalidInput("Invalid upscale mode".into())),
    };

    let output_path = build_output_path(
        &task.file_path,
        &task.config.container,
        task.output_name.clone(),
    );

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

    let mut video_filters: Vec<String> = Vec::new();

    if task.config.flip_horizontal {
        video_filters.push("hflip".to_string());
    }
    if task.config.flip_vertical {
        video_filters.push("vflip".to_string());
    }

    match task.config.rotation.as_str() {
        "90" => video_filters.push("transpose=1".to_string()),
        "180" => video_filters.push("transpose=1,transpose=1".to_string()),
        "270" => video_filters.push("transpose=2".to_string()),
        _ => {}
    }

    if let Some(crop) = &task.config.crop {
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

    if let Some(burn_path) = &task.config.subtitle_burn_path {
        if !burn_path.is_empty() {
            let escaped_path = burn_path.replace('\\', "/").replace(':', "\\:");
            video_filters.push(format!("subtitles='{}'", escaped_path));
        }
    }

    if !video_filters.is_empty() {
        dec_args.push("-vf".to_string());
        dec_args.push(video_filters.join(","));
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
        "conversion-started",
        StartedPayload {
            id: id_clone.clone(),
        },
    );

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
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(ConversionError::Worker(format!(
            "Upscaling failed: {}",
            last_error
        )));
    }

    let is_nvenc = is_nvenc_codec(&task.config.video_codec);
    let is_videotoolbox = is_videotoolbox_codec(&task.config.video_codec);

    let output_fps = if task.config.fps != "original" {
        task.config.fps.clone()
    } else {
        fps.to_string()
    };

    let mut enc_args = vec![
        "-framerate".to_string(),
        output_fps.clone(),
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

    match task.config.metadata.mode {
        MetadataMode::Clean => {
            enc_args.push("-map_metadata".to_string());
            enc_args.push("-1".to_string());
        }
        MetadataMode::Replace => {
            enc_args.push("-map_metadata".to_string());
            enc_args.push("-1".to_string());
            add_metadata_flags(&mut enc_args, &task.config.metadata);
        }
        MetadataMode::Preserve => {
            add_metadata_flags(&mut enc_args, &task.config.metadata);
        }
    }

    enc_args.push("-map".to_string());
    enc_args.push("0:v:0".to_string());

    if !task.config.selected_audio_tracks.is_empty() {
        for track_index in &task.config.selected_audio_tracks {
            enc_args.push("-map".to_string());
            enc_args.push(format!("1:{}", track_index));
        }
    } else {
        enc_args.push("-map".to_string());
        enc_args.push("1:a?".to_string());
    }

    if !task.config.selected_subtitle_tracks.is_empty() {
        for track_index in &task.config.selected_subtitle_tracks {
            enc_args.push("-map".to_string());
            enc_args.push(format!("1:{}", track_index));
        }
    } else {
        enc_args.push("-map".to_string());
        enc_args.push("1:s?".to_string());
    }

    enc_args.push("-c:v".to_string());
    enc_args.push(task.config.video_codec.clone());

    if task.config.video_bitrate_mode == "bitrate" {
        enc_args.push("-b:v".to_string());
        enc_args.push(format!("{}k", task.config.video_bitrate));
    } else if is_nvenc {
        let cq = (52.0 - (task.config.quality as f64 / 2.0))
            .round()
            .clamp(1.0, 51.0) as u32;
        enc_args.push("-rc:v".to_string());
        enc_args.push("vbr".to_string());
        enc_args.push("-cq:v".to_string());
        enc_args.push(cq.to_string());
    } else if is_videotoolbox {
        enc_args.push("-q:v".to_string());
        enc_args.push(task.config.quality.to_string());
    } else {
        enc_args.push("-crf".to_string());
        enc_args.push(task.config.crf.to_string());
    }

    if !is_videotoolbox {
        enc_args.push("-preset".to_string());
        let preset_value = if is_nvenc {
            map_nvenc_preset(&task.config.preset)
        } else {
            task.config.preset.clone()
        };
        enc_args.push(preset_value);
    }

    if is_nvenc {
        if task.config.nvenc_spatial_aq {
            enc_args.push("-spatial_aq".to_string());
            enc_args.push("1".to_string());
        }
        if task.config.nvenc_temporal_aq {
            enc_args.push("-temporal_aq".to_string());
            enc_args.push("1".to_string());
        }
    }

    if is_videotoolbox && task.config.videotoolbox_allow_sw {
        enc_args.push("-allow_sw".to_string());
        enc_args.push("1".to_string());
    }

    if !task.config.selected_audio_tracks.is_empty() {
        enc_args.push("-c:a".to_string());
        enc_args.push(task.config.audio_codec.clone());

        let lossless_audio_codecs = ["flac", "alac", "pcm_s16le"];
        if !lossless_audio_codecs.contains(&task.config.audio_codec.as_str()) {
            enc_args.push("-b:a".to_string());
            enc_args.push(format!("{}k", task.config.audio_bitrate));
        }
    } else {
        enc_args.push("-c:a".to_string());
        enc_args.push("copy".to_string());
    }

    match task.config.audio_channels.as_str() {
        "stereo" => {
            enc_args.push("-ac".to_string());
            enc_args.push("2".to_string());
        }
        "mono" => {
            enc_args.push("-ac".to_string());
            enc_args.push("1".to_string());
        }
        _ => {}
    }

    let mut audio_filters: Vec<String> = Vec::new();

    if task.config.audio_normalize {
        audio_filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
    }

    if (task.config.audio_volume - 100.0).abs() > VOLUME_EPSILON {
        let volume_factor = task.config.audio_volume / 100.0;
        audio_filters.push(format!("volume={:.2}", volume_factor));
    }

    if !audio_filters.is_empty() {
        enc_args.push("-af".to_string());
        enc_args.push(audio_filters.join(","));
    }

    if task.config.subtitle_burn_path.is_none()
        || task
            .config
            .subtitle_burn_path
            .as_ref()
            .map_or(true, |p| p.is_empty())
    {
        enc_args.push("-c:s".to_string());
        enc_args.push("copy".to_string());
    }

    if task.config.fps != "original" {
        enc_args.push("-r".to_string());
        enc_args.push(task.config.fps.clone());
    }

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

fn add_metadata_flags(
    args: &mut Vec<String>,
    metadata: &crate::conversion::types::MetadataConfig,
) {
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
