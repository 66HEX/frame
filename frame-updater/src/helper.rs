use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use crate::{InstallPlan, InstallResult, UpdateError, run_install_plan};

const PARENT_EXIT_TIMEOUT: Duration = Duration::from_secs(120);
const PARENT_POLL_INTERVAL: Duration = Duration::from_millis(250);

pub fn run_from_env_args() -> Result<(), UpdateError> {
    let plan_path = parse_plan_path(env::args().skip(1))?;
    run_from_plan_path(&plan_path)
}

pub fn run_from_plan_path(plan_path: &Path) -> Result<(), UpdateError> {
    let plan = read_plan(plan_path)?;
    let result = run_helper(&plan);
    write_result(&plan, &result)?;
    result
}

pub fn run_helper(plan: &InstallPlan) -> Result<(), UpdateError> {
    wait_for_process_exit(plan.parent_pid)?;
    run_install_plan(plan)
}

fn read_plan(plan_path: &Path) -> Result<InstallPlan, UpdateError> {
    let bytes = fs::read(plan_path)?;
    serde_json::from_slice(&bytes).map_err(Into::into)
}

fn write_result(plan: &InstallPlan, result: &Result<(), UpdateError>) -> Result<(), UpdateError> {
    if let Some(parent) = plan.result_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let install_result = match result {
        Ok(()) => InstallResult::success(plan),
        Err(error) => InstallResult::failure(plan, error),
    };
    fs::write(
        &plan.result_path,
        serde_json::to_vec_pretty(&install_result)?,
    )?;
    Ok(())
}

fn parse_plan_path(args: impl IntoIterator<Item = String>) -> Result<PathBuf, UpdateError> {
    let mut args = args.into_iter();
    let mut plan_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--plan" => {
                let Some(path) = args.next() else {
                    return Err(UpdateError::InstallFailed(
                        "missing value for --plan".to_string(),
                    ));
                };
                plan_path = Some(PathBuf::from(path));
            }
            "-h" | "--help" => {
                return Err(UpdateError::InstallFailed(
                    "usage: frame-update-helper --plan <install-plan.json>".to_string(),
                ));
            }
            other => {
                return Err(UpdateError::InstallFailed(format!(
                    "unknown update helper argument `{other}`"
                )));
            }
        }
    }

    plan_path.ok_or_else(|| UpdateError::InstallFailed("missing --plan".to_string()))
}

fn wait_for_process_exit(pid: u32) -> Result<(), UpdateError> {
    if pid == 0 {
        return Ok(());
    }

    let start = Instant::now();
    while process_exists(pid) {
        if start.elapsed() > PARENT_EXIT_TIMEOUT {
            return Err(UpdateError::InstallFailed(format!(
                "timed out waiting for parent process {pid} to exit"
            )));
        }
        thread::sleep(PARENT_POLL_INTERVAL);
    }

    Ok(())
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error()
        .raw_os_error()
        .is_some_and(|code| code == libc::EPERM)
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, WAIT_TIMEOUT},
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            WaitForSingleObject,
        },
    };

    let Ok(handle) = (unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            false,
            pid,
        )
    }) else {
        return false;
    };
    if handle.is_invalid() {
        return false;
    }

    let wait_result = unsafe { WaitForSingleObject(handle, 0) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    wait_result == WAIT_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_path_accepts_plan_argument() {
        let path = parse_plan_path(["--plan".to_string(), "/tmp/install-plan.json".to_string()])
            .expect("plan path should parse");

        assert_eq!(path, PathBuf::from("/tmp/install-plan.json"));
    }

    #[test]
    fn wait_for_process_exit_returns_immediately_for_zero_pid() {
        let result = wait_for_process_exit(0);

        assert!(result.is_ok(), "zero pid should be ignored: {result:?}");
    }
}
