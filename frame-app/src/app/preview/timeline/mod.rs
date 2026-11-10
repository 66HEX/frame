//! Preview timeline, timecode fields and interactive trim track.

mod timecode;
mod track;
mod transport;
mod view;

pub(super) use track::PreviewTimelineDragPreview;
#[cfg(test)]
pub(in crate::app) use track::{centered_offset, timeline_fraction_from_percent};
pub(in crate::app) use track::{
    timeline_keyboard_time_for_key, timeline_slider_percent_from_bounds,
};
pub(in crate::app) use transport::preview_transport;
#[cfg(test)]
pub(in crate::app) use view::{preview_timeline_labels, preview_trim_enabled};
