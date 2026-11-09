//! Overlay image layer, editing toolbar and opacity control.

mod controls;
mod layer;
mod opacity;

pub(in crate::app) use controls::preview_overlay_controls;
pub(in crate::app) use layer::preview_overlay_layer;
#[cfg(test)]
pub(in crate::app) use layer::preview_overlay_test_render_size;
pub(super) use layer::{PreviewOverlayDrag, overlay_drag_point_from_bounds};
