use crate::conversion_runner::ConversionProcessController;
use crate::frame_export::{
    SourceFrameExportOutcome, SourceFrameExportRequest, export_source_frame,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    input: PathBuf,
    output_directory: PathBuf,
    position_microseconds: u64,
    video_stream_index: u32,
}

/// # Errors
/// Returns an error if the request cannot be read or the source frame cannot be published.
pub fn export(path: &Path) -> Result<PathBuf, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("Could not read request: {error}"))?;
    let request: Request =
        serde_json::from_slice(&bytes).map_err(|error| format!("Invalid request JSON: {error}"))?;
    let request = SourceFrameExportRequest::new(
        "qa-source-frame".into(),
        request.input,
        request.position_microseconds,
        request.video_stream_index,
        request.output_directory,
    )
    .map_err(|error| error.to_string())?;
    match export_source_frame(&request, &ConversionProcessController::default())
        .map_err(|error| error.to_string())?
    {
        SourceFrameExportOutcome::Published { path } => Ok(path),
        SourceFrameExportOutcome::Cancelled => Err("Source frame export cancelled".into()),
    }
}
