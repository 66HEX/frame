use super::*;

#[test]
fn max_concurrency_runtime_settings_has_no_stepper_actions() {
    let mut root = FrameRoot::new();
    root.settings_ui.max_concurrency_draft = "1".to_string();
    root.text_input_runtime_mut(FrameTextInputKind::MaxConcurrency)
        .selected_range = 1..1;

    assert!(!root.replace_text_input_range(
        FrameTextInputKind::MaxConcurrency,
        None,
        "-",
        None,
        false,
    ));
    assert_eq!(root.settings_ui.max_concurrency_draft, "1");
}

#[test]
fn audio_slider_helpers_map_values_to_original_range() {
    assert_eq!(inspector::range_fraction(100, 0, 200), 0.5);
    assert_eq!(inspector::range_value_from_fraction(0.5, 0, 200), 100);
    assert_eq!(
        inspector::range_value_for_key(100, 0, 200, "right"),
        Some(101)
    );
    assert_eq!(
        inspector::range_value_for_key(100, 0, 200, "pageup"),
        Some(120)
    );
    assert_eq!(inspector::range_value_for_key(100, 0, 200, "home"), Some(0));
}

#[test]
fn timeline_keyboard_helper_maps_standard_slider_keys() {
    assert_eq!(
        timeline_keyboard_time_for_key(30.0, 60.0, "right"),
        Some(30.6)
    );
    assert_eq!(
        timeline_keyboard_time_for_key(30.0, 60.0, "pageup"),
        Some(36.0)
    );
    assert_eq!(
        timeline_keyboard_time_for_key(30.0, 60.0, "end"),
        Some(60.0)
    );
    assert_eq!(timeline_keyboard_time_for_key(30.0, 0.0, "right"), None);
}

#[test]
fn preview_keyboard_delta_helpers_ignore_irrelevant_keys() {
    assert_eq!(
        preview_canvas_keyboard_pan_delta("right"),
        Some(PreviewPoint { x: 24.0, y: 0.0 })
    );
    assert_eq!(preview_canvas_keyboard_pan_delta("enter"), None);
    assert_eq!(
        preview_crop_keyboard_delta(DragHandle::North, "up", false),
        Some(PreviewPoint { x: 0.0, y: -0.01 })
    );
    assert_eq!(
        preview_crop_keyboard_delta(DragHandle::North, "left", false),
        None
    );
    assert_eq!(
        preview_overlay_keyboard_delta("down", false),
        Some(PreviewPoint { x: 0.0, y: 0.01 })
    );
    assert_eq!(
        preview_crop_keyboard_delta(DragHandle::North, "up", true),
        Some(PreviewPoint { x: 0.0, y: -0.05 })
    );
    assert_eq!(
        preview_overlay_keyboard_delta("down", true),
        Some(PreviewPoint { x: 0.0, y: 0.05 })
    );
}

#[test]
fn preview_crop_handles_use_screen_space_cursors() {
    assert_eq!(
        crate::app::preview::crop_handle_screen_cursor(DragHandle::NorthEast),
        "nesw-resize"
    );
    assert_eq!(
        crate::app::preview::crop_handle_screen_cursor(DragHandle::NorthWest),
        "nwse-resize"
    );
}
