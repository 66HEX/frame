use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tauri::{AppHandle, Emitter, command};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use thiserror::Error;
use tokio::sync::mpsc;

#[cfg(unix)]
use libc;

#[cfg(windows)]
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, HMODULE},
        System::{
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            Threading::{OpenProcess, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE, TerminateProcess},
        },
    },
    core::{PCSTR, s},
};

const DEFAULT_MAX_CONCURRENCY: usize = 2;
const VOLUME_EPSILON: f64 = 0.01;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    pub index: u32,
    pub codec: String,
    pub channels: String,
    pub language: Option<String>,
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<f64>,
    pub sample_rate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMetadata {
    pub duration: Option<String>,
    pub bitrate: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_bitrate_kbps: Option<f64>,
    pub audio_tracks: Vec<AudioTrack>,
    #[serde(default)]
    pub tags: Option<FfprobeTags>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_primaries: Option<String>,
    pub profile: Option<String>,
}

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

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Shell command failed: {0}")]
    Shell(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Internal channel error: {0}")]
    Channel(String),
    #[error("Probe failed: {0}")]
    Probe(String),
    #[error("Worker process error: {0}")]
    Worker(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
}

impl Serialize for ConversionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone)]
struct ConversionTask {
    id: String,
    file_path: String,
    output_name: Option<String>,
    config: ConversionConfig,
}

enum ManagerMessage {
    Enqueue(ConversionTask),
    TaskStarted(String, u32),
    TaskCompleted(String),
    TaskError(String, ConversionError),
}

pub struct ConversionManager {
    sender: mpsc::Sender<ManagerMessage>,
    max_concurrency: Arc<AtomicUsize>,
    active_tasks: Arc<Mutex<HashMap<String, u32>>>,
}

impl ConversionManager {
    pub fn new(app: AppHandle) -> Self {
        let (tx, mut rx) = mpsc::channel(32);
        let tx_clone = tx.clone();
        let max_concurrency = Arc::new(AtomicUsize::new(DEFAULT_MAX_CONCURRENCY));
        let limiter = Arc::clone(&max_concurrency);
        let active_tasks = Arc::new(Mutex::new(HashMap::new()));
        let active_tasks_loop = Arc::clone(&active_tasks);

        tauri::async_runtime::spawn(async move {
            let mut queue: VecDeque<ConversionTask> = VecDeque::new();
            let mut running_tasks: HashMap<String, ()> = HashMap::new();

            while let Some(msg) = rx.recv().await {
                match msg {
                    ManagerMessage::Enqueue(task) => {
                        queue.push_back(task);
                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                        )
                        .await;
                    }
                    ManagerMessage::TaskStarted(id, pid) => {
                        let mut tasks = active_tasks_loop.lock().unwrap();
                        tasks.insert(id, pid);
                    }
                    ManagerMessage::TaskCompleted(id) => {
                        running_tasks.remove(&id);
                        {
                            let mut tasks = active_tasks_loop.lock().unwrap();
                            tasks.remove(&id);
                        }

                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                        )
                        .await;
                    }
                    ManagerMessage::TaskError(id, err) => {
                        eprintln!("Task {} failed: {}", id, err);
                        running_tasks.remove(&id);
                        {
                            let mut tasks = active_tasks_loop.lock().unwrap();
                            tasks.remove(&id);
                        }

                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                        )
                        .await;
                    }
                }
            }
        });

        Self {
            sender: tx,
            max_concurrency,
            active_tasks,
        }
    }

    async fn process_queue(
        app: &AppHandle,
        tx: &mpsc::Sender<ManagerMessage>,
        queue: &mut VecDeque<ConversionTask>,
        running_tasks: &mut HashMap<String, ()>,
        max_concurrency: Arc<AtomicUsize>,
    ) {
        let limit = max_concurrency.load(Ordering::SeqCst).max(1);

        while running_tasks.len() < limit {
            if let Some(task) = queue.pop_front() {
                running_tasks.insert(task.id.clone(), ());

                let app_clone = app.clone();
                let tx_worker = tx.clone();
                let task_clone = task.clone();

                tauri::async_runtime::spawn(async move {
                    if let Err(e) =
                        run_ffmpeg_worker(app_clone, tx_worker.clone(), task_clone.clone()).await
                    {
                        let _ = tx_worker
                            .send(ManagerMessage::TaskError(task_clone.id, e))
                            .await;
                    } else {
                        let _ = tx_worker
                            .send(ManagerMessage::TaskCompleted(task_clone.id))
                            .await;
                    }
                });
            } else {
                break;
            }
        }
    }

    pub fn current_max_concurrency(&self) -> usize {
        self.max_concurrency.load(Ordering::SeqCst)
    }

    pub fn update_max_concurrency(&self, value: usize) -> Result<(), ConversionError> {
        if value == 0 {
            return Err(ConversionError::InvalidInput(
                "Max concurrency must be at least 1".to_string(),
            ));
        }
        self.max_concurrency.store(value, Ordering::SeqCst);
        Ok(())
    }

    pub fn pause_task(&self, id: &str) -> Result<(), ConversionError> {
        let tasks = self.active_tasks.lock().unwrap();
        if let Some(&pid) = tasks.get(id) {
            #[cfg(unix)]
            unsafe {
                if libc::kill(pid as libc::pid_t, libc::SIGSTOP) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGSTOP".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                windows_suspend_resume(pid, true)?;
            }

            Ok(())
        } else {
            Err(ConversionError::TaskNotFound(id.to_string()))
        }
    }

    pub fn resume_task(&self, id: &str) -> Result<(), ConversionError> {
        let tasks = self.active_tasks.lock().unwrap();
        if let Some(&pid) = tasks.get(id) {
            #[cfg(unix)]
            unsafe {
                if libc::kill(pid as libc::pid_t, libc::SIGCONT) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGCONT".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                windows_suspend_resume(pid, false)?;
            }

            Ok(())
        } else {
            Err(ConversionError::TaskNotFound(id.to_string()))
        }
    }
}

#[cfg(windows)]
unsafe fn windows_suspend_resume(pid: u32, suspend: bool) -> Result<(), ConversionError> {
    let process_handle = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)
        .map_err(|e| ConversionError::Shell(format!("Failed to open process: {}", e)))?;

    let ntdll = GetModuleHandleA(s!("ntdll.dll")).map_err(|e| {
        let _ = CloseHandle(process_handle);
        ConversionError::Shell(format!("Failed to get ntdll handle: {}", e))
    })?;

    let fn_name = if suspend {
        s!("NtSuspendProcess")
    } else {
        s!("NtResumeProcess")
    };

    let func_ptr = GetProcAddress(ntdll, fn_name);

    if let Some(func) = func_ptr {
        let func: extern "system" fn(HANDLE) -> i32 = std::mem::transmute(func);
        let status = func(process_handle);
        let _ = CloseHandle(process_handle);

        if status != 0 {
            return Err(ConversionError::Shell(format!(
                "NtSuspendProcess/NtResumeProcess failed with status: {}",
                status
            )));
        }
        Ok(())
    } else {
        let _ = CloseHandle(process_handle);
        Err(ConversionError::Shell(
            "Could not find NtSuspendProcess/NtResumeProcess in ntdll".to_string(),
        ))
    }
}

impl ConversionManager {
    pub fn cancel_task(&self, id: &str) -> Result<(), ConversionError> {
        let tasks = self.active_tasks.lock().unwrap();
        if let Some(&pid) = tasks.get(id) {
            // First resume the process to ensure it can handle the kill signal properly
            #[cfg(unix)]
            unsafe {
                let _ = libc::kill(pid as libc::pid_t, libc::SIGCONT);
                if libc::kill(pid as libc::pid_t, libc::SIGKILL) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGKILL".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                // Resume first just in case
                let _ = windows_suspend_resume(pid, false);

                let process_handle = OpenProcess(
                    windows::Win32::System::Threading::PROCESS_TERMINATE,
                    false,
                    pid,
                )
                .map_err(|e| {
                    ConversionError::Shell(format!("Failed to open process for termination: {}", e))
                })?;

                let _ = windows::Win32::System::Threading::TerminateProcess(process_handle, 1);
                let _ = CloseHandle(process_handle);
            }

            Ok(())
        } else {
            // Task might not be running yet or already finished, which is fine for cancel
            Ok(())
        }
    }
}

#[command]
pub async fn cancel_conversion(
    manager: tauri::State<'_, ConversionManager>,
    id: String,
) -> Result<(), ConversionError> {
    manager.cancel_task(&id)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConversionConfig {
    pub container: String,
    pub video_codec: String,
    pub video_bitrate_mode: String,
    pub video_bitrate: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
    pub audio_channels: String,
    #[serde(default = "default_audio_volume")]
    pub audio_volume: f64,
    #[serde(default)]
    pub audio_normalize: bool,
    pub selected_audio_tracks: Vec<u32>,
    pub resolution: String,
    pub custom_width: Option<String>,
    pub custom_height: Option<String>,
    pub scaling_algorithm: String,
    pub fps: String,
    pub crf: u8,
    #[serde(default = "default_quality")]
    pub quality: u32,
    pub preset: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    #[serde(default)]
    pub metadata: MetadataConfig,
    #[serde(default = "default_rotation")]
    pub rotation: String,
    #[serde(default)]
    pub flip_horizontal: bool,
    #[serde(default)]
    pub flip_vertical: bool,
    #[serde(default)]
    pub crop: Option<CropConfig>,
}

fn default_rotation() -> String {
    "0".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CropConfig {
    pub enabled: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataConfig {
    pub mode: MetadataMode,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MetadataMode {
    #[default]
    Preserve,
    Clean,
    Replace,
}

fn default_quality() -> u32 {
    50
}

fn default_audio_volume() -> f64 {
    100.0
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

#[derive(Clone, Serialize)]
struct LogPayload {
    id: String,
    line: String,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

#[derive(Deserialize)]
struct FfprobeStream {
    index: u32,
    codec_type: String,
    codec_name: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    channels: Option<i32>,
    bit_rate: Option<String>,
    avg_frame_rate: Option<String>,
    #[allow(dead_code)]
    channel_layout: Option<String>,
    tags: Option<FfprobeTags>,
    pix_fmt: Option<String>,
    color_space: Option<String>,
    color_range: Option<String>,
    color_primaries: Option<String>,
    profile: Option<String>,
    sample_rate: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
    tags: Option<FfprobeTags>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FfprobeTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "creation_time")]
    pub creation_time: Option<String>,
    pub language: Option<String>,
    pub comment: Option<String>,
    #[serde(rename = "DESCRIPTION")]
    pub description_upper: Option<String>,
    #[serde(rename = "DATE")]
    pub date_upper: Option<String>,
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

    if let Some(end) = &config.end_time {
        if !end.is_empty() {
            args.push("-to".to_string());
            args.push(end.clone());
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

    if is_audio_only {
        args.push("-vn".to_string());
    } else {
        args.push("-c:v".to_string());
        args.push(config.video_codec.clone());

        if config.video_bitrate_mode == "bitrate" {
            args.push("-b:v".to_string());
            args.push(format!("{}k", config.video_bitrate));
        } else if config.video_codec == "h264_nvenc" || config.video_codec == "hevc_nvenc" {
            // NVENC uses -rc:v vbr and -cq:v (1-51), where 1 is best.
            // Map Quality (1-100, 100 best) to CQ (51-1).
            let cq = (52.0 - (config.quality as f64 / 2.0))
                .round()
                .clamp(1.0, 51.0) as u32;
            args.push("-rc:v".to_string());
            args.push("vbr".to_string());
            args.push("-cq:v".to_string());
            args.push(cq.to_string());
        } else if config.video_codec == "h264_videotoolbox"
            || config.video_codec == "hevc_videotoolbox"
        {
            // VideoToolbox uses -q:v (1-100), where 100 is best.
            args.push("-q:v".to_string());
            args.push(config.quality.to_string());
        } else {
            args.push("-crf".to_string());
            args.push(config.crf.to_string());
        }

        args.push("-preset".to_string());
        args.push(config.preset.clone());

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
                    // Fit within the box, preserving aspect ratio, and pad with black bars
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
                    "1080p" => format!("scale=-1:1080{}", algorithm),
                    "720p" => format!("scale=-1:720{}", algorithm),
                    "480p" => format!("scale=-1:480{}", algorithm),
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

    if !config.selected_audio_tracks.is_empty() && !is_audio_only {
        args.push("-map".to_string());
        args.push("0:v:0".to_string());
    }

    if !config.selected_audio_tracks.is_empty() {
        for track_index in &config.selected_audio_tracks {
            args.push("-map".to_string());
            args.push(format!("0:{}", track_index));
        }
    }

    args.push("-c:a".to_string());
    args.push(config.audio_codec.clone());

    let lossless_audio_codecs = ["flac", "alac", "pcm_s16le"];
    if !lossless_audio_codecs.contains(&config.audio_codec.as_str()) {
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

async fn run_ffmpeg_worker(
    app: AppHandle,
    tx: mpsc::Sender<ManagerMessage>,
    task: ConversionTask,
) -> Result<(), ConversionError> {
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

    // Notify manager about the PID
    let _ = tx
        .send(ManagerMessage::TaskStarted(id.clone(), child.pid()))
        .await;

    let duration_regex = Regex::new(r"Duration: (\d{2}:\d{2}:\d{2}\.\d{2})").unwrap();
    let time_regex = Regex::new(r"time=(\d{2}:\d{2}:\d{2}\.\d{2})").unwrap();

    let mut total_duration: Option<f64> = None;
    let mut exit_code: Option<i32> = None;

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

fn validate_task_input(file_path: &str, config: &ConversionConfig) -> Result<(), ConversionError> {
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
        // -1 is allowed for "keep aspect ratio", but strictly negative values < -1 are invalid for scale filter
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

#[command]
pub async fn queue_conversion(
    manager: tauri::State<'_, ConversionManager>,
    id: String,
    file_path: String,
    output_name: Option<String>,
    config: ConversionConfig,
) -> Result<(), ConversionError> {
    validate_task_input(&file_path, &config)?;

    let task = ConversionTask {
        id,
        file_path,
        output_name,
        config,
    };

    manager
        .sender
        .send(ManagerMessage::Enqueue(task))
        .await
        .map_err(|e| ConversionError::Channel(e.to_string()))?;
    Ok(())
}

#[command]
pub async fn pause_conversion(
    manager: tauri::State<'_, ConversionManager>,
    id: String,
) -> Result<(), ConversionError> {
    manager.pause_task(&id)
}

#[command]
pub async fn resume_conversion(
    manager: tauri::State<'_, ConversionManager>,
    id: String,
) -> Result<(), ConversionError> {
    manager.resume_task(&id)
}

#[command]
pub async fn probe_media(
    app: AppHandle,
    file_path: String,
) -> Result<ProbeMetadata, ConversionError> {
    let args = vec![
        "-v".to_string(),
        "quiet".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        file_path.clone(),
    ];

    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(args)
        .output()
        .await
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ConversionError::Probe(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let probe_data: FfprobeOutput = serde_json::from_str(&stdout)?;

    let mut metadata = ProbeMetadata::default();

    metadata.duration = probe_data.format.duration;
    metadata.bitrate = probe_data.format.bit_rate;

    if let Some(tags) = probe_data.format.tags {
        metadata.tags = Some(tags);
    }

    if let Some(video_stream) = probe_data.streams.iter().find(|s| s.codec_type == "video") {
        metadata.video_codec = video_stream.codec_name.clone();
        metadata.pixel_format = video_stream.pix_fmt.clone();
        metadata.color_space = video_stream.color_space.clone();
        metadata.color_range = video_stream.color_range.clone();
        metadata.color_primaries = video_stream.color_primaries.clone();
        metadata.profile = video_stream.profile.clone();

        if let (Some(w), Some(h)) = (video_stream.width, video_stream.height) {
            if w > 0 && h > 0 {
                metadata.width = Some(w as u32);
                metadata.height = Some(h as u32);
                metadata.resolution = Some(format!("{}x{}", w, h));
            }
        }

        if metadata.frame_rate.is_none() {
            metadata.frame_rate = parse_frame_rate_string(video_stream.avg_frame_rate.as_deref());
        }

        if metadata.video_bitrate_kbps.is_none() {
            metadata.video_bitrate_kbps = parse_probe_bitrate(video_stream.bit_rate.as_deref());
        }
    }

    for stream in probe_data
        .streams
        .iter()
        .filter(|s| s.codec_type == "audio")
    {
        let label = stream.tags.as_ref().and_then(|t| t.title.clone());
        let language = stream.tags.as_ref().and_then(|t| t.language.clone());

        let track_bitrate = parse_probe_bitrate(stream.bit_rate.as_deref());

        metadata.audio_tracks.push(AudioTrack {
            index: stream.index,
            codec: stream.codec_name.clone().unwrap_or("unknown".to_string()),
            channels: stream
                .channels
                .map(|c| c.to_string())
                .unwrap_or("?".to_string()),
            label,
            language,
            bitrate_kbps: track_bitrate,
            sample_rate: stream.sample_rate.clone(),
        });
    }

    if let Some(first_audio) = metadata.audio_tracks.first() {
        metadata.audio_codec = Some(first_audio.codec.clone());
    }

    if metadata.video_bitrate_kbps.is_none() {
        if let Some(container_kbps) = parse_probe_bitrate(metadata.bitrate.as_deref()) {
            let audio_sum: f64 = metadata
                .audio_tracks
                .iter()
                .filter_map(|track| track.bitrate_kbps)
                .sum();
            if container_kbps > audio_sum {
                metadata.video_bitrate_kbps = Some(container_kbps - audio_sum);
            }
        }
    }

    Ok(metadata)
}

#[command]
pub fn get_max_concurrency(
    manager: tauri::State<'_, ConversionManager>,
) -> Result<usize, ConversionError> {
    Ok(manager.current_max_concurrency())
}

#[command]
pub fn set_max_concurrency(
    manager: tauri::State<'_, ConversionManager>,
    value: usize,
) -> Result<(), ConversionError> {
    manager.update_max_concurrency(value)
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
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            resolution: "original".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 23,
            quality: 50,
            preset: "medium".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
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
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            resolution: "1080p".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 23,
            quality: 50,
            preset: "medium".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
        };
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-1:1080:flags=bicubic");
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
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            resolution: "720p".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 23,
            quality: 50,
            preset: "medium".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
        };

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-1:720:flags=bicubic");
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
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            resolution: "original".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 18,
            quality: 50,
            preset: "slow".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
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
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            resolution: "original".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 30,
            quality: 50,
            preset: "medium".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
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
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            resolution: "original".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 23,
            quality: 50,
            preset: "medium".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
        }
    }

    #[test]
    fn test_custom_resolution_and_fps() {
        let mut config = sample_config("mp4");
        config.resolution = "custom".into();
        config.custom_width = Some("1280".into());
        config.custom_height = Some("720".into());
        config.scaling_algorithm = "lanczos".into();
        config.fps = "60".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_arg = args.iter().find(|a| a.starts_with("scale=")).unwrap();
        assert_eq!(
            vf_arg,
            "scale=1280:720:force_original_aspect_ratio=decrease:flags=lanczos,pad=1280:720:(ow-iw)/2:(oh-ih)/2"
        );

        let fps_index = args.iter().position(|a| a == "-r").unwrap();
        assert_eq!(args[fps_index + 1], "60");
    }

    #[test]
    fn test_video_bitrate_mode() {
        let mut config = sample_config("mp4");
        config.video_bitrate_mode = "bitrate".into();
        config.video_bitrate = "2500".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        assert!(contains_args(&args, &["-b:v", "2500k"]));
        assert!(!args.iter().any(|a| a == "-crf"));
    }

    #[test]
    fn test_av1_codec() {
        let mut config = sample_config("mkv");
        config.video_codec = "libsvtav1".into();

        let args = build_ffmpeg_args("in.mp4", "out.mkv", &config);

        assert!(contains_args(&args, &["-c:v", "libsvtav1"]));
    }

    #[test]
    fn test_hardware_encoder_nvenc() {
        let mut config = sample_config("mp4");
        config.video_codec = "h264_nvenc".into();
        config.quality = 50; // Should map to CQ ~27 (52 - 25)

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        assert!(contains_args(&args, &["-c:v", "h264_nvenc"]));
        assert!(contains_args(&args, &["-rc:v", "vbr"]));
        assert!(contains_args(&args, &["-cq:v", "27"]));
        assert!(!args.iter().any(|a| a == "-crf"));

        // Test HEVC NVENC
        config.video_codec = "hevc_nvenc".into();
        let args_hevc = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(contains_args(&args_hevc, &["-c:v", "hevc_nvenc"]));
        assert!(contains_args(&args_hevc, &["-cq:v", "27"]));
    }

    #[test]
    fn test_hardware_encoder_videotoolbox() {
        let mut config = sample_config("mov");
        config.video_codec = "h264_videotoolbox".into();
        config.quality = 55;

        let args = build_ffmpeg_args("in.mov", "out.mov", &config);

        assert!(contains_args(&args, &["-c:v", "h264_videotoolbox"]));
        assert!(contains_args(&args, &["-q:v", "55"]));
        assert!(!args.iter().any(|a| a == "-crf"));

        config.video_codec = "hevc_videotoolbox".into();
        let args_hevc = build_ffmpeg_args("in.mov", "out.mov", &config);
        assert!(contains_args(&args_hevc, &["-c:v", "hevc_videotoolbox"]));
        assert!(contains_args(&args_hevc, &["-q:v", "55"]));
    }

    #[test]
    fn test_scaling_algorithms() {
        let algos = vec![
            ("lanczos", ":flags=lanczos"),
            ("bicubic", ":flags=bicubic"),
            ("nearest", ":flags=neighbor"),
        ];

        for (algo_name, expected_flag) in algos {
            let mut config = sample_config("mp4");
            config.resolution = "720p".into();
            config.scaling_algorithm = algo_name.into();

            let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
            let vf_arg = args.iter().find(|a| a.starts_with("scale=")).unwrap();
            assert!(
                vf_arg.ends_with(expected_flag),
                "Algorithm {} expected flag {}, got {}",
                algo_name,
                expected_flag,
                vf_arg
            );
        }
    }

    #[test]
    fn test_audio_volume_filter() {
        let config = sample_config("mp4");
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(!args.iter().any(|a| a == "-af"), "no -af at 100% volume");

        let mut config_reduced = sample_config("mp4");
        config_reduced.audio_volume = 50.0;
        let args_reduced = build_ffmpeg_args("in.mp4", "out.mp4", &config_reduced);
        let af_index = args_reduced.iter().position(|r| r == "-af").unwrap();
        assert_eq!(args_reduced[af_index + 1], "volume=0.50");

        let mut config_boosted = sample_config("mp4");
        config_boosted.audio_volume = 150.0;
        let args_boosted = build_ffmpeg_args("in.mp4", "out.mp4", &config_boosted);
        let af_index = args_boosted.iter().position(|r| r == "-af").unwrap();
        assert_eq!(args_boosted[af_index + 1], "volume=1.50");
    }
}
