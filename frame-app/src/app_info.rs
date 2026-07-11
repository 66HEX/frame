//! Application identity shared by runtime and packaging metadata.

pub const FRAME_APP_NAME: &str = "Frame";
// Matches the legacy Tauri bundle identifier so the GPUI rewrite remains the same app identity.
pub const FRAME_APP_ID: &str = "Frame";
pub const FRAME_APP_VERSION: &str = env!("CARGO_PKG_VERSION");
