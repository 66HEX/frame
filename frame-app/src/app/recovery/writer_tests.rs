use super::tests::{TestDirectory, fixture, open_loaded};
use super::*;
use crate::app_persistence::AppPersistence;
use crate::file_queue::{FileItem, FileQueue};
use gpui::{AppContext, TestAppContext};
use std::fs;
use std::path::Path;

fn saved_queue(settings: &Path) -> FileQueue {
    let bytes = fs::read(settings.with_file_name("workspace-recovery.json")).unwrap();
    serde_json::from_slice::<RecoverySnapshot>(&bytes)
        .unwrap()
        .restore()
        .unwrap()
        .queue
}

#[test]
fn older_background_write_cannot_replace_the_latest_close_time_flush() {
    let (directory, mut root, snapshot) = fixture();
    let (store, snapshot) = open_loaded(&directory, &snapshot);
    root.accept_recovery_store(store, Some(snapshot), false)
        .unwrap();
    root.resolve_workspace(true).unwrap();
    let older = root.prepare_recovery_write().unwrap().unwrap();
    let file = root.file_queue.file_by_id_mut("file-7").unwrap();
    file.output_name = "last edit before close".into();
    file.config.quality = 79;
    root.flush_workspace_recovery();
    let settings = directory.path().join("settings.json");
    let latest_bytes = fs::read(settings.with_file_name("workspace-recovery.json")).unwrap();

    // This is the ordering when close flush acquires the store before an
    // already captured background write. The stale write must be harmless.
    older.save().unwrap();
    assert_eq!(
        fs::read(settings.with_file_name("workspace-recovery.json")).unwrap(),
        latest_bytes
    );
    let saved = saved_queue(&settings);
    let file = saved.file_by_id("file-7").unwrap();
    assert_eq!(file.output_name, "last edit before close");
    assert_eq!(file.config.quality, 79);
}

#[test]
fn edit_after_autosave_capture_is_persisted_by_a_followup_save() {
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
        let mut file = FileItem::from_os_path("file-1", &source);
        file.config.quality = 11;
        root.file_queue.add_file(file);
        cx.notify();
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());

    // Stop the deterministic scheduler after capture and before its background
    // task is allowed to save. No real mutex wait can block the test dispatcher.
    for _ in 0..1000 {
        if root.read_with(&app, |root, _| root.workspace_recovery.revision > 0) {
            break;
        }
        assert!(
            app.dispatcher.tick(false),
            "autosave never captured the queue"
        );
    }
    let captured_epoch = root.read_with(&app, |root, _| {
        assert_eq!(root.workspace_recovery.revision, 1);
        assert!(root.workspace_recovery.scheduled);
        root.workspace_recovery.change_epoch
    });
    assert!(
        !settings.with_file_name("workspace-recovery.json").exists(),
        "the first writer must still be pending"
    );
    root.update(&mut app, |root, cx| {
        root.file_queue
            .file_by_id_mut("file-1")
            .unwrap()
            .config
            .quality = 77;
        cx.notify();
    });
    app.run_until_parked();
    assert_eq!(saved_queue(&settings).files()[0].config.quality, 11);
    root.read_with(&app, |root, _| {
        assert!(root.workspace_recovery.change_epoch > captured_epoch);
        assert!(
            root.workspace_recovery.scheduled,
            "the new edit needs another save"
        );
    });

    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    assert_eq!(saved_queue(&settings).files()[0].config.quality, 77);
    root.read_with(&app, |root, _| {
        assert_eq!(root.workspace_recovery.revision, 2);
        assert!(!root.workspace_recovery.scheduled);
    });
    app.update(|_| drop(root));
    app.run_until_parked();
}
