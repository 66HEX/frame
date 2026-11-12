use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "frame-runner-probe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cancellation_during_source_probe_is_terminal_and_allows_same_id_retry() {
    let fixture = Fixture::new();
    let executable = fixture.0.join("ffprobe");
    fs::write(&executable, b"#!/bin/sh\nfor source in \"$@\"; do :; done\nprintf started > \"${source}.started\"\nsleep 2\nexit 1\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let source = fixture.0.join("source.mp4");
    let marker = fixture.0.join("source.mp4.started");
    let controller = ConversionProcessController::default();
    let cancellation = controller.clone();
    let cancellation_marker = marker.clone();
    let cancel = thread::spawn(move || {
        let started = Instant::now();
        while !cancellation_marker.exists() && started.elapsed() < Duration::from_secs(1) {
            thread::sleep(Duration::from_millis(5));
        }
        cancellation.cancel_task("probe-task").unwrap();
    });
    let mut events = Vec::new();
    let started = Instant::now();
    let result = probe_conversion_source(
        source.to_str().unwrap(),
        executable.to_str().unwrap(),
        "probe-task",
        &controller,
        &mut |event| {
            if matches!(event, ConversionEvent::Log(_)) {
                controller.cancel_task("probe-task").unwrap();
            }
            events.push(event);
        },
    );
    cancel.join().unwrap();

    assert!(marker.exists(), "the child must start before cancellation");
    assert!(
        matches!(result, Ok(None)),
        "cancelled probe became a worker error: {result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "cancel waited for the silent probe"
    );
    assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
    assert!(!controller.is_cancelled("probe-task"));
    assert_eq!(controller.active_process_count().unwrap(), 0);

    fs::write(&executable, "#!/bin/sh\nprintf '%s\\n' '{\"streams\":[{\"index\":0,\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":16,\"height\":16}],\"format\":{\"duration\":\"1\"}}'\n").unwrap();
    let retry = probe_conversion_source(
        source.to_str().unwrap(),
        executable.to_str().unwrap(),
        "probe-task",
        &controller,
        &mut |_| {},
    )
    .unwrap();
    assert!(retry.is_some());
}
