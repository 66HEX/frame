use super::*;
use crate::file_queue::FileItem;
use crate::preview::MediaSnapshot;
use crate::preview_engine::PreviewSession;
use crate::settings::{SourceKind, SourceMetadata};

fn ready_root() -> FrameRoot {
    let mut root = FrameRoot::new();
    let mut file = FileItem::from_path("frame-source", "/tmp/zażółć source.mov", 12);
    file.config.start_time = Some("2.125".into());
    root.file_queue.add_file(file);
    root.default_output_directory = Some(PathBuf::from("/tmp/source frame output"));
    root.source_metadata.mark_ready(
        "frame-source",
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            video_stream_index: Some(3),
            width: Some(320),
            height: Some(180),
            duration: Some("10".into()),
            ..SourceMetadata::default()
        },
    );
    let entry = root.selected_source_metadata_entry();
    let request = root.selected_preview_runtime_request(&entry).unwrap();
    root.preview_ui.runtime_key = Some(request.key.clone());
    root.preview_ui.desired_runtime_key = Some(request.key);
    root.preview_ui.playback_file_id = Some("frame-source".into());
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 4.625_001,
        duration: 10.0,
        paused: true,
    });
    root
}

#[test]
fn paused_source_frame_intent_freezes_the_input_clock_without_adding_trim() {
    let root = ready_root();
    let before = root.file_queue.selected_file().unwrap().clone();
    let intent =
        source_frame_intent(&root, false, None).expect("ready paused source must permit an export");

    assert_eq!(intent.position_microseconds, 4_625_001);
    assert_eq!(intent.video_stream_index, 3);
    assert_eq!(intent.source_path, PathBuf::from("/tmp/zażółć source.mov"));
    assert_eq!(root.file_queue.selected_file().unwrap(), &before);
}

#[test]
fn playing_source_frame_intent_uses_the_live_accepted_session_clock() {
    let mut root = ready_root();
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 3.0,
        duration: 10.0,
        paused: false,
    });
    let request = root
        .selected_preview_runtime_request(&root.selected_source_metadata_entry())
        .unwrap();
    let mut snapshot = PreviewSession::new_for_test(request.config).snapshot();
    snapshot.playback.position_seconds = 5.875_001;
    snapshot.playback.playing = true;

    let intent = source_frame_intent(&root, false, Some(&snapshot)).unwrap();
    assert_eq!(intent.position_microseconds, 5_875_001);
    snapshot.file_id = "other-source".into();
    assert!(source_frame_intent(&root, false, Some(&snapshot)).is_err());
}

#[test]
fn source_frame_intent_does_not_follow_later_selection_or_folder_changes() {
    let mut root = ready_root();
    let intent = source_frame_intent(&root, false, None).unwrap();
    root.default_output_directory = Some(PathBuf::from("/tmp/changed folder"));
    root.file_queue
        .add_file(FileItem::from_path("other", "/tmp/other.mov", 3));
    root.file_queue.select_file(Some("other".into()));

    assert_eq!(intent.source_id, "frame-source");
    assert_eq!(
        intent.output_directory,
        PathBuf::from("/tmp/source frame output")
    );
    assert_eq!(intent.position_microseconds, 4_625_001);
}

#[test]
fn source_frame_intent_rejects_pending_preview_and_draft_edits() {
    let mut root = ready_root();
    assert!(source_frame_intent(&root, false, None).is_ok());
    root.preview_ui.pending_runtime_key = root.preview_ui.runtime_key.clone();
    assert!(source_frame_intent(&root, false, None).is_err());
    root.preview_ui.pending_runtime_key = None;
    root.preview_ui.crop_mode = true;
    assert!(source_frame_intent(&root, false, None).is_err());
    root.preview_ui.crop_mode = false;
    root.preview_ui.trim_preview_seek.pending_seconds = Some(8.0);
    assert!(source_frame_intent(&root, false, None).is_err());
}

#[test]
fn source_frame_intent_requires_ready_metadata_and_a_native_output_folder() {
    let mut root = ready_root();
    assert!(source_frame_intent(&root, false, None).is_ok());
    assert!(source_frame_intent(&root, true, None).is_err());
    root.default_output_directory = None;
    assert!(source_frame_intent(&root, false, None).is_err());
    root.default_output_directory = Some(PathBuf::from("/tmp/output"));
    root.source_metadata.mark_loading("frame-source");
    assert!(source_frame_intent(&root, false, None).is_err());
}

#[test]
fn still_source_export_uses_zero_instead_of_the_preview_clock() {
    let mut root = ready_root();
    root.source_metadata.mark_ready(
        "frame-source",
        SourceMetadata {
            media_kind: Some(SourceKind::Image),
            video_stream_index: Some(3),
            width: Some(320),
            height: Some(180),
            ..SourceMetadata::default()
        },
    );
    let intent = source_frame_intent(&root, false, None).unwrap();
    assert_eq!(intent.position_microseconds, 0);
}

#[test]
fn source_export_rejects_invalid_live_clocks() {
    let mut root = ready_root();
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 3.0,
        duration: 10.0,
        paused: false,
    });
    let request = root
        .selected_preview_runtime_request(&root.selected_source_metadata_entry())
        .unwrap();
    let mut snapshot = PreviewSession::new_for_test(request.config).snapshot();
    for position in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
        snapshot.playback.position_seconds = position;
        assert!(
            source_frame_intent(&root, false, Some(&snapshot)).is_err(),
            "invalid position {position} must not start an export"
        );
    }
}

fn export_request(id: &str) -> SourceFrameExportRequest {
    SourceFrameExportRequest::for_test(
        id.into(),
        "frame-source".into(),
        PathBuf::from("/tmp/frame-source.mov"),
        2_125_001,
        3,
        PathBuf::from("/tmp/frame-source_frame_2125001.png"),
    )
}

#[test]
fn source_frame_export_state_allows_only_one_attempt_until_completion() {
    let mut state = SourceFrameExportUiState::default();
    state.begin(export_request("export-1")).unwrap();

    assert_eq!(state.status, SourceFrameExportStatus::Exporting);
    assert!(state.begin(export_request("export-2")).is_err());
}

#[test]
fn source_frame_export_state_keeps_a_late_published_result_after_cancel() {
    let mut state = SourceFrameExportUiState::default();
    let controller = state.begin(export_request("export-1")).unwrap();
    assert!(state.cancel().unwrap());
    assert!(controller.is_cancelled("export-1"));
    assert_eq!(state.status, SourceFrameExportStatus::Cancelling);
    assert!(state.result.is_none());

    let actual = PathBuf::from("/tmp/actual collision result (2).png");
    assert!(state.complete(
        "export-1",
        Ok(SourceFrameExportOutcome::Published {
            path: actual.clone()
        })
    ));
    assert_eq!(state.status, SourceFrameExportStatus::Saved);
    assert_eq!(state.result.as_ref().unwrap().path, actual);
}

#[test]
fn source_frame_export_state_ignores_an_old_callback_and_can_retry() {
    let mut state = SourceFrameExportUiState::default();
    state.begin(export_request("export-1")).unwrap();
    assert!(state.complete("export-1", Ok(SourceFrameExportOutcome::Cancelled)));
    assert_eq!(state.status, SourceFrameExportStatus::Cancelled);
    state.begin(export_request("export-2")).unwrap();

    assert!(!state.complete(
        "export-1",
        Ok(SourceFrameExportOutcome::Published {
            path: PathBuf::from("/tmp/stale.png")
        })
    ));
    assert_eq!(state.status, SourceFrameExportStatus::Exporting);
    assert!(state.result.is_none());
}

#[test]
fn source_frame_export_state_drop_signals_only_its_own_attempt() {
    let mut state = SourceFrameExportUiState::default();
    let controller = state.begin(export_request("export-1")).unwrap();
    drop(state);

    assert!(controller.is_cancelled("export-1"));
    assert!(!controller.is_cancelled("frame-source"));
}

#[test]
fn source_frame_export_blocks_update_installation_until_terminal_callback() {
    let mut root = FrameRoot::new();
    root.update_ui.status = crate::app::updates::state::UpdateStatus::ReadyToInstall(Box::new(
        frame_updater::UpdatePackage {
            version: semver::Version::new(0, 34, 1),
            channel: frame_updater::UpdateChannel::Stable,
            asset_key: frame_updater::PlatformAssetKey::MacosAarch64,
            kind: frame_updater::UpdateAssetKind::MacosAppZip,
            file_name: "Frame.zip".into(),
            path: PathBuf::from("/tmp/Frame.zip"),
            size_bytes: 1,
            sha256: "00".repeat(32),
            installer_args: Vec::new(),
        },
    ));
    assert!(root.can_install_downloaded_update());
    root.preview_ui
        .source_export
        .begin(export_request("export-before-update"))
        .unwrap();
    assert!(
        !root.can_install_downloaded_update(),
        "installation must wait for a source frame export"
    );
    root.preview_ui.source_export.cancel().unwrap();
    assert!(
        !root.can_install_downloaded_update(),
        "requesting cancellation does not settle the export"
    );
    root.preview_ui.source_export.complete(
        "export-before-update",
        Ok(SourceFrameExportOutcome::Cancelled),
    );
    assert!(root.can_install_downloaded_update());
}
