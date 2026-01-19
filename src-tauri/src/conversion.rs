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
    pub video_bitrate_mode: String,
    pub video_bitrate: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMetadata {
    duration: Option<String>,
    bitrate: Option<String>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    resolution: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEstimate {
    video_kbps: u32,
    audio_kbps: u32,
    total_kbps: u32,
    size_mb: Option<f64>,
}

pub fn build_ffmpeg_args(input: &str, output: &str, config: &ConversionConfig) -> Vec<String> {
    let mut args = vec!["-i".to_string(), input.to_string()];

    let is_audio_only = is_audio_only_container(&config.container);

    if is_audio_only {
        args.push("-vn".to_string());
    } else {
        args.push("-c:v".to_string());
        args.push(config.video_codec.clone());

        if config.video_bitrate_mode == "bitrate" {
            args.push("-b:v".to_string());
            args.push(format!("{}k", config.video_bitrate));
        } else {
            args.push("-crf".to_string());
            args.push(config.crf.to_string());
        }

        args.push("-preset".to_string());
        args.push(config.preset.clone());

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
    }

    args.push("-c:a".to_string());
    args.push(config.audio_codec.clone());
    args.push("-b:a".to_string());
    args.push(format!("{}k", config.audio_bitrate));

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

#[command]
pub async fn probe_media(app: AppHandle, file_path: String) -> Result<ProbeMetadata, String> {
    let args = vec![
        "-hide_banner".to_string(),
        "-i".to_string(),
        file_path.clone(),
    ];

    let sidecar_command = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| e.to_string())?
        .args(args);

    let (mut rx, _) = sidecar_command.spawn().map_err(|e| e.to_string())?;

    let duration_regex = Regex::new(
        r"Duration:\s(?P<duration>\d{2}:\d{2}:\d{2}\.\d{2}),.*bitrate:\s(?P<bitrate>[^,\r\n]+)",
    )
    .unwrap();
    let video_regex = Regex::new(r"Stream #\d+:\d+.*Video:\s(?P<codec>[^,]+)").unwrap();
    let resolution_regex = Regex::new(r"(?P<resolution>\d{2,5}x\d{2,5})").unwrap();
    let audio_regex = Regex::new(r"Stream #\d+:\d+.*Audio:\s(?P<codec>[^,]+)").unwrap();

    let mut metadata = ProbeMetadata::default();

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stderr(line_bytes) = event {
            let line = String::from_utf8_lossy(&line_bytes);

            if metadata.duration.is_none() {
                if let Some(caps) = duration_regex.captures(&line) {
                    if let Some(duration) = caps.name("duration") {
                        metadata.duration = Some(duration.as_str().to_string());
                    }
                    if let Some(bitrate) = caps.name("bitrate") {
                        metadata.bitrate = Some(bitrate.as_str().trim().to_string());
                    }
                }
            }

            if let Some(caps) = video_regex.captures(&line) {
                if metadata.video_codec.is_none() {
                    if let Some(codec) = caps.name("codec") {
                        metadata.video_codec = Some(codec.as_str().trim().to_string());
                    }
                }
                if metadata.resolution.is_none() {
                    if let Some(res_caps) = resolution_regex.captures(&line) {
                        if let Some(res) = res_caps.name("resolution") {
                            metadata.resolution = Some(res.as_str().to_string());
                        }
                    }
                }
            }

            if metadata.audio_codec.is_none() {
                if let Some(caps) = audio_regex.captures(&line) {
                    if let Some(codec) = caps.name("codec") {
                        metadata.audio_codec = Some(codec.as_str().trim().to_string());
                    }
                }
            }
        }
    }

    Ok(metadata)
}

fn parse_duration_to_seconds(duration: Option<&String>) -> Option<f64> {
    let duration = duration?;
    let parts: Vec<&str> = duration.split([':', '.']).collect();
    if parts.len() != 4 {
        return None;
    }
    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;
    let seconds: f64 = parts[2].parse().ok()?;
    let centiseconds: f64 = parts[3].parse().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds + centiseconds / 100.0)
}

fn parse_resolution_height(resolution: Option<&String>) -> Option<i32> {
    let resolution = resolution?;
    let parts: Vec<&str> = resolution.split('x').collect();
    if parts.len() != 2 {
        return None;
    }
    parts[1].parse().ok()
}

fn infer_target_height(config: &ConversionConfig, metadata: Option<&ProbeMetadata>) -> i32 {
    match config.resolution.as_str() {
        "480p" => 480,
        "720p" => 720,
        "1080p" => 1080,
        "original" => {
            parse_resolution_height(metadata.and_then(|m| m.resolution.as_ref())).unwrap_or(720)
        }
        _ => 720,
    }
}

fn base_video_bitrate(height: i32) -> f64 {
    if height >= 2160 {
        25000.0
    } else if height >= 1440 {
        16000.0
    } else if height >= 1080 {
        8000.0
    } else if height >= 720 {
        5000.0
    } else if height >= 480 {
        2500.0
    } else {
        1500.0
    }
}

fn codec_scale(codec: &str) -> f64 {
    match codec.to_lowercase().as_str() {
        "libx265" | "h265" => 0.65,
        "vp9" | "libvpx-vp9" => 0.7,
        "prores" => 1.6,
        _ => 1.0,
    }
}

fn parse_source_bitrate(metadata: Option<&ProbeMetadata>) -> Option<f64> {
    let raw = metadata?.bitrate.as_ref()?;
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn crf_scale(crf: u8) -> f64 {
    let diff = 23i32 - crf as i32;
    (2f64).powf(diff as f64 / 6.0)
}

fn is_audio_only_container(container: &str) -> bool {
    matches!(container.to_lowercase().as_str(), "mp3")
}

#[command]
pub async fn estimate_output(
    config: ConversionConfig,
    metadata: Option<ProbeMetadata>,
) -> Result<OutputEstimate, String> {
    let metadata_ref = metadata.as_ref();
    let audio_only = is_audio_only_container(&config.container);

    let video_kbps = if audio_only {
        0.0
    } else if config.video_bitrate_mode == "bitrate" {
        config.video_bitrate.parse::<f64>().unwrap_or(0.0)
    } else {
        let height = infer_target_height(&config, metadata_ref);
        let source_height =
            parse_resolution_height(metadata_ref.and_then(|m| m.resolution.as_ref()))
                .unwrap_or(height);

        let mut kbps = if let Some(source_kbps) = parse_source_bitrate(metadata_ref) {
            let scale_factor = (height as f64 / source_height as f64).powf(1.75);
            source_kbps * scale_factor
        } else {
            base_video_bitrate(height) * codec_scale(&config.video_codec)
        };

        kbps *= crf_scale(config.crf);
        if kbps < 400.0 {
            kbps = 400.0;
        }
        kbps
    };

    let audio_kbps = if audio_only || metadata_ref.and_then(|m| m.audio_codec.as_ref()).is_some() {
        config.audio_bitrate.parse::<f64>().unwrap_or(128.0)
    } else {
        0.0
    };

    let total_kbps = video_kbps + audio_kbps;

    let size_mb = parse_duration_to_seconds(metadata_ref.and_then(|m| m.duration.as_ref()))
        .map(|seconds| (total_kbps * seconds) / 8.0 / 1024.0)
        .filter(|size| *size > 0.5);

    Ok(OutputEstimate {
        video_kbps: video_kbps.round() as u32,
        audio_kbps: audio_kbps.round() as u32,
        total_kbps: total_kbps.round() as u32,
        size_mb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::async_runtime;

    fn contains_args(args: &[String], expected: &[&str]) -> bool {
        expected.iter().all(|e| args.iter().any(|a| a == e))
    }

    #[test]
    fn test_default_mp4_h264() {
        let config = ConversionConfig {
            container: "mp4".into(),
            video_codec: "libx264".into(),
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
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
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
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
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
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
            video_bitrate_mode: "crf".into(),
            video_bitrate: "8000".into(),
            audio_codec: "ac3".into(),
            audio_bitrate: "192".into(),
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
            video_bitrate_mode: "crf".into(),
            video_bitrate: "2500".into(),
            audio_codec: "libopus".into(),
            audio_bitrate: "96".into(),
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

    #[test]
    fn test_build_output_path_with_custom_name() {
        let custom = build_output_path(
            "/Users/hex/Videos/clip.mov",
            "mp4",
            Some("final_render".into()),
        );
        assert_eq!(custom, "/Users/hex/Videos/final_render.mp4");

        let default = build_output_path("/tmp/sample.mov", "mp4", None);
        assert_eq!(default, "/tmp/sample.mov_converted.mp4");
    }

    fn sample_config(container: &str) -> ConversionConfig {
        ConversionConfig {
            container: container.into(),
            video_codec: "libx264".into(),
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
            resolution: "original".into(),
            crf: 23,
            preset: "medium".into(),
        }
    }

    fn sample_metadata() -> ProbeMetadata {
        ProbeMetadata {
            duration: Some("00:01:00.00".into()),
            bitrate: Some("4000 kb/s".into()),
            video_codec: Some("h264".into()),
            audio_codec: Some("aac".into()),
            resolution: Some("1920x1080".into()),
        }
    }

    #[test]
    fn test_estimate_output_standard_video() {
        let config = sample_config("mp4");
        let metadata = sample_metadata();

        let estimate = async_runtime::block_on(async {
            estimate_output(config, Some(metadata)).await.unwrap()
        });

        assert_eq!(estimate.video_kbps, 4000);
        assert_eq!(estimate.audio_kbps, 128);
        assert_eq!(estimate.total_kbps, 4128);
        let size = estimate.size_mb.expect("size should exist");
        assert!((size - 30.2).abs() < 0.5);
    }

    #[test]
    fn test_estimate_output_without_audio_stream() {
        let config = sample_config("mp4");
        let mut metadata = sample_metadata();
        metadata.audio_codec = None;

        let estimate = async_runtime::block_on(async {
            estimate_output(config, Some(metadata)).await.unwrap()
        });

        assert_eq!(estimate.audio_kbps, 0);
        assert!(estimate.video_kbps > 0);
    }

    #[test]
    fn test_estimate_output_audio_only_container() {
        let mut config = sample_config("mp3");
        config.audio_codec = "mp3".into();
        let metadata = sample_metadata();

        let estimate = async_runtime::block_on(async {
            estimate_output(config, Some(metadata)).await.unwrap()
        });

        assert_eq!(estimate.video_kbps, 0);
        assert_eq!(estimate.audio_kbps, 128);
        assert_eq!(estimate.total_kbps, 128);
    }
}
