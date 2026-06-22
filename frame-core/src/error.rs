use serde::Serialize;
use thiserror::Error;

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
