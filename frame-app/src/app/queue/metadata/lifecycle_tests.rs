use super::*;
use crate::bounded_process::{CommandSpec, Limits};
use crate::file_queue::FileItem;
use crate::source_metadata::{MetadataStatus, source_metadata_from_probe};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-metadata-lifecycle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn command(&self, mode: &str) -> CommandSpec {
        let mut command = CommandSpec::new(std::env::current_exe().unwrap());
        command.args = [
            "--exact",
            "app::queue::metadata::lifecycle_tests::metadata_child",
            "--ignored",
            "--nocapture",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        command.env = vec![
            ("FRAME_METADATA_CHILD_MODE".into(), mode.into()),
            (
                "FRAME_METADATA_CHILD_ROOT".into(),
                self.0.clone().into_os_string(),
            ),
        ];
        command
    }

    fn valid_metadata_executable(&self) -> String {
        const JSON: &str = r#"{"streams":[{"index":0,"codec_type":"audio","codec_name":"pcm_s16le","sample_rate":"48000","channels":1}],"format":{"duration":"1"}}"#;
        let path = self.0.join(if cfg!(windows) {
            "metadata.cmd"
        } else {
            "metadata.sh"
        });
        let script = if cfg!(windows) {
            format!("@echo off\r\necho {JSON}\r\n")
        } else {
            format!("#!/bin/sh\nprintf '%s\\n' '{JSON}'\n")
        };
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        path.to_str().unwrap().to_string()
    }

    fn wait_until_running(&self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.0.join("ready").exists() {
            assert!(Instant::now() < deadline, "metadata child did not start");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!self.0.join("released").exists());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "native child fixture"]
fn metadata_child() {
    let Ok(mode) = std::env::var("FRAME_METADATA_CHILD_MODE") else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("FRAME_METADATA_CHILD_ROOT").unwrap());
    std::fs::write(root.join("pid"), std::process::id().to_string()).unwrap();
    std::fs::write(root.join("ready"), b"ready").unwrap();
    if mode == "wait" {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !root.join("release").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::write(root.join("released"), b"released").unwrap();
    }
    std::process::exit(7);
}

fn execute(
    request: &SourceMetadataProbe,
    command: &CommandSpec,
) -> Result<SourceMetadata, ConversionError> {
    request.run_with(|path, cancelled| {
        crate::source_probe::probe_command(
            command,
            Path::new(path),
            Limits {
                timeout: Duration::from_secs(15),
                cleanup_timeout: Duration::from_secs(1),
                stdout_bytes: 4 * 1024 * 1024,
                stderr_bytes: 64 * 1024,
            },
            cancelled,
        )
        .map(source_metadata_from_probe)
    })
}

fn add_request(root: &mut FrameRoot, id: &str) -> SourceMetadataProbe {
    let path = format!("{id}.wav");
    root.file_queue.add_file(FileItem::from_path(id, &path, 1));
    root.begin_source_metadata_probe(id.into(), path).unwrap()
}

fn cancel_running_probe(
    fixture: &Fixture,
    request: &SourceMetadataProbe,
    cancel: impl FnOnce(),
) -> Result<SourceMetadata, ConversionError> {
    let command = fixture.command("wait");
    let worker_request = request.clone();
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        sender.send(execute(&worker_request, &command)).unwrap();
    });
    fixture.wait_until_running();
    cancel();
    let stopped = receiver.recv_timeout(Duration::from_secs(2));
    // Release a broken implementation before asserting, so RED leaves no child behind.
    std::fs::write(fixture.0.join("release"), b"release").unwrap();
    worker.join().unwrap();
    let result = stopped.expect("cancelled import kept its live probe instead of terminating");
    assert!(
        matches!(&result, Err(error) if error.to_string().contains("cancelled")),
        "{result:?}"
    );
    assert!(
        !fixture.0.join("released").exists(),
        "child reached its natural release instead of being terminated"
    );
    result
}

#[test]
fn removing_a_running_import_cancels_its_process_and_frees_the_next_import() {
    let _serial = crate::source_probe::serial_test_probes();
    let fixture = Fixture::new();
    let mut root = FrameRoot::new();
    let request = add_request(&mut root, "old");
    let result = cancel_running_probe(&fixture, &request, || {
        assert!(root.remove_file_from_queue("old"));
    });
    root.complete_source_metadata_probe(&request, result, true, || {
        panic!("removed probe notified")
    });
    assert_eq!(root.source_metadata.status_for("old"), MetadataStatus::Idle);

    let next_fixture = Fixture::new();
    let next = add_request(&mut root, "next");
    let started = Instant::now();
    let next_result = execute(&next, &next_fixture.command("exit"));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "next import waited for the old 15 second deadline"
    );
    assert!(next_fixture.0.join("ready").exists());
    assert!(
        matches!(&next_result, Err(error) if error.to_string().contains("code Some(7)")),
        "{next_result:?}"
    );
    root.complete_source_metadata_probe(&next, next_result, true, || {});
    assert_eq!(
        root.source_metadata.status_for("next"),
        MetadataStatus::Error
    );
    let retry = root
        .begin_source_metadata_probe("next".into(), "next.wav".into())
        .unwrap();
    let executable = next_fixture.valid_metadata_executable();
    let retry_result = retry.run_with(|path, cancelled| {
        crate::source_metadata::probe_source_metadata_with_executable_cancellable(
            path,
            &executable,
            cancelled,
        )
    });
    assert!(retry_result.is_ok(), "{retry_result:?}");
    root.complete_source_metadata_probe(&retry, retry_result, true, || {});
    assert_eq!(
        root.source_metadata.status_for("next"),
        MetadataStatus::Ready
    );
}

#[test]
fn superseded_success_and_error_cannot_change_the_replacement_or_notify() {
    for result in [
        Ok(SourceMetadata::default()),
        Err(ConversionError::Probe("old error".into())),
    ] {
        let mut root = FrameRoot::new();
        let previous = add_request(&mut root, "same");
        let current = root
            .begin_source_metadata_probe("same".into(), "same.wav".into())
            .unwrap();
        let before = root.file_queue.clone();
        root.complete_source_metadata_probe(&previous, result, true, || {
            panic!("superseded request notified")
        });
        assert_eq!(
            root.source_metadata.status_for("same"),
            MetadataStatus::Loading
        );
        assert_eq!(root.file_queue, before);
        let mut notified = 0;
        root.complete_source_metadata_probe(&current, Ok(SourceMetadata::default()), true, || {
            notified += 1;
        });
        root.complete_source_metadata_probe(
            &current,
            Err(ConversionError::Probe("duplicate".into())),
            true,
            || notified += 1,
        );
        assert_eq!(notified, 1);
        assert_eq!(
            root.source_metadata.status_for("same"),
            MetadataStatus::Ready
        );
    }
}

#[test]
fn replacing_a_running_request_cancels_it_without_publishing_its_error() {
    let _serial = crate::source_probe::serial_test_probes();
    let fixture = Fixture::new();
    let mut root = FrameRoot::new();
    let previous = add_request(&mut root, "same");
    let result = cancel_running_probe(&fixture, &previous, || {
        root.begin_source_metadata_probe("same".into(), "same.wav".into())
            .unwrap();
    });
    root.complete_source_metadata_probe(&previous, result, true, || {
        panic!("replaced request notified")
    });
    assert_eq!(
        root.source_metadata.status_for("same"),
        MetadataStatus::Loading
    );
}

#[test]
fn dropping_the_root_cancels_its_live_metadata_process() {
    let _serial = crate::source_probe::serial_test_probes();
    let fixture = Fixture::new();
    let mut root = FrameRoot::new();
    let request = add_request(&mut root, "closing");
    let result = cancel_running_probe(&fixture, &request, || drop(root));
    assert!(result.is_err());
}

#[test]
fn removed_and_cancelled_all_requests_cannot_spawn_or_publish() {
    let _serial = crate::source_probe::serial_test_probes();
    for remove in [true, false] {
        let fixture = Fixture::new();
        let mut root = FrameRoot::new();
        let request = add_request(&mut root, "queued");
        if remove {
            assert!(root.remove_file_from_queue("queued"));
        } else {
            root.metadata_probes.cancel_all();
        }
        let result = execute(&request, &fixture.command("exit"));
        assert!(
            matches!(&result, Err(error) if error.to_string().contains("cancelled")),
            "{result:?}"
        );
        assert!(!fixture.0.join("ready").exists());
        root.complete_source_metadata_probe(&request, result, true, || {
            panic!("cancelled request notified")
        });
    }
}

#[test]
fn a_changed_queue_path_rejects_the_old_result_and_accepts_a_fresh_probe() {
    let mut root = FrameRoot::new();
    let previous = add_request(&mut root, "same");
    root.file_queue.file_by_id_mut("same").unwrap().path = "replacement.wav".into();
    let before = root.file_queue.clone();
    root.complete_source_metadata_probe(&previous, Ok(SourceMetadata::default()), true, || {
        panic!("old path notified")
    });
    assert_eq!(root.file_queue, before);
    assert_eq!(
        root.source_metadata.status_for("same"),
        MetadataStatus::Loading
    );
    assert!(
        root.begin_source_metadata_probe("same".into(), "same.wav".into())
            .is_none()
    );
    let current = root
        .begin_source_metadata_probe("same".into(), "replacement.wav".into())
        .unwrap();
    let mut notifications = 0;
    root.complete_source_metadata_probe(&current, Ok(SourceMetadata::default()), true, || {
        notifications += 1;
    });
    assert_eq!(notifications, 1);
    assert_eq!(
        root.source_metadata.status_for("same"),
        MetadataStatus::Ready
    );
}
