use super::super::{ExecutionDomain, Limits, StopReason, tests};
use super::handle;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

#[derive(Clone, Copy)]
enum Completion {
    Success,
    Deadline,
    Cancelled,
}

fn open_fixture(marker: &Path) -> OwnedHandle {
    let startup_deadline = Instant::now() + Duration::from_secs(3);
    let pid = loop {
        if let Ok(contents) = std::fs::read_to_string(marker)
            && let Ok(pid) = contents.parse::<u32>()
        {
            break pid;
        }
        assert!(Instant::now() < startup_deadline, "fixture did not start");
        std::thread::sleep(Duration::from_millis(2));
    };
    // SAFETY: this is our fixture PID, which remains alive until explicitly released.
    unsafe {
        let raw = OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
        .unwrap();
        OwnedHandle::from_raw_handle(raw.0)
    }
}

fn hold_running_member(folder: &Path) -> OwnedHandle {
    let leader = open_fixture(&folder.join("leader.pid"));
    let member = open_fixture(&folder.join("member.pid"));
    std::fs::write(folder.join("leader.release"), []).unwrap();
    // SAFETY: both independently owned handles have synchronization/query rights.
    // The leader exits normally; only the live member keeps the job nonempty.
    unsafe {
        assert_eq!(WaitForSingleObject(handle(&leader), 3000), WAIT_OBJECT_0);
        let mut exit_code = 1;
        GetExitCodeProcess(handle(&leader), &raw mut exit_code).unwrap();
        assert_eq!(exit_code, 0);
        assert_eq!(WaitForSingleObject(handle(&member), 0), WAIT_TIMEOUT);
    }
    member
}

fn wait_for_cleanup(domain: &ExecutionDomain) {
    let cleanup_deadline = Instant::now() + Duration::from_secs(3);
    while domain.cleanup_in_progress() && Instant::now() < cleanup_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !domain.cleanup_in_progress(),
        "rundown must release its owner"
    );
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
    let mut command = tests::spec("detached-descendant");
    for (key, file) in [
        ("FRAME_BOUNDED_MARKER", "leader.pid"),
        ("FRAME_BOUNDED_MEMBER_MARKER", "member.pid"),
        ("FRAME_BOUNDED_LEADER_RELEASE", "leader.release"),
    ] {
        command.env.push((key.into(), folder.join(file).into()));
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let limits = Limits {
        timeout: Duration::from_secs(5),
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
    let member = hold_running_member(&folder);
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
        Duration::from_secs(6)
    };
    let early = match before_cancel {
        Some(Ok(result)) => Ok(result),
        Some(Err(mpsc::RecvTimeoutError::Disconnected)) => {
            Err(mpsc::RecvTimeoutError::Disconnected)
        }
        Some(Err(mpsc::RecvTimeoutError::Timeout)) | None => receive.recv_timeout(wait),
    };
    let waited_for_release = matches!(&early, Err(mpsc::RecvTimeoutError::Timeout));
    if matches!(completion, Completion::Success) {
        // SAFETY: this owned handle has terminate rights; release our live fixture.
        unsafe { TerminateProcess(handle(&member), 0).unwrap() };
    }
    let result = early.unwrap_or_else(|_| receive.recv_timeout(Duration::from_secs(3)).unwrap());
    worker.join().unwrap();
    // SAFETY: our handle stays live even after the job terminates the member.
    assert_eq!(
        unsafe { WaitForSingleObject(handle(&member), 3000) },
        WAIT_OBJECT_0,
        "completion must leave no live member"
    );
    wait_for_cleanup(&DOMAIN);
    std::fs::remove_dir_all(folder).unwrap();
    match completion {
        Completion::Success => {
            assert!(waited_for_release, "job members must drain before success");
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
                "stop must return without externally releasing the member"
            );
            let error = result.expect_err("rundown must remain bounded and cancellable");
            let reason = if matches!(completion, Completion::Deadline) {
                StopReason::TimedOut
            } else {
                StopReason::Cancelled
            };
            assert_eq!(error.reason, reason);
        }
    }
    let next = DOMAIN
        .run(&tests::spec("empty"), &[], limits, || false)
        .expect("finished cleanup must admit the next process");
    assert!(next.success);
}

#[test]
fn successful_exit_waits_for_job_members_to_drain() {
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
