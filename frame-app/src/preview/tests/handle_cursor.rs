use super::*;

#[test]
fn swaps_cardinal_cursors_for_side_rotation() {
    assert_eq!(
        super::super::handle_cursor(DragHandle::North, false),
        "ns-resize"
    );
    assert_eq!(
        super::super::handle_cursor(DragHandle::North, true),
        "ew-resize"
    );
}

#[test]
fn swaps_corner_cursors_for_side_rotation() {
    assert_eq!(
        super::super::handle_cursor(DragHandle::NorthEast, false),
        "nesw-resize"
    );
    assert_eq!(
        super::super::handle_cursor(DragHandle::NorthEast, true),
        "nwse-resize"
    );
}
