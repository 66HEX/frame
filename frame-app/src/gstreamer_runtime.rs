//! Controlled GStreamer runtime discovery for the native preview backend.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::Deserialize;
use thiserror::Error;

const MANIFEST_ENV: &str = "FRAME_GSTREAMER_MANIFEST";
const MACOS_FRAMEWORK_ROOT: &str = "/Library/Frameworks/GStreamer.framework/Versions/1.0";
const MACOS_BUNDLED_FRAMEWORK_RELATIVE: &[&str] = &[
    "..",
    "Frameworks",
    "GStreamer.framework",
    "Versions",
    "Current",
];
const WINDOWS_BUNDLED_RUNTIME_DIR: &str = "gstreamer";
const LINUX_BUNDLED_RUNTIME_DIR: &str = "gstreamer";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstreamerRuntimeInfo {
    pub platform: String,
    pub plugin_dir: PathBuf,
    pub scanner_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum GstreamerRuntimeError {
    #[error("failed to read GStreamer manifest: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse GStreamer manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GstreamerRuntimeManifest {
    platform: String,
    mode: Option<String>,
    root: PathBuf,
    bin_dir: PathBuf,
    lib_dir: PathBuf,
    plugin_dir: PathBuf,
    scanner_path: PathBuf,
}

impl Default for GstreamerRuntimeManifest {
    fn default() -> Self {
        Self {
            platform: String::new(),
            mode: None,
            root: PathBuf::new(),
            bin_dir: PathBuf::new(),
            lib_dir: PathBuf::new(),
            plugin_dir: PathBuf::new(),
            scanner_path: PathBuf::new(),
        }
    }
}

pub fn configure_gstreamer_runtime() -> Result<Option<GstreamerRuntimeInfo>, GstreamerRuntimeError>
{
    let Some(manifest) = load_process_manifest()? else {
        return Ok(None);
    };

    validate_manifest(&manifest)?;
    let registry_path = registry_cache_path();
    if let Some(parent) = registry_path.parent() {
        fs::create_dir_all(parent)?;
    }
    set_runtime_environment(&manifest, Some(&registry_path));

    Ok(Some(GstreamerRuntimeInfo {
        platform: manifest.platform,
        plugin_dir: manifest.plugin_dir,
        scanner_path: manifest.scanner_path,
    }))
}

fn load_process_manifest() -> Result<Option<GstreamerRuntimeManifest>, GstreamerRuntimeError> {
    if let Some(manifest) = bundled_macos_framework_manifest() {
        return Ok(Some(manifest));
    }
    if let Some(manifest) = bundled_windows_manifest() {
        return Ok(Some(manifest));
    }
    if let Some(manifest) = bundled_linux_manifest() {
        return Ok(Some(manifest));
    }

    if let Ok(path) = env::var(MANIFEST_ENV) {
        return load_manifest_from_path(Path::new(&path));
    }
    if let Some(path) = dev_manifest_path() {
        return load_manifest_from_path(&path);
    }
    if cfg!(target_os = "macos") && Path::new(MACOS_FRAMEWORK_ROOT).exists() {
        return Ok(Some(macos_framework_manifest()));
    }

    Ok(None)
}

fn load_manifest_from_path(
    manifest_path: &Path,
) -> Result<Option<GstreamerRuntimeManifest>, GstreamerRuntimeError> {
    if !manifest_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(manifest_path)?;
    let mut manifest: GstreamerRuntimeManifest = serde_json::from_str(&content)?;
    relocate_resource_manifest(manifest_path, &mut manifest);
    Ok(Some(manifest))
}

fn relocate_resource_manifest(manifest_path: &Path, manifest: &mut GstreamerRuntimeManifest) {
    if manifest.platform != "linux" || manifest.mode.as_deref() != Some("bundled-resource") {
        return;
    }

    let Some(runtime_root) = manifest_path.parent() else {
        return;
    };
    let Some(lib_triplet) = linux_lib_triplet() else {
        return;
    };

    let runtime_root = runtime_root.to_path_buf();
    let lib_dir = runtime_root.join("lib").join(lib_triplet);
    manifest.root = runtime_root.clone();
    manifest.bin_dir = runtime_root.join("bin");
    manifest.lib_dir = lib_dir.clone();
    manifest.plugin_dir = lib_dir.join("gstreamer-1.0");
    manifest.scanner_path = runtime_root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner");
}

fn dev_manifest_path() -> Option<PathBuf> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("gstreamer")
        .join("manifest.json");
    manifest_path.exists().then_some(manifest_path)
}

fn bundled_macos_framework_manifest() -> Option<GstreamerRuntimeManifest> {
    if !cfg!(target_os = "macos") {
        return None;
    }

    let executable_dir = env::current_exe().ok()?.parent()?.to_path_buf();
    let root = MACOS_BUNDLED_FRAMEWORK_RELATIVE
        .iter()
        .fold(executable_dir, |path, component| path.join(component));
    root.is_dir()
        .then(|| framework_manifest(root, Some("bundled-app")))
}

fn bundled_windows_manifest() -> Option<GstreamerRuntimeManifest> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    let executable_dir = env::current_exe().ok()?.parent()?.to_path_buf();
    let runtime_root = executable_dir.join(WINDOWS_BUNDLED_RUNTIME_DIR);
    let plugin_dir = runtime_root.join("lib").join("gstreamer-1.0");
    let scanner_path = runtime_root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner.exe");
    let loader_probe = executable_dir.join("gstreamer-1.0-0.dll");

    (runtime_root.is_dir()
        && plugin_dir.is_dir()
        && scanner_path.is_file()
        && loader_probe.is_file())
    .then_some(GstreamerRuntimeManifest {
        platform: "windows".to_string(),
        mode: Some("bundled-app".to_string()),
        root: runtime_root.clone(),
        bin_dir: executable_dir,
        lib_dir: runtime_root.join("lib"),
        plugin_dir,
        scanner_path,
    })
}

fn bundled_linux_manifest() -> Option<GstreamerRuntimeManifest> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let executable_dir = env::current_exe().ok()?.parent()?.to_path_buf();
    let runtime_root = executable_dir.join(LINUX_BUNDLED_RUNTIME_DIR);
    let lib_triplet = linux_lib_triplet()?;
    let lib_dir = runtime_root.join("lib").join(lib_triplet);
    let plugin_dir = lib_dir.join("gstreamer-1.0");
    let scanner_path = runtime_root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner");
    let loader_probe = lib_dir.join("libgstreamer-1.0.so.0");

    (runtime_root.is_dir()
        && lib_dir.is_dir()
        && plugin_dir.is_dir()
        && scanner_path.is_file()
        && loader_probe.is_file())
    .then_some(GstreamerRuntimeManifest {
        platform: "linux".to_string(),
        mode: Some("bundled-app".to_string()),
        root: runtime_root.clone(),
        bin_dir: runtime_root.join("bin"),
        lib_dir,
        plugin_dir,
        scanner_path,
    })
}

fn linux_lib_triplet() -> Option<&'static str> {
    match env::consts::ARCH {
        "x86_64" => Some("x86_64-linux-gnu"),
        "aarch64" => Some("aarch64-linux-gnu"),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn macos_framework_manifest() -> GstreamerRuntimeManifest {
    framework_manifest(PathBuf::from(MACOS_FRAMEWORK_ROOT), Some("dev-detected"))
}

#[cfg(not(target_os = "macos"))]
fn macos_framework_manifest() -> GstreamerRuntimeManifest {
    unreachable!("macOS framework detection is only available on macOS")
}

fn framework_manifest(root: PathBuf, mode: Option<&str>) -> GstreamerRuntimeManifest {
    GstreamerRuntimeManifest {
        platform: "macos".to_string(),
        mode: mode.map(ToOwned::to_owned),
        bin_dir: root.join("bin"),
        lib_dir: root.join("lib"),
        plugin_dir: root.join("lib").join("gstreamer-1.0"),
        scanner_path: root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner"),
        root,
    }
}

fn validate_manifest(manifest: &GstreamerRuntimeManifest) -> Result<(), GstreamerRuntimeError> {
    if !manifest.plugin_dir.is_dir() {
        return Err(GstreamerRuntimeError::Invalid(format!(
            "Bundled GStreamer plugin directory not found: {}",
            manifest.plugin_dir.display()
        )));
    }
    if !manifest.scanner_path.is_file() {
        return Err(GstreamerRuntimeError::Invalid(format!(
            "Bundled GStreamer plugin scanner not found: {}",
            manifest.scanner_path.display()
        )));
    }
    Ok(())
}

fn registry_cache_path() -> PathBuf {
    ProjectDirs::from("", "", "Frame")
        .map(|dirs| dirs.cache_dir().join("gstreamer-registry.bin"))
        .unwrap_or_else(|| env::temp_dir().join("frame-gstreamer-registry.bin"))
}

fn set_runtime_environment(manifest: &GstreamerRuntimeManifest, registry_path: Option<&Path>) {
    #[cfg(target_os = "windows")]
    prepend_runtime_path("PATH", &manifest.bin_dir);
    #[cfg(target_os = "linux")]
    prepend_runtime_path("LD_LIBRARY_PATH", &manifest.lib_dir);

    set_env_var("GST_PLUGIN_SYSTEM_PATH_1_0", "");
    set_env_var("GST_PLUGIN_PATH_1_0", &manifest.plugin_dir);
    set_env_var("GST_PLUGIN_SCANNER_1_0", &manifest.scanner_path);
    if let Some(path) = registry_path {
        set_env_var("GST_REGISTRY", path);
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn prepend_runtime_path(variable: &str, path: &Path) {
    let current_path = env::var_os(variable).unwrap_or_default();
    let mut paths = env::split_paths(&current_path).collect::<Vec<_>>();
    if paths.iter().any(|existing| existing == path) {
        return;
    }

    let mut updated = Vec::with_capacity(paths.len() + 1);
    updated.push(path.to_path_buf());
    updated.append(&mut paths);

    if let Ok(joined) = env::join_paths(updated) {
        set_env_var(variable, joined);
    }
}

fn set_env_var(key: &str, value: impl AsRef<std::ffi::OsStr>) {
    // SAFETY: This module is called from app startup before preview sessions or
    // GStreamer threads exist. The process environment is configured once so
    // GStreamer cannot fall back to system plugin discovery.
    unsafe {
        env::set_var(key, value);
    }
}
