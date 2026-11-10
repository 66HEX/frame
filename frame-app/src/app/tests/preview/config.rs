use super::*;

#[test]
fn update_selected_config_mutates_only_selected_file() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
    root.file_queue.select_existing_file("second");

    root.update_selected_config(|config| {
        config.container = "webm".to_string();
        true
    });

    assert_eq!(
        root.file_queue
            .file_by_id("first")
            .map(|file| file.config.container.as_str()),
        Some("mp4")
    );
    assert_eq!(
        root.file_queue
            .file_by_id("second")
            .map(|file| file.config.container.as_str()),
        Some("webm")
    );
}

#[test]
fn normalize_selected_config_clears_trim_for_selected_image_only() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.file_queue
        .add_file(FileItem::from_path("image", "/tmp/two.png", 1));

    for id in ["video", "image"] {
        root.file_queue.select_existing_file(id);
        root.update_selected_config(|config| {
            config.start_time = Some("00:00:05.000".to_string());
            config.end_time = Some("00:00:30.000".to_string());
            true
        });
    }
    root.file_queue.select_existing_file("image");

    root.normalize_selected_config(Some(&SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    }));

    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.start_time.as_deref()),
        Some("00:00:05.000")
    );
    assert_eq!(
        root.file_queue
            .file_by_id("image")
            .and_then(|file| file.config.start_time.as_deref()),
        None
    );
}
