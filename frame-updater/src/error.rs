use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("update checks are disabled: {0}")]
    Disabled(String),
    #[error("config directory is unavailable")]
    ConfigDirectoryUnavailable,
    #[error("network request failed: {0}")]
    Network(String),
    #[error("invalid update manifest: {0}")]
    InvalidManifest(String),
    #[error("update manifest signature verification failed")]
    SignatureVerificationFailed,
    #[error("unsupported update platform: {0}")]
    UnsupportedPlatform(String),
    #[error("no update is available")]
    NoUpdateAvailable,
    #[error("failed to read or write update files: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse update JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to parse semantic version: {0}")]
    Semver(#[from] semver::Error),
    #[error("downloaded package hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("failed to start update helper: {0}")]
    HelperSpawnFailed(String),
    #[error("update installation failed: {0}")]
    InstallFailed(String),
}
