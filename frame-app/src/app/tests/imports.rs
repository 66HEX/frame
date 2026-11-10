use super::*;

#[test]
fn allocate_file_imports_assigns_incrementing_ids() {
    let mut root = FrameRoot::new();

    let imports = root.allocate_file_imports(vec![
        PathBuf::from("/tmp/one.mp4"),
        PathBuf::from("/tmp/two.mp4"),
    ]);

    assert_eq!(imports[0].0, "file-1");
    assert_eq!(imports[1].0, "file-2");
}

#[test]
fn drag_drop_overlay_stays_present_until_close_motion_finishes() {
    let mut root = FrameRoot::new();
    root.drag_drop_ui.is_open = true;
    root.drag_drop_ui.is_present = true;

    assert!(root.close_drag_drop_overlay());

    assert!(!root.drag_drop_ui.is_open);
    assert!(root.drag_drop_ui.is_present);
    assert!(root.finish_drag_drop_overlay_close());
    assert!(!root.drag_drop_ui.is_present);
    assert!(!root.finish_drag_drop_overlay_close());
}

#[test]
fn drag_drop_overlay_open_is_stable_without_pointer_motion() {
    let mut root = FrameRoot::new();

    assert!(root.open_drag_drop_overlay());
    assert!(!root.open_drag_drop_overlay());
    assert!(root.drag_drop_ui.is_open);
    assert!(root.drag_drop_ui.is_present);
}

#[test]
fn allocate_file_imports_continues_after_previous_batch() {
    let mut root = FrameRoot::new();
    root.allocate_file_imports(vec![PathBuf::from("/tmp/one.mp4")]);

    let imports = root.allocate_file_imports(vec![PathBuf::from("/tmp/two.mp4")]);

    assert_eq!(imports[0].0, "file-2");
}

#[test]
fn allocate_file_imports_returns_empty_for_empty_drop() {
    let mut root = FrameRoot::new();

    let imports = root.allocate_file_imports(Vec::new());

    assert!(imports.is_empty());
}

#[test]
fn allocate_file_imports_skips_unsupported_source_extensions() {
    let mut root = FrameRoot::new();

    let imports = root.allocate_file_imports(vec![
        PathBuf::from("/tmp/one.mp4"),
        PathBuf::from("/tmp/readme.txt"),
        PathBuf::from("/tmp/transport.TS"),
        PathBuf::from("/tmp/archive.zip"),
        PathBuf::from("/tmp/mixed.tS"),
        PathBuf::from("/tmp/no-extension"),
        PathBuf::from("/tmp/lower.ts"),
        PathBuf::from("/tmp/two.PNG"),
        PathBuf::from("/tmp/export.opus"),
        PathBuf::from("/tmp/export.OPUS"),
    ]);

    assert_eq!(
        imports,
        [
            ("file-1".to_string(), PathBuf::from("/tmp/one.mp4")),
            ("file-2".to_string(), PathBuf::from("/tmp/transport.TS")),
            ("file-3".to_string(), PathBuf::from("/tmp/mixed.tS")),
            ("file-4".to_string(), PathBuf::from("/tmp/lower.ts")),
            ("file-5".to_string(), PathBuf::from("/tmp/two.PNG")),
            ("file-6".to_string(), PathBuf::from("/tmp/export.opus")),
            ("file-7".to_string(), PathBuf::from("/tmp/export.OPUS")),
        ]
    );
}

#[test]
fn allocate_file_imports_does_not_advance_ids_for_unsupported_sources() {
    let mut root = FrameRoot::new();
    root.allocate_file_imports(vec![
        PathBuf::from("/tmp/readme.txt"),
        PathBuf::from("/tmp/archive.zip"),
        PathBuf::from("/tmp/no-extension"),
    ]);

    let imports = root.allocate_file_imports(vec![PathBuf::from("/tmp/clip.mov")]);

    assert_eq!(imports[0].0, "file-1");
}
