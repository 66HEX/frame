use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::conversion::error::ConversionError;
use crate::conversion::types::ProbeMetadata;

pub async fn probe_media_file(
    app: &AppHandle,
    file_path: &str,
) -> Result<ProbeMetadata, ConversionError> {
    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(frame_core::probe::ffprobe_json_args(file_path))
        .output()
        .await
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ConversionError::Probe(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    frame_core::probe::parse_ffprobe_stdout(file_path, stdout)
}
