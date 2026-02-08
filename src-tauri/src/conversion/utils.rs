use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

pub static FRAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"frame=\s*(\d+)").unwrap());

pub static DURATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Duration:\s*(\d+(?::\d+){0,3}(?:\.\d+)?)").unwrap());

pub static TIME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"time=\s*(\d+(?::\d+){0,3}(?:\.\d+)?)").unwrap());

pub fn parse_frame_rate_string(value: Option<&str>) -> Option<f64> {
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

pub fn parse_probe_bitrate(raw: Option<&str>) -> Option<f64> {
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

pub fn is_audio_only_container(container: &str) -> bool {
    matches!(
        container.to_lowercase().as_str(),
        "mp3" | "wav" | "flac" | "aac" | "m4a"
    )
}

pub fn is_nvenc_codec(codec: &str) -> bool {
    matches!(codec, "h264_nvenc" | "hevc_nvenc" | "av1_nvenc")
}

pub fn is_videotoolbox_codec(codec: &str) -> bool {
    matches!(codec, "h264_videotoolbox" | "hevc_videotoolbox")
}

pub fn map_nvenc_preset(preset: &str) -> String {
    match preset {
        "fast" | "medium" | "slow" => preset.to_string(),
        "default" => "default".to_string(),
        "p1" | "p2" | "p3" | "p4" | "p5" | "p6" | "p7" => preset.to_string(),
        "ultrafast" | "superfast" | "veryfast" | "faster" => "fast".to_string(),
        "slower" | "veryslow" => "slow".to_string(),
        _ => "medium".to_string(),
    }
}

pub fn parse_time(time_str: &str) -> Option<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<f64>().ok(),
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].trim().parse().ok()?;
            Some(m * 60.0 + s)
        }
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].trim().parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        _ => None,
    }
}

pub fn get_hwaccel_args(video_codec: &str) -> Vec<String> {
    if is_nvenc_codec(video_codec) {
        vec![
            "-hwaccel".to_string(),
            "cuda".to_string(),
            "-hwaccel_output_format".to_string(),
            "cuda".to_string(),
        ]
    } else if is_videotoolbox_codec(video_codec) {
        vec!["-hwaccel".to_string(), "videotoolbox".to_string()]
    } else {
        vec![]
    }
}

pub fn sanitize_external_tool_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        let raw = path.to_string_lossy();
        if let Some(stripped_unc) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", stripped_unc);
        }
        if let Some(stripped) = raw.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
        raw.into_owned()
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}
