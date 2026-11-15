use super::super::{CleanupOutcome, ExecutionDomain, Limits, StopReason, tests};
use super::handle;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

#[derive(Clone, Copy)]
enum Completion {
    Success,
    Deadline,
    Cancelled,
}

fn hold_exited_fixture(marker: &std::path::Path) -> OwnedHandle {
    let startup_deadline = Instant::now() + Duration::from_secs(1);
    let pid = loop {
        if let Ok(contents) = std::fs::read_to_string(marker)
            && let Ok(pid) = contents.parse::<u32>()
        {
            break pid;
        }
        assert!(Instant::now() < startup_deadline, "fixture did not start");
        std::thread::sleep(Duration::from_millis(2));
    };
    // SAFETY: this is our fixture PID. Keep an independent process reference
    // after exit so job accounting cannot finish until this handle is dropped.
    let process = unsafe {
        let raw = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid).unwrap();
        OwnedHandle::from_raw_handle(raw.0)
    };
    // SAFETY: the owned process handle has terminate/synchronize rights. Exit 0
    // makes the distinction between process completion and job rundown explicit.
    unsafe {
        TerminateProcess(handle(&process), 0).unwrap();
        assert_eq!(WaitForSingleObject(handle(&process), 1000), WAIT_OBJECT_0);
    }
    process
}

fn check_rundown(completion: Completion) {
    static DOMAIN: ExecutionDomain = ExecutionDomain::new();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let _serial = tests::serial();
    let folder = std::env::temp_dir().join(format!(
        "frame-job-rundown-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&folder).unwrap();
    let marker = folder.join("child.pid");
    let mut command = tests::spec("no-read");
    command
        .env
        .push(("FRAME_BOUNDED_MARKER".into(), marker.as_os_str().to_owned()));
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let limits = Limits {
        timeout: Duration::from_secs(2),
        cleanup_timeout: Duration::from_millis(20),
        stdout_bytes: 1024,
        stderr_bytes: 1024,
    };
    let (send, receive) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        send.send(DOMAIN.run(&command, &[], limits, || {
            worker_cancelled.load(Ordering::Acquire)
        }))
        .unwrap();
    });
    let process = hold_exited_fixture(&marker);
    let before_cancel = if matches!(completion, Completion::Cancelled) {
        let result = receive.recv_timeout(Duration::from_millis(150));
        cancelled.store(true, Ordering::Release);
        Some(result)
    } else {
        None
    };
    let waited_before_cancel = matches!(&before_cancel, Some(Err(mpsc::RecvTimeoutError::Timeout)));
    let wait = if matches!(completion, Completion::Success) {
        Duration::from_millis(150)
    } else {
        Duration::from_secs(3)
    };
    let early = match before_cancel {
        Some(Ok(result)) => Ok(result),
        Some(Err(mpsc::RecvTimeoutError::Disconnected)) => {
            Err(mpsc::RecvTimeoutError::Disconnected)
        }
        Some(Err(mpsc::RecvTimeoutError::Timeout)) | None => receive.recv_timeout(wait),
    };
    let waited_for_release = matches!(&early, Err(mpsc::RecvTimeoutError::Timeout));
    drop(process);
    let result = early.unwrap_or_else(|_| receive.recv_timeout(Duration::from_secs(3)).unwrap());
    worker.join().unwrap();
    let cleanup_deadline = Instant::now() + Duration::from_secs(3);
    while DOMAIN.cleanup_in_progress() && Instant::now() < cleanup_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::remove_dir_all(folder).unwrap();
    assert!(
        !DOMAIN.cleanup_in_progress(),
        "rundown must release its owner"
    );
    match completion {
        Completion::Success => {
            assert!(
                waited_for_release,
                "job references must drain before success"
            );
            let output = result.expect("job rundown must not replace the successful exit");
            assert!(output.success);
            assert_eq!(output.code, Some(0));
        }
        Completion::Deadline | Completion::Cancelled => {
            if matches!(completion, Completion::Cancelled) {
                assert!(
                    waited_before_cancel,
                    "cancel must happen during job rundown"
                );
            }
            assert!(
                !waited_for_release,
                "stop must return while the reference is held"
            );
            let error = result.expect_err("rundown must remain bounded and cancellable");
            let reason = if matches!(completion, Completion::Deadline) {
                StopReason::TimedOut
            } else {
                StopReason::Cancelled
            };
            assert_eq!(error.reason, reason);
            assert_eq!(error.cleanup, CleanupOutcome::Pending);
        }
    }
}

#[test]
fn successful_exit_waits_for_job_references_to_drain() {
    check_rundown(Completion::Success);
}

#[test]
fn job_rundown_still_obeys_the_original_deadline() {
    check_rundown(Completion::Deadline);
}

#[test]
fn cancellation_during_job_rundown_retains_cleanup_ownership() {
    check_rundown(Completion::Cancelled);
}
