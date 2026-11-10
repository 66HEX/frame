//! Coordinates app-owned process launches with Windows' inheritable pipe handles.
//!
//! The gate covers spawning, never waiting for a process. It is cooperative:
//! unrelated native launchers (including shell activation) are outside this gate.
//!
//! Participants: capability listing, metadata probes, conversion, preview,
//! automatic crop, and the updater's post-staging launcher. The separate updater
//! installer has no concurrent probes. GPUI's unused platform restart launcher
//! and shell activation are not covered by this cooperative app contract.

use std::io;
use std::process::{Child, Command, Output, Stdio};

#[cfg(windows)]
static LAUNCH_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(windows)]
pub type LaunchGuard = std::sync::MutexGuard<'static, ()>;

/// Attempts to enter the native inheritable-handle window without blocking.
#[cfg(windows)]
pub fn try_launch_gate() -> Option<LaunchGuard> {
    match LAUNCH_GATE.try_lock() {
        Ok(guard) => Some(guard),
        // The gate has no mutable state; unwinding closes owned handles before
        // releasing it, so a previous panic does not invalidate the lock.
        Err(std::sync::TryLockError::Poisoned(error)) => Some(error.into_inner()),
        Err(std::sync::TryLockError::WouldBlock) => None,
    }
}

pub fn with_launch_gate<T>(launch: impl FnOnce() -> T) -> T {
    #[cfg(windows)]
    let _guard = LAUNCH_GATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    launch()
}

pub fn spawn(command: &mut Command) -> io::Result<Child> {
    with_launch_gate(|| command.spawn())
}

/// Captures the same default streams as `Command::output`, waiting outside the gate.
pub fn output(command: &mut Command) -> io::Result<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    spawn(command)?.wait_with_output()
}

#[cfg(test)]
pub fn status(command: &mut Command) -> io::Result<std::process::ExitStatus> {
    spawn(command)?.wait()
}
