use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use serde::{Deserialize, Serialize};

use crate::{
    UpdateAssetKind, UpdateChannel, UpdateError, file_sha256_hex,
    manifest::{FRAME_APP_ID, UPDATE_MANIFEST_SCHEMA_VERSION},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    pub schema_version: u32,
    pub app_id: String,
    pub from_version: String,
    pub to_version: String,
    pub channel: UpdateChannel,
    pub asset_kind: UpdateAssetKind,
    pub package_path: PathBuf,
    pub package_sha256: String,
    pub install_root: PathBuf,
    pub executable_path: PathBuf,
    pub parent_pid: u32,
    pub relaunch: bool,
    #[serde(default)]
    pub installer_args: Vec<String>,
    pub result_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub schema_version: u32,
    pub app_id: String,
    pub from_version: String,
    pub to_version: String,
    pub success: bool,
    pub error: Option<String>,
}

impl InstallResult {
    #[must_use]
    pub fn success(plan: &InstallPlan) -> Self {
        Self {
            schema_version: UPDATE_MANIFEST_SCHEMA_VERSION,
            app_id: plan.app_id.clone(),
            from_version: plan.from_version.clone(),
            to_version: plan.to_version.clone(),
            success: true,
            error: None,
        }
    }

    #[must_use]
    pub fn failure(plan: &InstallPlan, error: &UpdateError) -> Self {
        Self {
            schema_version: UPDATE_MANIFEST_SCHEMA_VERSION,
            app_id: plan.app_id.clone(),
            from_version: plan.from_version.clone(),
            to_version: plan.to_version.clone(),
            success: false,
            error: Some(error.to_string()),
        }
    }
}

pub fn run_install_plan(plan: &InstallPlan) -> Result<(), UpdateError> {
    validate_install_plan(plan)?;
    verify_package_hash(plan)?;

    match plan.asset_kind {
        UpdateAssetKind::MacosAppZip => install_macos_app_zip(plan),
        UpdateAssetKind::WindowsInno => install_windows_inno(plan),
        UpdateAssetKind::LinuxManagedTar => install_linux_managed_tar(plan),
    }?;

    if plan.relaunch {
        relaunch(plan)?;
    }

    Ok(())
}

fn validate_install_plan(plan: &InstallPlan) -> Result<(), UpdateError> {
    if plan.schema_version != UPDATE_MANIFEST_SCHEMA_VERSION {
        return Err(UpdateError::InstallFailed(format!(
            "unsupported install plan schema version {}",
            plan.schema_version
        )));
    }
    if plan.app_id != FRAME_APP_ID {
        return Err(UpdateError::InstallFailed(format!(
            "install plan app_id `{}` does not match `{FRAME_APP_ID}`",
            plan.app_id
        )));
    }

    let from_version = semver::Version::parse(&plan.from_version)?;
    let to_version = semver::Version::parse(&plan.to_version)?;
    if to_version <= from_version {
        return Err(UpdateError::InstallFailed(format!(
            "refusing downgrade or same-version install from {from_version} to {to_version}"
        )));
    }
    if !plan.package_path.is_file() {
        return Err(UpdateError::InstallFailed(format!(
            "update package does not exist: {}",
            plan.package_path.display()
        )));
    }

    Ok(())
}

fn verify_package_hash(plan: &InstallPlan) -> Result<(), UpdateError> {
    let actual = file_sha256_hex(&plan.package_path)?;
    if actual == plan.package_sha256 {
        Ok(())
    } else {
        Err(UpdateError::HashMismatch {
            expected: plan.package_sha256.clone(),
            actual,
        })
    }
}

#[cfg(target_os = "macos")]
fn install_macos_app_zip(plan: &InstallPlan) -> Result<(), UpdateError> {
    if plan
        .install_root
        .extension()
        .is_none_or(|extension| extension != "app")
    {
        return Err(UpdateError::InstallFailed(format!(
            "macOS install root is not an app bundle: {}",
            plan.install_root.display()
        )));
    }

    let parent = plan.install_root.parent().ok_or_else(|| {
        UpdateError::InstallFailed(format!(
            "macOS install root has no parent: {}",
            plan.install_root.display()
        ))
    })?;
    let unpack_dir = parent.join(".frame-update-unpack");
    let update_tmp = parent.join("Frame.app.update-tmp");
    let backup = parent.join("Frame.app.previous");

    remove_path_if_exists(&unpack_dir)?;
    remove_path_if_exists(&update_tmp)?;
    fs::create_dir_all(&unpack_dir)?;
    checked_status(
        Command::new("ditto")
            .arg("-xk")
            .arg(&plan.package_path)
            .arg(&unpack_dir)
            .status(),
        "ditto",
    )?;

    let unpacked_app = unpack_dir.join("Frame.app");
    validate_macos_bundle(&unpacked_app)?;
    fs::rename(&unpacked_app, &update_tmp)?;
    remove_path_if_exists(&unpack_dir)?;
    replace_install_root(&plan.install_root, &update_tmp, &backup)
}

#[cfg(not(target_os = "macos"))]
fn install_macos_app_zip(_plan: &InstallPlan) -> Result<(), UpdateError> {
    Err(UpdateError::InstallFailed(
        "macOS app zip installation is only supported on macOS".to_string(),
    ))
}

#[cfg(target_os = "linux")]
fn install_linux_managed_tar(plan: &InstallPlan) -> Result<(), UpdateError> {
    if plan
        .install_root
        .file_name()
        .is_none_or(|name| name != "frame.app")
    {
        return Err(UpdateError::InstallFailed(format!(
            "Linux install root is not a frame.app layout: {}",
            plan.install_root.display()
        )));
    }

    let parent = plan.install_root.parent().ok_or_else(|| {
        UpdateError::InstallFailed(format!(
            "Linux install root has no parent: {}",
            plan.install_root.display()
        ))
    })?;
    let unpack_dir = parent.join(".frame-update-unpack");
    let update_tmp = parent.join("frame.app.update-tmp");
    let backup = parent.join("frame.app.previous");

    remove_path_if_exists(&unpack_dir)?;
    remove_path_if_exists(&update_tmp)?;
    fs::create_dir_all(&unpack_dir)?;
    checked_status(
        Command::new("tar")
            .arg("-xzf")
            .arg(&plan.package_path)
            .arg("-C")
            .arg(&unpack_dir)
            .status(),
        "tar",
    )?;

    let unpacked_app = unpack_dir.join("frame.app");
    validate_linux_layout(&unpacked_app)?;
    fs::rename(&unpacked_app, &update_tmp)?;
    remove_path_if_exists(&unpack_dir)?;
    replace_install_root(&plan.install_root, &update_tmp, &backup)
}

#[cfg(not(target_os = "linux"))]
fn install_linux_managed_tar(_plan: &InstallPlan) -> Result<(), UpdateError> {
    Err(UpdateError::InstallFailed(
        "Linux managed tar installation is only supported on Linux".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn install_windows_inno(plan: &InstallPlan) -> Result<(), UpdateError> {
    let mut args = if plan.installer_args.is_empty() {
        vec![
            "/SP-".to_string(),
            "/VERYSILENT".to_string(),
            "/SUPPRESSMSGBOXES".to_string(),
            "/NORESTART".to_string(),
        ]
    } else {
        plan.installer_args.clone()
    };
    if let Some(result_dir) = plan.result_path.parent() {
        args.push(format!("/LOG={}", result_dir.join("install.log").display()));
    }

    checked_status(
        Command::new(&plan.package_path).args(args).status(),
        "Inno Setup",
    )
}

#[cfg(not(target_os = "windows"))]
fn install_windows_inno(_plan: &InstallPlan) -> Result<(), UpdateError> {
    Err(UpdateError::InstallFailed(
        "Windows Inno installation is only supported on Windows".to_string(),
    ))
}

fn replace_install_root(
    current: &Path,
    update_tmp: &Path,
    backup: &Path,
) -> Result<(), UpdateError> {
    remove_path_if_exists(backup)?;
    fs::rename(current, backup)?;
    match fs::rename(update_tmp, current) {
        Ok(()) => {
            remove_path_if_exists(backup).ok();
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(backup, current);
            Err(UpdateError::InstallFailed(format!(
                "failed to move update into place: {error}"
            )))
        }
    }
}

fn checked_status(
    status: Result<ExitStatus, std::io::Error>,
    program: &str,
) -> Result<(), UpdateError> {
    let status = status
        .map_err(|error| UpdateError::InstallFailed(format!("failed to run {program}: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(UpdateError::InstallFailed(format!(
            "{program} exited with status {status}"
        )))
    }
}

fn remove_path_if_exists(path: &Path) -> Result<(), UpdateError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path).map_err(Into::into),
        Ok(_) => fs::remove_file(path).map_err(Into::into),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn relaunch(plan: &InstallPlan) -> Result<(), UpdateError> {
    #[cfg(target_os = "macos")]
    {
        checked_status(
            Command::new("open").arg(&plan.install_root).status(),
            "open",
        )
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        Command::new(&plan.executable_path)
            .spawn()
            .map(|_| ())
            .map_err(|error| {
                UpdateError::InstallFailed(format!(
                    "failed to relaunch {}: {error}",
                    plan.executable_path.display()
                ))
            })
    }
}

#[cfg(target_os = "macos")]
fn validate_macos_bundle(path: &Path) -> Result<(), UpdateError> {
    let executable = path.join("Contents").join("MacOS").join("frame");
    if executable.is_file() {
        Ok(())
    } else {
        Err(UpdateError::InstallFailed(format!(
            "updated macOS bundle is missing executable: {}",
            executable.display()
        )))
    }
}

#[cfg(target_os = "linux")]
fn validate_linux_layout(path: &Path) -> Result<(), UpdateError> {
    let executable = path.join("bin").join("frame");
    let helper = path.join("bin").join("frame-update-helper");
    if executable.is_file() && helper.is_file() {
        Ok(())
    } else {
        Err(UpdateError::InstallFailed(format!(
            "updated Linux package is missing Frame executables under {}",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_result_failure_uses_plan_identity() {
        let plan = InstallPlan {
            schema_version: 1,
            app_id: FRAME_APP_ID.to_string(),
            from_version: "0.1.0".to_string(),
            to_version: "0.2.0".to_string(),
            channel: UpdateChannel::Stable,
            asset_kind: UpdateAssetKind::LinuxManagedTar,
            package_path: PathBuf::from("/tmp/package"),
            package_sha256: "a".repeat(64),
            install_root: PathBuf::from("/tmp/frame.app"),
            executable_path: PathBuf::from("/tmp/frame.app/bin/frame"),
            parent_pid: 1,
            relaunch: true,
            installer_args: Vec::new(),
            result_path: PathBuf::from("/tmp/result.json"),
        };
        let error = UpdateError::InstallFailed("broken".to_string());

        let result = InstallResult::failure(&plan, &error);

        assert_eq!(result.to_version, "0.2.0");
    }
}
