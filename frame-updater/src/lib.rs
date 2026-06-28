//! Signed manifest based updater for the native Frame application.

mod client;
mod error;
pub mod helper;
mod install;
mod manifest;
mod platform;
mod security;

pub use client::{
    DownloadProgress, InstallContext, UpdateCheck, UpdateClient, UpdateClientConfig, UpdateInfo,
    UpdatePackage, default_cache_dir, default_manifest_url, detect_install_context,
};
pub use error::UpdateError;
pub use install::{InstallPlan, InstallResult, run_install_plan};
pub use manifest::{UpdateAsset, UpdateAssetKind, UpdateChannel, UpdateManifest};
pub use platform::PlatformAssetKey;
pub use security::{file_sha256_hex, sign_manifest_bytes, verify_manifest_signature};
