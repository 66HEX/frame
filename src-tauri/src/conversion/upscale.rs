use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc;

use crate::conversion::args::{add_metadata_flags, build_output_path};
use crate::conversion::codec::{
    add_audio_codec_args, add_fps_args, add_subtitle_codec_args, add_video_codec_args,
};
use crate::conversion::error::ConversionError;
use crate::conversion::filters::{build_audio_filters, build_video_filters};
use crate::conversion::manager::ManagerMessage;
use crate::conversion::types::{
    CompletedPayload, ConversionTask, LogPayload, MetadataMode, ProgressPayload, StartedPayload,
};
use crate::conversion::utils::{FRAME_REGEX, parse_time, sanitize_external_tool_path};

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

    let mut dec_args = Vec::new();

    // Hardware decode acceleration (only -hwaccel, no output_format since we need CPU frames)
    if task.config.hw_decode {
        if crate::conversion::utils::is_nvenc_codec(&task.config.video_codec) {
            dec_args.push("-hwaccel".to_string());
            dec_args.push("cuda".to_string());
        } else if crate::conversion::utils::is_videotoolbox_codec(&task.config.video_codec) {
            dec_args.push("-hwaccel".to_string());
            dec_args.push("videotoolbox".to_string());
        }
    }

    if let Some(start) = &task.config.start_time {
        if !start.is_empty() {
            dec_args.push("-ss".to_string());
            dec_args.push(start.clone());
        }
    }

    dec_args.push("-i".to_string());
    dec_args.push(task.file_path.clone());

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

    let video_filters = build_video_filters(&task.config, false);
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
                    if let Some(caps) = FRAME_REGEX.captures(&line) {
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
        sanitize_external_tool_path(&input_frames_dir),
        "-o".to_string(),
        sanitize_external_tool_path(&output_frames_dir),
        "-s".to_string(),
        scale.to_string(),
        "-f".to_string(),
        "png".to_string(),
        "-m".to_string(),
        sanitize_external_tool_path(&models_path),
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
    } else if task
        .config
        .subtitle_burn_path
        .as_ref()
        .map_or(true, |path| path.trim().is_empty())
    {
        enc_args.push("-map".to_string());
        enc_args.push("1:s?".to_string());
    }

    add_video_codec_args(&mut enc_args, &task.config);
    add_audio_codec_args(&mut enc_args, &task.config);

    let audio_filters = build_audio_filters(&task.config);
    if !audio_filters.is_empty() {
        enc_args.push("-af".to_string());
        enc_args.push(audio_filters.join(","));
    }

    if !task.config.selected_subtitle_tracks.is_empty()
        || task
            .config
            .subtitle_burn_path
            .as_ref()
            .map_or(true, |path| path.trim().is_empty())
    {
        add_subtitle_codec_args(&mut enc_args, &task.config);
    }
    add_fps_args(&mut enc_args, &task.config);

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
                    if let Some(caps) = FRAME_REGEX.captures(&line) {
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

    let _ = std::fs::remove_dir_all(&temp_dir);
    Err(ConversionError::Worker(
        "Encoder terminated unexpectedly before reporting exit status".to_string(),
    ))
}
