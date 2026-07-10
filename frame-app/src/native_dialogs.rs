//! Cross-platform native dialogs used by the GPUI frontend.

use std::path::PathBuf;

use crate::file_filters::{
    AUDIO_FILE_EXTENSIONS, IMAGE_FILE_EXTENSIONS, SOURCE_FILE_EXTENSIONS, SUBTITLE_FILE_EXTENSIONS,
    VIDEO_FILE_EXTENSIONS,
};
use gpui::Window;
use rfd::{AsyncFileDialog, FileHandle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeDialogFilterSpec {
    pub label: &'static str,
    pub extensions: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeDialogSpec {
    pub title: &'static str,
    pub filters: &'static [NativeDialogFilterSpec],
    pub allows_multiple: bool,
}

pub const SOURCE_FILE_DIALOG_FILTERS: [NativeDialogFilterSpec; 4] = [
    NativeDialogFilterSpec {
        label: "Media Files",
        extensions: SOURCE_FILE_EXTENSIONS,
    },
    NativeDialogFilterSpec {
        label: "Videos",
        extensions: VIDEO_FILE_EXTENSIONS,
    },
    NativeDialogFilterSpec {
        label: "Audio",
        extensions: AUDIO_FILE_EXTENSIONS,
    },
    NativeDialogFilterSpec {
        label: "Images",
        extensions: IMAGE_FILE_EXTENSIONS,
    },
];

pub const SUBTITLE_FILE_DIALOG_FILTERS: [NativeDialogFilterSpec; 1] = [NativeDialogFilterSpec {
    label: "Subtitles",
    extensions: SUBTITLE_FILE_EXTENSIONS,
}];

pub const OVERLAY_IMAGE_DIALOG_FILTERS: [NativeDialogFilterSpec; 1] = [NativeDialogFilterSpec {
    label: "Images",
    extensions: IMAGE_FILE_EXTENSIONS,
}];

pub const SOURCE_FILE_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Add Source",
    filters: &SOURCE_FILE_DIALOG_FILTERS,
    allows_multiple: true,
};

pub const SOURCE_FOLDER_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Open Folder",
    filters: &[],
    allows_multiple: false,
};

pub const OUTPUT_FOLDER_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Choose Default Output Folder",
    filters: &[],
    allows_multiple: false,
};

pub const SUBTITLE_FILE_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Select subtitle file",
    filters: &SUBTITLE_FILE_DIALOG_FILTERS,
    allows_multiple: false,
};

pub const OVERLAY_IMAGE_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Select overlay image",
    filters: &OVERLAY_IMAGE_DIALOG_FILTERS,
    allows_multiple: false,
};

pub async fn pick_source_files(dialog: AsyncFileDialog) -> Option<Vec<PathBuf>> {
    dialog
        .pick_files()
        .await
        .as_deref()
        .map(file_handles_to_paths)
}

pub async fn pick_source_folder(dialog: AsyncFileDialog) -> Option<PathBuf> {
    dialog.pick_folder().await.as_ref().map(file_handle_to_path)
}

pub async fn pick_output_folder(dialog: AsyncFileDialog) -> Option<PathBuf> {
    dialog.pick_folder().await.as_ref().map(file_handle_to_path)
}

pub async fn pick_subtitle_file(dialog: AsyncFileDialog) -> Option<PathBuf> {
    dialog.pick_file().await.as_ref().map(file_handle_to_path)
}

pub async fn pick_overlay_image_file(dialog: AsyncFileDialog) -> Option<PathBuf> {
    dialog.pick_file().await.as_ref().map(file_handle_to_path)
}

#[must_use]
pub fn source_file_dialog(parent: &Window) -> AsyncFileDialog {
    file_dialog_from_spec(SOURCE_FILE_DIALOG_SPEC).set_parent(parent)
}

#[must_use]
pub fn source_folder_dialog(parent: &Window) -> AsyncFileDialog {
    file_dialog_from_spec(SOURCE_FOLDER_DIALOG_SPEC).set_parent(parent)
}

#[must_use]
pub fn output_folder_dialog(parent: &Window) -> AsyncFileDialog {
    file_dialog_from_spec(OUTPUT_FOLDER_DIALOG_SPEC).set_parent(parent)
}

#[must_use]
pub fn subtitle_file_dialog(parent: &Window) -> AsyncFileDialog {
    file_dialog_from_spec(SUBTITLE_FILE_DIALOG_SPEC).set_parent(parent)
}

#[must_use]
pub fn overlay_image_dialog(parent: &Window) -> AsyncFileDialog {
    file_dialog_from_spec(OVERLAY_IMAGE_DIALOG_SPEC).set_parent(parent)
}

fn file_dialog_from_spec(spec: NativeDialogSpec) -> AsyncFileDialog {
    let mut dialog = AsyncFileDialog::new().set_title(spec.title);
    for filter in spec.filters {
        dialog = dialog.add_filter(filter.label, filter.extensions);
    }
    dialog
}

fn file_handles_to_paths(handles: &[FileHandle]) -> Vec<PathBuf> {
    handles.iter().map(file_handle_to_path).collect()
}

fn file_handle_to_path(handle: &FileHandle) -> PathBuf {
    handle.path().to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_dialog_spec_matches_runtime_source_validation_extensions() {
        assert_eq!(
            SOURCE_FILE_DIALOG_SPEC.filters[0],
            NativeDialogFilterSpec {
                label: "Media Files",
                extensions: SOURCE_FILE_EXTENSIONS,
            }
        );
    }

    #[test]
    fn source_dialog_spec_keeps_platform_group_filters() {
        assert_eq!(
            SOURCE_FILE_DIALOG_SPEC.filters,
            [
                NativeDialogFilterSpec {
                    label: "Media Files",
                    extensions: SOURCE_FILE_EXTENSIONS,
                },
                NativeDialogFilterSpec {
                    label: "Videos",
                    extensions: VIDEO_FILE_EXTENSIONS,
                },
                NativeDialogFilterSpec {
                    label: "Audio",
                    extensions: AUDIO_FILE_EXTENSIONS,
                },
                NativeDialogFilterSpec {
                    label: "Images",
                    extensions: IMAGE_FILE_EXTENSIONS,
                },
            ]
        );
    }

    #[test]
    fn subtitle_dialog_spec_matches_runtime_subtitle_validation_extensions() {
        assert_eq!(
            SUBTITLE_FILE_DIALOG_SPEC.filters,
            [NativeDialogFilterSpec {
                label: "Subtitles",
                extensions: SUBTITLE_FILE_EXTENSIONS,
            }]
        );
    }

    #[test]
    fn overlay_image_dialog_spec_matches_runtime_image_validation_extensions() {
        assert_eq!(
            OVERLAY_IMAGE_DIALOG_SPEC.filters,
            [NativeDialogFilterSpec {
                label: "Images",
                extensions: IMAGE_FILE_EXTENSIONS,
            }]
        );
    }

    #[test]
    fn dialog_specs_capture_selection_mode() {
        const {
            assert!(SOURCE_FILE_DIALOG_SPEC.allows_multiple);
            assert!(!SOURCE_FOLDER_DIALOG_SPEC.allows_multiple);
            assert!(!SUBTITLE_FILE_DIALOG_SPEC.allows_multiple);
            assert!(!OVERLAY_IMAGE_DIALOG_SPEC.allows_multiple);
        }
    }

    #[test]
    fn source_folder_dialog_spec_uses_folder_selection_title_without_filters() {
        assert_eq!(
            SOURCE_FOLDER_DIALOG_SPEC,
            NativeDialogSpec {
                title: "Open Folder",
                filters: &[],
                allows_multiple: false,
            }
        );
    }
}
