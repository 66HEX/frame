use super::*;

#[test]
fn preserves_center_when_possible() {
    let adjusted = super::super::adjust_rect_to_ratio(
        CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.6,
            height: 0.4,
        },
        1.0,
        1920.0,
        1080.0,
        false,
    );

    assert_close(adjusted.x + adjusted.width / 2.0, 0.5);
    assert_close(adjusted.y + adjusted.height / 2.0, 0.4);
    assert_close(adjusted.width / adjusted.height, 1.0 / (1920.0 / 1080.0));
}
