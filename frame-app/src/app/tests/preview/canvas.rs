use super::*;

#[test]
fn sync_preview_canvas_for_selection_resets_canvas_when_file_changes() {
    let mut root = FrameRoot::new();
    root.sync_preview_canvas_for_selection(Some("first"));
    root.preview_ui.canvas.current_zoom = 2.0;
    root.preview_ui.canvas.target_zoom = 3.0;
    root.preview_ui.canvas.current_pan_x = 0.5;
    root.preview_ui.canvas.target_pan_y = -0.5;
    root.preview_ui.canvas.auto_fit_pending = false;

    root.sync_preview_canvas_for_selection(Some("first"));

    assert_eq!(root.preview_ui.canvas.current_zoom, 2.0);

    root.sync_preview_canvas_for_selection(Some("second"));

    assert_eq!(root.preview_ui.canvas, PreviewCanvasState::default());
}

#[test]
fn lerp_preview_canvas_value_moves_toward_target_without_overshooting() {
    let next = lerp_preview_canvas_value(1.0, 2.0);

    assert!(next > 1.0 && next < 2.0);
}

#[test]
fn lerp_preview_canvas_value_uses_elapsed_time() {
    let normal = lerp_preview_canvas_value_for_elapsed(0.0, 100.0, Duration::from_millis(16));
    let delayed = lerp_preview_canvas_value_for_elapsed(0.0, 100.0, Duration::from_millis(64));

    assert!((normal - 20.0).abs() < 0.000_001);
    assert!(delayed > normal);
    assert!(delayed < 100.0);
}

#[test]
fn preview_canvas_wheel_zoom_multiplier_ignores_tail_micro_delta() {
    let multiplier = preview_canvas_wheel_zoom_multiplier(0.06);

    assert_eq!(multiplier, None);
}

#[test]
fn preview_canvas_wheel_zoom_multiplier_scales_with_delta_magnitude() {
    let small = preview_canvas_wheel_zoom_multiplier(-0.50).expect("small multiplier");
    let large = preview_canvas_wheel_zoom_multiplier(-2.00).expect("large multiplier");

    assert!(large > small && small > 1.0);
}

#[test]
fn preview_canvas_wheel_zoom_multiplier_zooms_out_for_positive_normalized_delta() {
    let multiplier = preview_canvas_wheel_zoom_multiplier(1.00).expect("multiplier");

    assert!(multiplier < 1.0);
}

#[test]
fn upward_line_scroll_zooms_in() {
    let delta_y = preview_scroll_delta_y(&ScrollDelta::Lines(point(0.0, 1.0)));
    let multiplier = preview_canvas_wheel_zoom_multiplier(delta_y).expect("multiplier");

    assert!(multiplier > 1.0);
}

#[test]
fn downward_pixel_scroll_zooms_out() {
    let delta_y = preview_scroll_delta_y(&ScrollDelta::Pixels(point(px(0.0), px(-100.0))));
    let multiplier = preview_canvas_wheel_zoom_multiplier(delta_y).expect("multiplier");

    assert!(multiplier < 1.0);
}

#[test]
fn preview_canvas_wheel_zoom_multiplier_caps_extreme_delta() {
    let capped = preview_canvas_wheel_zoom_multiplier(-100.0).expect("capped multiplier");
    let expected = PREVIEW_CANVAS_WHEEL_ZOOM_STEP.powi(8);

    assert!((capped - expected).abs() < 0.000_001);
}

#[test]
fn preview_canvas_transform_waits_for_all_axes_to_settle() {
    assert!(!preview_canvas_transform_settled(
        1.0, 1.000_01, 10.0, 10.005, -10.0, -10.005,
    ));
    assert!(preview_canvas_transform_settled(
        1.0,
        1.000_000_5,
        10.0,
        10.005,
        -10.0,
        -10.005,
    ));
}

#[test]
fn preview_canvas_transform_visual_delta_measures_rendered_bounds() {
    let delta = preview_canvas_transform_visual_delta(
        1000.0, 500.0, 16.0, 9.0, 1.0, 1.000_4, 0.0, 0.20, 0.0, -0.20,
    )
    .expect("visual delta");

    assert!(delta < PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON);
}

#[test]
fn tick_preview_canvas_animation_keeps_pan_moving_until_zoom_settles() {
    let mut root = FrameRoot::new();
    root.preview_ui.canvas.current_zoom = 1.0;
    root.preview_ui.canvas.target_zoom = 1.000_01;
    root.preview_ui.canvas.current_pan_x = 10.0;
    root.preview_ui.canvas.target_pan_x = 10.005;
    root.preview_ui.canvas.current_pan_y = -10.0;
    root.preview_ui.canvas.target_pan_y = -10.005;

    assert!(root.tick_preview_canvas_animation());
    assert!(root.preview_ui.canvas.current_zoom > 1.0);
    assert!(root.preview_ui.canvas.current_pan_x > 10.0);
    assert!(root.preview_ui.canvas.current_pan_y < -10.0);
}

#[test]
fn tick_preview_canvas_animation_settles_subpixel_visual_motion() {
    let mut root = root_with_preview_canvas_media(16, 9);
    root.preview_ui.canvas.current_zoom = 1.0;
    root.preview_ui.canvas.target_zoom = 1.000_4;
    root.preview_ui.canvas.current_pan_x = 0.0;
    root.preview_ui.canvas.target_pan_x = 0.20;
    root.preview_ui.canvas.current_pan_y = 0.0;
    root.preview_ui.canvas.target_pan_y = -0.20;

    assert!(root.tick_preview_canvas_animation());
    assert_eq!(
        root.preview_ui.canvas.current_zoom,
        root.preview_ui.canvas.target_zoom
    );
    assert_eq!(
        root.preview_ui.canvas.current_pan_x,
        root.preview_ui.canvas.target_pan_x
    );
    assert_eq!(
        root.preview_ui.canvas.current_pan_y,
        root.preview_ui.canvas.target_pan_y
    );
}

#[test]
fn tick_preview_canvas_animation_keeps_visible_motion_lerping() {
    let mut root = root_with_preview_canvas_media(16, 9);
    root.preview_ui.canvas.current_zoom = 1.0;
    root.preview_ui.canvas.target_zoom = 1.01;
    root.preview_ui.canvas.current_pan_x = 0.0;
    root.preview_ui.canvas.target_pan_x = 2.0;

    assert!(root.tick_preview_canvas_animation());
    assert_ne!(
        root.preview_ui.canvas.current_zoom,
        root.preview_ui.canvas.target_zoom
    );
}

#[test]
fn tick_preview_canvas_animation_settles_exactly_inside_visual_epsilon() {
    let mut root = FrameRoot::new();
    root.preview_ui.canvas.current_zoom = 1.0;
    root.preview_ui.canvas.target_zoom = 1.000_000_5;
    root.preview_ui.canvas.current_pan_x = 10.0;
    root.preview_ui.canvas.target_pan_x = 10.005;
    root.preview_ui.canvas.current_pan_y = -10.0;
    root.preview_ui.canvas.target_pan_y = -10.005;

    assert!(root.tick_preview_canvas_animation());
    assert_eq!(
        root.preview_ui.canvas.current_zoom,
        root.preview_ui.canvas.target_zoom
    );
    assert_eq!(
        root.preview_ui.canvas.current_pan_x,
        root.preview_ui.canvas.target_pan_x
    );
    assert_eq!(
        root.preview_ui.canvas.current_pan_y,
        root.preview_ui.canvas.target_pan_y
    );
}
