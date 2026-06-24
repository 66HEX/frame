use frame_core::error::ConversionError;

#[cfg(unix)]
pub(super) fn pause_process(pid: u32) -> Result<(), ConversionError> {
    signal_process(pid, libc::SIGSTOP, "SIGSTOP")
}

#[cfg(not(unix))]
pub(super) fn pause_process(_pid: u32) -> Result<(), ConversionError> {
    Err(ConversionError::Shell(
        "Pausing conversions is not supported on this platform yet".to_string(),
    ))
}

#[cfg(unix)]
pub(super) fn resume_process(pid: u32) -> Result<(), ConversionError> {
    signal_process(pid, libc::SIGCONT, "SIGCONT")
}

#[cfg(not(unix))]
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

#[cfg(not(unix))]
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
