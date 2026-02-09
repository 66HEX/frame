use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc;

use crate::conversion::args::{build_ffmpeg_args, build_output_path};
use crate::conversion::error::ConversionError;
use crate::conversion::manager::ManagerMessage;
use crate::conversion::types::{
    CompletedPayload, ConversionTask, LogPayload, ProgressPayload, StartedPayload,
};
use crate::conversion::upscale::run_upscale_worker;
use crate::conversion::utils::{DURATION_REGEX, TIME_REGEX, parse_time};

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

    let output_path = build_output_path(
        &task.file_path,
        &task.config.container,
        task.output_name.clone(),
    );
    let args = build_ffmpeg_args(&task.file_path, &output_path, &task.config);

    let sidecar_command = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(args);

    let (mut rx, child) = sidecar_command
        .spawn()
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    let id = task.id.clone();

    let _ = tx
        .send(ManagerMessage::TaskStarted(id.clone(), child.pid()))
        .await;

    let _ = app.emit("conversion-started", StartedPayload { id: id.clone() });

    let _ = app.emit(
        "conversion-progress",
        ProgressPayload {
            id: id.clone(),
            progress: 0.0,
        },
    );

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
                let raw_output = String::from_utf8_lossy(&line_bytes).to_string();

                for segment in raw_output.split(['\r', '\n']) {
                    let line = segment.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let _ = app.emit(
                        "conversion-log",
                        LogPayload {
                            id: id.clone(),
                            line: line.to_string(),
                        },
                    );

                    if let Some(caps) = TIME_REGEX.captures(line) {
                        if let Some(match_str) = caps.get(1) {
                            if let Some(current_time) = parse_time(match_str.as_str()) {
                                let duration = if expected_duration > 0.0 {
                                    expected_duration
                                } else if let Some(d) = total_duration {
                                    d
                                } else if let Some(caps) = DURATION_REGEX.captures(line) {
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
                                    let _ = app.emit(
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
            }
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
            }
            _ => {}
        }
    }

    if exit_code == Some(0) {
        let _ = app.emit(
            "conversion-completed",
            CompletedPayload {
                id: id.clone(),
                output_path,
            },
        );
        Ok(())
    } else {
        let err_msg = format!("Process terminated with code {:?}", exit_code);
        Err(ConversionError::Worker(err_msg))
    }
}
