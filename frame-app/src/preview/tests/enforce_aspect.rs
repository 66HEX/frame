use super::*;

#[test]
fn anchors_the_dragged_edge() {
    let start_rect = CropRect {
        x: 0.2,
        y: 0.2,
        width: 0.4,
        height: 0.4,
    };
    let next = super::super::enforce_aspect(
        CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.55,
            height: 0.4,
        },
        DragHandle::East,
        start_rect,
        16.0 / 9.0,
        1920.0,
        1080.0,
        false,
    );

    assert_eq!(next.x, start_rect.x);
    assert_close(
        next.y + next.height / 2.0,
        start_rect.y + start_rect.height / 2.0,
    );
    assert_close(next.width / next.height, 1.0);
}
