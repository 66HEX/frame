use super::tests::{TestDirectory, fixture, open_loaded};
use super::*;
use crate::app_persistence::AppPersistence;
use crate::file_queue::FileItem;
use gpui::{AppContext, TestAppContext};
use std::fs;

#[test]
fn app_quit_flushes_the_last_edit_while_the_root_is_still_owned() {
    let directory = TestDirectory::new();
    let settings = directory.path().join("settings.json");
    let saved_path = directory.path().join("workspace-recovery.json");
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
    root.update(&mut app, |root, _cx| {
        assert!(root.workspace_recovery.ready);
        let mut file = FileItem::from_os_path("file-1", &source);
        file.output_name = "last edit at quit".into();
        file.config.quality = 47;
        root.file_queue.add_file(file);
    });
    assert!(!saved_path.exists(), "no save timer has been advanced");

    // TestAppContext::quit runs App::shutdown. App::quit would call the test
    // platform's no-op quit instead, and would not exercise on_app_quit.
    app.quit();
    let snapshot: RecoverySnapshot =
        serde_json::from_slice(&fs::read(&saved_path).unwrap()).unwrap();
    let restored = snapshot.restore().unwrap();
    assert_eq!(restored.queue.files()[0].output_name, "last edit at quit");
    assert_eq!(restored.queue.files()[0].config.quality, 47);
    assert!(
        root.downgrade().upgrade().is_some(),
        "on_release cannot be the writer while this strong owner is alive"
    );
    app.update(|_| drop(root));
    assert!(
        RecoveryStore::open(&settings).is_ok(),
        "release must relinquish the profile lock"
    );
}

#[test]
fn app_quit_preserves_unresolved_and_corrupt_recovery_bytes() {
    for corrupt in [false, true] {
        let (directory, root, snapshot) = fixture();
        let (store, _) = open_loaded(&directory, &snapshot);
        drop(store);
        let saved_path = directory.path().join("workspace-recovery.json");
        if corrupt {
            fs::write(&saved_path, b"{\"broken snapshot with user data\"").unwrap();
        }
        let original = fs::read(&saved_path).unwrap();
        let mut app = TestAppContext::single();
        let root = app.new(|cx| {
            let mut root = root;
            root.start_workspace_recovery(false, cx);
            root
        });
        app.run_until_parked();
        root.read_with(&app, |root, _| {
            assert!(!root.workspace_recovery.ready);
            assert_eq!(root.recovery_prompt_count(), (!corrupt).then_some(1));
            assert_eq!(root.recovery_notice().is_some(), corrupt);
        });
        app.quit();
        assert_eq!(fs::read(&saved_path).unwrap(), original);
        app.update(|_| drop(root));
        assert_eq!(fs::read(&saved_path).unwrap(), original);
        assert!(RecoveryStore::open(&directory.path().join("settings.json")).is_ok());
    }
}

#[test]
fn a_root_without_persistence_does_not_start_recovery_storage() {
    let mut app = TestAppContext::single();
    let root = app.new(|cx| {
        let mut root = FrameRoot::new();
        root.start_workspace_recovery(false, cx);
        root
    });
    app.run_until_parked();
    app.quit();
    root.read_with(&app, |root, _| {
        assert!(root.workspace_recovery.store.is_none());
        assert!(!root.workspace_recovery.ready);
        assert!(!root.workspace_recovery.loading);
        assert!(root.recovery_prompt_count().is_none());
        assert!(root.recovery_notice().is_none());
    });
    app.update(|_| drop(root));
}
