use super::*;

#[test]
fn toggle_selected_crop_mode_initializes_default_video_draft() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );

    let changed = root.toggle_selected_crop_mode();

    assert!(changed);
    assert!(root.preview_ui.crop_mode);
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
    assert_eq!(root.preview_ui.crop_aspect, "free");
}

#[test]
fn select_preview_crop_aspect_keeps_side_rotation_preview_square() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );

    assert!(root.rotate_selected_preview());
    assert!(root.toggle_selected_crop_mode());
    assert!(root.select_preview_crop_aspect("1:1"));

    let config = &root.file_queue.file_by_id("video").unwrap().config;
    let metadata_entry = root.source_metadata.entry_for("video");
    let crop_state = root.preview_crop_render_state(metadata_entry.metadata.as_ref(), config);
    let visual_rect = preview_crop_visual_rect(&crop_state);
    let visual_width = visual_rect.width * 1080.0;
    let visual_height = visual_rect.height * 1920.0;

    assert!((visual_width - visual_height).abs() < 0.000_001);
    assert!(root.apply_selected_crop());
    let crop = root
        .file_queue
        .file_by_id("video")
        .and_then(|file| file.config.crop.as_ref())
        .expect("crop settings");
    assert_eq!(crop.width, crop.height);
}

#[test]
fn apply_selected_crop_writes_selected_file_crop_pixels() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
    root.file_queue
        .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
    root.file_queue.select_existing_file("second");
    root.source_metadata.mark_ready(
        "second".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );
    root.preview_ui.crop_mode = true;
    root.preview_ui.draft_crop = Some(CropRect {
        x: 0.25,
        y: 0.25,
        width: 0.5,
        height: 0.5,
    });
    root.preview_ui.crop_aspect = "16:9".to_string();

    let changed = root.apply_selected_crop();

    assert!(changed);
    assert_eq!(
        root.file_queue
            .file_by_id("first")
            .and_then(|file| file.config.crop.as_ref()),
        None
    );
    assert_eq!(
        root.file_queue
            .file_by_id("second")
            .and_then(|file| file.config.crop.as_ref()),
        Some(&CropSettings {
            enabled: true,
            x: 480,
            y: 270,
            width: 960,
            height: 540,
            source_width: Some(1920),
            source_height: Some(1080),
            aspect_ratio: Some("16:9".to_string()),
        })
    );
}

#[test]
fn apply_selected_full_crop_clears_existing_crop() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );
    root.update_selected_config(|config| {
        config.crop = Some(CropSettings {
            enabled: true,
            x: 100,
            y: 100,
            width: 1000,
            height: 600,
            source_width: Some(1920),
            source_height: Some(1080),
            aspect_ratio: None,
        });
        true
    });
    root.preview_ui.crop_mode = true;
    root.preview_ui.draft_crop = Some(full_crop_rect());

    let changed = root.apply_selected_crop();

    assert!(changed);
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.crop.as_ref()),
        None
    );
}

#[test]
fn rotate_and_flip_preview_update_selected_config() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );

    assert!(root.rotate_selected_preview());
    assert!(root.toggle_selected_flip(FlipAxis::Horizontal));

    let config = &root.file_queue.file_by_id("video").unwrap().config;
    assert_eq!(config.rotation, "90");
    assert!(config.flip_horizontal);
    assert!(!config.flip_vertical);
}

#[test]
fn apply_preview_crop_drag_updates_draft_without_persisting_config() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );
    root.preview_ui.crop_mode = true;
    root.preview_ui.draft_crop = Some(CropRect {
        x: 0.10,
        y: 0.10,
        width: 0.50,
        height: 0.50,
    });

    assert!(!root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.50, y: 0.50 },));
    assert!(root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.60, y: 0.55 },));

    let draft = root.preview_ui.draft_crop.unwrap();
    assert!((draft.x - 0.20).abs() < 0.000_001);
    assert!((draft.y - 0.15).abs() < 0.000_001);
    assert_eq!(draft.width, 0.50);
    assert_eq!(draft.height, 0.50);
    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.crop.as_ref()),
        None
    );
}

#[test]
fn apply_preview_crop_drag_moves_visual_rect_after_side_rotation() {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
    root.source_metadata.mark_ready(
        "video".to_string(),
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );

    assert!(root.rotate_selected_preview());
    assert!(root.toggle_selected_crop_mode());
    root.preview_ui.draft_crop = Some(default_crop_rect());
    assert!(root.select_preview_crop_aspect("1:1"));

    let config = &root.file_queue.file_by_id("video").unwrap().config;
    let metadata_entry = root.source_metadata.entry_for("video");
    let crop_state = root.preview_crop_render_state(metadata_entry.metadata.as_ref(), config);
    let before = preview_crop_visual_rect(&crop_state);

    assert!(!root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.50, y: 0.50 }));
    assert!(root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.60, y: 0.50 }));

    let config = &root.file_queue.file_by_id("video").unwrap().config;
    let metadata_entry = root.source_metadata.entry_for("video");
    let crop_state = root.preview_crop_render_state(metadata_entry.metadata.as_ref(), config);
    let after = preview_crop_visual_rect(&crop_state);

    assert!((after.x - before.x - 0.10).abs() < 0.000_001);
    assert!((after.y - before.y).abs() < 0.000_001);
}

#[test]
fn pending_crop_source_keeps_draft_but_hides_and_blocks_old_frame_handles() {
    let mut root = FrameRoot::new();
    seed_ready_video(&mut root, "crop-pending", "/tmp/crop.mkv");
    let entry = root.selected_source_metadata_entry();
    let accepted = root.selected_preview_runtime_request(&entry).unwrap();
    root.preview_ui.render_image = root_with_preview_canvas_media(128, 128)
        .preview_ui
        .render_image
        .clone();
    root.preview_ui.rendered_runtime_key = Some(accepted.key);
    root.preview_ui.crop_mode = true;
    root.preview_ui.draft_crop = Some(CropRect {
        x: 0.1,
        y: 0.1,
        width: 0.5,
        height: 0.5,
    });
    let draft = root.preview_ui.draft_crop;
    let editing = root.selected_preview_runtime_request(&entry).unwrap();
    root.preview_ui.desired_runtime_key = Some(editing.key.clone());
    root.preview_ui.pending_runtime_key = Some(editing.key.clone());
    let render =
        root.preview_crop_render_state(entry.metadata.as_ref(), root.selected_config().unwrap());
    assert!(render.crop_mode);
    assert_eq!(render.draft_crop, draft);
    assert!(!render.frame_ready);
    assert!(!root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.3, y: 0.3 }));
    assert!(!root.adjust_preview_crop_from_keyboard_with_step(DragHandle::Move, "right", false));
    assert_eq!(root.preview_ui.draft_crop, draft);
    root.preview_ui.pending_runtime_key = None;
    // Completion without a matching frame still does not make old pixels editable.
    root.preview_ui.runtime_key = Some(editing.key.clone());
    assert!(!root.preview_edit_frame_ready());
    root.preview_ui.rendered_runtime_key = Some(editing.key);
    assert!(root.preview_edit_frame_ready());
    assert!(root.adjust_preview_crop_from_keyboard_with_step(DragHandle::Move, "right", false));
}
