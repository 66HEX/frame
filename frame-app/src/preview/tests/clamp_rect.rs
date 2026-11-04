use super::*;

#[test]
fn keeps_crop_inside_normalized_bounds() {
    assert_rect_close(
        super::super::clamp_rect(CropRect {
            x: -0.2,
            y: 0.9,
            width: 0.2,
            height: 0.3,
        }),
        CropRect {
            x: 0.0,
            y: 0.7,
            width: 0.2,
            height: 0.3,
        },
    );
}

#[test]
fn enforces_original_minimum_crop_size() {
    assert_rect_close(
        super::super::clamp_rect(CropRect {
            x: 0.9,
            y: 0.9,
            width: 0.01,
            height: 0.01,
        }),
        CropRect {
            x: 0.9,
            y: 0.9,
            width: 0.05,
            height: 0.05,
        },
    );
}
