//! Owned synchronous I/O workers; cancellation never detaches their resources.

mod command;
mod launch;

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows::Win32::System::IO::CancelSynchronousIo;
use windows::Win32::System::JobObjects::{
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JobObjectBasicAccountingInformation,
    QueryInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};

use super::{
    CapturedOutput, CleanupGuard, CleanupOutcome, CleanupSlot, CleanupTask, CommandSpec, Duration,
    Instant, Limits, Permit, ProcessError, ProcessOutput, ProgressParser, ProgressProof,
    StopReason, early_error, stop_reason, supervise_cleanup,
};
use launch::handle;

pub(super) fn resolve_program(
    program: &std::ffi::OsStr,
) -> io::Result<(std::path::PathBuf, Option<std::path::PathBuf>)> {
    command::resolve(program)
}

struct Worker<T> {
    thread: Option<JoinHandle<T>>,
    result: Option<Result<T, String>>,
}

impl<T> Worker<T> {
    fn poll(&mut self) {
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished)
            && let Some(thread) = self.thread.take()
        {
            self.result = Some(
                thread
                    .join()
                    .map_err(|_| "process I/O worker panicked".into()),
            );
        }
    }

    fn cancel(&self) {
        if let Some(thread) = &self.thread {
            // SAFETY: the owned JoinHandle keeps this actual worker thread handle
            // live. ERROR_NOT_FOUND is normal when no synchronous I/O is pending.
            unsafe {
                let _ = CancelSynchronousIo(HANDLE(thread.as_raw_handle()));
            }
        }
    }

    const fn finished(&self) -> bool {
        self.thread.is_none()
    }
}

#[derive(Default)]
struct WriteResult {
    count: usize,
    error: Option<String>,
}
#[derive(Default)]
struct ReadResult {
    progress: ProgressProof,
    error: Option<String>,
}
type SharedCapture = Arc<Mutex<CapturedOutput>>;

struct OwnedProcess {
    job: OwnedHandle,
    process: Option<OwnedHandle>,
    exit_code: Option<u32>,
    stopping: Arc<AtomicBool>,
    writer: Option<Worker<WriteResult>>,
    stdout_reader: Option<Worker<ReadResult>>,
    stderr_reader: Option<Worker<ReadResult>>,
    stdout: SharedCapture,
    stderr: SharedCapture,
    _permit: Permit,
}

impl OwnedProcess {
    fn poll_process(&mut self) -> io::Result<()> {
        let Some(process) = &self.process else {
            return Ok(());
        };
        // SAFETY: borrowed process handle stays valid; timeout zero never waits.
        match unsafe { WaitForSingleObject(handle(process), 0) } {
            WAIT_OBJECT_0 => {
                let mut code = 0;
                // SAFETY: exited process is still owned and code is writable.
                unsafe {
                    GetExitCodeProcess(handle(process), &raw mut code)?;
                }
                self.exit_code = Some(code);
                // Release references before checking job accounting rundown.
                self.process = None;
            }
            WAIT_FAILED => return Err(io::Error::last_os_error()),
            _ => {}
        }
        Ok(())
    }

    fn job_empty(&self) -> io::Result<bool> {
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: info is the layout and size matching this information class.
        unsafe {
            QueryInformationJobObject(
                Some(handle(&self.job)),
                JobObjectBasicAccountingInformation,
                (&raw mut info).cast(),
                u32::try_from(size_of_val(&info)).unwrap_or(u32::MAX),
                None,
            )?;
        }
        Ok(info.ActiveProcesses == 0)
    }

    fn poll_workers(&mut self) {
        if let Some(worker) = &mut self.writer {
            worker.poll();
        }
        if let Some(worker) = &mut self.stdout_reader {
            worker.poll();
        }
        if let Some(worker) = &mut self.stderr_reader {
            worker.poll();
        }
    }

    fn workers_finished(&self) -> bool {
        self.writer.as_ref().is_none_or(Worker::finished)
            && self.stdout_reader.as_ref().is_none_or(Worker::finished)
            && self.stderr_reader.as_ref().is_none_or(Worker::finished)
    }

    fn captures(&self) -> (CapturedOutput, CapturedOutput) {
        let capture = |value: &SharedCapture| {
            value
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        };
        (capture(&self.stdout), capture(&self.stderr))
    }

    fn cancel_workers(&self) {
        if let Some(worker) = &self.writer {
            worker.cancel();
        }
        if let Some(worker) = &self.stdout_reader {
            worker.cancel();
        }
        if let Some(worker) = &self.stderr_reader {
            worker.cancel();
        }
    }
}

impl CleanupTask for OwnedProcess {
    fn cleanup_step(&mut self) -> bool {
        self.stopping.store(true, Ordering::Release);
        // SAFETY: our noninherited job stays owned until all workers and members
        // finish. Repetition covers the check-to-blocking-syscall cancellation race.
        unsafe {
            let _ = TerminateJobObject(handle(&self.job), 1);
        }
        self.cancel_workers();
        let observed = self.poll_process().is_ok();
        self.poll_workers();
        observed
            && self.process.is_none()
            && self.workers_finished()
            && self.job_empty().unwrap_or(false)
    }
}

fn writer(
    mut pipe: File,
    input: Vec<u8>,
    stopping: Arc<AtomicBool>,
) -> io::Result<Worker<WriteResult>> {
    let thread = std::thread::Builder::new()
        .name("frame-probe-stdin".into())
        .spawn(move || {
            let mut result = WriteResult::default();
            while result.count < input.len() && !stopping.load(Ordering::Acquire) {
                let end = input.len().min(result.count.saturating_add(8192));
                match pipe.write(&input[result.count..end]) {
                    Ok(0) => {
                        result.error = Some("child stdin accepted zero bytes".into());
                        break;
                    }
                    Ok(count) => result.count += count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => {
                        result.error = Some(error.to_string());
                        break;
                    }
                }
            }
            result
        })?;
    Ok(Worker {
        thread: Some(thread),
        result: None,
    })
}

fn reader(
    mut pipe: File,
    capture: SharedCapture,
    limit: usize,
    stopping: Arc<AtomicBool>,
    parse: bool,
) -> io::Result<Worker<ReadResult>> {
    let thread = std::thread::Builder::new()
        .name("frame-probe-drain".into())
        .spawn(move || {
            let mut parser = ProgressParser::default();
            let mut error = None;
            let mut bytes = [0_u8; 8192];
            while !stopping.load(Ordering::Acquire) {
                match pipe.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(count) => {
                        capture
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .extend(&bytes[..count], limit);
                        if parse {
                            parser.feed(&bytes[..count]);
                        }
                    }
                    Err(value) if value.kind() == io::ErrorKind::Interrupted => {}
                    Err(value)
                        if value.raw_os_error() == i32::try_from(ERROR_BROKEN_PIPE.0).ok() =>
                    {
                        break;
                    }
                    Err(value) => {
                        error = Some(value.to_string());
                        break;
                    }
                }
            }
            parser.eof();
            ReadResult {
                progress: parser.proof,
                error,
            }
        })?;
    Ok(Worker {
        thread: Some(thread),
        result: None,
    })
}

fn stop(
    mut owner: CleanupGuard<OwnedProcess>,
    reason: StopReason,
    limits: Limits,
    slot: &CleanupSlot,
) -> ProcessError {
    let deadline = Instant::now()
        .checked_add(limits.cleanup_timeout)
        .unwrap_or_else(Instant::now);
    loop {
        let complete = owner.cleanup_step();
        if complete || Instant::now() >= deadline {
            let (stdout, stderr) = owner.captures();
            if complete {
                owner.complete();
            } else {
                supervise_cleanup(slot, Box::new(owner.into_inner()));
            }
            return ProcessError {
                reason,
                cleanup: if complete {
                    CleanupOutcome::Complete
                } else {
                    CleanupOutcome::Pending
                },
                stdout,
                stderr,
            };
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn take_result<T>(worker: &mut Option<Worker<T>>) -> Result<T, StopReason> {
    worker
        .as_mut()
        .and_then(|worker| worker.result.take())
        .ok_or_else(|| StopReason::Io("missing process I/O completion".into()))?
        .map_err(StopReason::Io)
}

pub(super) fn run(
    spec: &CommandSpec,
    input: &[u8],
    limits: Limits,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
    permit: Permit,
    slot: &CleanupSlot,
) -> Result<ProcessOutput, ProcessError> {
    let launched = launch::spawn(spec, deadline, cancelled).map_err(early_error)?;
    let mut owner = CleanupGuard::new(
        OwnedProcess {
            job: launched.job,
            process: Some(launched.process),
            exit_code: None,
            stopping: Arc::new(AtomicBool::new(false)),
            writer: None,
            stdout_reader: None,
            stderr_reader: None,
            stdout: Arc::new(Mutex::new(CapturedOutput::default())),
            stderr: Arc::new(Mutex::new(CapturedOutput::default())),
            _permit: permit,
        },
        slot,
    );
    let setup = (|| {
        launch::fault(spec, "process")?;
        owner.stdout_reader = Some(reader(
            launched.stdout,
            Arc::clone(&owner.stdout),
            limits.stdout_bytes,
            Arc::clone(&owner.stopping),
            true,
        )?);
        launch::fault(spec, "stdout-worker")?;
        owner.stderr_reader = Some(reader(
            launched.stderr,
            Arc::clone(&owner.stderr),
            limits.stderr_bytes,
            Arc::clone(&owner.stopping),
            false,
        )?);
        launch::fault(spec, "stderr-worker")?;
        owner.writer = Some(writer(
            launched.stdin,
            input.to_vec(),
            Arc::clone(&owner.stopping),
        )?);
        launch::fault(spec, "stdin-worker")?;
        io::Result::Ok(())
    })();
    if let Err(error) = setup {
        return Err(stop(owner, StopReason::Io(error.to_string()), limits, slot));
    }
    loop {
        if let Some(reason) = stop_reason(deadline, cancelled) {
            return Err(stop(owner, reason, limits, slot));
        }
        if let Err(error) = owner.poll_process() {
            return Err(stop(owner, StopReason::Io(error.to_string()), limits, slot));
        }
        owner.poll_workers();
        if owner.process.is_none() && owner.workers_finished() {
            match owner.job_empty() {
                Ok(true) => {}
                Ok(false) => {
                    return Err(stop(
                        owner,
                        StopReason::Io("process descendants have not finished".into()),
                        limits,
                        slot,
                    ));
                }
                Err(error) => {
                    return Err(stop(owner, StopReason::Io(error.to_string()), limits, slot));
                }
            }
            let values = (|| {
                Ok::<_, StopReason>((
                    take_result(&mut owner.writer)?,
                    take_result(&mut owner.stdout_reader)?,
                    take_result(&mut owner.stderr_reader)?,
                ))
            })();
            let (written, read_stdout, read_stderr) = match values {
                Ok(values) => values,
                Err(reason) => return Err(stop(owner, reason, limits, slot)),
            };
            if let Some(error) = read_stdout.error.or(read_stderr.error) {
                return Err(stop(owner, StopReason::Io(error), limits, slot));
            }
            let (stdout, stderr) = owner.captures();
            let exit_code = owner.exit_code;
            owner.complete();
            return Ok(ProcessOutput {
                success: exit_code == Some(0),
                code: exit_code.map(|code| i32::from_ne_bytes(code.to_ne_bytes())),
                stdin_written: written.count,
                stdin_error: written.error,
                stdout,
                stderr,
                progress: read_stdout.progress,
            });
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
