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
pub fn is_supported_overlay_image_path(path: &Path) -> bool {
    path_has_extension(path, IMAGE_FILE_EXTENSIONS)
}

#[must_use]
pub fn filter_supported_source_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|path| is_supported_source_path(path))
        .collect()
}

#[must_use]
pub fn discover_supported_source_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .flat_map(discover_supported_source_path)
        .collect()
}

fn discover_supported_source_path(path: PathBuf) -> Vec<PathBuf> {
    if path.is_dir() {
        return supported_source_paths_in_directory(&path);
    }

    filter_supported_source_paths(vec![path])
}

fn supported_source_paths_in_directory(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_supported_source_paths_in_directory(root, &mut paths);
    paths.sort();
    paths
}

fn collect_supported_source_paths_in_directory(root: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            collect_supported_source_paths_in_directory(&path, paths);
        } else if file_type.is_file() && is_supported_source_path(&path) {
            paths.push(path);
        }
    }
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
    fn is_supported_overlay_image_path_uses_image_extensions() {
        assert!(is_supported_overlay_image_path(Path::new("/tmp/logo.PNG")));
        assert!(!is_supported_overlay_image_path(Path::new("/tmp/logo.mp4")));
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
    fn discover_supported_source_paths_preserves_supported_file_inputs() {
        let paths = discover_supported_source_paths(vec![
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
    fn discover_supported_source_paths_expands_directories_recursively() {
        let root = unique_test_dir("recursive-discovery");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).expect("test media directory should be created");
        std::fs::write(root.join("clip.mp4"), b"").expect("test video should be written");
        std::fs::write(root.join("notes.txt"), b"").expect("test text file should be written");
        std::fs::write(nested.join("still.PNG"), b"").expect("test image should be written");

        let paths = discover_supported_source_paths(vec![root.clone()]);

        std::fs::remove_dir_all(&root).expect("test media directory should be removed");
        assert_eq!(paths, [root.join("clip.mp4"), nested.join("still.PNG")]);
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

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());

        std::env::temp_dir().join(format!(
            "frame-file-filter-{name}-{}-{nanos}",
            std::process::id()
        ))
    }
}
