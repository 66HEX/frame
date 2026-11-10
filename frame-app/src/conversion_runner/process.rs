use frame_core::error::ConversionError;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE, TerminateProcess,
};
#[cfg(windows)]
use windows::core::s;

#[cfg(unix)]
pub(super) fn pause_process(pid: u32) -> Result<(), ConversionError> {
    signal_process(pid, libc::SIGSTOP, "SIGSTOP")
}

#[cfg(windows)]
pub(super) fn pause_process(pid: u32) -> Result<(), ConversionError> {
    windows_suspend_resume(pid, true)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn pause_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Pausing conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
pub(super) fn resume_process(pid: u32) -> Result<(), ConversionError> {
    signal_process(pid, libc::SIGCONT, "SIGCONT")
}

#[cfg(windows)]
pub(super) fn resume_process(pid: u32) -> Result<(), ConversionError> {
    windows_suspend_resume(pid, false)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn resume_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Resuming conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
pub(super) fn terminate_process(pid: u32) -> Result<(), ConversionError> {
    let unix_pid = pid_to_unix_pid(pid)?;
    unsafe {
        let _ = libc::kill(unix_pid, libc::SIGCONT);
        if libc::kill(unix_pid, libc::SIGKILL) != 0 {
            return Err(ConversionError::Shell("Failed to send SIGKILL".to_string()));
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn terminate_process(pid: u32) -> Result<(), ConversionError> {
    let _ = windows_suspend_resume(pid, false);

    unsafe {
        let process_handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|error| {
            ConversionError::Shell(format!("Failed to open process for termination: {error}"))
        })?;

        let _ = TerminateProcess(process_handle, 1);
        let _ = CloseHandle(process_handle);
    }

    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(super) fn terminate_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Cancelling running conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
fn signal_process(pid: u32, signal: libc::c_int, label: &str) -> Result<(), ConversionError> {
    let unix_pid = pid_to_unix_pid(pid)?;
    unsafe {
        if libc::kill(unix_pid, signal) != 0 {
            return Err(ConversionError::Shell(format!("Failed to send {label}")));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn pid_to_unix_pid(pid: u32) -> Result<libc::pid_t, ConversionError> {
    libc::pid_t::try_from(pid)
        .map_err(|_| ConversionError::Shell(format!("PID {pid} is out of range for libc::pid_t")))
}

#[cfg(windows)]
fn windows_suspend_resume(pid: u32, suspend: bool) -> Result<(), ConversionError> {
    unsafe {
        let process_handle = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)
            .map_err(|error| ConversionError::Shell(format!("Failed to open process: {error}")))?;

        let ntdll = GetModuleHandleA(s!("ntdll.dll")).map_err(|error| {
            let _ = CloseHandle(process_handle);
            ConversionError::Shell(format!("Failed to get ntdll handle: {error}"))
        })?;

        let function_name = if suspend {
            s!("NtSuspendProcess")
        } else {
            s!("NtResumeProcess")
        };

        let Some(function) = GetProcAddress(ntdll, function_name) else {
            let _ = CloseHandle(process_handle);
            return Err(ConversionError::Shell(
                "Could not find NtSuspendProcess/NtResumeProcess in ntdll".to_string(),
            ));
        };

        let function: extern "system" fn(HANDLE) -> i32 = std::mem::transmute(function);
        let status = function(process_handle);
        let _ = CloseHandle(process_handle);

        if status != 0 {
            return Err(ConversionError::Shell(format!(
                "NtSuspendProcess/NtResumeProcess failed with status: {status}"
            )));
        }

        Ok(())
    }
}
