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

#[test]
fn side_rotations_follow_ffmpeg_source_axis_flips_in_both_directions() {
    let source = CropRect {
        x: 0.1,
        y: 0.2,
        width: 0.6,
        height: 0.5,
    };
    for (rotation, horizontal, vertical, x, y) in [
        (PreviewRotation::Deg90, false, false, 0.3, 0.1),
        (PreviewRotation::Deg90, true, false, 0.3, 0.3),
        (PreviewRotation::Deg90, false, true, 0.2, 0.1),
        (PreviewRotation::Deg90, true, true, 0.2, 0.3),
        (PreviewRotation::Deg270, false, false, 0.2, 0.3),
        (PreviewRotation::Deg270, true, false, 0.2, 0.1),
        (PreviewRotation::Deg270, false, true, 0.3, 0.3),
        (PreviewRotation::Deg270, true, true, 0.3, 0.1),
    ] {
        let visual = CropRect {
            x,
            y,
            width: 0.5,
            height: 0.6,
        };
        assert_rect_close(
            super::super::transform_crop_rect(source, rotation, horizontal, vertical, false),
            visual,
        );
        assert_rect_close(
            super::super::transform_crop_rect(visual, rotation, horizontal, vertical, true),
            source,
        );
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
