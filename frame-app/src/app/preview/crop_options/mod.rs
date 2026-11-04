//! Crop dimension fields, aspect select and crop-option interaction.

#[cfg(test)]
use crate::app::input::FrameTextInputKind;
#[cfg(test)]
use crate::app::preview::crop::full_crop_rect;
#[cfg(test)]
use crate::app::root::FrameRoot;
pub(in crate::app) use view::{preview_crop_custom_fields, preview_crop_options};

mod actions;
mod select;
mod view;

#[cfg(test)]
mod tests;
