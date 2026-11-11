use super::{output, spawn};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::process::{Command, Output, Stdio};
use windows::Win32::System::Console::GetConsoleWindow;
use windows::Win32::System::Threading::CREATE_NEW_CONSOLE;

const CHILD_TEST: &str = "FRAME_WINDOWS_LAUNCH_TEST_CHILD";
const CHILD_EXIT_CODE: i32 = 23;

fn child_command(test_name: &str) -> Command {
    // The libtest executable is a console application, just like FFmpeg and the
    // updater helper. Select one test so the child never recursively runs this suite.
    let mut command = Command::new(std::env::current_exe().unwrap());
    // libtest names omit the crate prefix included by module_path!().
    let (_, filter) = test_name.split_once("::").unwrap();
    command
        .args(["--exact", filter, "--nocapture"])
        .env(CHILD_TEST, test_name);
    command
}

fn run_child_probe(test_name: &str, expect_console_window: bool) {
    if std::env::var(CHILD_TEST).as_deref() != Ok(test_name) {
        return;
    }
    // A windowless console can still have a valid code page. Check its window.
    // SAFETY: this function only inspects the current process's console state.
    let has_console_window = !unsafe { GetConsoleWindow() }.0.is_null();
    assert_eq!(has_console_window, expect_console_window);
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    println!("frame-child-stdout:{input}");
    eprintln!("frame-child-stderr");
    std::io::stdout().flush().unwrap();
    std::io::stderr().flush().unwrap();
    std::process::exit(CHILD_EXIT_CODE);
}

fn assert_child_output(output: &Output, expected_input: &str) {
    assert_eq!(
        output.status.code(),
        Some(CHILD_EXIT_CODE),
        "child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("frame-child-stdout:{expected_input}\n"))
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("frame-child-stderr\n"));
}

#[test]
fn spawn_preserves_pipes_and_exit_status_without_console_window() {
    let name = concat!(
        module_path!(),
        "::spawn_preserves_pipes_and_exit_status_without_console_window"
    );
    run_child_probe(name, false);
    // The launcher must override this request, regardless of the runner's console.
    let mut child = spawn(
        child_command(name)
            .creation_flags(CREATE_NEW_CONSOLE.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
    .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"frame-input")
        .unwrap();
    assert_child_output(&child.wait_with_output().unwrap(), "frame-input");
}

#[test]
fn output_captures_streams_and_exit_status_without_console_window() {
    let name = concat!(
        module_path!(),
        "::output_captures_streams_and_exit_status_without_console_window"
    );
    run_child_probe(name, false);
    let mut command = child_command(name);
    command.creation_flags(CREATE_NEW_CONSOLE.0);
    assert_child_output(&output(&mut command).unwrap(), "");
}

#[test]
fn ordinary_spawn_can_create_a_console_window() {
    let name = concat!(
        module_path!(),
        "::ordinary_spawn_can_create_a_console_window"
    );
    run_child_probe(name, true);
    // Create a fresh console so the control does not depend on the test runner's
    // console. Bypass our launcher, which intentionally replaces this flag.
    let mut child = child_command(name)
        .creation_flags(CREATE_NEW_CONSOLE.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"frame-input")
        .unwrap();
    assert_child_output(&child.wait_with_output().unwrap(), "frame-input");
}
