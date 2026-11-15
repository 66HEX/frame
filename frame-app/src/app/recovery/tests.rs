use super::*;
use crate::app::model::ActiveView;
use crate::app_persistence::AppPersistence;
use crate::file_queue::{FileItem, FileStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) struct TestDirectory(PathBuf);
impl TestDirectory {
    pub(super) fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-recovery-app-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn fixture() -> (TestDirectory, FrameRoot, RecoverySnapshot) {
    let directory = TestDirectory::new();
    let source = directory.path().join("source.mp4");
    fs::write(&source, b"source").unwrap();
    let persistence = AppPersistence::from_settings_path(directory.path().join("settings.json"));
    let mut root = FrameRoot::new_with_persistence(persistence);
    let mut file = FileItem::from_os_path("file-7", &source);
    file.output_name = "prepared-name".into();
    file.config.quality = 31;
    file.status = FileStatus::Converting;
    root.file_queue.add_file(file);
    root.file_queue.select_file(Some("file-7".into()));
    let snapshot = RecoverySnapshot::capture(
        &root.file_queue,
        ActiveView::Logs,
        Some(directory.path().join("exports")),
        "previous",
        1,
        1,
    )
    .unwrap();
    root.file_queue = crate::file_queue::FileQueue::new();
    (directory, root, snapshot)
}

pub(super) fn open_loaded(
    directory: &TestDirectory,
    snapshot: &RecoverySnapshot,
) -> (RecoveryStore, RecoverySnapshot) {
    let settings = directory.path().join("settings.json");
    {
        let mut store = RecoveryStore::open(&settings).unwrap();
        assert!(store.load().unwrap().is_none());
        store.resolve_existing().unwrap();
        store.save(snapshot).unwrap();
    }
    let mut store = RecoveryStore::open(&settings).unwrap();
    let loaded = store.load().unwrap().unwrap();
    (store, loaded)
}

#[test]
fn startup_preserves_saved_queue_until_explicit_restore_and_never_starts_work() {
    let (directory, mut root, snapshot) = fixture();
    let (store, snapshot) = open_loaded(&directory, &snapshot);
    let original = fs::read(directory.path().join("workspace-recovery.json")).unwrap();
    root.accept_recovery_store(store, Some(snapshot), false)
        .unwrap();
    assert_eq!(root.recovery_prompt_count(), Some(1));
    assert!(root.file_queue.files().is_empty());
    assert!(root.prepare_recovery_write().unwrap().is_none());
    assert_eq!(
        fs::read(directory.path().join("workspace-recovery.json")).unwrap(),
        original
    );

    let probes = root.resolve_workspace(true).unwrap();
    assert_eq!(probes.len(), 1);
    let restored = root.file_queue.file_by_id("file-7").unwrap();
    assert_eq!(restored.config.quality, 31);
    assert_eq!(restored.output_name, "prepared-name");
    assert_eq!(restored.status, FileStatus::Error);
    assert_eq!(root.file_queue.selected_file_id(), Some("file-7"));
    assert_eq!(root.active_view, ActiveView::Logs);
    assert_eq!(
        root.default_output_directory,
        Some(directory.path().join("exports"))
    );
    assert_eq!(root.next_file_sequence, 7);
    assert!(!root.is_processing);
    assert!(root.active_conversion_task_ids.is_empty());
    root.prepare_recovery_write()
        .unwrap()
        .unwrap()
        .save()
        .unwrap();
}

#[test]
fn discard_publishes_empty_queue_and_deduplicates_unchanged_intent() {
    let (directory, mut root, snapshot) = fixture();
    let (store, snapshot) = open_loaded(&directory, &snapshot);
    root.accept_recovery_store(store, Some(snapshot), false)
        .unwrap();
    root.resolve_workspace(false).unwrap();
    let write = root
        .prepare_recovery_write()
        .unwrap()
        .expect("discard must be persisted");
    assert_eq!(write.snapshot.file_count(), 0);
    write.save().unwrap();
    assert!(root.prepare_recovery_write().unwrap().is_none());
    root.default_output_directory = Some(directory.path().join("new destination"));
    assert!(root.prepare_recovery_write().unwrap().is_some());
}

#[test]
fn updater_restore_takes_priority_and_old_queue_cannot_replace_it() {
    let (directory, mut root, snapshot) = fixture();
    let (store, snapshot) = open_loaded(&directory, &snapshot);
    root.file_queue.add_file(FileItem::from_os_path(
        "file-2",
        &directory.path().join("updated.mp4"),
    ));
    root.accept_recovery_store(store, Some(snapshot), true)
        .unwrap();
    assert_eq!(root.recovery_prompt_count(), None);
    assert!(root.file_queue.file_by_id("file-2").is_some());
    assert!(root.file_queue.file_by_id("file-7").is_none());
    assert!(root.resolve_workspace(true).unwrap().is_empty());
    let write = root.prepare_recovery_write().unwrap().unwrap();
    let restored = write.snapshot.clone().restore().unwrap();
    assert!(restored.queue.file_by_id("file-2").is_some());
    assert!(restored.queue.file_by_id("file-7").is_none());
    write.save().unwrap();
}

#[test]
fn notification_burst_saves_latest_intent_and_release_flushes_without_a_timer() {
    use gpui::{AppContext, TestAppContext};
    let directory = TestDirectory::new();
    let settings = directory.path().join("settings.json");
    let source = directory.path().join("source.mp4");
    fs::write(&source, b"source").unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root =
            FrameRoot::new_with_persistence(AppPersistence::from_settings_path(&settings));
        root.start_workspace_recovery(false, cx);
        root
    });
    app.run_until_parked();
    root.update(&mut app, |root, cx| {
        root.file_queue
            .add_file(FileItem::from_os_path("file-1", &source));
        for quality in 1..=90 {
            root.file_queue
                .file_by_id_mut("file-1")
                .unwrap()
                .config
                .quality = quality;
            cx.notify();
        }
    });
    app.run_until_parked();
    assert!(
        app.dispatcher.advance_clock_to_next_timer(),
        "changes should schedule a coalesced save"
    );
    app.run_until_parked();
    let saved = fs::read(settings.with_file_name("workspace-recovery.json")).unwrap();
    let snapshot: RecoverySnapshot = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        snapshot.restore().unwrap().queue.files()[0].config.quality,
        90
    );

    // Closing before the next timer must still preserve the final edit.
    root.update(&mut app, |root, _cx| {
        root.file_queue
            .file_by_id_mut("file-1")
            .unwrap()
            .output_name = "last edit before close".into();
    });
    app.update(|_| drop(root));
    app.run_until_parked();
    let saved = fs::read(settings.with_file_name("workspace-recovery.json")).unwrap();
    let snapshot: RecoverySnapshot = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        snapshot.restore().unwrap().queue.files()[0].output_name,
        "last edit before close"
    );
    assert!(
        RecoveryStore::open(&settings).is_ok(),
        "closing must release the profile lock"
    );
}

#[test]
fn native_startup_offers_saved_queue_and_preserves_it_if_window_closes_before_decision() {
    use gpui::{AppContext, TestAppContext};
    let (directory, root, snapshot) = fixture();
    let (store, _) = open_loaded(&directory, &snapshot);
    drop(store);
    let path = directory.path().join("workspace-recovery.json");
    let original = fs::read(&path).unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = root;
        root.initialize_workspace(cx);
        root
    });
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert_eq!(root.recovery_prompt_count(), Some(1));
        assert!(root.recovery_blocks_workspace());
        assert!(root.file_queue.files().is_empty());
    });
    app.update(|_| drop(root));
    app.run_until_parked();
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn native_startup_consumes_updater_queue_before_enabling_recovery_autosave() {
    use crate::update_session::{UpdateSessionSnapshot, UpdateSessionStore};
    use gpui::{AppContext, TestAppContext};
    let (directory, root, snapshot) = fixture();
    let (store, _) = open_loaded(&directory, &snapshot);
    drop(store);
    let source = directory.path().join("newer-source.mp4");
    fs::write(&source, b"updated source").unwrap();
    let mut queue = crate::file_queue::FileQueue::new();
    let mut file = FileItem::from_os_path("file-2", &source);
    file.output_name = "newer updater edit".into();
    queue.add_file(file);
    let update =
        UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.33.0", "0.34.0", 2)
            .unwrap();
    let settings = directory.path().join("settings.json");
    UpdateSessionStore::from_settings_path(&settings)
        .save(&update)
        .unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = root;
        root.initialize_workspace(cx);
        root
    });
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert_eq!(root.recovery_prompt_count(), None);
        assert!(
            root.workspace_recovery.ready,
            "successful updater restore must enable new recovery saves"
        );
        assert_eq!(root.file_queue.files().len(), 1);
        assert_eq!(root.file_queue.files()[0].output_name, "newer updater edit");
    });
    assert!(
        UpdateSessionStore::from_settings_path(&settings)
            .load()
            .unwrap()
            .is_some(),
        "keep the updater queue until recovery has actually been published"
    );
    app.update(|_| drop(root));
    app.run_until_parked();
    assert!(
        UpdateSessionStore::from_settings_path(&settings)
            .load()
            .unwrap()
            .is_none()
    );
    let saved: RecoverySnapshot = serde_json::from_slice(
        &fs::read(directory.path().join("workspace-recovery.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        saved.restore().unwrap().queue.files()[0].output_name,
        "newer updater edit"
    );
}

#[test]
fn failed_updater_restore_preserves_both_sessions_and_disables_autosave() {
    use gpui::{AppContext, TestAppContext};
    let (directory, root, snapshot) = fixture();
    let (store, _) = open_loaded(&directory, &snapshot);
    drop(store);
    let recovery_path = directory.path().join("workspace-recovery.json");
    let saved = fs::read(&recovery_path).unwrap();
    let update_path = directory.path().join("pending-update-session.json");
    fs::write(&update_path, b"invalid update session").unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = root;
        root.initialize_workspace(cx);
        root
    });
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert!(root.recovery_notice().is_some());
        assert!(root.recovery_prompt_count().is_none());
        assert!(!root.workspace_recovery.ready);
        assert!(root.file_queue.files().is_empty());
    });
    app.quit();
    assert_eq!(fs::read(&recovery_path).unwrap(), saved);
    assert_eq!(fs::read(&update_path).unwrap(), b"invalid update session");
}

#[test]
fn import_waits_for_startup_decision_without_allocating_file_ids() {
    let (directory, mut root, snapshot) = fixture();
    let source = directory.path().join("source.mp4");
    root.workspace_recovery.loading = true;
    let sequence = root.next_file_sequence;
    assert!(root.allocate_file_imports(vec![source.clone()]).is_empty());
    let (store, snapshot) = open_loaded(&directory, &snapshot);
    root.accept_recovery_store(store, Some(snapshot), false)
        .unwrap();
    assert!(root.allocate_file_imports(vec![source.clone()]).is_empty());
    assert_eq!(root.next_file_sequence, sequence);
    root.resolve_workspace(false).unwrap();
    assert_eq!(root.allocate_file_imports(vec![source]).len(), 1);
}
