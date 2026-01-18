use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, command};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConversionConfig {
    pub container: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub resolution: String,
    pub crf: u8,
    pub preset: String,
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    id: String,
    progress: f64,
}

#[derive(Clone, Serialize)]
struct CompletedPayload {
    id: String,
    output_path: String,
}

#[derive(Clone, Serialize)]
struct ErrorPayload {
    id: String,
    error: String,
}

pub fn build_ffmpeg_args(input: &str, output: &str, config: &ConversionConfig) -> Vec<String> {
    let mut args = vec![
        "-i".to_string(),
        input.to_string(),
        "-c:v".to_string(),
        config.video_codec.clone(),
        "-crf".to_string(),
        config.crf.to_string(),
        "-preset".to_string(),
        config.preset.clone(),
        "-c:a".to_string(),
        config.audio_codec.clone(),
    ];

    if config.resolution != "original" {
        let scale = match config.resolution.as_str() {
            "1080p" => "scale=-1:1080",
            "720p" => "scale=-1:720",
            "480p" => "scale=-1:480",
            _ => "scale=-1:-1",
        };
        args.push("-vf".to_string());
        args.push(scale.to_string());
    }

    args.push("-y".to_string());
    args.push(output.to_string());

    args
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

fn build_output_path(file_path: &str, container: &str, output_name: Option<String>) -> String {
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

#[command]
pub async fn start_conversion(
    app: AppHandle,
    id: String,
    file_path: String,
    output_name: Option<String>,
    config: ConversionConfig,
) -> Result<(), String> {
    let output_path = build_output_path(&file_path, &config.container, output_name);
    let args = build_ffmpeg_args(&file_path, &output_path, &config);

    let sidecar_command = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| e.to_string())?
        .args(args);

    let (mut rx, _) = sidecar_command.spawn().map_err(|e| e.to_string())?;

    let id_clone = id.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let duration_regex = Regex::new(r"Duration: (\d{2}:\d{2}:\d{2}\.\d{2})").unwrap();
        let time_regex = Regex::new(r"time=(\d{2}:\d{2}:\d{2}\.\d{2})").unwrap();

        let mut total_duration: Option<f64> = None;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(line_bytes) => {
                    let line = String::from_utf8_lossy(&line_bytes);

                    if total_duration.is_none() {
                        if let Some(caps) = duration_regex.captures(&line) {
                            if let Some(match_str) = caps.get(1) {
                                total_duration = parse_time(match_str.as_str());
                            }
                        }
                    }

                    if let Some(duration) = total_duration {
                        if let Some(caps) = time_regex.captures(&line) {
                            if let Some(match_str) = caps.get(1) {
                                if let Some(current_time) = parse_time(match_str.as_str()) {
                                    let progress = (current_time / duration * 100.0).min(100.0);
                                    let _ = app_clone.emit(
                                        "conversion-progress",
                                        ProgressPayload {
                                            id: id_clone.clone(),
                                            progress,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
                CommandEvent::Terminated(payload) => {
                    if payload.code == Some(0) {
                        let _ = app_clone.emit(
                            "conversion-completed",
                            CompletedPayload {
                                id: id_clone.clone(),
                                output_path: output_path.clone(),
                            },
                        );
                    } else {
                        let _ = app_clone.emit(
                            "conversion-error",
                            ErrorPayload {
                                id: id_clone.clone(),
                                error: format!("Process terminated with code {:?}", payload.code),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains_args(args: &[String], expected: &[&str]) -> bool {
        expected.iter().all(|e| args.iter().any(|a| a == e))
    }

    #[test]
    fn test_default_mp4_h264() {
        let config = ConversionConfig {
            container: "mp4".into(),
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            resolution: "original".into(),
            crf: 23,
            preset: "medium".into(),
        };
        let args = build_ffmpeg_args("input.mov", "output.mp4", &config);

        assert_eq!(args[0], "-i");
        assert_eq!(args[1], "input.mov");

        assert!(contains_args(&args, &["-c:v", "libx264"]));
        assert!(contains_args(&args, &["-c:a", "aac"]));

        assert!(contains_args(&args, &["-crf", "23"]));
        assert!(contains_args(&args, &["-preset", "medium"]));

        assert!(!args.iter().any(|a| a == "-vf"));
    }

    #[test]
    fn test_resolution_scaling_1080p() {
        let config = ConversionConfig {
            container: "mp4".into(),
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            resolution: "1080p".into(),
            crf: 23,
            preset: "medium".into(),
        };
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-1:1080");
    }

    #[test]
    fn test_resolution_scaling_720p() {
        let config = ConversionConfig {
            container: "mp4".into(),
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            resolution: "720p".into(),
            crf: 23,
            preset: "medium".into(),
        };
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-1:720");
    }

    #[test]
    fn test_high_quality_h265() {
        let config = ConversionConfig {
            container: "mkv".into(),
            video_codec: "libx265".into(),
            audio_codec: "ac3".into(),
            resolution: "original".into(),
            crf: 18,
            preset: "slow".into(),
        };
        let args = build_ffmpeg_args("raw.mov", "archive.mkv", &config);

        assert!(contains_args(&args, &["-c:v", "libx265"]));
        assert!(contains_args(&args, &["-crf", "18"]));
        assert!(contains_args(&args, &["-preset", "slow"]));
        assert!(contains_args(&args, &["-c:a", "ac3"]));
        assert_eq!(args.last().unwrap(), "archive.mkv");
    }

    #[test]
    fn test_web_optimization_vp9() {
        let config = ConversionConfig {
            container: "webm".into(),
            video_codec: "libvpx-vp9".into(),
            audio_codec: "libopus".into(),
            resolution: "original".into(),
            crf: 30,
            preset: "medium".into(),
        };
        let args = build_ffmpeg_args("clip.mp4", "web.webm", &config);

        assert!(contains_args(&args, &["-c:v", "libvpx-vp9"]));
        assert!(contains_args(&args, &["-c:a", "libopus"]));
        assert!(args.last().unwrap().ends_with(".webm"));
    }

    #[test]
    fn test_time_parsing() {
        assert_eq!(parse_time("00:00:10.50"), Some(10.5));
        assert_eq!(parse_time("01:00:00.00"), Some(3600.0));
        assert_eq!(parse_time("00:01:05.10"), Some(65.1));

        assert_eq!(parse_time("invalid"), None);
        assert_eq!(parse_time("00:10"), None);
    }
}
