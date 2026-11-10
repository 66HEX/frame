use super::*;

#[test]
fn unsupported_output_suspends_an_open_overlay_editor_and_keeps_its_draft() {
    for target in ["gif", "mp3", "copy"] {
        let mut root = root_with_overlay();
        root.preview_ui.overlay_file_id = Some("video".to_string());
        assert!(root.set_selected_overlay_opacity(0.4));
        assert!(root.apply_preview_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.5,
                y: 0.5,
                width: Some(0.2),
                height: Some(0.2),
            }
        ));
        assert!(root.apply_preview_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.6,
                y: 0.55,
                width: Some(0.2),
                height: Some(0.2),
            }
        ));
        let draft = root.preview_ui.overlay.overlay().unwrap().clone();
        root.update_selected_config(|config| {
            if target == "copy" {
                crate::settings::apply_processing_mode(
                    config,
                    None,
                    crate::settings::ProcessingMode::Copy,
                )
            } else {
                crate::settings::apply_output_container(config, target)
            }
        });
        assert!(!root.preview_ui.overlay.overlay_mode());
        assert!(!root.preview_ui.overlay.is_dragging());
        assert_eq!(root.preview_ui.overlay.render_overlay(), None);
        assert!(!root.set_selected_overlay_mode(true));
        assert!(!root.set_selected_overlay_mode(false));
        let stored = root.selected_config().unwrap().overlay.as_ref().unwrap();
        assert!(!stored.enabled);
        assert_eq!(stored.path, draft.path);
        assert!((stored.x - draft.x).abs() < 0.000_001);
        assert!((stored.y - draft.y).abs() < 0.000_001);
        assert!((stored.opacity - draft.opacity).abs() < 0.000_001);

        root.update_selected_config(|config| {
            crate::settings::apply_output_container(config, "mp4");
            crate::settings::apply_processing_mode(
                config,
                None,
                crate::settings::ProcessingMode::Reencode,
            );
            true
        });
        assert!(root.set_selected_overlay_mode(true));
        assert!(root.preview_ui.overlay.render_overlay().unwrap().enabled);
        assert!(root.set_selected_overlay_mode(false));
        assert!(
            root.selected_config()
                .unwrap()
                .overlay
                .as_ref()
                .unwrap()
                .enabled
        );
    }
}

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

#[test]
fn pending_overlay_source_blocks_drag_arrows_and_size_shortcuts_until_matching_frame() {
    let mut root = root_with_overlay();
    let entry = root.selected_source_metadata_entry();
    let editing = root.selected_preview_runtime_request(&entry).unwrap();
    let mut accepted = editing.key.clone();
    accepted.edit_overlay = false;
    root.preview_ui.render_image = root_with_preview_canvas_media(128, 128)
        .preview_ui
        .render_image
        .clone();
    root.preview_ui.rendered_runtime_key = Some(accepted);
    root.preview_ui.desired_runtime_key = Some(editing.key.clone());
    root.preview_ui.pending_runtime_key = Some(editing.key.clone());
    let draft = root.preview_ui.overlay.overlay().cloned();
    assert_eq!(
        root.preview_overlay_render_state().overlay.as_ref(),
        draft.as_ref()
    );
    assert!(!root.preview_overlay_render_state().frame_ready);
    assert!(!root.apply_preview_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.4,
            y: 0.4,
            width: None,
            height: None
        }
    ));
    assert!(!root.adjust_preview_overlay_from_keyboard_with_step(
        OverlayDragHandle::Move,
        "right",
        None,
        false
    ));
    for direction in [
        crate::preview::OverlaySizeDirection::Increase,
        crate::preview::OverlaySizeDirection::Decrease,
    ] {
        assert!(!root.nudge_selected_overlay_size(direction, None));
    }
    assert_eq!(root.preview_ui.overlay.overlay(), draft.as_ref());
    root.preview_ui.pending_runtime_key = None;
    root.preview_ui.runtime_key = Some(editing.key.clone());
    assert!(!root.preview_edit_frame_ready());
    root.preview_ui.rendered_runtime_key = Some(editing.key);
    assert!(root.nudge_selected_overlay_size(crate::preview::OverlaySizeDirection::Increase, None));
}
