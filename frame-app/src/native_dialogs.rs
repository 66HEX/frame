//! Cross-platform native dialogs used by the GPUI frontend.

use std::path::PathBuf;

use crate::file_filters::{
    AUDIO_FILE_EXTENSIONS, IMAGE_FILE_EXTENSIONS, SOURCE_FILE_EXTENSIONS, SUBTITLE_FILE_EXTENSIONS,
    VIDEO_FILE_EXTENSIONS,
};

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

pub const SOURCE_FILE_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Add Source",
    filters: &SOURCE_FILE_DIALOG_FILTERS,
    allows_multiple: true,
};

pub const SUBTITLE_FILE_DIALOG_SPEC: NativeDialogSpec = NativeDialogSpec {
    title: "Select subtitle file",
    filters: &SUBTITLE_FILE_DIALOG_FILTERS,
    allows_multiple: false,
};

pub fn pick_source_files() -> Option<Vec<PathBuf>> {
    source_file_dialog().pick_files()
}

pub fn pick_subtitle_file() -> Option<PathBuf> {
    subtitle_file_dialog().pick_file()
}

fn source_file_dialog() -> rfd::FileDialog {
    file_dialog_from_spec(SOURCE_FILE_DIALOG_SPEC)
}

fn subtitle_file_dialog() -> rfd::FileDialog {
    file_dialog_from_spec(SUBTITLE_FILE_DIALOG_SPEC)
}

fn file_dialog_from_spec(spec: NativeDialogSpec) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new().set_title(spec.title);
    for filter in spec.filters {
        dialog = dialog.add_filter(filter.label, filter.extensions);
    }
    dialog
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
    fn dialog_specs_capture_selection_mode() {
        const {
            assert!(SOURCE_FILE_DIALOG_SPEC.allows_multiple);
            assert!(!SUBTITLE_FILE_DIALOG_SPEC.allows_multiple);
        }
    }
}
