use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use super::*;

static PROCESS_TESTS: Mutex<()> = Mutex::new(());

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-bounded-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) fn spec(mode: &str) -> CommandSpec {
    let mut spec = CommandSpec::new(std::env::current_exe().expect("test executable"));
    spec.args = [
        "--exact",
        "bounded_process::tests::fake_process",
        "--ignored",
        "--nocapture",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    spec.env.push(("FRAME_BOUNDED_FAKE".into(), mode.into()));
    spec
}

fn limits() -> Limits {
    Limits {
        timeout: Duration::from_secs(5),
        cleanup_timeout: Duration::from_secs(2),
        stdout_bytes: 2048,
        stderr_bytes: 1024,
    }
}

pub(super) fn serial() -> std::sync::MutexGuard<'static, ()> {
    PROCESS_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A native test executable works on Windows too, without Python or shell quoting.
#[test]
#[ignore = "subprocess fixture, invoked only with FRAME_BOUNDED_FAKE"]
fn fake_process() {
    let Ok(mode) = std::env::var("FRAME_BOUNDED_FAKE") else {
        return;
    };
    if let Some(path) = std::env::var_os("FRAME_BOUNDED_MARKER") {
        std::fs::write(path, std::process::id().to_string()).expect("fixture marker");
    }
    match mode.as_str() {
        "success" | "slow" => {
            let mut input = Vec::new();
            if mode == "slow" {
                let mut bytes = [0; 1023];
                loop {
                    let count = std::io::stdin().read(&mut bytes).unwrap();
                    if count == 0 {
                        break;
                    }
                    input.extend_from_slice(&bytes[..count]);
                    std::thread::sleep(Duration::from_millis(1));
                }
            } else {
                std::io::stdin().read_to_end(&mut input).unwrap();
            }
            assert!(input.iter().all(|byte| *byte == 37));
            eprintln!("received={}", input.len());
            for chunk in ["fra", "me=3\npro", "gress=end\n"] {
                print!("{chunk}");
                std::io::stdout().flush().unwrap();
            }
        }
        "failure" => {
            eprintln!("encoder initialization failed");
            std::process::exit(7);
        }
        "empty" => {}
        #[cfg(windows)]
        "argv" => {
            use std::os::windows::ffi::OsStrExt;
            let args: Vec<_> = std::env::args_os().collect();
            let marker = args.iter().position(|arg| arg == "--").unwrap();
            let encoded: Vec<Vec<u16>> = args[marker + 1..]
                .iter()
                .map(|arg| arg.encode_wide().collect())
                .collect();
            std::fs::write(
                std::env::var_os("FRAME_ARGV_OUTPUT").unwrap(),
                serde_json::to_vec(&encoded).unwrap(),
            )
            .unwrap();
        }
        "no-read" => std::thread::sleep(Duration::from_secs(30)),
        "flood" | "endless-flood" => {
            let bytes = vec![b'x'; 8192];
            for _ in 0..if mode == "flood" { 256 } else { usize::MAX } {
                std::io::stdout().write_all(&bytes).unwrap();
                std::io::stderr().write_all(&bytes).unwrap();
            }
            println!("\nframe=1\nprogress=end");
        }
        "descendant" => {
            let child = spec("no-read");
            let mut command = Command::new(child.program);
            command
                .args(child.args)
                .envs(child.env)
                .stdin(Stdio::null());
            #[expect(
                clippy::zombie_processes,
                reason = "fixture deliberately exits without waiting; the owned group/job must clean its descendant"
            )]
            let descendant = crate::process_launch::spawn(&mut command).unwrap();
            eprintln!("descendant={}", descendant.id());
        }
        _ => panic!("unknown fixture mode"),
    }
    std::process::exit(0);
}

#[test]
fn progress_requires_an_explicit_complete_final_record() {
    let cases: &[(&[u8], Option<u64>, bool)] = &[
        (b"frame=0\nprogress=end\n", Some(0), true),
        (b"frame=1\nprogress=end\n", Some(1), true),
        (b"progress=end\n", None, true),
        (b"frame=1\nprogress=continue\nprogress=end\n", None, true),
        (b"frame=1\nprogress=end", None, false),
        (b"frame=1\nprogress=end\nframe=2", None, false),
        (b"frame=1\nframe=NaN\nprogress=end\n", None, true),
        (
            b"frame=1\nframe=18446744073709551616\nprogress=end\n",
            None,
            true,
        ),
        (b"frame=1\nframe=\xff\nprogress=end\n", None, true),
        (b"frame=1\n  frame=\xff\nprogress=end\n", None, true),
        (b"frame=1\nprogress=end\n  progress=\xff\n", None, false),
        (b"frame=1\nprogress=banana\nprogress=end\n", None, true),
        (b"frame=1\nprogress=end\nframe=2\n", None, false),
        (
            b"frame=1\nprogress=end\nframe=2\nprogress=continue\n",
            Some(2),
            false,
        ),
        (
            b"frame=1\nprogress=continue\nframe=2\nprogress=end\n",
            Some(2),
            true,
        ),
    ];
    for &(input, frames, ended) in cases {
        let mut parser = ProgressParser::default();
        for chunk in input.chunks(3) {
            parser.feed(chunk);
        }
        parser.eof();
        assert_eq!(parser.proof, ProgressProof { frames, ended }, "{input:?}");
    }
    let mut parser = ProgressParser::default();
    parser.feed(b"frame=1\nframe=");
    parser.feed(&vec![b'9'; 4096]);
    parser.feed(b"\nprogress=end\n");
    parser.eof();
    assert_eq!(
        parser.proof,
        ProgressProof {
            frames: None,
            ended: true
        }
    );
    assert!(parser.pending.is_empty());
    for key in [b"  frame=".as_slice(), b"  progress=".as_slice()] {
        let mut parser = ProgressParser::default();
        parser.feed(b"frame=1\nprogress=end\n");
        parser.feed(key);
        parser.feed(&vec![b'9'; 4096]);
        parser.feed(b"\nprogress=end\n");
        parser.eof();
        assert_eq!(
            parser.proof,
            ProgressProof {
                frames: None,
                ended: true
            }
        );
    }
}

#[test]
fn native_process_preserves_input_exit_status_and_progress_after_capture_limit() {
    let _serial = serial();
    let input = vec![37; 128 * 128 * 3 / 2];
    for mode in ["success", "slow"] {
        let result = run(&spec(mode), &input, limits(), || false).expect(mode);
        assert!(result.success);
        assert_eq!(result.code, Some(0));
        assert_eq!(result.stdin_written, input.len());
        assert_eq!(result.stdin_error, None);
        assert_eq!(
            result.progress,
            ProgressProof {
                frames: Some(3),
                ended: true
            }
        );
        assert!(String::from_utf8_lossy(&result.stderr.bytes).contains("received=24576"));
    }
    let flood = run(&spec("flood"), &[], limits(), || false).unwrap();
    assert!(flood.success);
    assert_eq!(flood.stdout.bytes.len(), 2048);
    assert_eq!(flood.stderr.bytes.len(), 1024);
    assert!(flood.stdout.truncated && flood.stderr.truncated);
    assert_eq!(
        flood.progress,
        ProgressProof {
            frames: Some(1),
            ended: true
        }
    );
    let failure = run(&spec("failure"), &input, limits(), || false).unwrap();
    assert!(!failure.success);
    assert_eq!(failure.code, Some(7));
    assert!(String::from_utf8_lossy(&failure.stderr.bytes).contains("initialization failed"));
    let empty = run(&spec("empty"), &input, limits(), || false).unwrap();
    assert!(empty.success);
    assert_eq!(
        empty.progress.frames, None,
        "exit zero and a successful write do not prove encoding"
    );
}

#[test]
fn deadlines_and_cancellation_own_children_without_killing_a_sibling() {
    let _serial = serial();
    let sibling_spec = spec("no-read");
    let mut command = Command::new(sibling_spec.program);
    command
        .args(sibling_spec.args)
        .envs(sibling_spec.env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut sibling = crate::process_launch::spawn(&mut command).unwrap();
    let results = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for mode in ["no-read", "endless-flood", "descendant"] {
            let started = Instant::now();
            let result = run(
                &spec(mode),
                &vec![37; 256 * 1024],
                Limits {
                    timeout: Duration::from_millis(400),
                    ..limits()
                },
                || false,
            )
            .unwrap_err();
            assert_eq!(result.reason, StopReason::TimedOut, "{mode}: {result}");
            assert_eq!(result.cleanup, CleanupOutcome::Complete, "{mode}: {result}");
            assert!(result.stdout.bytes.len() <= limits().stdout_bytes);
            assert!(result.stderr.bytes.len() <= limits().stderr_bytes);
            if mode == "endless-flood" {
                assert!(result.stdout.truncated && result.stderr.truncated);
                assert!(!result.stdout.bytes.is_empty() && !result.stderr.bytes.is_empty());
            }
            assert!(started.elapsed() < Duration::from_secs(4));
            assert!(
                sibling.try_wait().unwrap().is_none(),
                "unrelated sibling must remain alive"
            );
        }
        let started = Instant::now();
        let cancelled = run(&spec("no-read"), &vec![37; 256 * 1024], limits(), || {
            started.elapsed() > Duration::from_millis(150)
        })
        .unwrap_err();
        assert_eq!(cancelled.reason, StopReason::Cancelled);
        assert_eq!(cancelled.cleanup, CleanupOutcome::Complete);
        assert!(sibling.try_wait().unwrap().is_none());
    }));
    sibling.kill().unwrap();
    sibling.wait().unwrap();
    if let Err(panic) = results {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn cancellation_and_permit_deadline_happen_before_spawn() {
    let _serial = serial();
    let directory = TestDirectory::new();
    let marker = directory.0.join("started");
    let mut command = spec("empty");
    command
        .env
        .push(("FRAME_BOUNDED_MARKER".into(), marker.as_os_str().to_owned()));
    assert_eq!(
        run(&command, &[], limits(), || true).unwrap_err().reason,
        StopReason::Cancelled
    );
    assert!(!marker.exists());
    let permit = acquire_permit(Instant::now() + Duration::from_secs(1), &|| false).unwrap();
    let result = run(
        &command,
        &[],
        Limits {
            timeout: Duration::from_millis(80),
            ..limits()
        },
        || false,
    )
    .unwrap_err();
    assert_eq!(result.reason, StopReason::TimedOut);
    assert!(!marker.exists());
    drop(permit);
    assert!(
        run(
            &command,
            &[],
            Limits {
                cleanup_timeout: Duration::MAX,
                ..limits()
            },
            || false
        )
        .is_err()
    );
    assert!(!marker.exists());
}

#[test]
fn unwind_retains_child_and_permit_until_supervised_cleanup_finishes() {
    let _serial = serial();
    let directory = TestDirectory::new();
    let marker = directory.0.join("started");
    let mut command = spec("no-read");
    command
        .env
        .push(("FRAME_BOUNDED_MARKER".into(), marker.as_os_str().to_owned()));
    let panic = std::panic::catch_unwind(|| {
        run(&command, &vec![37; 256 * 1024], limits(), || {
            assert!(!marker.exists(), "injected cancellation callback panic");
            false
        })
    });
    assert!(panic.is_err());
    let deadline = Instant::now() + Duration::from_secs(3);
    while (ACTIVE.load(Ordering::Acquire) || CLEANUP_PENDING.load(Ordering::Acquire))
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !ACTIVE.load(Ordering::Acquire),
        "supervisor must reap the child and release its permit"
    );
    assert!(
        run(&spec("empty"), &[], limits(), || false)
            .unwrap()
            .success
    );
}

#[cfg(windows)]
#[test]
fn launch_gate_wait_obeys_the_same_deadline() {
    let _serial = serial();
    let gate = crate::process_launch::try_launch_gate().unwrap();
    let result = run(
        &spec("empty"),
        &[],
        Limits {
            timeout: Duration::from_millis(80),
            ..limits()
        },
        || false,
    )
    .unwrap_err();
    assert_eq!(result.reason, StopReason::TimedOut);
    assert_eq!(result.cleanup, CleanupOutcome::Complete);
    drop(gate);
}

#[cfg(windows)]
#[test]
fn native_launch_failures_close_all_owned_handles() {
    use ::windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
    let _serial = serial();
    assert!(
        run(&spec("empty"), &[], limits(), || false)
            .unwrap()
            .success
    );
    let count = || {
        let mut count = 0;
        // SAFETY: the pseudo-handle identifies this process; count is writable.
        unsafe {
            GetProcessHandleCount(GetCurrentProcess(), &raw mut count).unwrap();
        }
        count
    };
    let before = count();
    for point in [
        "job",
        "stdin",
        "stdout",
        "stderr",
        "attributes",
        "handle-list",
        "job-list",
        "process",
        "stdout-worker",
        "stderr-worker",
        "stdin-worker",
    ] {
        let mut command = spec("no-read");
        command
            .env
            .push(("FRAME_BOUNDED_FAIL".into(), point.into()));
        let error = run(&command, &vec![37; 256 * 1024], limits(), || false).unwrap_err();
        assert!(
            matches!(error.reason, StopReason::Io(_)),
            "{point}: {error}"
        );
        assert_eq!(error.cleanup, CleanupOutcome::Complete, "{point}: {error}");
    }
    assert!(
        count() <= before + 2,
        "native launch failure must not leak handles"
    );
}

#[cfg(windows)]
#[test]
fn batch_wrapper_matches_std_command_arguments_and_rejects_newlines() {
    let _serial = serial();
    let directory = TestDirectory::new();
    let executable = std::env::current_exe().unwrap();
    let sentinel = directory.0.join("injected.txt");
    let payloads: Vec<OsString> = [
        "",
        "plain",
        "with spaces",
        "tab\tvalue",
        "Zażółć-日本-🙂",
        "trailing\\",
        "a\\\"b",
        "%FRAME_PROBE_LITERAL%",
        "%CD%",
        "!literal!",
        "&|<>()^=;,",
        "a\"\"b",
    ]
    .into_iter()
    .map(OsString::from)
    .chain([OsString::from(format!(
        "& echo injected > {}",
        sentinel.display()
    ))])
    .collect();
    for extension in ["CmD", "BaT"] {
        let script = directory.0.join(format!("probe & 日本.{extension}"));
        let body = format!(
            "@echo off\r\n\"{}\" --exact bounded_process::tests::fake_process --ignored --nocapture -- %*\r\n",
            executable.display()
        );
        std::fs::write(&script, body).unwrap();
        let ordinary_output = directory.0.join(format!("ordinary-{extension}.json"));
        let bounded_output = directory.0.join(format!("bounded-{extension}.json"));
        let environment = [
            (OsString::from("FRAME_BOUNDED_FAKE"), OsString::from("argv")),
            ("FRAME_PROBE_LITERAL".into(), "EXPANDED".into()),
            (
                "COMSPEC".into(),
                directory.0.join("fake-cmd.exe").into_os_string(),
            ),
        ];
        let ordinary = crate::process_launch::output(
            Command::new(&script)
                .args(&payloads)
                .envs(environment.iter().cloned())
                .env("FRAME_ARGV_OUTPUT", &ordinary_output),
        )
        .unwrap();
        assert!(
            ordinary.status.success(),
            "{}",
            String::from_utf8_lossy(&ordinary.stderr)
        );
        let mut command = CommandSpec::new(&script);
        command.args = payloads.clone();
        command.env = environment.to_vec();
        command
            .env
            .push(("FRAME_ARGV_OUTPUT".into(), bounded_output.into_os_string()));
        let result = run(&command, &[], limits(), || false).unwrap();
        assert!(
            result.success,
            "{}",
            String::from_utf8_lossy(&result.stderr.bytes)
        );
        assert_eq!(
            std::fs::read(&ordinary_output).unwrap(),
            std::fs::read(directory.0.join(format!("bounded-{extension}.json"))).unwrap()
        );
        assert!(!sentinel.exists());
        for invalid in ["line\nbreak", "line\rbreak", "nul\0value"] {
            assert!(crate::process_launch::spawn(Command::new(&script).arg(invalid)).is_err());
            command.args = vec![invalid.into()];
            assert!(matches!(
                run(&command, &[], limits(), || false).unwrap_err().reason,
                StopReason::Io(_)
            ));
        }
    }
}
