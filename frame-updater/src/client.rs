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
    /// Creates an update client using the supplied runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns [`UpdateError::Network`] if the HTTP client cannot be built.
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

    /// Checks the signed update manifest for the current platform.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform is unsupported, the manifest or
    /// signature cannot be downloaded, the signature does not verify, the
    /// manifest JSON is invalid, or the manifest does not match the current
    /// application identity.
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

    /// Downloads and validates the selected update package.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be prepared, the package
    /// cannot be downloaded or written, or the downloaded asset fails hash or
    /// size validation.
    pub fn download(
        &self,
        update: &UpdateInfo,
        mut on_progress: impl FnMut(DownloadProgress),
    ) -> Result<UpdatePackage, UpdateError> {
        let version_dir = self.version_cache_dir(&update.version);
        fs::create_dir_all(&version_dir)?;
        let final_path = version_dir.join(&update.asset.file_name);
        if final_path.is_file() {
            match validate_package_file(&final_path, &update.asset) {
                Ok(_) => return Ok(update_package(update, final_path)),
                Err(_) => fs::remove_file(&final_path)?,
            }
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
        let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();

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

        let actual_size = match validate_package_file(&part_path, &update.asset) {
            Ok(actual_size) => actual_size,
            Err(error) => {
                fs::remove_file(&part_path).ok();
                return Err(error);
            }
        };
        replace_file(&part_path, &final_path)?;

        on_progress(DownloadProgress {
            received_bytes: actual_size,
            total_bytes: Some(actual_size),
        });
        Ok(update_package(update, final_path))
    }

    /// Writes an install plan for the update helper.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created, the install
    /// plan cannot be serialized, or the plan file cannot be written
    /// atomically.
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

    /// Starts the staged update helper for an install plan.
    ///
    /// # Errors
    ///
    /// Returns an error if the helper cannot be staged or if spawning the
    /// helper process fails.
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

    /// Prepares the install plan and verifies that the helper can be staged.
    ///
    /// # Errors
    ///
    /// Returns an error if the install plan cannot be written or the helper
    /// cannot be staged.
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

#[must_use]
pub fn default_manifest_url() -> String {
    std::env::var("FRAME_UPDATE_MANIFEST_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MANIFEST_URL.to_string())
}

/// Returns the default update cache directory for Frame.
///
/// # Errors
///
/// Returns [`UpdateError::ConfigDirectoryUnavailable`] when the operating
/// system does not expose a suitable user cache directory.
pub fn default_cache_dir() -> Result<PathBuf, UpdateError> {
    ProjectDirs::from("", "", "Frame")
        .map(|dirs| dirs.cache_dir().join("updates"))
        .ok_or(UpdateError::ConfigDirectoryUnavailable)
}

/// Detects the current install root, executable path, and helper path.
///
/// # Errors
///
/// Returns an error if the current executable cannot be resolved, the install
/// root cannot be inferred for the current platform, or the helper path cannot
/// be derived.
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
        Ok(bin_dir.to_path_buf())
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

fn validate_package_file(path: &Path, asset: &UpdateAsset) -> Result<u64, UpdateError> {
    let actual_hash = file_sha256_hex(path)?;
    if actual_hash != asset.sha256 {
        return Err(UpdateError::HashMismatch {
            expected: asset.sha256.clone(),
            actual: actual_hash,
        });
    }

    let actual_size = fs::metadata(path)?.len();
    if actual_size != asset.size_bytes {
        return Err(UpdateError::InvalidManifest(format!(
            "asset size mismatch: expected {}, got {actual_size}",
            asset.size_bytes
        )));
    }

    Ok(actual_size)
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
    use std::{
        net::TcpListener,
        thread::{self, JoinHandle},
    };

    use sha2::{Digest, Sha256};

    use super::*;

    fn serve_package(body: &[u8], request_count: usize) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should have an address");
        let body = body.to_vec();
        let server = thread::spawn(move || {
            for _ in 0..request_count {
                let (mut stream, _) = listener.accept().expect("test request should arrive");
                let mut request = [0_u8; 1024];
                let request_size = stream
                    .read(&mut request)
                    .expect("test request should be readable");
                assert!(request_size > 0, "test request should not be empty");
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("test response headers should be writable");
                stream
                    .write_all(&body)
                    .expect("test response body should be writable");
            }
        });

        (format!("http://{address}/frame-update.zip"), server)
    }

    fn test_client(cache_dir: PathBuf) -> UpdateClient {
        UpdateClient::new(UpdateClientConfig {
            app_id: "Frame".to_string(),
            current_version: Version::new(0, 31, 1),
            channel: UpdateChannel::Stable,
            manifest_url: "https://example.com/update-manifest.json".to_string(),
            public_keys: Vec::new(),
            install_context: InstallContext {
                install_root: cache_dir.join("Frame.app"),
                executable_path: cache_dir.join("Frame.app/frame"),
                helper_path: cache_dir.join("frame-update-helper"),
                relaunch: false,
            },
            cache_dir,
        })
        .expect("test client should be created")
    }

    fn test_update(url: String, body: &[u8], size_bytes: u64) -> UpdateInfo {
        let asset_key = PlatformAssetKey::current().expect("test platform should be supported");
        let sha256 = hex::encode(Sha256::digest(body));

        UpdateInfo {
            version: Version::new(0, 32, 0),
            channel: UpdateChannel::Stable,
            asset_key,
            asset: UpdateAsset {
                target_triple: asset_key.target_triple().to_string(),
                kind: asset_key.asset_kind(),
                file_name: "frame-update.zip".to_string(),
                url,
                size_bytes,
                sha256,
                installer_args: Vec::new(),
            },
            release_notes_url: None,
            release_notes_markdown: None,
        }
    }

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

    #[test]
    fn download_rejects_size_mismatch_on_repeated_attempts() {
        let package = b"update package";
        let (url, server) = serve_package(package, 2);
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let client = test_client(temp_dir.path().to_path_buf());
        let update = test_update(url, package, package.len() as u64 + 1);

        let first = client.download(&update, |_| {});
        assert!(
            matches!(&first, Err(UpdateError::InvalidManifest(message)) if message.contains("asset size mismatch")),
            "first attempt should reject the size mismatch, got {first:?}"
        );

        let second = client.download(&update, |_| {});
        assert!(
            matches!(&second, Err(UpdateError::InvalidManifest(message)) if message.contains("asset size mismatch")),
            "second attempt should reject the size mismatch, got {second:?}"
        );

        server.join().expect("test server should finish");
        let cached_package = client
            .version_cache_dir(&update.version)
            .join(&update.asset.file_name);
        assert!(!cached_package.exists());
    }

    #[test]
    fn download_revalidates_cached_package_size() {
        let package = b"update package";
        let (url, server) = serve_package(package, 1);
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let client = test_client(temp_dir.path().to_path_buf());
        let update = test_update(url, package, package.len() as u64 + 1);
        let cached_package = client
            .version_cache_dir(&update.version)
            .join(&update.asset.file_name);
        fs::create_dir_all(
            cached_package
                .parent()
                .expect("cached package should have a parent"),
        )
        .expect("cache directory should be created");
        fs::write(&cached_package, package).expect("cached package should be written");

        let result = client.download(&update, |_| {});

        assert!(
            matches!(&result, Err(UpdateError::InvalidManifest(message)) if message.contains("asset size mismatch")),
            "cached package should be revalidated, got {result:?}"
        );
        server.join().expect("test server should finish");
        assert!(!cached_package.exists());
    }
}
