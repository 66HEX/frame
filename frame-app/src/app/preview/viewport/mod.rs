//! Infinite-canvas composition, clipping frame, media painting and bounds tracking.

#[cfg(test)]
use crate::app::preview::viewport::media::preview_image_transformation;
#[cfg(test)]
use crate::app::preview::viewport::view::{
    preview_canvas_layout_ready, preview_waiting_for_visual_render,
};
pub(in crate::app) use bounds::normalized_point_from_bounds;
#[cfg(test)]
pub(in crate::app) use bounds::preview_scroll_delta_y;
#[cfg(test)]
pub(in crate::app) use view::preview_visual_controls_visible;
pub(in crate::app) use view::{preview_viewport, preview_visual_controls_enabled};

mod bounds;
mod frame;
mod media;
mod view;

#[cfg(test)]
mod tests;
