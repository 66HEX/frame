use super::*;

#[test]
fn transform_crop_rect_round_trips_the_rotation_and_flip_matrix() {
    for (rotation, flip_horizontal, flip_vertical) in [
        (PreviewRotation::Deg0, false, false),
        (PreviewRotation::Deg90, true, true),
        (PreviewRotation::Deg180, true, false),
        (PreviewRotation::Deg270, false, true),
    ] {
        assert_round_trip(rotation, flip_horizontal, flip_vertical);
    }
}

fn assert_round_trip(rotation: PreviewRotation, flip_horizontal: bool, flip_vertical: bool) {
    let rect = CropRect {
        x: 0.2,
        y: 0.15,
        width: 0.35,
        height: 0.45,
    };

    let transformed =
        super::super::transform_crop_rect(rect, rotation, flip_horizontal, flip_vertical, false);
    let round_trip = super::super::transform_crop_rect(
        transformed,
        rotation,
        flip_horizontal,
        flip_vertical,
        true,
    );

    assert_rect_close(round_trip, rect);
}
