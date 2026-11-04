use super::*;

#[test]
fn apply_preview_overlay_drag_updates_draft_until_done_persists_position() {
    let mut root = root_with_overlay();

    assert!(root.apply_preview_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.50,
            y: 0.50,
            width: Some(0.20),
            height: Some(0.20),
        },
    ));
    assert!(root.apply_preview_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.60,
            y: 0.55,
            width: Some(0.20),
            height: Some(0.20),
        },
    ));

    let committed_overlay = root
        .file_queue
        .file_by_id("video")
        .and_then(|file| file.config.overlay.as_ref())
        .unwrap();
    assert!((committed_overlay.x - 0.50).abs() < 0.000_001);
    assert!((committed_overlay.y - 0.50).abs() < 0.000_001);

    let draft = root.preview_ui.overlay.overlay().unwrap();
    assert!((draft.x - 0.60).abs() < 0.000_001);
    assert!((draft.y - 0.55).abs() < 0.000_001);

    assert!(root.set_selected_overlay_mode(false));
    let overlay = root
        .file_queue
        .file_by_id("video")
        .and_then(|file| file.config.overlay.as_ref())
        .unwrap();
    assert!((overlay.x - 0.60).abs() < 0.000_001);
    assert!((overlay.y - 0.55).abs() < 0.000_001);
    assert_eq!(overlay.anchor, "custom");
}

#[test]
fn commit_preview_overlay_opacity_at_position_updates_draft_until_done() {
    let mut root = root_with_overlay();
    root.set_preview_overlay_opacity_slider_bounds(Bounds {
        origin: point(px(10.0), px(0.0)),
        size: size(px(100.0), px(30.0)),
    });

    assert!(root.commit_preview_overlay_opacity_at_position(point(px(60.0), px(0.0))));

    let committed_overlay = root
        .file_queue
        .file_by_id("video")
        .and_then(|file| file.config.overlay.as_ref())
        .unwrap();
    assert!((committed_overlay.opacity - 1.0).abs() < 0.000_001);

    let draft = root.preview_ui.overlay.overlay().unwrap();
    assert!((draft.opacity - 0.50).abs() < 0.000_001);

    assert!(root.set_selected_overlay_mode(false));
    let overlay = root
        .file_queue
        .file_by_id("video")
        .and_then(|file| file.config.overlay.as_ref())
        .unwrap();
    assert!((overlay.opacity - 0.50).abs() < 0.000_001);
}

#[test]
fn remove_selected_overlay_clears_selected_file_overlay_config() {
    let mut root = root_with_overlay();

    assert!(root.remove_selected_overlay());

    assert_eq!(
        root.file_queue
            .file_by_id("video")
            .and_then(|file| file.config.overlay.as_ref()),
        None
    );
    assert!(root.preview_ui.overlay.overlay().is_none());
}
