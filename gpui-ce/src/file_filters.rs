//! File extension filters for native source and subtitle pickers.

use std::path::{Path, PathBuf};

pub const VIDEO_FILE_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "webm", "gif"];
pub const AUDIO_FILE_EXTENSIONS: &[&str] = &["mp3", "m4a", "wav", "flac"];
pub const IMAGE_FILE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "avif", "heic", "heif",
];
pub const SOURCE_FILE_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "avi", "webm", "gif", "mp3", "m4a", "wav", "flac", "png", "jpg", "jpeg",
    "webp", "bmp", "tif", "tiff", "avif", "heic", "heif",
];

pub const SUBTITLE_FILE_EXTENSIONS: &[&str] = &["srt", "ass", "vtt"];

#[must_use]
pub fn is_supported_source_path(path: &Path) -> bool {
    path_has_extension(path, SOURCE_FILE_EXTENSIONS)
}

#[must_use]
pub fn is_supported_subtitle_path(path: &Path) -> bool {
    path_has_extension(path, SUBTITLE_FILE_EXTENSIONS)
}

#[must_use]
pub fn filter_supported_source_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|path| is_supported_source_path(path))
        .collect()
}

fn path_has_extension(path: &Path, allowed_extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            allowed_extensions
                .iter()
                .any(|allowed| extension.eq_ignore_ascii_case(allowed))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_supported_source_path_accepts_original_media_extensions() {
        assert!(is_supported_source_path(Path::new("/tmp/clip.MOV")));
        assert!(is_supported_source_path(Path::new("/tmp/still.heif")));
    }

    #[test]
    fn is_supported_source_path_rejects_unknown_extensions() {
        assert!(!is_supported_source_path(Path::new("/tmp/archive.zip")));
        assert!(!is_supported_source_path(Path::new("/tmp/no-extension")));
    }

    #[test]
    fn is_supported_subtitle_path_accepts_original_subtitle_extensions() {
        assert!(is_supported_subtitle_path(Path::new("/tmp/dialogue.srt")));
        assert!(is_supported_subtitle_path(Path::new("/tmp/dialogue.ASS")));
        assert!(is_supported_subtitle_path(Path::new("/tmp/dialogue.vtt")));
    }

    #[test]
    fn filter_supported_source_paths_preserves_only_supported_paths() {
        let paths = filter_supported_source_paths(vec![
            PathBuf::from("/tmp/one.mp4"),
            PathBuf::from("/tmp/readme.txt"),
            PathBuf::from("/tmp/two.PNG"),
        ]);

        assert_eq!(
            paths,
            [PathBuf::from("/tmp/one.mp4"), PathBuf::from("/tmp/two.PNG")]
        );
    }

    #[test]
    fn source_file_extensions_match_original_dialog_groups() {
        let grouped = VIDEO_FILE_EXTENSIONS
            .iter()
            .chain(AUDIO_FILE_EXTENSIONS)
            .chain(IMAGE_FILE_EXTENSIONS)
            .copied()
            .collect::<Vec<_>>();

        assert_eq!(SOURCE_FILE_EXTENSIONS, grouped);
    }
}
