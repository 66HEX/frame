//! Cross-platform native dialogs used by the GPUI frontend.

use std::path::PathBuf;

use crate::file_filters::{
    AUDIO_FILE_EXTENSIONS, IMAGE_FILE_EXTENSIONS, SOURCE_FILE_EXTENSIONS, SUBTITLE_FILE_EXTENSIONS,
    VIDEO_FILE_EXTENSIONS,
};

pub fn pick_source_files() -> Option<Vec<PathBuf>> {
    source_file_dialog().pick_files()
}

pub fn pick_subtitle_file() -> Option<PathBuf> {
    subtitle_file_dialog().pick_file()
}

fn source_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title("Add Source")
        .add_filter("Media Files", SOURCE_FILE_EXTENSIONS)
        .add_filter("Videos", VIDEO_FILE_EXTENSIONS)
        .add_filter("Audio", AUDIO_FILE_EXTENSIONS)
        .add_filter("Images", IMAGE_FILE_EXTENSIONS)
}

fn subtitle_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title("Select subtitle file")
        .add_filter("Subtitles", SUBTITLE_FILE_EXTENSIONS)
}
