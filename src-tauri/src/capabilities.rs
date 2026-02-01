use regex::Regex;
use tauri::{AppHandle, command};
use tauri_plugin_shell::ShellExt;

#[derive(serde::Serialize, Clone, Debug)]
pub struct AvailableEncoders {
    pub h264_videotoolbox: bool,
    pub h264_nvenc: bool,
    pub hevc_videotoolbox: bool,
    pub hevc_nvenc: bool,
    pub av1_nvenc: bool,
}

#[command]
pub async fn get_available_encoders(app: AppHandle) -> Result<AvailableEncoders, String> {
    let output = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| e.to_string())?
        .args(["-encoders"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let has_encoder = |name: &str| -> bool {
        let pattern = format!(r"(?m)^\s*[A-Z.]+\s+{}\s+", regex::escape(name));
        if let Ok(re) = Regex::new(&pattern) {
            re.is_match(&stdout)
        } else {
            stdout.contains(name)
        }
    };

    Ok(AvailableEncoders {
        h264_videotoolbox: has_encoder("h264_videotoolbox"),
        h264_nvenc: has_encoder("h264_nvenc"),
        hevc_videotoolbox: has_encoder("hevc_videotoolbox"),
        hevc_nvenc: has_encoder("hevc_nvenc"),
        av1_nvenc: has_encoder("av1_nvenc"),
    })
}
