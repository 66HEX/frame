//! Nonblocking pipe I/O keeps cancellation independent of child cooperation.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

use super::{
    CapturedOutput, CleanupGuard, CleanupOutcome, CleanupSlot, CleanupTask, CommandSpec, Duration,
    Instant, Limits, Permit, ProcessError, ProcessOutput, ProgressParser, StopReason, early_error,
    stop_reason, supervise_cleanup,
};

struct OwnedProcess {
    child: Child,
    group: libc::pid_t,
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
    status: Option<ExitStatus>,
    signal_attempted: bool,
    _permit: Permit,
}

impl OwnedProcess {
    fn exit_observed(&self) -> io::Result<bool> {
        // SAFETY: zero is a valid empty siginfo_t. waitid initializes it; WNOWAIT
        // deliberately retains our child PID until group cleanup has signalled.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &raw mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: waitid populated the SIGCHLD variant, or the zeroed no-event value.
        Ok(unsafe { info.si_pid() } != 0)
    }

    fn kill_group(&mut self) {
        if !self.signal_attempted && self.status.is_none() {
            // SAFETY: group is the strictly positive PID of our unreaped child,
            // created with process_group(0). It cannot yet have been recycled.
            let result = unsafe { libc::kill(-self.group, libc::SIGKILL) };
            // macOS returns EPERM for a group containing only our zombie. The
            // attempt precedes reap, but signal success is not required to reap
            // an exited child. Group disappearance still decides cleanup success.
            // Retry EINTR before releasing ownership of the leader's PID.
            self.signal_attempted =
                result == 0 || io::Error::last_os_error().raw_os_error() != Some(libc::EINTR);
        }
    }

    fn group_gone(&self) -> bool {
        // A zero signal observes, but never kills a potentially reused identifier.
        // SAFETY: the negative nonzero ID names only the group originally owned.
        (unsafe { libc::kill(-self.group, 0) }) == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    }

    fn close_pipes(&mut self) {
        self.stdin = None;
        self.stdout = None;
        self.stderr = None;
    }
}

impl CleanupTask for OwnedProcess {
    fn cleanup_step(&mut self) -> bool {
        self.kill_group();
        self.close_pipes();
        if self.status.is_none() && self.signal_attempted && self.exit_observed().unwrap_or(false) {
            // WNOWAIT above proved exit; wait only reaps, with no running child to block on.
            if let Ok(status) = self.child.wait() {
                self.status = Some(status);
            }
        }
        self.status.is_some() && self.group_gone()
    }
}

fn nonblocking(file: &File) -> io::Result<()> {
    // SAFETY: the descriptor is borrowed from a live File throughout both calls.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags == -1
        || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn drain(
    pipe: &mut Option<File>,
    capture: &mut CapturedOutput,
    limit: usize,
    mut progress: Option<&mut ProgressParser>,
) -> io::Result<()> {
    let Some(file) = pipe.as_mut() else {
        return Ok(());
    };
    let mut bytes = [0_u8; 8192];
    // A continuously writing child must not starve the other pipe or the timer.
    for _ in 0..8 {
        match file.read(&mut bytes) {
            Ok(0) => {
                *pipe = None;
                break;
            }
            Ok(count) => {
                capture.extend(&bytes[..count], limit);
                if let Some(parser) = progress.as_deref_mut() {
                    parser.feed(&bytes[..count]);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn stop(
    mut owner: CleanupGuard<OwnedProcess>,
    reason: StopReason,
    limits: Limits,
    slot: &CleanupSlot,
    stdout: CapturedOutput,
    stderr: CapturedOutput,
) -> ProcessError {
    let deadline = Instant::now()
        .checked_add(limits.cleanup_timeout)
        .unwrap_or_else(Instant::now);
    loop {
        if owner.cleanup_step() {
            owner.complete();
            return ProcessError {
                reason,
                cleanup: CleanupOutcome::Complete,
                stdout,
                stderr,
            };
        }
        if Instant::now() >= deadline {
            supervise_cleanup(slot, Box::new(owner.into_inner()));
            return ProcessError {
                reason,
                cleanup: CleanupOutcome::Pending,
                stdout,
                stderr,
            };
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the nonblocking lifecycle loop keeps I/O, deadlines, and ownership transitions together"
)]
pub(super) fn run(
    spec: &CommandSpec,
    input: &[u8],
    limits: Limits,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
    permit: Permit,
    slot: &CleanupSlot,
) -> Result<ProcessOutput, ProcessError> {
    let mut command = Command::new(&spec.program);
    if let Some(argv0) = &spec.argv0 {
        command.arg0(argv0);
    }
    command
        .args(&spec.args)
        .envs(spec.env.iter().cloned())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    if let Some(reason) = stop_reason(deadline, cancelled) {
        return Err(early_error(reason));
    }
    let mut child = crate::process_launch::spawn(&mut command)
        .map_err(|error| early_error(StopReason::Io(error.to_string())))?;
    // Unix PIDs fit pid_t; the child remains unreaped for group ownership.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "POSIX child IDs are positive pid_t values"
    )]
    let group = child.id() as libc::pid_t;
    let stdin = child
        .stdin
        .take()
        .map(|pipe| File::from(OwnedFd::from(pipe)));
    let stdout = child
        .stdout
        .take()
        .map(|pipe| File::from(OwnedFd::from(pipe)));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| File::from(OwnedFd::from(pipe)));
    let mut owner = CleanupGuard::new(
        OwnedProcess {
            child,
            group,
            stdin,
            stdout,
            stderr,
            status: None,
            signal_attempted: false,
            _permit: permit,
        },
        slot,
    );
    let setup = [&owner.stdin, &owner.stdout, &owner.stderr]
        .into_iter()
        .flatten()
        .try_for_each(nonblocking);
    if let Err(error) = setup {
        return Err(stop(
            owner,
            StopReason::Io(error.to_string()),
            limits,
            slot,
            CapturedOutput::default(),
            CapturedOutput::default(),
        ));
    }
    let mut stdout = CapturedOutput::default();
    let mut stderr = CapturedOutput::default();
    let mut progress = ProgressParser::default();
    let mut written = 0;
    let mut stdin_error = None;
    loop {
        if let Some(reason) = stop_reason(deadline, cancelled) {
            return Err(stop(owner, reason, limits, slot, stdout, stderr));
        }
        if written == input.len() {
            owner.stdin = None;
        }
        if let Some(pipe) = owner.stdin.as_mut() {
            match pipe.write(&input[written..input.len().min(written.saturating_add(8192))]) {
                Ok(0) => {
                    stdin_error = Some("child stdin accepted zero bytes".into());
                    owner.stdin = None;
                }
                Ok(count) => written += count,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(error) => {
                    stdin_error = Some(error.to_string());
                    owner.stdin = None;
                }
            }
        }
        let drained = drain(
            &mut owner.stdout,
            &mut stdout,
            limits.stdout_bytes,
            Some(&mut progress),
        )
        .and_then(|()| drain(&mut owner.stderr, &mut stderr, limits.stderr_bytes, None));
        if let Err(error) = drained {
            return Err(stop(
                owner,
                StopReason::Io(error.to_string()),
                limits,
                slot,
                stdout,
                stderr,
            ));
        }
        match owner.exit_observed() {
            Ok(true) if owner.stdout.is_none() && owner.stderr.is_none() => {
                progress.eof();
                if owner.cleanup_step() {
                    let status = owner.status;
                    owner.complete();
                    return Ok(ProcessOutput {
                        success: status.is_some_and(|status| status.success()),
                        code: status.and_then(|status| status.code()),
                        stdin_written: written,
                        stdin_error,
                        stdout,
                        stderr,
                        progress: progress.proof,
                    });
                }
                return Err(stop(
                    owner,
                    StopReason::Io("process descendants have not finished".into()),
                    limits,
                    slot,
                    stdout,
                    stderr,
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(stop(
                    owner,
                    StopReason::Io(error.to_string()),
                    limits,
                    slot,
                    stdout,
                    stderr,
                ));
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_exited_leader_is_reaped_even_when_zombie_group_rejects_signal() {
        let _serial = super::super::tests::serial();
        let spec = super::super::tests::spec("empty");
        let permit =
            super::super::acquire_permit(Instant::now() + Duration::from_secs(2), &|| false)
                .unwrap();
        let slot = super::super::cleanup_slot().unwrap();
        let mut command = Command::new(spec.program);
        command
            .args(spec.args)
            .envs(spec.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        let child = crate::process_launch::spawn(&mut command).unwrap();
        let group = libc::pid_t::try_from(child.id()).unwrap();
        let mut owner = CleanupGuard::new(
            OwnedProcess {
                child,
                group,
                stdin: None,
                stdout: None,
                stderr: None,
                status: None,
                signal_attempted: false,
                _permit: permit,
            },
            slot,
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while !owner.exit_observed().unwrap() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            owner.exit_observed().unwrap(),
            "fixture must already be an unreaped zombie"
        );
        while !owner.cleanup_step() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(owner.status.is_some_and(|status| status.success()));
        assert!(owner.group_gone());
        owner.complete();
    }
}
