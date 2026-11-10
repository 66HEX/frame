use super::*;

#[test]
fn frame_edges_are_one_logical_pixel_on_every_platform() {
    assert_eq!(frame_highlight_px(), 1.0);
}
