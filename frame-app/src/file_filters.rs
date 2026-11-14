//! File extension filters for native source and subtitle pickers.

use crate::settings::SourceKind;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const VIDEO_FILE_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "avi", "webm", "ts", "mts", "m2t", "m2ts", "gif",
];
pub const AUDIO_FILE_EXTENSIONS: &[&str] = &["mp3", "m4a", "wav", "flac", "opus"];
pub const IMAGE_FILE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "avif", "heic", "heif",
];
pub const SOURCE_FILE_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "avi", "webm", "ts", "mts", "m2t", "m2ts", "gif", "mp3", "m4a", "wav",
    "flac", "opus", "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "avif", "heic", "heif",
];

pub const BURN_SUBTITLE_FILE_EXTENSIONS: &[&str] = &["srt", "ass", "vtt"];
pub const SELECTABLE_SUBTITLE_FILE_EXTENSIONS: &[&str] = &["srt", "ass", "vtt", "sup"];
pub const PGS_SUBTITLE_FILE_EXTENSIONS: &[&str] = &["sup"];

#[must_use]
pub fn source_kind_for_extension(extension: &str) -> Option<SourceKind> {
    [
        (VIDEO_FILE_EXTENSIONS, SourceKind::Video),
        (AUDIO_FILE_EXTENSIONS, SourceKind::Audio),
        (IMAGE_FILE_EXTENSIONS, SourceKind::Image),
    ]
    .into_iter()
    .find_map(|(extensions, kind)| {
        extensions
            .iter()
            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            .then_some(kind)
    })
}

#[must_use]
pub fn is_supported_source_path(path: &Path) -> bool {
    path_has_extension(path, SOURCE_FILE_EXTENSIONS)
}

#[must_use]
pub fn is_supported_subtitle_path(path: &Path) -> bool {
    path_has_extension(path, BURN_SUBTITLE_FILE_EXTENSIONS)
}

#[must_use]
pub fn is_supported_selectable_subtitle_path(path: &Path) -> bool {
    path_has_extension(path, SELECTABLE_SUBTITLE_FILE_EXTENSIONS)
}

#[must_use]
pub fn selectable_subtitle_extensions_for_container(container: &str) -> &'static [&'static str] {
    if frame_core::container::is_transport_stream_container(container) {
        PGS_SUBTITLE_FILE_EXTENSIONS
    } else if container.eq_ignore_ascii_case("mkv") {
        SELECTABLE_SUBTITLE_FILE_EXTENSIONS
    } else if matches!(
        container.to_ascii_lowercase().as_str(),
        "mp4" | "mov" | "webm"
    ) {
        BURN_SUBTITLE_FILE_EXTENSIONS
    } else {
        &[]
    }
}

#[must_use]
pub fn is_supported_selectable_subtitle_path_for_container(path: &Path, container: &str) -> bool {
    path_has_extension(
        path,
        selectable_subtitle_extensions_for_container(container),
    )
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
    let mut visited_directories = HashSet::new();
    if let Ok(canonical_root) = std::fs::canonicalize(root) {
        visited_directories.insert(canonical_root);
    }
    collect_supported_source_paths_in_directory(root, &mut paths, &mut visited_directories);
    paths.sort();
    paths
}

fn collect_supported_source_paths_in_directory(
    root: &Path,
    paths: &mut Vec<PathBuf>,
    visited_directories: &mut HashSet<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        // `DirEntry::file_type` does not follow symlinks, so a symlinked
        // directory reports neither `is_dir()` nor `is_file()` and was
        // silently skipped. `metadata` follows symlinks instead, so linked
        // directories are traversed and linked files are collected under
        // the path the user actually picked.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };

        if metadata.is_dir() {
            // Guard against symlink loops: only recurse into a directory
            // whose canonical path is not already on the current walk. A
            // directory reached through a different symlink path is still
            // traversed, so the same files can legitimately appear under
            // each distinct path.
            let Ok(canonical_path) = std::fs::canonicalize(&path) else {
                continue;
            };
            if !visited_directories.insert(canonical_path.clone()) {
                continue;
            }
            collect_supported_source_paths_in_directory(&path, paths, visited_directories);
            visited_directories.remove(&canonical_path);
        } else if metadata.is_file() && is_supported_source_path(&path) {
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
    fn source_kind_for_extension_classifies_all_supported_formats_case_insensitively() {
        for (extensions, expected) in [
            (VIDEO_FILE_EXTENSIONS, SourceKind::Video),
            (AUDIO_FILE_EXTENSIONS, SourceKind::Audio),
            (IMAGE_FILE_EXTENSIONS, SourceKind::Image),
        ] {
            for extension in extensions {
                for spelling in [extension.to_string(), extension.to_ascii_uppercase()] {
                    assert_eq!(
                        source_kind_for_extension(&spelling),
                        Some(expected),
                        "{spelling}"
                    );
                }
            }
        }
        assert_eq!(source_kind_for_extension("gIf"), Some(SourceKind::Video));
    }

    #[test]
    fn source_kind_for_extension_leaves_unknown_formats_unclassified() {
        for extension in ["", "unknown", "zip", "srt", ".mp4", "clip.mp4"] {
            assert_eq!(source_kind_for_extension(extension), None, "{extension}");
        }
    }

    #[test]
    fn is_supported_source_path_accepts_original_media_extensions() {
        assert!(is_supported_source_path(Path::new("/tmp/clip.MOV")));
        assert!(is_supported_source_path(Path::new("/tmp/still.heif")));
    }

    #[test]
    fn transport_stream_extensions_are_supported_case_insensitively() {
        for extension in ["ts", "TS", "tS", "m2t", "MTS", "M2TS"] {
            let path = format!("clip.{extension}");
            assert!(is_supported_source_path(Path::new(&path)), "{path}");
            assert_eq!(
                source_kind_for_extension(extension),
                Some(SourceKind::Video),
                "{path}"
            );
        }
    }

    #[test]
    fn sup_is_selectable_but_not_offered_for_text_burn_in() {
        let path = Path::new("/tmp/captions.SUP");
        assert!(is_supported_selectable_subtitle_path(path));
        assert!(!is_supported_subtitle_path(path));
    }

    #[test]
    fn transport_stream_selectable_subtitles_accept_only_pgs_sidecars() {
        for container in ["m2t", "mts", "m2ts"] {
            assert!(is_supported_selectable_subtitle_path_for_container(
                Path::new("/tmp/captions.SUP"),
                container
            ));
            assert!(!is_supported_selectable_subtitle_path_for_container(
                Path::new("/tmp/captions.ass"),
                container
            ));
        }
    }

    #[test]
    fn mp4_selectable_subtitles_accept_text_but_not_pgs_sidecars() {
        assert!(is_supported_selectable_subtitle_path_for_container(
            Path::new("/tmp/captions.vtt"),
            "mp4"
        ));
        assert!(!is_supported_selectable_subtitle_path_for_container(
            Path::new("/tmp/captions.sup"),
            "mp4"
        ));
    }

    #[test]
    fn is_supported_source_path_rejects_unknown_extensions() {
        assert!(!is_supported_source_path(Path::new("/tmp/readme.txt")));
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
            PathBuf::from("/tmp/transport.ts"),
            PathBuf::from("/tmp/archive.zip"),
            PathBuf::from("/tmp/no-extension"),
            PathBuf::from("/tmp/two.PNG"),
        ]);

        assert_eq!(
            paths,
            [
                PathBuf::from("/tmp/one.mp4"),
                PathBuf::from("/tmp/transport.ts"),
                PathBuf::from("/tmp/two.PNG")
            ]
        );
    }

    #[test]
    fn discover_supported_source_paths_preserves_supported_file_inputs() {
        let paths = discover_supported_source_paths(vec![
            PathBuf::from("/tmp/one.mp4"),
            PathBuf::from("/tmp/readme.txt"),
            PathBuf::from("/tmp/transport.TS"),
            PathBuf::from("/tmp/archive.zip"),
            PathBuf::from("/tmp/no-extension"),
            PathBuf::from("/tmp/two.PNG"),
        ]);

        assert_eq!(
            paths,
            [
                PathBuf::from("/tmp/one.mp4"),
                PathBuf::from("/tmp/transport.TS"),
                PathBuf::from("/tmp/two.PNG")
            ]
        );
    }

    #[test]
    fn discover_supported_source_paths_expands_directories_recursively() {
        let root = unique_test_dir("recursive-discovery");
        let media = root.join("media");
        let nested = media.join("nested");
        std::fs::create_dir_all(&nested).expect("test media directory should be created");
        for path in [
            root.join("direct.tS"),
            root.join("after.flac"),
            media.join("clip.mp4"),
            media.join("transport.ts"),
            media.join("notes.txt"),
            nested.join("archive.zip"),
            nested.join("no-extension"),
            nested.join("still.PNG"),
            nested.join("transport.TS"),
        ] {
            std::fs::write(path, b"").expect("test source file should be written");
        }

        let paths = discover_supported_source_paths(vec![
            root.join("direct.tS"),
            media.join("notes.txt"),
            media.clone(),
            nested.join("archive.zip"),
            root.join("after.flac"),
        ]);

        std::fs::remove_dir_all(&root).expect("test media directory should be removed");
        assert_eq!(
            paths,
            [
                root.join("direct.tS"),
                media.join("clip.mp4"),
                nested.join("still.PNG"),
                nested.join("transport.TS"),
                media.join("transport.ts"),
                root.join("after.flac"),
            ]
        );
    }

    #[test]
    #[cfg(unix)]
    fn discover_supported_source_paths_follows_symlinked_directories() {
        let root = unique_test_dir("symlinked-directory");
        let target = unique_test_dir("symlinked-directory-target");
        std::fs::create_dir_all(&root).expect("test media directory should be created");
        std::fs::create_dir_all(&target).expect("test media directory should be created");
        std::fs::write(target.join("clip.mp4"), b"").expect("test source file should be written");
        std::fs::write(target.join("notes.txt"), b"").expect("test source file should be written");
        std::os::unix::fs::symlink(&target, root.join("linked"))
            .expect("symlinked directory should be created");

        let paths = discover_supported_source_paths(vec![root.clone()]);

        std::fs::remove_dir_all(&root).expect("test media directory should be removed");
        std::fs::remove_dir_all(&target).expect("test media directory should be removed");
        assert_eq!(paths, [root.join("linked").join("clip.mp4")]);
    }

    #[test]
    #[cfg(unix)]
    fn discover_supported_source_paths_skips_symlink_loops_and_duplicate_targets() {
        let root = unique_test_dir("symlink-loop");
        let target = root.join("target");
        std::fs::create_dir_all(&target).expect("test media directory should be created");
        std::fs::write(target.join("clip.mp4"), b"").expect("test source file should be written");
        std::os::unix::fs::symlink(&target, root.join("linked"))
            .expect("symlinked directory should be created");
        std::os::unix::fs::symlink(&target, root.join("also-linked"))
            .expect("symlinked directory should be created");
        std::os::unix::fs::symlink(&root, target.join("loop-back"))
            .expect("symlinked directory should be created");

        let paths = discover_supported_source_paths(vec![root.clone()]);

        std::fs::remove_dir_all(&root).expect("test media directory should be removed");
        assert_eq!(
            paths,
            [
                root.join("also-linked").join("clip.mp4"),
                root.join("linked").join("clip.mp4"),
                root.join("target").join("clip.mp4"),
            ]
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
