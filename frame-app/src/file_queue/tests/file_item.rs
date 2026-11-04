use super::*;
use std::path::PathBuf;

#[test]
fn from_path_derives_name_from_unix_path() {
    let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

    assert_eq!(file.name, "video.mp4");
}

#[test]
fn from_path_derives_name_from_windows_path() {
    let file = FileItem::from_path("1", r"C:\Users\hex\video.mp4", 10);

    assert_eq!(file.name, "video.mp4");
}

#[test]
fn from_path_initializes_conversion_selection_like_original_add_flow() {
    let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

    assert!(file.is_selected_for_conversion);
}

#[test]
fn from_path_initializes_per_file_conversion_config() {
    let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

    assert_eq!(file.config, ConversionConfig::default());
}

#[test]
fn from_os_path_reads_size_from_existing_file() {
    let path = temp_file_path("size-source.mp4");
    std::fs::write(&path, [1_u8, 2, 3]).expect("test file should be written");

    let file = FileItem::from_os_path("1", &path);

    std::fs::remove_file(&path).expect("test file should be removed");
    assert_eq!(file.size_bytes, 3);
}

#[test]
fn from_os_path_uses_zero_size_when_metadata_is_unavailable() {
    let path = temp_file_path("missing-source.mp4");

    let file = FileItem::from_os_path("1", &path);

    assert_eq!(file.size_bytes, 0);
}

#[test]
fn converting_row_state_uses_progress_percent() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Converting;
    file.progress_percent = 42;

    assert_eq!(file.row_state_label(), "42%");
    assert_eq!(file.row_state_tone(), FileStateTone::Amber);
}

#[test]
fn completed_row_state_matches_ready_label() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Completed;

    assert_eq!(file.row_state_label(), "ready");
    assert_eq!(file.row_state_tone(), FileStateTone::Blue);
}

#[test]
fn error_row_state_uses_red_tone() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Error;

    assert_eq!(file.row_state_label(), "error");
    assert_eq!(file.row_state_tone(), FileStateTone::Red);
}

#[test]
fn converting_row_can_pause_but_not_delete_directly() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Converting;

    assert_eq!(
        file.row_actions(),
        RowActionAvailability {
            primary: RowPrimaryAction::Pause,
            secondary: RowSecondaryAction::Cancel,
        }
    );
}

#[test]
fn paused_row_can_resume_or_cancel_without_removing_source() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Paused;

    assert_eq!(
        file.row_actions(),
        RowActionAvailability {
            primary: RowPrimaryAction::Resume,
            secondary: RowSecondaryAction::Cancel,
        }
    );
}

#[test]
fn completed_row_can_be_prepared_again_or_removed() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Completed;

    assert_eq!(
        file.row_actions(),
        RowActionAvailability {
            primary: RowPrimaryAction::Reconvert,
            secondary: RowSecondaryAction::Delete,
        }
    );
}

#[test]
fn cancelling_row_has_no_repeatable_actions() {
    let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
    file.status = FileStatus::Cancelling;
    file.progress_percent = 42;

    assert_eq!(file.row_actions(), RowActionAvailability::default());
    assert_eq!(file.row_state_label(), "42%");
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("frame-app-{}-{name}", std::process::id()))
}
