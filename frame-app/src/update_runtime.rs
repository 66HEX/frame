//! Runtime configuration for Frame update checks.

use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use frame_updater::{
    InstallContext, UpdateChannel, UpdateClient, UpdateClientConfig, UpdateError,
    default_cache_dir, default_manifest_url, detect_install_context,
};
use semver::Version;

use crate::app_info::FRAME_APP_ID;

pub const AUTO_UPDATE_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const UPDATE_EXPLANATION_ENV: &str = "FRAME_UPDATE_EXPLANATION";
const UPDATE_PUBLIC_KEY_ENV: &str = "FRAME_UPDATE_PUBLIC_KEY";
const FLATPAK_INFO_PATH: &str = "/.flatpak-info";

/// Builds an update client for the configured update channel.
///
/// # Errors
///
/// Returns an error when update signing keys are missing, the current app
/// version cannot be parsed, the cache directory is unavailable, or client
/// configuration validation fails.
pub fn build_update_client(channel: UpdateChannel) -> Result<UpdateClient, UpdateError> {
    let public_keys = configured_public_keys();
    if public_keys.is_empty() {
        return Err(UpdateError::Disabled(
            "update signing public key is not configured".to_string(),
        ));
    }

    UpdateClient::new(UpdateClientConfig {
        app_id: FRAME_APP_ID.to_string(),
        current_version: current_version()?,
        channel,
        manifest_url: default_manifest_url(),
        public_keys,
        cache_dir: default_cache_dir()?,
        install_context: detect_install_context().unwrap_or_else(|_| fallback_install_context()),
    })
}

#[must_use]
pub fn updates_disabled_explanation() -> Option<String> {
    if let Some(explanation) = std::env::var(UPDATE_EXPLANATION_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Some(explanation);
    }

    package_manager_update_explanation(
        std::env::var_os("APPIMAGE").is_some(),
        std::env::var_os("FLATPAK_ID").is_some(),
        Path::new(FLATPAK_INFO_PATH).is_file(),
    )
}

#[must_use]
pub fn update_check_is_due(last_update_check_at: Option<u64>) -> bool {
    let Some(last_update_check_at) = last_update_check_at else {
        return true;
    };
    unix_timestamp().saturating_sub(last_update_check_at) >= AUTO_UPDATE_CHECK_INTERVAL_SECS
}

#[must_use]
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn current_version() -> Result<Version, UpdateError> {
    Version::parse(env!("CARGO_PKG_VERSION")).map_err(Into::into)
}

fn configured_public_keys() -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(value) = option_env!("FRAME_UPDATE_PUBLIC_KEY") {
        push_public_keys(value, &mut keys);
    }
    if let Ok(value) = std::env::var(UPDATE_PUBLIC_KEY_ENV) {
        push_public_keys(&value, &mut keys);
    }
    keys
}

fn push_public_keys(value: &str, keys: &mut Vec<String>) {
    keys.extend(
        value
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(ToOwned::to_owned),
    );
}

fn package_manager_update_explanation(
    appimage_runtime: bool,
    flatpak_runtime: bool,
    flatpak_info_exists: bool,
) -> Option<String> {
    if flatpak_runtime || flatpak_info_exists {
        Some(
            "This Flatpak build is managed by Flatpak. Install updates through Flatpak or Flathub."
                .to_string(),
        )
    } else if appimage_runtime {
        Some(
            "This AppImage build is updated manually. Download the latest AppImage from GitHub Releases."
                .to_string(),
        )
    } else {
        None
    }
}

fn fallback_install_context() -> InstallContext {
    let executable_path = std::env::current_exe().unwrap_or_else(|_| "frame".into());
    let install_root = executable_path
        .parent()
        .map_or_else(|| ".".into(), std::path::Path::to_path_buf);
    let helper_path = executable_path.with_file_name(if cfg!(target_os = "windows") {
        "frame-update-helper.exe"
    } else {
        "frame-update-helper"
    });

    InstallContext {
        install_root,
        executable_path,
        helper_path,
        relaunch: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_check_is_due_returns_true_when_never_checked() {
        assert!(update_check_is_due(None));
    }

    #[test]
    fn update_check_is_due_returns_false_for_recent_check() {
        assert!(!update_check_is_due(Some(unix_timestamp())));
    }

    #[test]
    fn package_manager_update_explanation_detects_appimage() {
        assert_eq!(
            package_manager_update_explanation(true, false, false),
            Some(
                "This AppImage build is updated manually. Download the latest AppImage from GitHub Releases."
                    .to_string()
            )
        );
    }

    #[test]
    fn package_manager_update_explanation_detects_flatpak() {
        assert_eq!(
            package_manager_update_explanation(false, true, false),
            Some(
                "This Flatpak build is managed by Flatpak. Install updates through Flatpak or Flathub."
                    .to_string()
            )
        );
    }

    #[test]
    fn package_manager_update_explanation_prefers_flatpak_over_appimage() {
        assert_eq!(
            package_manager_update_explanation(true, false, true),
            Some(
                "This Flatpak build is managed by Flatpak. Install updates through Flatpak or Flathub."
                    .to_string()
            )
        );
    }
}
