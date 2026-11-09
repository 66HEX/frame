//! Bounded, owned processes for runtime discovery and short capability probes.
//!
//! A permit limits this helper to one active process or supervised cleanup.
//! Ordinary conversions do not use that permit and remain concurrent.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

#[cfg(all(test, unix))]
pub fn serial_test_processes() -> std::sync::MutexGuard<'static, ()> {
    tests::serial()
}

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(any(windows, test))]
mod windows_batch;

#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub argv0: Option<OsString>,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
    #[cfg(windows)]
    pub interpreter: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            argv0: None,
            args: Vec::new(),
            env: Vec::new(),
            #[cfg(windows)]
            interpreter: None,
        }
    }
}

#[cfg(windows)]
pub fn resolve_windows_program(
    program: &std::ffi::OsStr,
) -> std::io::Result<(PathBuf, Option<PathBuf>)> {
    windows::resolve_program(program)
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub timeout: Duration,
    pub cleanup_timeout: Duration,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            cleanup_timeout: Duration::from_secs(1),
            stdout_bytes: 16 * 1024,
            stderr_bytes: 16 * 1024,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CapturedOutput {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

impl CapturedOutput {
    fn extend(&mut self, bytes: &[u8], limit: usize) {
        let remaining = limit.saturating_sub(self.bytes.len());
        self.bytes
            .extend_from_slice(&bytes[..remaining.min(bytes.len())]);
        self.truncated |= bytes.len() > remaining;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProgressProof {
    pub frames: Option<u64>,
    pub ended: bool,
}

#[derive(Default)]
struct ProgressParser {
    pending: Vec<u8>,
    discard_line: bool,
    proof: ProgressProof,
    record_frames: Option<u64>,
}

impl ProgressParser {
    fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                self.finish_line();
            } else if !self.discard_line {
                if self.pending.len() == 1024 {
                    if self.pending.trim_ascii_start().starts_with(b"progress=") {
                        self.record_frames = None;
                    }
                    self.pending.clear();
                    self.discard_line = true;
                } else {
                    self.pending.push(byte);
                    if self.pending.trim_ascii_start() == b"frame=" {
                        // A malformed, oversized, or unfinished replacement
                        // counter cannot reuse a preceding valid frame count.
                        self.proof = ProgressProof::default();
                        self.record_frames = None;
                    } else if self.pending.trim_ascii_start() == b"progress=" {
                        self.proof = ProgressProof::default();
                    }
                }
            }
        }
    }

    fn finish_line(&mut self) {
        if !self.discard_line {
            let line = self.pending.trim_ascii();
            if let Some(value) = line.strip_prefix(b"frame=") {
                self.proof = ProgressProof::default();
                self.record_frames = std::str::from_utf8(value.trim_ascii())
                    .ok()
                    .and_then(|value| value.parse().ok());
            } else if let Some(value) = line.strip_prefix(b"progress=") {
                if matches!(value, b"end" | b"continue") {
                    self.proof = ProgressProof {
                        frames: self.record_frames.take(),
                        ended: value == b"end",
                    };
                } else {
                    self.proof = ProgressProof::default();
                    self.record_frames = None;
                }
            }
        }
        self.pending.clear();
        self.discard_line = false;
    }

    fn eof(&mut self) {
        // FFmpeg terminates each progress record with a newline. A partial last
        // record is not evidence that encoding completed, even if it says end.
        if !self.pending.is_empty() || self.discard_line {
            self.proof = ProgressProof::default();
            self.record_frames = None;
        }
        self.pending.clear();
        self.discard_line = false;
    }
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdin_written: usize,
    pub stdin_error: Option<String>,
    pub stdout: CapturedOutput,
    pub stderr: CapturedOutput,
    pub progress: ProgressProof,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StopReason {
    TimedOut,
    Cancelled,
    Io(String),
    CleanupInProgress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupOutcome {
    Complete,
    Pending,
}

#[derive(Debug)]
pub struct ProcessError {
    pub reason: StopReason,
    pub cleanup: CleanupOutcome,
    pub stdout: CapturedOutput,
    pub stderr: CapturedOutput,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} (cleanup: {:?})", self.reason, self.cleanup)
    }
}

impl std::error::Error for ProcessError {}

fn early_error(reason: StopReason) -> ProcessError {
    ProcessError {
        cleanup: if reason == StopReason::CleanupInProgress {
            CleanupOutcome::Pending
        } else {
            CleanupOutcome::Complete
        },
        reason,
        stdout: CapturedOutput::default(),
        stderr: CapturedOutput::default(),
    }
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CLEANUP_PENDING: AtomicBool = AtomicBool::new(false);

struct Permit;

impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}

fn stop_reason(deadline: Instant, cancelled: &dyn Fn() -> bool) -> Option<StopReason> {
    if cancelled() {
        Some(StopReason::Cancelled)
    } else if Instant::now() >= deadline {
        Some(StopReason::TimedOut)
    } else {
        None
    }
}

fn acquire_permit(deadline: Instant, cancelled: &dyn Fn() -> bool) -> Result<Permit, ProcessError> {
    loop {
        if let Some(reason) = stop_reason(deadline, cancelled) {
            return Err(early_error(reason));
        }
        if CLEANUP_PENDING.load(Ordering::Acquire) {
            return Err(early_error(StopReason::CleanupInProgress));
        }
        if ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Ok(Permit);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

trait CleanupTask: Send {
    /// Performs only nonblocking cleanup steps; true means all ownership ended.
    fn cleanup_step(&mut self) -> bool;
}

/// Transfers all ownership to the supervisor even when a caller callback panics.
struct CleanupGuard<T: CleanupTask + 'static> {
    inner: Option<T>,
    slot: CleanupSlot,
}

impl<T: CleanupTask + 'static> CleanupGuard<T> {
    fn new(inner: T, slot: &CleanupSlot) -> Self {
        Self {
            inner: Some(inner),
            slot: Arc::clone(slot),
        }
    }

    fn into_inner(mut self) -> T {
        self.inner
            .take()
            .expect("cleanup guard retains ownership until transfer")
    }

    /// Disarms after complete cleanup, without inspecting a reusable Unix group again.
    fn complete(mut self) {
        self.inner.take();
    }
}

impl<T: CleanupTask + 'static> std::ops::Deref for CleanupGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.inner.as_ref().expect("live cleanup owner")
    }
}

impl<T: CleanupTask + 'static> std::ops::DerefMut for CleanupGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.inner.as_mut().expect("live cleanup owner")
    }
}

impl<T: CleanupTask + 'static> Drop for CleanupGuard<T> {
    fn drop(&mut self) {
        if let Some(mut owner) = self.inner.take()
            && !owner.cleanup_step()
        {
            supervise_cleanup(&self.slot, Box::new(owner));
        }
    }
}

type CleanupSlot = Arc<(Mutex<Option<Box<dyn CleanupTask>>>, Condvar)>;

fn cleanup_slot() -> Result<&'static CleanupSlot, ProcessError> {
    static SLOT: OnceLock<Result<CleanupSlot, String>> = OnceLock::new();
    SLOT.get_or_init(|| {
        let slot: CleanupSlot = Arc::new((Mutex::new(None), Condvar::new()));
        let worker = Arc::clone(&slot);
        std::thread::Builder::new()
            .name("frame-probe-cleanup".into())
            .spawn(move || {
                loop {
                    let (lock, wake) = &*worker;
                    let mut pending = lock
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    while pending.is_none() {
                        pending = wake
                            .wait(pending)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                    }
                    let mut task = pending.take().expect("cleanup task is present");
                    drop(pending);
                    while !task.cleanup_step() {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    drop(task);
                    CLEANUP_PENDING.store(false, Ordering::Release);
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(slot)
    })
    .as_ref()
    .map_err(|error| early_error(StopReason::Io(error.clone())))
}

fn supervise_cleanup(slot: &CleanupSlot, task: Box<dyn CleanupTask>) {
    CLEANUP_PENDING.store(true, Ordering::Release);
    let (lock, wake) = &**slot;
    // The single permit remains inside task, so no second task can reach here.
    *lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(task);
    wake.notify_one();
}

pub fn run(
    command: &CommandSpec,
    input: &[u8],
    limits: Limits,
    cancelled: impl Fn() -> bool,
) -> Result<ProcessOutput, ProcessError> {
    let deadline = Instant::now()
        .checked_add(limits.timeout)
        .ok_or_else(|| early_error(StopReason::Io("invalid process deadline".into())))?;
    if Instant::now().checked_add(limits.cleanup_timeout).is_none() {
        return Err(early_error(StopReason::Io(
            "invalid cleanup deadline".into(),
        )));
    }
    let permit = acquire_permit(deadline, &cancelled)?;
    let slot = cleanup_slot()?;
    #[cfg(unix)]
    return unix::run(command, input, limits, deadline, &cancelled, permit, slot);
    #[cfg(windows)]
    return windows::run(command, input, limits, deadline, &cancelled, permit, slot);
    #[cfg(not(any(unix, windows)))]
    Err(early_error(StopReason::Io(
        "bounded processes are unavailable on this platform".into(),
    )))
}
