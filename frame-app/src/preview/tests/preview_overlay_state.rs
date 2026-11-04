use super::*;

#[test]
fn clamp_overlay_center_keeps_overlay_inside_bounds() {
    let center = super::super::clamp_overlay_center(0.98, 0.01, 0.4, 0.2);

    assert_close(center.x, 0.8);
    assert_close(center.y, 0.1);
}

#[test]
fn max_overlay_width_accounts_for_tall_overlay_aspect() {
    assert_close(super::super::max_overlay_width(2.0), 0.5);
}

#[test]
fn clamp_overlay_width_uses_original_minimum_width() {
    assert_close(super::super::clamp_overlay_width(0.01, 1.0), 0.03);
}

#[test]
fn create_default_overlay_matches_original_centered_overlay() {
    let overlay = super::super::create_default_overlay("/tmp/logo.png");

    assert_eq!(overlay.path, "/tmp/logo.png");
    assert_close(overlay.x, 0.5);
    assert_close(overlay.y, 0.5);
    assert_close(overlay.width, DEFAULT_OVERLAY_WIDTH);
    assert_close(overlay.opacity, 1.0);
}

#[test]
fn normalize_overlay_clamps_position_size_and_opacity() {
    let overlay = super::super::normalize_overlay(&PreviewOverlay {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.99,
        y: -0.1,
        width: 2.0,
        opacity: 2.0,
        anchor: "ignored".to_string(),
    });

    assert_close(overlay.x, 0.6);
    assert_close(overlay.y, 0.4);
    assert_close(overlay.width, MAX_OVERLAY_WIDTH);
    assert_close(overlay.opacity, 1.0);
    assert_eq!(overlay.anchor, "custom");
}

#[test]
fn sync_initial_overlay_discards_disabled_or_empty_overlay() {
    let mut state = PreviewOverlayState::new();
    state.sync_initial_overlay(Some(&PreviewOverlay {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.5,
        y: 0.5,
        width: 0.2,
        opacity: 1.0,
        anchor: "custom".to_string(),
    }));

    state.sync_initial_overlay(Some(&PreviewOverlay {
        enabled: false,
        path: "/tmp/logo.png".to_string(),
        x: 0.5,
        y: 0.5,
        width: 0.2,
        opacity: 1.0,
        anchor: "custom".to_string(),
    }));

    assert_eq!(state.overlay(), None);
    assert!(!state.overlay_mode());
}

#[test]
fn sync_initial_overlay_ignores_external_changes_while_dragging() {
    let mut state = state_with_overlay();
    assert!(state.begin_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.5,
            y: 0.5,
            width: Some(0.18),
            height: Some(0.18),
        },
        false,
    ));

    state.sync_initial_overlay(None);

    assert!(state.overlay().is_some());
    assert!(state.is_dragging());
}

#[test]
fn set_overlay_from_path_is_blocked_when_controls_are_disabled() {
    let mut state = PreviewOverlayState::new();

    assert_eq!(state.set_overlay_from_path("/tmp/logo.png", true), None);
    assert_eq!(state.overlay(), None);
}

#[test]
fn set_overlay_from_path_persists_overlay_and_enters_overlay_mode() {
    let mut state = PreviewOverlayState::new();

    let overlay = state.set_overlay_from_path("/tmp/logo.png", false).unwrap();

    assert_eq!(overlay.path, "/tmp/logo.png");
    assert!(state.overlay_mode());
    assert!(state.overlay().is_some());
}

#[test]
fn toggle_overlay_mode_requests_crop_deactivation_when_enabling() {
    let mut state = state_with_committed_overlay();

    let change = state.toggle_overlay_mode(false);

    assert!(change.changed);
    assert!(change.should_deactivate_crop);
    assert!(state.overlay_mode());
}

#[test]
fn toggle_overlay_mode_does_nothing_without_overlay() {
    let mut state = PreviewOverlayState::new();

    let change = state.toggle_overlay_mode(false);

    assert!(!change.changed);
    assert!(!change.should_deactivate_crop);
}

#[test]
fn set_overlay_mode_refuses_enabling_when_controls_are_disabled() {
    let mut state = state_with_overlay();
    state.set_overlay_mode(false, false);

    let change = state.set_overlay_mode(true, true);

    assert!(!change.changed);
    assert!(!state.overlay_mode());
}

#[test]
fn begin_overlay_drag_requires_overlay_mode() {
    let mut state = state_with_overlay();
    state.set_overlay_mode(false, false);

    assert!(!state.begin_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.5,
            y: 0.5,
            width: Some(0.18),
            height: Some(0.18),
        },
        false,
    ));
}

#[test]
fn move_drag_updates_center_in_normalized_overlay_space() {
    let mut state = state_with_overlay();
    state.begin_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.5,
            y: 0.5,
            width: Some(0.2),
            height: Some(0.1),
        },
        false,
    );

    let overlay = state
        .update_overlay_drag(OverlayDragPoint {
            x: 0.6,
            y: 0.45,
            width: Some(0.2),
            height: Some(0.1),
        })
        .unwrap();

    assert_close(overlay.x, 0.6);
    assert_close(overlay.y, 0.45);
}

#[test]
fn resize_drag_preserves_overlay_aspect_from_pointer_metrics() {
    let mut state = state_with_overlay();
    state.sync_initial_overlay(Some(&PreviewOverlay {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.5,
        y: 0.5,
        width: 0.2,
        opacity: 1.0,
        anchor: "custom".to_string(),
    }));
    state.set_overlay_mode(true, false);
    state.begin_overlay_drag(
        OverlayDragHandle::SouthEast,
        OverlayDragPoint {
            x: 0.6,
            y: 0.55,
            width: Some(0.2),
            height: Some(0.1),
        },
        false,
    );

    let overlay = state
        .update_overlay_drag(OverlayDragPoint {
            x: 0.8,
            y: 0.7,
            width: Some(0.2),
            height: Some(0.1),
        })
        .unwrap();

    assert_close(overlay.width, 0.5);
    assert_close(overlay.x, 0.65);
    assert_close(overlay.y, 0.575);
}

#[test]
fn resize_drag_requires_pointer_dimensions() {
    let mut state = state_with_overlay();
    state.begin_overlay_drag(
        OverlayDragHandle::SouthEast,
        OverlayDragPoint {
            x: 0.6,
            y: 0.6,
            width: None,
            height: Some(0.1),
        },
        false,
    );

    assert_eq!(
        state.update_overlay_drag(OverlayDragPoint {
            x: 0.8,
            y: 0.7,
            width: None,
            height: Some(0.1),
        }),
        None
    );
}

#[test]
fn end_overlay_drag_clears_dragging_state() {
    let mut state = state_with_overlay();
    state.begin_overlay_drag(
        OverlayDragHandle::Move,
        OverlayDragPoint {
            x: 0.5,
            y: 0.5,
            width: Some(0.18),
            height: Some(0.18),
        },
        false,
    );

    state.end_overlay_drag();

    assert!(!state.is_dragging());
}

#[test]
fn set_opacity_clamps_to_original_zero_one_range() {
    let mut state = state_with_overlay();

    let overlay = state.set_opacity(2.0, false).unwrap();

    assert_close(overlay.opacity, 1.0);
}

#[test]
fn nudge_size_uses_original_step_and_recenters_inside_bounds() {
    let mut state = PreviewOverlayState::new();
    state.sync_initial_overlay(Some(&PreviewOverlay {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.99,
        y: 0.99,
        width: 0.79,
        opacity: 1.0,
        anchor: "custom".to_string(),
    }));
    state.set_overlay_mode(true, false);

    let overlay = state
        .nudge_size(OverlaySizeDirection::Increase, Some(1.0), false)
        .unwrap();

    assert_close(overlay.width, 0.8);
    assert_close(overlay.x, 0.6);
    assert_close(overlay.y, 0.6);
}

#[test]
fn nudge_size_respects_height_ratio_max_width() {
    let mut state = state_with_overlay();

    let overlay = state
        .nudge_size(OverlaySizeDirection::Increase, Some(4.0), false)
        .unwrap();

    assert_close(overlay.width, 0.205);
}

#[test]
fn remove_overlay_clears_overlay_and_mode() {
    let mut state = state_with_overlay();

    assert_eq!(state.remove_overlay(false), Some(None));
    assert_eq!(state.overlay(), None);
    assert!(!state.overlay_mode());
}

#[test]
fn remove_overlay_is_blocked_when_controls_are_disabled() {
    let mut state = state_with_overlay();

    assert_eq!(state.remove_overlay(true), None);
    assert!(state.overlay().is_some());
}

fn state_with_overlay() -> PreviewOverlayState {
    let mut state = PreviewOverlayState::new();
    state.set_overlay_from_path("/tmp/logo.png", false);
    state
}

fn state_with_committed_overlay() -> PreviewOverlayState {
    let mut state = PreviewOverlayState::new();
    state.sync_initial_overlay(Some(&PreviewOverlay {
        enabled: true,
        path: "/tmp/logo.png".to_string(),
        x: 0.5,
        y: 0.5,
        width: 0.18,
        opacity: 1.0,
        anchor: "custom".to_string(),
    }));
    state
}
