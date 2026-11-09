//! Native launch atomically assigns the child to its noninheritable job.

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows::Win32::Foundation::{HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS, SetHandleInformation};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::JobObjects::{
    CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute,
};
use windows::core::{PCWSTR, PWSTR};

use super::super::{CommandSpec, Duration, Instant, StopReason, stop_reason};
use super::command::{self, wide};

pub(super) struct Launched {
    pub job: OwnedHandle,
    pub process: OwnedHandle,
    pub stdin: File,
    pub stdout: File,
    pub stderr: File,
}

pub(super) fn handle(value: &impl AsRawHandle) -> HANDLE {
    HANDLE(value.as_raw_handle())
}

#[cfg_attr(
    not(test),
    expect(
        clippy::missing_const_for_fn,
        clippy::unnecessary_wraps,
        reason = "Test-only injected failures require runtime checks and a shared Result-returning call site."
    )
)]
pub(super) fn fault(spec: &CommandSpec, point: &str) -> io::Result<()> {
    #[cfg(test)]
    if spec
        .env
        .iter()
        .any(|(key, value)| key == "FRAME_BOUNDED_FAIL" && value == point)
    {
        return Err(io::Error::other(format!(
            "injected process failure: {point}"
        )));
    }
    #[cfg(not(test))]
    let _ = (spec, point);
    Ok(())
}

fn own(value: HANDLE) -> OwnedHandle {
    // SAFETY: every caller transfers a fresh, successful Win32 allocation once.
    unsafe { OwnedHandle::from_raw_handle(value.0) }
}

fn job() -> io::Result<OwnedHandle> {
    // SAFETY: null security/name creates an unnamed noninheritable owned job.
    let job = own(unsafe { CreateJobObjectW(None, PCWSTR::null()) }?);
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: info has exactly the layout/size required by this information class.
    unsafe {
        SetInformationJobObject(
            handle(&job),
            JobObjectExtendedLimitInformation,
            (&raw const info).cast(),
            u32::try_from(size_of_val(&info)).unwrap_or(u32::MAX),
        )?;
    }
    Ok(job)
}

fn pipe(parent_reads: bool) -> io::Result<(File, OwnedHandle)> {
    let mut read = HANDLE::default();
    let mut write = HANDLE::default();
    let security = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };
    // SAFETY: writable handles and initialized attributes live throughout the call.
    unsafe {
        CreatePipe(&raw mut read, &raw mut write, Some(&raw const security), 0)?;
    }
    let (read, write) = (own(read), own(write));
    let (parent, child) = if parent_reads {
        (read, write)
    } else {
        (write, read)
    };
    // SAFETY: parent is a valid owned handle. Only the child end stays inheritable.
    unsafe {
        SetHandleInformation(handle(&parent), HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))?;
    }
    Ok((File::from(parent), child))
}

struct Attributes {
    storage: Vec<usize>,
}

impl Attributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0;
        // SAFETY: first call intentionally only queries required allocation size.
        unsafe {
            let _ = InitializeProcThreadAttributeList(None, 2, None, &raw mut bytes);
        }
        if bytes == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut storage = vec![0; bytes.div_ceil(size_of::<usize>())];
        let pointer = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        // SAFETY: allocation has pointer alignment and the requested byte capacity.
        unsafe {
            InitializeProcThreadAttributeList(Some(pointer), 2, None, &raw mut bytes)?;
        }
        Ok(Self { storage })
    }

    const fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.storage.as_mut_ptr().cast())
    }

    fn set_handles(&mut self, attribute: u32, handles: &[HANDLE]) -> io::Result<()> {
        // SAFETY: caller keeps these arrays alive until after attribute destruction.
        unsafe {
            UpdateProcThreadAttribute(
                self.pointer(),
                0,
                attribute as usize,
                Some(handles.as_ptr().cast()),
                size_of_val(handles),
                None,
                None,
            )?;
        }
        Ok(())
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: the list was initialized and its backing allocation is still live.
        unsafe {
            DeleteProcThreadAttributeList(self.pointer());
        }
    }
}

fn quote(value: &[u16], output: &mut Vec<u16>) {
    output.push(u16::from(b'"'));
    let mut slashes = 0;
    for &unit in value {
        if unit == u16::from(b'\\') {
            slashes += 1;
            continue;
        }
        let count = if unit == u16::from(b'"') {
            slashes * 2 + 1
        } else {
            slashes
        };
        output.extend(std::iter::repeat_n(u16::from(b'\\'), count));
        output.push(unit);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2));
    output.push(u16::from(b'"'));
}

fn environment(spec: &CommandSpec) -> io::Result<Vec<u16>> {
    let mut values = std::env::vars_os().collect::<Vec<_>>();
    for (key, value) in &spec.env {
        let key_units = wide(key)?;
        if key_units.is_empty()
            || key_units
                .iter()
                .any(|unit| *unit > 0x7f || *unit == u16::from(b'='))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "probe environment keys must be ASCII names",
            ));
        }
        values.retain(|(existing, _)| {
            !existing
                .to_string_lossy()
                .eq_ignore_ascii_case(&key.to_string_lossy())
        });
        values.push((key.clone(), value.clone()));
    }
    values.sort_by_cached_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (key, value) in values {
        block.extend(wide(&key)?);
        block.push(u16::from(b'='));
        block.extend(wide(&value)?);
        block.push(0);
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    Ok(block)
}

pub(super) fn spawn(
    spec: &CommandSpec,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<Launched, StopReason> {
    spawn_inner(spec, deadline, cancelled).map_err(|error| match error {
        LaunchError::Stopped(reason) => reason,
        LaunchError::Io(error) => StopReason::Io(error.to_string()),
    })
}

enum LaunchError {
    Stopped(StopReason),
    Io(io::Error),
}
impl From<io::Error> for LaunchError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<windows::core::Error> for LaunchError {
    fn from(value: windows::core::Error) -> Self {
        Self::Io(value.into())
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Keep native handle ownership, inheritance gate and atomic job launch in auditable acquisition/cleanup order."
)]
fn spawn_inner(
    spec: &CommandSpec,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<Launched, LaunchError> {
    if !spec.program.is_absolute()
        && !spec
            .program
            .as_os_str()
            .as_encoded_bytes()
            .starts_with(br"\??\")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "probe executable must be absolute",
        )
        .into());
    }
    let interpreter = match &spec.interpreter {
        Some(interpreter) => Some(interpreter.clone()),
        None => command::batch_interpreter(&spec.program)?,
    };
    let mut application = wide(interpreter.as_ref().unwrap_or(&spec.program).as_os_str())?;
    let mut command_line = if interpreter.is_some() {
        let script = command::to_user_path(&spec.program)?;
        let args = spec
            .args
            .iter()
            .map(|arg| wide(arg))
            .collect::<io::Result<Vec<_>>>()?;
        super::super::windows_batch::make_bat_command_line(&script, &args, false)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?
    } else {
        let mut line = Vec::new();
        quote(
            &wide(spec.argv0.as_deref().unwrap_or(spec.program.as_os_str()))?,
            &mut line,
        );
        for argument in &spec.args {
            line.push(u16::from(b' '));
            quote(&wide(argument)?, &mut line);
        }
        line
    };
    application.push(0);
    command_line.push(0);
    let environment = environment(spec)?;
    let job = job()?;
    fault(spec, "job")?;
    // The gate lives longer than every inheritable parent copy, including all
    // error paths. Ordinary app launches enter the same gate only for spawn.
    let _gate = loop {
        if let Some(reason) = stop_reason(deadline, cancelled) {
            return Err(LaunchError::Stopped(reason));
        }
        if let Some(guard) = crate::process_launch::try_launch_gate() {
            break guard;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let (stdin, child_stdin) = pipe(false)?;
    fault(spec, "stdin")?;
    let (stdout, child_stdout) = pipe(true)?;
    fault(spec, "stdout")?;
    let (stderr, child_stderr) = pipe(true)?;
    fault(spec, "stderr")?;
    let inherited = [
        handle(&child_stdin),
        handle(&child_stdout),
        handle(&child_stderr),
    ];
    let jobs = [handle(&job)];
    let mut attributes = Attributes::new()?;
    fault(spec, "attributes")?;
    attributes.set_handles(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &inherited)?;
    fault(spec, "handle-list")?;
    attributes.set_handles(PROC_THREAD_ATTRIBUTE_JOB_LIST, &jobs)?;
    fault(spec, "job-list")?;
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = u32::try_from(size_of_val(&startup)).unwrap_or(u32::MAX);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = inherited[0];
    startup.StartupInfo.hStdOutput = inherited[1];
    startup.StartupInfo.hStdError = inherited[2];
    startup.lpAttributeList = attributes.pointer();
    let mut result = PROCESS_INFORMATION::default();
    if let Some(reason) = stop_reason(deadline, cancelled) {
        return Err(LaunchError::Stopped(reason));
    }
    // SAFETY: buffers, arrays, attributes and the three inheritable handles stay
    // live throughout CreateProcess. JOB_LIST assigns ownership before execution.
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
            Some(environment.as_ptr().cast()),
            PCWSTR::null(),
            &raw const startup.StartupInfo,
            &raw mut result,
        )?;
    }
    let process = own(result.hProcess);
    drop(own(result.hThread));
    drop(attributes);
    drop((child_stdin, child_stdout, child_stderr));
    Ok(Launched {
        job,
        process,
        stdin,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_preserve_quotes_and_trailing_backslashes() {
        for (input, expected) in [
            ("", "\"\""),
            ("a b", "\"a b\""),
            ("a\\", "\"a\\\\\""),
            ("a\"b", "\"a\\\"b\""),
        ] {
            let mut quoted = Vec::new();
            quote(&input.encode_utf16().collect::<Vec<_>>(), &mut quoted);
            assert_eq!(String::from_utf16(&quoted).unwrap(), expected);
        }
    }
}
