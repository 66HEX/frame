use std::{collections::BTreeMap, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{PlatformAssetKey, UpdateError};

pub const UPDATE_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const FRAME_APP_ID: &str = "Frame";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub schema_version: u32,
    pub app_id: String,
    pub channel: UpdateChannel,
    pub version: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub min_supported_version: Option<String>,
    #[serde(default)]
    pub release_notes_url: Option<String>,
    #[serde(default)]
    pub release_notes_markdown: Option<String>,
    pub assets: BTreeMap<String, UpdateAsset>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub target_triple: String,
    pub kind: UpdateAssetKind,
    pub file_name: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub installer_args: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    #[default]
    Stable,
}

impl UpdateChannel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
        }
    }
}

impl fmt::Display for UpdateChannel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for UpdateChannel {
    type Err = UpdateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "stable" => Ok(Self::Stable),
            other => Err(UpdateError::InvalidManifest(format!(
                "unsupported update channel `{other}`"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateAssetKind {
    MacosAppZip,
    WindowsInno,
    LinuxManagedTar,
}

impl UpdateAssetKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MacosAppZip => "macos_app_zip",
            Self::WindowsInno => "windows_inno",
            Self::LinuxManagedTar => "linux_managed_tar",
        }
    }
}

impl fmt::Display for UpdateAssetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for UpdateAssetKind {
    type Err = UpdateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "macos_app_zip" => Ok(Self::MacosAppZip),
            "windows_inno" => Ok(Self::WindowsInno),
            "linux_managed_tar" => Ok(Self::LinuxManagedTar),
            other => Err(UpdateError::InvalidManifest(format!(
                "unsupported update asset kind `{other}`"
            ))),
        }
    }
}

impl UpdateManifest {
    pub fn validate_for(
        &self,
        app_id: &str,
        channel: UpdateChannel,
        current_version: &semver::Version,
        platform: PlatformAssetKey,
    ) -> Result<(semver::Version, UpdateAsset), UpdateError> {
        if self.schema_version != UPDATE_MANIFEST_SCHEMA_VERSION {
            return Err(UpdateError::InvalidManifest(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        if self.app_id != app_id {
            return Err(UpdateError::InvalidManifest(format!(
                "manifest app_id `{}` does not match `{app_id}`",
                self.app_id
            )));
        }
        if self.channel != channel {
            return Err(UpdateError::NoUpdateAvailable);
        }

        let version = semver::Version::parse(&self.version)?;
        if version <= *current_version {
            return Err(UpdateError::NoUpdateAvailable);
        }
        if let Some(min_supported) = &self.min_supported_version {
            let min_supported = semver::Version::parse(min_supported)?;
            if min_supported > *current_version {
                return Err(UpdateError::InvalidManifest(format!(
                    "current version {current_version} is older than minimum supported {min_supported}"
                )));
            }
        }

        let asset = self
            .assets
            .get(platform.as_str())
            .ok_or_else(|| {
                UpdateError::InvalidManifest(format!(
                    "manifest has no asset for platform `{}`",
                    platform.as_str()
                ))
            })?
            .clone();
        validate_asset(&asset, platform)?;

        Ok((version, asset))
    }
}

pub fn validate_asset(asset: &UpdateAsset, platform: PlatformAssetKey) -> Result<(), UpdateError> {
    if asset.target_triple != platform.target_triple() {
        return Err(UpdateError::InvalidManifest(format!(
            "asset target `{}` does not match `{}`",
            asset.target_triple,
            platform.target_triple()
        )));
    }
    if asset.kind != platform.asset_kind() {
        return Err(UpdateError::InvalidManifest(format!(
            "asset kind `{}` does not match `{}`",
            asset.kind,
            platform.asset_kind()
        )));
    }
    if !asset.url.starts_with("https://") {
        return Err(UpdateError::InvalidManifest(format!(
            "asset URL is not HTTPS: {}",
            asset.url
        )));
    }
    if !is_safe_file_name(&asset.file_name) {
        return Err(UpdateError::InvalidManifest(format!(
            "asset file name is not safe: {}",
            asset.file_name
        )));
    }
    if !is_sha256_hex(&asset.sha256) {
        return Err(UpdateError::InvalidManifest(format!(
            "asset sha256 is not 64-character lowercase hex: {}",
            asset.sha256
        )));
    }
    if asset.size_bytes == 0 {
        return Err(UpdateError::InvalidManifest(format!(
            "asset `{}` has zero size",
            asset.file_name
        )));
    }

    Ok(())
}

fn is_safe_file_name(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.contains('/')
        && !value.contains('\\')
        && value != "."
        && value != ".."
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset_for(platform: PlatformAssetKey) -> UpdateAsset {
        UpdateAsset {
            target_triple: platform.target_triple().to_string(),
            kind: platform.asset_kind(),
            file_name: "Frame-test.tar.gz".to_string(),
            url: "https://example.com/Frame-test.tar.gz".to_string(),
            size_bytes: 10,
            sha256: "a".repeat(64),
            installer_args: Vec::new(),
        }
    }

    #[test]
    fn validate_for_returns_asset_for_newer_matching_manifest() {
        let platform = PlatformAssetKey::current().expect("test platform should be supported");
        let mut assets = BTreeMap::new();
        assets.insert(platform.as_str().to_string(), asset_for(platform));
        let manifest = UpdateManifest {
            schema_version: UPDATE_MANIFEST_SCHEMA_VERSION,
            app_id: FRAME_APP_ID.to_string(),
            channel: UpdateChannel::Stable,
            version: "0.2.0".to_string(),
            published_at: None,
            min_supported_version: Some("0.1.0".to_string()),
            release_notes_url: None,
            release_notes_markdown: None,
            assets,
        };

        let (_, asset) = manifest
            .validate_for(
                FRAME_APP_ID,
                UpdateChannel::Stable,
                &semver::Version::parse("0.1.0").expect("version"),
                platform,
            )
            .expect("manifest should validate");

        assert_eq!(asset.target_triple, platform.target_triple());
    }

    #[test]
    fn validate_for_rejects_downgrade() {
        let platform = PlatformAssetKey::current().expect("test platform should be supported");
        let mut assets = BTreeMap::new();
        assets.insert(platform.as_str().to_string(), asset_for(platform));
        let manifest = UpdateManifest {
            schema_version: UPDATE_MANIFEST_SCHEMA_VERSION,
            app_id: FRAME_APP_ID.to_string(),
            channel: UpdateChannel::Stable,
            version: "0.1.0".to_string(),
            published_at: None,
            min_supported_version: None,
            release_notes_url: None,
            release_notes_markdown: None,
            assets,
        };

        let result = manifest.validate_for(
            FRAME_APP_ID,
            UpdateChannel::Stable,
            &semver::Version::parse("0.1.0").expect("version"),
            platform,
        );

        assert!(matches!(result, Err(UpdateError::NoUpdateAvailable)));
    }
}
