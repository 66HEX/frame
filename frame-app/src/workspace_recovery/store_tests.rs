use super::tests::{Fixture, capture};
use super::*;
use crate::ActiveView;
use crate::file_queue::{FileItem, FileQueue};
use std::{fs, io};

fn snapshot(fixture: &Fixture, revision: u64) -> RecoverySnapshot {
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_os_path(
        "file-1",
        &fixture.source("source.mp4"),
    ));
    RecoverySnapshot::capture(
        &queue,
        ActiveView::Workspace,
        None,
        "session-a",
        revision,
        42,
    )
    .unwrap()
}

fn ready_store(fixture: &Fixture) -> RecoveryStore {
    let mut store = RecoveryStore::open(&fixture.0.join("settings.json")).unwrap();
    assert!(store.load().unwrap().is_none());
    store.resolve_existing().unwrap();
    store
}

#[test]
fn store_requires_load_and_explicit_startup_resolution_before_writing() {
    let fixture = Fixture::new();
    let mut store = RecoveryStore::open(&fixture.0.join("settings.json")).unwrap();
    let snapshot = snapshot(&fixture, 1);
    assert!(matches!(
        store.save(&snapshot),
        Err(RecoveryError::Unresolved)
    ));
    assert!(matches!(
        store.resolve_existing(),
        Err(RecoveryError::Unresolved)
    ));
    store.load().unwrap();
    assert!(matches!(
        store.save(&snapshot),
        Err(RecoveryError::Unresolved)
    ));
    store.resolve_existing().unwrap();
    store.save(&snapshot).unwrap();
    assert!(fixture.0.join("workspace-recovery.json").is_file());
}

#[test]
fn profile_has_one_writer_and_drop_releases_the_lock() {
    let fixture = Fixture::new();
    let path = fixture.0.join("settings.json");
    let owner = RecoveryStore::open(&path).unwrap();
    assert!(matches!(
        RecoveryStore::open(&path),
        Err(RecoveryError::ProfileInUse)
    ));
    drop(owner);
    let _next = RecoveryStore::open(&path).unwrap();
}

#[test]
fn flatpak_refuses_before_creating_any_directory_or_file() {
    let fixture = Fixture::new();
    let directory = fixture.0.join("must-not-exist");
    assert!(matches!(
        RecoveryStore::open_for_environment(&directory.join("settings.json"), true),
        Err(RecoveryError::Unavailable)
    ));
    assert!(!directory.exists());
}

#[test]
fn successful_save_reopens_with_identical_snapshot_and_no_temp_files() {
    let fixture = Fixture::new();
    let mut store = ready_store(&fixture);
    let expected = snapshot(&fixture, 1);
    store.save(&expected).unwrap();
    drop(store);
    let mut reopened = RecoveryStore::open(&fixture.0.join("settings.json")).unwrap();
    assert_eq!(reopened.load().unwrap(), Some(expected));
    assert!(fs::read_dir(&fixture.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

#[test]
fn failure_before_publication_preserves_previous_snapshot_and_removes_own_temp() {
    let fixture = Fixture::new();
    let mut store = ready_store(&fixture);
    store.save(&snapshot(&fixture, 1)).unwrap();
    let path = fixture.0.join("workspace-recovery.json");
    let before = fs::read(&path).unwrap();
    let error = store
        .save_before_publish(&snapshot(&fixture, 2), |temporary| {
            assert!(temporary.is_file());
            assert_ne!(temporary, path);
            Err(io::Error::from(io::ErrorKind::StorageFull))
        })
        .unwrap_err();
    assert!(matches!(error, RecoveryError::Io(_)));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(fs::read_dir(&fixture.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
    store.save(&snapshot(&fixture, 2)).unwrap();
}

#[test]
fn stale_revision_or_foreign_session_does_not_replace_latest_snapshot() {
    let fixture = Fixture::new();
    let mut store = ready_store(&fixture);
    store.save(&snapshot(&fixture, 2)).unwrap();
    let path = fixture.0.join("workspace-recovery.json");
    let before = fs::read(&path).unwrap();
    assert!(matches!(
        store.save(&snapshot(&fixture, 1)),
        Err(RecoveryError::StaleRevision)
    ));
    let mut value = serde_json::to_value(snapshot(&fixture, 3)).unwrap();
    value["sessionId"] = serde_json::json!("another-writer-session");
    assert!(matches!(
        store.save(&serde_json::from_value(value).unwrap()),
        Err(RecoveryError::StaleRevision)
    ));
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn invalid_or_future_snapshot_remains_byte_identical_and_disables_writes() {
    let fixture = Fixture::new();
    let path = fixture.0.join("workspace-recovery.json");
    let mut future = serde_json::to_value(snapshot(&fixture, 1)).unwrap();
    future["version"] = serde_json::json!(42);
    for bytes in [
        b"{broken json".to_vec(),
        serde_json::to_vec(&future).unwrap(),
    ] {
        fs::write(&path, &bytes).unwrap();
        let mut store = RecoveryStore::open(&fixture.0.join("settings.json")).unwrap();
        assert!(store.load().is_err());
        assert!(matches!(
            store.resolve_existing(),
            Err(RecoveryError::Unresolved)
        ));
        assert!(matches!(
            store.save(&snapshot(&fixture, 2)),
            Err(RecoveryError::Unresolved)
        ));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn too_large_snapshot_is_rejected_before_reading_or_replacing_it() {
    let fixture = Fixture::new();
    let path = fixture.0.join("workspace-recovery.json");
    let file = fs::File::create(&path).unwrap();
    file.set_len(super::model::MAX_BYTES as u64 + 1).unwrap();
    drop(file);
    let mut store = RecoveryStore::open(&fixture.0.join("settings.json")).unwrap();
    assert!(matches!(store.load(), Err(RecoveryError::TooLarge)));
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        super::model::MAX_BYTES as u64 + 1
    );
}

#[test]
fn same_workspace_ignores_writer_metadata_but_detects_actual_intent_changes() {
    let fixture = Fixture::new();
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_os_path(
        "file-1",
        &fixture.source("source.mp4"),
    ));
    let first = capture(&queue);
    let second = RecoverySnapshot::capture(
        &queue,
        ActiveView::Logs,
        Some("/exports".into()),
        "session-b",
        99,
        900,
    )
    .unwrap();
    assert!(first.same_workspace_as(&second));
    queue.files_mut()[0].config.crf = 17;
    assert!(!first.same_workspace_as(&capture(&queue)));
}

#[cfg(unix)]
#[test]
fn non_regular_snapshot_is_rejected_without_waiting_for_a_fifo_writer() {
    use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits};
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::time::Duration;
    static DOMAIN: ExecutionDomain = ExecutionDomain::new();
    let fixture = Fixture::new();
    let path = fixture.0.join("workspace-recovery.json");
    let raw_path = CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: The NUL-terminated path buffer lives through this synchronous call.
    assert_eq!(unsafe { libc::mkfifo(raw_path.as_ptr(), 0o600) }, 0);
    let mut command = CommandSpec::new(std::env::current_exe().unwrap());
    command.args = [
        "--exact",
        "workspace_recovery::store_tests::read_fifo_snapshot_child",
        "--ignored",
    ]
    .map(Into::into)
    .to_vec();
    command.env.push((
        "FRAME_RECOVERY_FIFO_SOURCE".into(),
        fixture.0.join("settings.json").into_os_string(),
    ));
    let result = DOMAIN
        .run(
            &command,
            &[],
            Limits {
                timeout: Duration::from_secs(2),
                cleanup_timeout: Duration::from_secs(1),
                stdout_bytes: 4096,
                stderr_bytes: 4096,
            },
            || false,
        )
        .unwrap();
    assert!(
        result.success,
        "Recovery must reject a FIFO without waiting: {result:?}"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "Only invoked as a bounded child for the FIFO recovery regression."]
fn read_fifo_snapshot_child() {
    let Some(path) = std::env::var_os("FRAME_RECOVERY_FIFO_SOURCE") else {
        return;
    };
    let mut store = RecoveryStore::open(std::path::Path::new(&path)).unwrap();
    assert!(matches!(store.load(), Err(RecoveryError::Invalid(_))));
}
