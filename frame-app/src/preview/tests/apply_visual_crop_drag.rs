use super::*;

#[test]
fn keeps_drag_deltas_in_visual_space_for_side_rotations() {
    let next = super::super::apply_visual_crop_drag(VisualCropDrag {
        start_rect: CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.4,
            height: 0.4,
        },
        handle: DragHandle::East,
        start_point: Point { x: 0.6, y: 0.4 },
        current_point: Point { x: 0.7, y: 0.4 },
        aspect_id: "free",
        source_width: 1920.0,
        source_height: 1080.0,
        is_side_rotation: true,
    });

    assert_rect_close(
        next,
        CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.5,
            height: 0.4,
        },
    );
}

#[test]
fn moves_crop_directly_in_visual_space_for_side_rotations() {
    let next = super::super::apply_visual_crop_drag(VisualCropDrag {
        start_rect: CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.4,
            height: 0.4,
        },
        handle: DragHandle::Move,
        start_point: Point { x: 0.4, y: 0.4 },
        current_point: Point { x: 0.5, y: 0.45 },
        aspect_id: "free",
        source_width: 1920.0,
        source_height: 1080.0,
        is_side_rotation: true,
    });

    assert_rect_close(
        next,
        CropRect {
            x: 0.3,
            y: 0.25,
            width: 0.4,
            height: 0.4,
        },
    );
}

#[test]
fn enforces_fixed_aspect_ratios_when_dragging_corner_handles() {
    let next = super::super::apply_visual_crop_drag(VisualCropDrag {
        start_rect: CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.4,
            height: 0.4,
        },
        handle: DragHandle::SouthEast,
        start_point: Point { x: 0.6, y: 0.6 },
        current_point: Point { x: 0.8, y: 0.6 },
        aspect_id: "16:9",
        source_width: 1920.0,
        source_height: 1080.0,
        is_side_rotation: false,
    });

    assert_close(next.width / next.height, 1.0);
}
