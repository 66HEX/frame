use super::*;

#[test]
fn keeps_transparent_titlebar_with_native_controls_in_frame_slot() {
    let options = frame_window_options(Bounds::default());
    let titlebar = options
        .titlebar
        .as_ref()
        .expect("Frame keeps a transparent native titlebar host for macOS controls");

    assert!(titlebar.appears_transparent);
    // AppKit must not drag the entire native titlebar over our interactive controls.
    // Other platforms still need their normal native window movement enabled.
    assert_eq!(options.is_movable, !cfg!(target_os = "macos"));
    assert!(options.app_owns_titlebar_drag);
    assert_eq!(
        titlebar.traffic_light_position,
        Some(point(
            px(TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_X),
            px(TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_Y),
        ))
    );
    let position = titlebar.traffic_light_position.unwrap();
    for coordinate in [position.x.as_f32(), position.y.as_f32()] {
        assert!(coordinate.is_finite());
        for backing_scale in [1.0, 2.0] {
            let physical = coordinate * backing_scale;
            assert!(
                physical.is_finite() && physical.fract().abs() < f32::EPSILON,
                "native mask edge must align to a pixel: {coordinate} points at {backing_scale}x"
            );
        }
    }
}

#[test]
fn preserves_original_minimum_window_size() {
    let options = frame_window_options(Bounds::default());

    assert_eq!(
        options.window_min_size,
        Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)))
    );
}

#[test]
fn sets_the_frame_application_id() {
    let options = frame_window_options(Bounds::default());

    assert_eq!(options.app_id.as_deref(), Some(FRAME_APP_ID));
}

#[test]
fn sets_the_native_window_title() {
    let options = frame_window_options(Bounds::default());
    let titlebar = options
        .titlebar
        .as_ref()
        .expect("Frame should configure native titlebar metadata");

    assert_eq!(titlebar.title.as_deref(), Some("Frame"));
}
