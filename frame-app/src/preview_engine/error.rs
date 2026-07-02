use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreviewEngineError {
    #[error("Invalid preview input: {0}")]
    InvalidInput(String),
    #[error("Failed to load preview image `{path}`: {source}")]
    ImageLoad {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error("Unsupported preview frame layout: {0}")]
    UnsupportedFrameLayout(String),
    #[error("FFmpeg preview error: {0}")]
    Ffmpeg(String),
    #[error("Preview IO error: {0}")]
    Io(#[from] std::io::Error),
}
