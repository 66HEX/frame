use tauri::{AppHandle, command};

use crate::conversion::error::ConversionError;
use crate::conversion::ffmpeg::validate_task_input;
use crate::conversion::manager::{ConversionManager, ManagerMessage};
use crate::conversion::probe::probe_media_file;
use crate::conversion::types::{ConversionConfig, ConversionTask, ProbeMetadata};

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
pub async fn cancel_conversion(
    manager: tauri::State<'_, ConversionManager>,
    id: String,
) -> Result<(), ConversionError> {
    manager.cancel_task(&id)
}

#[command]
pub async fn probe_media(
    app: AppHandle,
    file_path: String,
) -> Result<ProbeMetadata, ConversionError> {
    probe_media_file(&app, &file_path).await
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
