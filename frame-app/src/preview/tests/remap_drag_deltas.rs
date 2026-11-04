use super::*;

#[test]
fn remap_drag_deltas_handles_the_rotation_matrix() {
    let cases = [
        (PreviewRotation::Deg0, DragDelta { dx: 0.2, dy: 0.1 }),
        (PreviewRotation::Deg90, DragDelta { dx: 0.1, dy: -0.2 }),
        (PreviewRotation::Deg180, DragDelta { dx: -0.2, dy: -0.1 }),
        (PreviewRotation::Deg270, DragDelta { dx: -0.1, dy: 0.2 }),
    ];

    for (rotation, expected) in cases {
        assert_eq!(
            super::super::remap_drag_deltas(0.2, 0.1, rotation, false, false),
            expected,
            "delta remapping failed for rotation={rotation:?}"
        );
    }
}

#[test]
fn applies_flips_after_rotation() {
    assert_eq!(
        super::super::remap_drag_deltas(0.2, 0.1, PreviewRotation::Deg90, true, true),
        DragDelta { dx: -0.1, dy: 0.2 }
    );
}
