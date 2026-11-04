#![expect(
    clippy::float_cmp,
    reason = "Preview model tests compare exact deterministic geometry values."
)]

use super::*;

fn assert_rect_close(actual: CropRect, expected: CropRect) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.width, expected.width);
    assert_close(actual.height, expected.height);
}

fn assert_close(actual: f64, expected: f64) {
    const EPSILON: f64 = 0.000_001;
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be within {EPSILON} of {expected}"
    );
}

mod aspect_value;

mod transform_crop_rect;

mod remap_drag_deltas;

mod clamp_rect;

mod adjust_rect_to_ratio;

mod enforce_aspect;

mod apply_visual_crop_drag;

mod handle_cursor;

mod preview_rotation;

mod preview_media_kind;

mod preview_control_availability;

mod time_formatting;

mod preview_playback_state;

mod preview_overlay_state;
