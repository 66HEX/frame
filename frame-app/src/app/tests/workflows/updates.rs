use super::*;

fn ready_update_package() -> UpdatePackage {
    UpdatePackage {
        version: Version::new(0, 32, 0),
        channel: UpdateChannel::Stable,
        asset_key: PlatformAssetKey::MacosAarch64,
        kind: UpdateAssetKind::MacosAppZip,
        file_name: "Frame.zip".to_string(),
        path: PathBuf::from("/tmp/Frame.zip"),
        size_bytes: 1,
        sha256: "00".repeat(32),
        installer_args: Vec::new(),
    }
}

#[test]
fn update_install_readiness_requires_terminal_queue_and_no_worker_processes() {
    for status in [FileStatus::Idle, FileStatus::Completed, FileStatus::Error] {
        let mut root = FrameRoot::new();
        root.update_ui.status = UpdateStatus::ReadyToInstall(Box::new(ready_update_package()));
        let mut file = FileItem::from_path("first", "/tmp/one.mp4", 1);
        file.status = status;
        root.file_queue.add_file(file);
        assert!(root.can_install_downloaded_update(), "status: {status:?}");
    }

    for status in [
        FileStatus::Queued,
        FileStatus::Converting,
        FileStatus::Paused,
        FileStatus::Cancelling,
    ] {
        let mut root = FrameRoot::new();
        root.update_ui.status = UpdateStatus::ReadyToInstall(Box::new(ready_update_package()));
        let mut file = FileItem::from_path("first", "/tmp/one.mp4", 1);
        file.status = status;
        root.file_queue.add_file(file);
        assert!(!root.can_install_downloaded_update(), "status: {status:?}");
    }

    let mut root = FrameRoot::new();
    root.update_ui.status = UpdateStatus::ReadyToInstall(Box::new(ready_update_package()));
    let mut file = FileItem::from_path("first", "/tmp/one.mp4", 1);
    file.status = FileStatus::Completed;
    root.file_queue.add_file(file);
    root.conversion_processes
        .register_started_process("stale-worker", 0)
        .expect("test worker should be registered");
    assert!(!root.can_install_downloaded_update());

    root.conversion_processes
        .finish_task("stale-worker")
        .expect("test worker should be finished");
    assert!(root.can_install_downloaded_update());
}

#[test]
fn update_install_readiness_requires_ready_to_install_status() {
    let root = FrameRoot::new();

    assert!(!root.can_install_downloaded_update());
}

#[test]
fn installation_barrier_blocks_workspace_mutations() {
    let mut root = FrameRoot::new();
    let mut file = FileItem::from_path("first", "/tmp/one.mp4", 1);
    file.status = FileStatus::Completed;
    root.file_queue.add_file(file);
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
    root.update_ui.status = UpdateStatus::Installing;

    assert!(!root.prepare_file_for_reconversion("first"));
    assert!(!root.remove_file_from_queue("first"));
    assert!(root.text_input_disabled(FrameTextInputKind::MaxConcurrency));
    assert!(root.text_input_disabled(FrameTextInputKind::OutputName));
    assert!(root.toggle_file_batch_selection("first").is_none());
    assert!(root.toggle_all_file_batch_selection().is_none());
    assert!(!root.select_workspace_file("second"));
    assert!(!root.update_selected_config(|config| {
        config.video_bitrate = "1".to_string();
        true
    }));
    assert_eq!(root.file_queue.selected_file_id(), Some("first"));
    assert_eq!(root.file_queue.files().len(), 2);
    assert_eq!(root.file_queue.files()[0].status, FileStatus::Completed);
}
