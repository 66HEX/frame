use super::tests::{TestDirectory, fixture, open_loaded};
use super::*;
use crate::ActiveView;
use crate::file_queue::{FileItem, FileQueue};
use crate::update_session::{UpdateSessionSnapshot, UpdateSessionStore};
use gpui::{AppContext, TestAppContext};
use std::fs;

fn update_snapshot(directory: &TestDirectory, name: &str, created: u64) -> UpdateSessionSnapshot {
    let source = directory.path().join("installer-source.mp4");
    fs::write(&source, b"source").unwrap();
    let mut queue = FileQueue::new();
    let mut file = FileItem::from_os_path("file-1", &source);
    file.output_name = name.into();
    file.config.quality = 61;
    queue.add_file(file);
    UpdateSessionSnapshot::capture(&queue, ActiveView::Workspace, "0.33.0", "0.34.0", created)
        .unwrap()
}

#[test]
fn failed_recovery_keeps_a_successfully_restored_updater_snapshot_through_quit() {
    for unsupported in [false, true] {
        let (directory, root, previous) = fixture();
        let (store, _) = open_loaded(&directory, &previous);
        drop(store);
        let recovery_path = directory.path().join("workspace-recovery.json");
        let invalid_bytes = if unsupported {
            let mut value = serde_json::to_value(&previous).unwrap();
            value["version"] = 999.into();
            serde_json::to_vec(&value).unwrap()
        } else {
            b"corrupt recovery with user data".to_vec()
        };
        fs::write(&recovery_path, &invalid_bytes).unwrap();
        let settings = directory.path().join("settings.json");
        let pending_path = directory.path().join("pending-update-session.json");
        let update_store = UpdateSessionStore::from_settings_path(&settings);
        let update = update_snapshot(&directory, "restored installer edit", 2);
        update_store.save(&update).unwrap();
        let pending_bytes = fs::read(&pending_path).unwrap();
        let mut app = TestAppContext::single();
        let root = app.new(|cx| {
            let mut root = root;
            root.initialize_workspace(cx);
            root
        });
        app.run_until_parked();
        root.read_with(&app, |root, _| {
            assert!(!root.workspace_recovery.ready);
            assert!(root.recovery_notice().is_some());
            assert_eq!(root.file_queue.files().len(), 1);
            assert_eq!(
                root.file_queue.files()[0].output_name,
                "restored installer edit"
            );
            assert_eq!(root.file_queue.files()[0].config.quality, 61);
        });
        app.quit();
        app.update(|_| drop(root));
        assert_eq!(fs::read(&pending_path).unwrap(), pending_bytes);
        assert_eq!(update_store.load().unwrap(), Some(update));
        assert_eq!(fs::read(&recovery_path).unwrap(), invalid_bytes);
    }
}

#[test]
fn newer_installer_snapshot_survives_an_older_recovery_write_completion() {
    let (directory, root, previous) = fixture();
    let (store, _) = open_loaded(&directory, &previous);
    drop(store);
    let settings = directory.path().join("settings.json");
    let recovery_path = directory.path().join("workspace-recovery.json");
    let previous_bytes = fs::read(&recovery_path).unwrap();
    let pending_path = directory.path().join("pending-update-session.json");
    let update_store = UpdateSessionStore::from_settings_path(&settings);
    update_store
        .save(&update_snapshot(&directory, "first installer", 2))
        .unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = root;
        root.initialize_workspace(cx);
        root
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());
    // Pause after the old queue is captured but before its background write.
    for _ in 0..1000 {
        if root.read_with(&app, |root, _| root.workspace_recovery.revision > 0) {
            break;
        }
        assert!(
            app.dispatcher.tick(false),
            "autosave never captured the restored queue"
        );
    }
    assert_eq!(
        root.read_with(&app, |root, _| root.workspace_recovery.revision),
        1
    );
    assert_eq!(fs::read(&recovery_path).unwrap(), previous_bytes);
    let newer = update_snapshot(&directory, "second installer", 3);
    update_store.save(&newer).unwrap();
    let newer_bytes = fs::read(&pending_path).unwrap();

    app.run_until_parked();
    let saved: RecoverySnapshot =
        serde_json::from_slice(&fs::read(&recovery_path).unwrap()).unwrap();
    assert_eq!(
        saved.restore().unwrap().queue.files()[0].output_name,
        "first installer"
    );
    assert_eq!(fs::read(&pending_path).unwrap(), newer_bytes);
    assert_eq!(update_store.load().unwrap(), Some(newer));
    app.quit();
    app.update(|_| drop(root));
    assert_eq!(fs::read(&pending_path).unwrap(), newer_bytes);
}

#[test]
fn autosave_consumes_matching_updater_snapshot_only_after_durable_publication() {
    let (directory, root, previous) = fixture();
    let (store, _) = open_loaded(&directory, &previous);
    drop(store);
    let settings = directory.path().join("settings.json");
    let update_store = UpdateSessionStore::from_settings_path(&settings);
    let update = update_snapshot(&directory, "autosaved installer", 2);
    update_store.save(&update).unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = root;
        root.initialize_workspace(cx);
        root
    });
    app.run_until_parked();
    assert_eq!(update_store.load().unwrap(), Some(update));
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();

    assert!(update_store.load().unwrap().is_none());
    let saved: RecoverySnapshot = serde_json::from_slice(
        &fs::read(directory.path().join("workspace-recovery.json")).unwrap(),
    )
    .unwrap();
    let restored = saved.restore().unwrap();
    assert_eq!(restored.queue.files()[0].output_name, "autosaved installer");
    assert_eq!(restored.queue.files()[0].config.quality, 61);
    root.read_with(&app, |root, _| {
        assert!(root.workspace_recovery.ready);
        assert!(root.workspace_recovery.pending_update.is_none());
    });
    // The assertions above precede both shutdown and release.
    app.quit();
    app.update(|_| drop(root));
}

#[test]
fn starting_a_new_install_prevents_consuming_the_previous_pending_update() {
    let (directory, root, previous) = fixture();
    let (store, _) = open_loaded(&directory, &previous);
    drop(store);
    let settings = directory.path().join("settings.json");
    let update_store = UpdateSessionStore::from_settings_path(&settings);
    let update = update_snapshot(&directory, "previous installer", 2);
    update_store.save(&update).unwrap();
    let pending_path = directory.path().join("pending-update-session.json");
    let original = fs::read(&pending_path).unwrap();
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = root;
        root.initialize_workspace(cx);
        root
    });
    app.run_until_parked();
    root.update(&mut app, |root, _cx| {
        // Installation enters this state on the UI thread before spawning its
        // background snapshot writer. Finalization must exclude that window.
        root.update_ui.status = crate::app::updates::state::UpdateStatus::Installing;
        root.flush_workspace_recovery();
    });
    let saved: RecoverySnapshot = serde_json::from_slice(
        &fs::read(directory.path().join("workspace-recovery.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        saved.restore().unwrap().queue.files()[0].output_name,
        "previous installer"
    );
    assert_eq!(update_store.load().unwrap(), Some(update));
    assert_eq!(fs::read(&pending_path).unwrap(), original);
    app.quit();
    app.update(|_| drop(root));
    assert_eq!(fs::read(&pending_path).unwrap(), original);
}
