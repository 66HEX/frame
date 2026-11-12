use std::io;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub(super) fn spawn(
    command: &mut Command,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> io::Result<Child> {
    #[cfg(windows)]
    loop {
        check_startup(cancelled, deadline)?;
        if let Some(_guard) = crate::process_launch::try_launch_gate() {
            use std::os::windows::process::CommandExt;
            use windows::Win32::System::Threading::CREATE_NO_WINDOW;
            check_startup(cancelled, deadline)?;
            command.creation_flags(CREATE_NO_WINDOW.0);
            return command.spawn();
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    #[cfg(not(windows))]
    {
        check_startup(cancelled, deadline)?;
        crate::process_launch::spawn(command)
    }
}

fn check_startup(cancelled: &AtomicBool, deadline: Instant) -> io::Result<()> {
    if cancelled.load(Ordering::Acquire) {
        return Err(io::Error::other("preview startup cancelled"));
    }
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "preview startup timed out; retry the preview",
        ));
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn launch_gate_wait_observes_startup_cancellation() {
        let gate = crate::process_launch::try_launch_gate().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            signal.store(true, Ordering::Release);
        });
        let started = Instant::now();
        let result = spawn(
            &mut Command::new(std::env::current_exe().unwrap()),
            &cancelled,
            started + Duration::from_secs(2),
        );
        drop(gate);
        worker.join().unwrap();
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
