use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use directories::ProjectDirs;
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    InstallPlan, PlatformAssetKey, UpdateAsset, UpdateAssetKind, UpdateChannel, UpdateError,
    UpdateManifest, file_sha256_hex, verify_manifest_signature,
};

const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/66HEX/frame/releases/latest/download/update-manifest.json";
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const HELPER_FILE_STEM: &str = "frame-update-helper";

#[derive(Clone, Debug)]
pub struct UpdateClientConfig {
    pub app_id: String,
    pub current_version: Version,
    pub channel: UpdateChannel,
    pub manifest_url: String,
    pub public_keys: Vec<String>,
    pub cache_dir: PathBuf,
    pub install_context: InstallContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallContext {
    pub install_root: PathBuf,
    pub executable_path: PathBuf,
    pub helper_path: PathBuf,
    pub relaunch: bool,
}

#[derive(Clone, Debug)]
pub struct UpdateClient {
    config: UpdateClientConfig,
    http: Client,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateCheck {
    UpToDate,
    Available(Box<UpdateInfo>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateInfo {
    pub version: Version,
    pub channel: UpdateChannel,
    pub asset_key: PlatformAssetKey,
    pub asset: UpdateAsset,
    pub release_notes_url: Option<String>,
    pub release_notes_markdown: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdatePackage {
    pub version: Version,
    pub channel: UpdateChannel,
    pub asset_key: PlatformAssetKey,
    pub kind: UpdateAssetKind,
    pub file_name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
    pub installer_args: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl DownloadProgress {
    #[must_use]
    pub fn percent(self) -> Option<u8> {
        let total = self.total_bytes?;
        if total == 0 {
            return None;
        }
        let percent = self.received_bytes.saturating_mul(100) / total;
        Some(percent.min(100) as u8)
    }
}

impl UpdateClient {
    pub fn new(config: UpdateClientConfig) -> Result<Self, UpdateError> {
        let http = Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent(format!("Frame/{}", config.current_version))
            .build()
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        Ok(Self { config, http })
    }

    #[must_use]
    pub const fn config(&self) -> &UpdateClientConfig {
        &self.config
    }

    pub fn check(&self) -> Result<UpdateCheck, UpdateError> {
        let platform = PlatformAssetKey::current()?;
        let manifest_bytes = self.fetch_bytes(&self.config.manifest_url)?;
        let signature = self.fetch_text(&manifest_signature_url(&self.config.manifest_url))?;
        verify_manifest_signature(&manifest_bytes, &signature, &self.config.public_keys)?;

        let manifest: UpdateManifest = serde_json::from_slice(&manifest_bytes)?;
        let check = match manifest.validate_for(
            &self.config.app_id,
            self.config.channel,
            &self.config.current_version,
            platform,
        ) {
            Ok((version, asset)) => UpdateCheck::Available(Box::new(UpdateInfo {
                version,
                channel: manifest.channel,
                asset_key: platform,
                asset,
                release_notes_url: manifest.release_notes_url,
                release_notes_markdown: manifest.release_notes_markdown,
            })),
            Err(UpdateError::NoUpdateAvailable) => UpdateCheck::UpToDate,
            Err(error) => return Err(error),
        };

        self.cache_manifest(&manifest_bytes, &signature)?;
        Ok(check)
    }

    pub fn download(
        &self,
        update: &UpdateInfo,
        mut on_progress: impl FnMut(DownloadProgress),
    ) -> Result<UpdatePackage, UpdateError> {
        let version_dir = self.version_cache_dir(&update.version);
        fs::create_dir_all(&version_dir)?;
        let final_path = version_dir.join(&update.asset.file_name);
        if final_path.is_file() && file_sha256_hex(&final_path)? == update.asset.sha256 {
            return Ok(update_package(update, final_path));
        }

        let tmp_dir = self.config.cache_dir.join("tmp");
        fs::create_dir_all(&tmp_dir)?;
        let part_path = tmp_dir.join(format!(
            "{}.{}.part",
            update.asset.file_name,
            std::process::id()
        ));

        let mut response = self
            .http
            .get(&update.asset.url)
            .send()
            .map_err(|error| UpdateError::Network(error.to_string()))?
            .error_for_status()
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        let total_bytes = response.content_length().or(Some(update.asset.size_bytes));
        let mut file = File::create(&part_path)?;
        let mut received_bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];

        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| UpdateError::Network(error.to_string()))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            received_bytes = received_bytes.saturating_add(read as u64);
            on_progress(DownloadProgress {
                received_bytes,
                total_bytes,
            });
        }
        file.sync_all()?;
        drop(file);

        replace_file(&part_path, &final_path)?;
        let actual_hash = file_sha256_hex(&final_path)?;
        if actual_hash != update.asset.sha256 {
            fs::remove_file(&final_path).ok();
            return Err(UpdateError::HashMismatch {
                expected: update.asset.sha256.clone(),
                actual: actual_hash,
            });
        }
        let actual_size = fs::metadata(&final_path)?.len();
        if actual_size != update.asset.size_bytes {
            return Err(UpdateError::InvalidManifest(format!(
                "asset size mismatch: expected {}, got {actual_size}",
                update.asset.size_bytes
            )));
        }

        on_progress(DownloadProgress {
            received_bytes: actual_size,
            total_bytes: Some(actual_size),
        });
        Ok(update_package(update, final_path))
    }

    pub fn write_install_plan(&self, package: &UpdatePackage) -> Result<PathBuf, UpdateError> {
        let version_dir = self.version_cache_dir(&package.version);
        fs::create_dir_all(&version_dir)?;
        let plan_path = version_dir.join("install-plan.json");
        let result_path = version_dir.join("install-result.json");
        let plan = InstallPlan {
            schema_version: 1,
            app_id: self.config.app_id.clone(),
            from_version: self.config.current_version.to_string(),
            to_version: package.version.to_string(),
            channel: package.channel,
            asset_kind: package.kind,
            package_path: package.path.clone(),
            package_sha256: package.sha256.clone(),
            install_root: self.config.install_context.install_root.clone(),
            executable_path: self.config.install_context.executable_path.clone(),
            parent_pid: std::process::id(),
            relaunch: self.config.install_context.relaunch,
            installer_args: package.installer_args.clone(),
            result_path,
        };
        let json = serde_json::to_vec_pretty(&plan)?;
        let temp_path = plan_path.with_extension("json.tmp");
        fs::write(&temp_path, json)?;
        replace_file(&temp_path, &plan_path)?;
        Ok(plan_path)
    }

    pub fn spawn_helper(&self, plan_path: &Path) -> Result<(), UpdateError> {
        let helper_path = staged_helper_path(
            &self.config.install_context.helper_path,
            &self.config.cache_dir,
        )?;
        Command::new(&helper_path)
            .arg("--plan")
            .arg(plan_path)
            .spawn()
            .map_err(|error| {
                UpdateError::HelperSpawnFailed(format!("{}: {error}", helper_path.display()))
            })?;

        Ok(())
    }

    pub fn prepare_install(&self, package: &UpdatePackage) -> Result<PathBuf, UpdateError> {
        let plan_path = self.write_install_plan(package)?;
        staged_helper_path(
            &self.config.install_context.helper_path,
            &self.config.cache_dir,
        )?;
        Ok(plan_path)
    }

    fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        if !url.starts_with("https://") {
            return Err(UpdateError::InvalidManifest(format!(
                "manifest URL is not HTTPS: {url}"
            )));
        }
        let response = self
            .http
            .get(url)
            .send()
            .map_err(|error| UpdateError::Network(error.to_string()))?
            .error_for_status()
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| UpdateError::Network(error.to_string()))
    }

    fn fetch_text(&self, url: &str) -> Result<String, UpdateError> {
        let bytes = self.fetch_bytes(url)?;
        String::from_utf8(bytes).map_err(|error| {
            UpdateError::InvalidManifest(format!("signature is not UTF-8: {error}"))
        })
    }

    fn version_cache_dir(&self, version: &Version) -> PathBuf {
        self.config
            .cache_dir
            .join("updates")
            .join(version.to_string())
    }

    fn cache_manifest(&self, manifest_bytes: &[u8], signature: &str) -> Result<(), UpdateError> {
        let manifest_dir = self.config.cache_dir.join("updates").join("latest");
        fs::create_dir_all(&manifest_dir)?;
        fs::write(manifest_dir.join("update-manifest.json"), manifest_bytes)?;
        fs::write(manifest_dir.join("update-manifest.json.sig"), signature)?;
        Ok(())
    }
}

pub fn default_manifest_url() -> String {
    std::env::var("FRAME_UPDATE_MANIFEST_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MANIFEST_URL.to_string())
}

pub fn default_cache_dir() -> Result<PathBuf, UpdateError> {
    ProjectDirs::from("", "", "Frame")
        .map(|dirs| dirs.cache_dir().join("updates"))
        .ok_or(UpdateError::ConfigDirectoryUnavailable)
}

pub fn detect_install_context() -> Result<InstallContext, UpdateError> {
    let executable_path = std::env::current_exe()?;
    let install_root = detect_install_root(&executable_path)?;
    let helper_path = helper_path_for_executable(&executable_path)?;
    Ok(InstallContext {
        install_root,
        executable_path,
        helper_path,
        relaunch: true,
    })
}

fn detect_install_root(executable_path: &Path) -> Result<PathBuf, UpdateError> {
    if let Ok(root) = std::env::var("FRAME_UPDATE_INSTALL_ROOT")
        && !root.trim().is_empty()
    {
        return Ok(PathBuf::from(root));
    }

    #[cfg(target_os = "macos")]
    {
        for ancestor in executable_path.ancestors() {
            if ancestor
                .extension()
                .is_some_and(|extension| extension == "app")
            {
                return Ok(ancestor.to_path_buf());
            }
        }
        executable_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                UpdateError::InstallFailed(format!(
                    "current executable has no parent: {}",
                    executable_path.display()
                ))
            })
    }

    #[cfg(target_os = "linux")]
    {
        let Some(bin_dir) = executable_path.parent() else {
            return Err(UpdateError::InstallFailed(format!(
                "current executable has no parent: {}",
                executable_path.display()
            )));
        };
        if bin_dir.file_name().is_some_and(|name| name == "bin")
            && let Some(root) = bin_dir.parent()
            && root.file_name().is_some_and(|name| name == "frame.app")
        {
            return Ok(root.to_path_buf());
        }
        return Ok(bin_dir.to_path_buf());
    }

    #[cfg(target_os = "windows")]
    {
        executable_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                UpdateError::InstallFailed(format!(
                    "current executable has no parent: {}",
                    executable_path.display()
                ))
            })
    }
}

fn helper_path_for_executable(executable_path: &Path) -> Result<PathBuf, UpdateError> {
    if let Ok(path) = std::env::var("FRAME_UPDATE_HELPER")
        && !path.trim().is_empty()
    {
        return Ok(PathBuf::from(path));
    }

    let Some(executable_dir) = executable_path.parent() else {
        return Err(UpdateError::InstallFailed(format!(
            "current executable has no parent: {}",
            executable_path.display()
        )));
    };
    let helper_name = if cfg!(target_os = "windows") {
        format!("{HELPER_FILE_STEM}.exe")
    } else {
        HELPER_FILE_STEM.to_string()
    };
    Ok(executable_dir.join(helper_name))
}

fn staged_helper_path(helper_path: &Path, cache_dir: &Path) -> Result<PathBuf, UpdateError> {
    let helper_dir = cache_dir.join("helper");
    fs::create_dir_all(&helper_dir)?;
    let file_name = helper_path
        .file_name()
        .ok_or_else(|| {
            UpdateError::HelperSpawnFailed(format!(
                "helper path has no file name: {}",
                helper_path.display()
            ))
        })?
        .to_owned();
    let staged = helper_dir.join(file_name);
    fs::copy(helper_path, &staged)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
    }

    Ok(staged)
}

fn update_package(update: &UpdateInfo, path: PathBuf) -> UpdatePackage {
    UpdatePackage {
        version: update.version.clone(),
        channel: update.channel,
        asset_key: update.asset_key,
        kind: update.asset.kind,
        file_name: update.asset.file_name.clone(),
        path,
        size_bytes: update.asset.size_bytes,
        sha256: update.asset.sha256.clone(),
        installer_args: update.asset.installer_args.clone(),
    }
}

fn manifest_signature_url(manifest_url: &str) -> String {
    format!("{manifest_url}.sig")
}

fn replace_file(temp_path: &Path, final_path: &Path) -> Result<(), io::Error> {
    match fs::rename(temp_path, final_path) {
        Ok(()) => Ok(()),
        Err(_) if final_path.exists() => {
            fs::remove_file(final_path)?;
            fs::rename(temp_path, final_path)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_progress_percent_uses_total_when_available() {
        let progress = DownloadProgress {
            received_bytes: 25,
            total_bytes: Some(100),
        };

        assert_eq!(progress.percent(), Some(25));
    }

    #[test]
    fn manifest_signature_url_appends_sig_suffix() {
        assert_eq!(
            manifest_signature_url("https://example.com/update-manifest.json"),
            "https://example.com/update-manifest.json.sig"
        );
    }
}
