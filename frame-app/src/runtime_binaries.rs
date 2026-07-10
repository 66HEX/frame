//! Runtime binary resolution for bundled conversion tools.

use std::{
    env,
    path::{Path, PathBuf},
};

pub const BINARIES_RESOURCE_DIR: &str = "resources/binaries";
pub const BUNDLED_BINARIES_DIR: &str = "binaries";

const FFMPEG_ENV_VAR: &str = "FRAME_FFMPEG_PATH";
const FFPROBE_ENV_VAR: &str = "FRAME_FFPROBE_PATH";
const SYSTEM_MEDIA_TOOLS_ENV_VAR: &str = "FRAME_USE_SYSTEM_MEDIA_TOOLS";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const SETUP_TARGET_TRIPLE: Option<&str> = Some("x86_64-apple-darwin");
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const SETUP_TARGET_TRIPLE: Option<&str> = Some("aarch64-apple-darwin");
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SETUP_TARGET_TRIPLE: Option<&str> = Some("x86_64-unknown-linux-gnu");
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const SETUP_TARGET_TRIPLE: Option<&str> = Some("aarch64-unknown-linux-gnu");
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const SETUP_TARGET_TRIPLE: Option<&str> = Some("x86_64-pc-windows-msvc");
#[cfg(not(any(
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
const SETUP_TARGET_TRIPLE: Option<&str> = None;

#[must_use]
pub fn ffmpeg_executable() -> String {
    resolve_tool_executable(FFMPEG_ENV_VAR, "ffmpeg")
}

#[must_use]
pub fn ffprobe_executable() -> String {
    resolve_tool_executable(FFPROBE_ENV_VAR, "ffprobe")
}

fn resolve_tool_executable(env_var: &str, tool_name: &str) -> String {
    let env_value = env::var(env_var).ok();
    resolve_tool_executable_with_mode(env_value.as_deref(), tool_name, use_system_media_tools())
}

fn resolve_tool_executable_with_mode(
    env_value: Option<&str>,
    tool_name: &str,
    system_media_tools: bool,
) -> String {
    if system_media_tools {
        return resolved_executable(env_value, tool_name, &[]);
    }
    let candidates = runtime_binary_file_name(tool_name)
        .map(|file_name| binary_candidates(&file_name))
        .unwrap_or_default();

    resolved_executable(env_value, tool_name, &candidates)
}

fn use_system_media_tools() -> bool {
    env::var(SYSTEM_MEDIA_TOOLS_ENV_VAR)
        .ok()
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn resolved_executable(env_value: Option<&str>, tool_name: &str, candidates: &[PathBuf]) -> String {
    if let Some(value) = env_value.map(str::trim).filter(|value| !value.is_empty()) {
        return value.to_string();
    }

    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .map_or_else(
            || tool_name.to_string(),
            |candidate| path_to_string(candidate),
        )
}

fn runtime_binary_file_name(tool_name: &str) -> Option<String> {
    let target = target_triple()?;
    Some(format!("{tool_name}-{target}{}", executable_extension()))
}

fn binary_candidates(file_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(
            Path::new(manifest_dir)
                .join(BINARIES_RESOURCE_DIR)
                .join(file_name),
        );
    }

    if let Ok(current_exe) = env::current_exe()
        && let Some(exe_dir) = current_exe.parent()
    {
        candidates.push(exe_dir.join(BINARIES_RESOURCE_DIR).join(file_name));
        candidates.push(exe_dir.join(BUNDLED_BINARIES_DIR).join(file_name));

        #[cfg(target_os = "macos")]
        {
            candidates.push(exe_dir.join("../Resources/binaries").join(file_name));
            candidates.push(
                exe_dir
                    .join("../Resources")
                    .join(BINARIES_RESOURCE_DIR)
                    .join(file_name),
            );
        }
    }

    candidates
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

const fn executable_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    }
}

const fn target_triple() -> Option<&'static str> {
    SETUP_TARGET_TRIPLE
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn runtime_binary_file_name_matches_setup_script_target_name() {
        let target = target_triple().expect("test platform should have a setup-script target");

        assert_eq!(
            runtime_binary_file_name("ffmpeg"),
            Some(format!("ffmpeg-{target}{}", executable_extension()))
        );
    }

    #[test]
    fn resolved_executable_prefers_env_override() {
        let candidates = [PathBuf::from("/does/not/exist/ffmpeg")];

        assert_eq!(
            resolved_executable(Some(" /custom/ffmpeg "), "ffmpeg", &candidates),
            "/custom/ffmpeg"
        );
    }

    #[test]
    fn resolved_executable_prefers_existing_candidate_before_path_fallback() {
        let dir = env::temp_dir().join(format!(
            "frame-gpui-runtime-binaries-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp binary directory should be created");
        let binary_path = dir.join("ffmpeg-test");
        fs::write(&binary_path, b"").expect("temp binary should be written");

        assert_eq!(
            resolved_executable(None, "ffmpeg", std::slice::from_ref(&binary_path)),
            path_to_string(&binary_path)
        );

        fs::remove_dir_all(dir).expect("temp binary directory should be removed");
    }

    #[test]
    fn resolved_executable_falls_back_to_tool_name() {
        assert_eq!(resolved_executable(None, "ffmpeg", &[]), "ffmpeg");
    }

    #[test]
    fn system_media_tools_mode_skips_bundled_candidates() {
        assert_eq!(
            resolve_tool_executable_with_mode(None, "ffmpeg", true),
            "ffmpeg"
        );
        assert_eq!(
            resolve_tool_executable_with_mode(Some(" /custom/ffmpeg "), "ffmpeg", true),
            "/custom/ffmpeg"
        );
    }

    #[test]
    fn binary_candidates_include_macos_bundle_resource_path() {
        let candidates = binary_candidates("ffmpeg-test");

        if cfg!(target_os = "macos") {
            assert!(
                candidates.iter().any(
                    |candidate| candidate.ends_with("Resources/resources/binaries/ffmpeg-test")
                )
            );
        }
    }

    #[test]
    fn binary_candidates_include_executable_sibling_binaries_directory() {
        let candidates = binary_candidates("ffmpeg-test");
        let expected = env::current_exe()
            .expect("test executable path should be available")
            .parent()
            .expect("test executable should have a parent directory")
            .join(BUNDLED_BINARIES_DIR)
            .join("ffmpeg-test");

        assert!(candidates.iter().any(|candidate| candidate == &expected));
    }
}
