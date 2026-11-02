use anyhow::{Context as _, Result};
use std::process::Stdio;

/// A wrapper around `smol::process::Child` that ensures all subprocesses
/// are killed when the process is terminated: on Unix by using process
/// groups, and on Windows by using job objects.
///
/// On Windows, dropping this struct closes the job object handle, which
/// terminates all processes in the job. This also applies when the Zed
/// process exits for any reason (including crashes), since the OS closes
/// its handles, so spawned process trees can never outlive Zed.
pub struct Child {
    process: smol::process::Child,
    #[cfg(windows)]
    job: Option<windows_job::JobObject>,
}

impl std::ops::Deref for Child {
    type Target = smol::process::Child;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

impl std::ops::DerefMut for Child {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}

impl Child {
    #[cfg(not(windows))]
    pub fn spawn(
        mut command: std::process::Command,
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio,
    ) -> Result<Self> {
        crate::set_pre_exec_to_start_new_session(&mut command);
        let mut command = smol::process::Command::from(command);
        let process = command
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .with_context(|| {
                format!(
                    "failed to spawn command {}",
                    crate::redact::redact_command(&format!("{command:?}"))
                )
            })?;
        Ok(Self { process })
    }

    #[cfg(windows)]
    pub fn spawn(
        command: std::process::Command,
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio,
    ) -> Result<Self> {
        let mut command = smol::process::Command::from(command);
        let process = command
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .with_context(|| {
                format!(
                    "failed to spawn command {}",
                    crate::redact::redact_command(&format!("{command:?}"))
                )
            })?;

        // Assign the child to a job object configured to kill the entire
        // process tree when the last job handle is closed, so descendants
        // (e.g. node workers and MCP servers spawned by agent servers) are
        // reaped even if the direct child doesn't clean them up. Any process
        // the child spawns after this assignment is automatically part of the
        // job.
        //
        // There is a small race: descendants the child spawns between the
        // `spawn()` call returning and the assignment below escape the job.
        // Closing it fully would require creating the process suspended
        // (`CREATE_SUSPENDED`), assigning it, then resuming it, which the
        // std/smol process APIs don't support without reimplementing process
        // creation. The window is microseconds, and the children we care
        // about (`npx`, `node`, etc.) take far longer to load their runtime
        // and spawn anything, so in practice nothing escapes.
        let job = windows_job::JobObject::new()
            .and_then(|job| {
                job.assign_process(process.id())?;
                Ok(job)
            })
            .map_err(|error| {
                log::error!("failed to assign spawned process to a job object: {error:#}");
            })
            .ok();

        Ok(Self { process, job })
    }

    /// Consumes the child, draining its stdout/stderr and waiting for it to
    /// exit, then returns the collected output.
    pub async fn output(self) -> Result<std::process::Output> {
        // NOTE: Keep `self` alive across this await, do not destructure it to
        // pull `process` out first. On Windows that drops the job object early,
        // which triggers `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and kills the
        // child before `output()` finishes collecting its stdout/stderr.
        Ok(self.process.output().await?)
    }

    #[cfg(not(windows))]
    pub fn kill(&mut self) -> Result<()> {
        let pid = self.process.id();
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
        Ok(())
    }

    #[cfg(windows)]
    pub fn kill(&mut self) -> Result<()> {
        if let Some(job) = &self.job {
            job.terminate()
        } else {
            self.process.kill()?;
            Ok(())
        }
    }
}

#[cfg(windows)]
mod windows_job {
    use crate::ResultExt as _;
    use anyhow::{Context as _, Result};
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE},
        },
    };

    /// A Win32 job object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`:
    /// all processes assigned to the job (and their descendants) are terminated
    /// when the last handle to the job is closed, which happens when this struct
    /// is dropped, or when the OS closes the owning process's handles after it
    /// exits for any reason.
    pub(crate) struct JobObject(HANDLE);

    // SAFETY: Job object handles can be used from any thread.
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        pub(crate) fn new() -> Result<Self> {
            unsafe {
                let job =
                    Self(CreateJobObjectW(None, None).context("failed to create job object")?);
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .context("failed to set job object limits")?;
                Ok(job)
            }
        }

        pub(crate) fn assign_process(&self, pid: u32) -> Result<()> {
            unsafe {
                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                    .context("failed to open process")?;
                let result = AssignProcessToJobObject(self.0, process)
                    .context("failed to assign process to job object");
                CloseHandle(process).log_err();
                result
            }
        }

        pub(crate) fn terminate(&self) -> Result<()> {
            unsafe { TerminateJobObject(self.0, 1).context("failed to terminate job object") }
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0).log_err();
            }
        }
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Spawns a process tree `powershell -> ping` via `Child::spawn` and
    /// returns the `Child` along with the pid of the grandchild (`ping`).
    fn spawn_process_tree(temp_dir: &std::path::Path) -> (Child, u32) {
        let pid_file = temp_dir.join("grandchild_pid");
        // Single quotes don't escape themselves in PowerShell; double them so
        // temp paths containing `'` can't break out of the literal.
        let pid_file_literal = pid_file.display().to_string().replace('\'', "''");
        let mut command = std::process::Command::new("powershell.exe");
        // `-ExecutionPolicy Bypass` keeps locked-down CI images from refusing
        // to run the snippet, and writing the pid with the .NET API (rather
        // than `Set-Content`) pins the encoding to plain ASCII without a BOM
        // on both Windows PowerShell 5.1 and PowerShell 7+.
        command
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"])
            .arg(format!(
                "$ErrorActionPreference='Stop'; \
                 $p = Start-Process -FilePath ping.exe -ArgumentList @('-n','60','127.0.0.1') -PassThru -WindowStyle Hidden; \
                 [System.IO.File]::WriteAllText('{pid_file_literal}', \"$($p.Id)\", [System.Text.Encoding]::ASCII); \
                 Wait-Process -Id $p.Id"
            ));
        // Capture powershell's stderr so a startup failure (missing binary,
        // policy violation, bad path quoting, ...) is visible in the panic
        // below instead of vanishing into the null device.
        let stderr_log = std::fs::File::create(temp_dir.join("powershell-stderr.log"))
            .expect("failed to create powershell stderr log");
        let child = Child::spawn(
            command,
            Stdio::null(),
            Stdio::null(),
            Stdio::from(stderr_log),
        )
        .expect("failed to spawn powershell");

        // PowerShell cold start (JIT + Defender scan) can take several seconds
        // on CI, especially with sibling tests starting their own instances
        // concurrently, so allow ample headroom before declaring the handoff
        // lost.
        let deadline = Instant::now() + Duration::from_secs(20);
        let grandchild_pid = loop {
            if let Some(pid) = read_grandchild_pid(&pid_file) {
                break pid;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for grandchild pid file: {}",
                describe_pid_handoff(temp_dir, &pid_file)
            );
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            process_is_alive(grandchild_pid),
            "grandchild should be alive after spawning"
        );
        (child, grandchild_pid)
    }

    fn process_is_alive(pid: u32) -> bool {
        use windows::Win32::{
            Foundation::{CloseHandle, STILL_ACTIVE},
            System::Threading::{
                GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        };

        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return false;
            };
            let mut exit_code = 0u32;
            let alive = GetExitCodeProcess(handle, &mut exit_code).is_ok()
                && exit_code == STILL_ACTIVE.0 as u32;
            CloseHandle(handle).expect("failed to close process handle");
            alive
        }
    }

    /// Reads the grandchild pid without assuming an encoding: PowerShell
    /// output may carry a BOM or trailing newlines depending on version and
    /// host configuration, so decode lossily and parse the digit run.
    fn read_grandchild_pid(pid_file: &std::path::Path) -> Option<u32> {
        let bytes = std::fs::read(pid_file).ok()?;
        let text = String::from_utf8_lossy(&bytes);
        // Strip a UTF-8/UTF-16 BOM if one was emitted, then accept the pid
        // surrounded by arbitrary whitespace/newlines.
        let text = text.trim().trim_start_matches('\u{feff}').trim();
        text.parse::<u32>().ok()
    }

    /// Builds a diagnostic summary for a pid-handoff timeout: whether the
    /// file appeared (and what it contains), what else is in the temp dir,
    /// and any stderr PowerShell left behind.
    fn describe_pid_handoff(temp_dir: &std::path::Path, pid_file: &std::path::Path) -> String {
        let pid_state = match std::fs::read(pid_file) {
            Ok(bytes) => format!(
                "pid file exists ({} bytes, contents: {:?})",
                bytes.len(),
                String::from_utf8_lossy(&bytes)
            ),
            Err(error) => format!("pid file missing ({error})"),
        };
        let dir_state = match std::fs::read_dir(temp_dir) {
            Ok(entries) => {
                let names: Vec<String> = entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    })
                    .collect();
                format!("temp dir entries: {names:?}")
            }
            Err(error) => format!("temp dir unreadable ({error})"),
        };
        let stderr_state = match std::fs::read_to_string(temp_dir.join("powershell-stderr.log")) {
            Ok(log) if log.trim().is_empty() => "powershell stderr: <empty>".to_string(),
            Ok(log) => format!("powershell stderr: {log:?}"),
            Err(error) => format!("powershell stderr log unreadable ({error})"),
        };
        format!(
            "{pid_state}; {dir_state}; {stderr_state}; pid file: {}",
            pid_file.display()
        )
    }

    fn assert_process_exits(pid: u32, message: &str) {
        // Job-object termination is normally near-instant, but allow headroom
        // for a loaded CI worker to reap the whole tree.
        let deadline = Instant::now() + Duration::from_secs(10);
        while process_is_alive(pid) {
            assert!(Instant::now() < deadline, "{message} (pid {pid})");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn test_kill_terminates_grandchildren() {
        let temp_dir = tempfile::tempdir().unwrap();
        let (mut child, grandchild_pid) = spawn_process_tree(temp_dir.path());

        child.kill().expect("failed to kill child");

        assert_process_exits(
            grandchild_pid,
            "grandchild should be terminated after killing the child",
        );
    }

    #[test]
    fn test_drop_terminates_grandchildren() {
        let temp_dir = tempfile::tempdir().unwrap();
        let (child, grandchild_pid) = spawn_process_tree(temp_dir.path());

        drop(child);

        assert_process_exits(
            grandchild_pid,
            "grandchild should be terminated after dropping the child",
        );
    }
}
