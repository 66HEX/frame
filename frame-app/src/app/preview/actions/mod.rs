//! Preview coordination, split by runtime, canvas, crop, overlay and playback.

#[cfg(test)]
use crate::preview_engine::PreviewEngineError;
#[cfg(test)]
pub(in crate::app) use canvas::{
    lerp_preview_canvas_value, lerp_preview_canvas_value_for_elapsed, preview_canvas_initial_zoom,
    preview_canvas_pan_limits, preview_canvas_transform_settled,
    preview_canvas_transform_visual_delta, preview_canvas_wheel_zoom_multiplier,
};
pub(in crate::app) use canvas::{preview_canvas_keyboard_pan_delta, preview_canvas_layout_metrics};
#[cfg(test)]
pub(in crate::app) use crop::preview_crop_keyboard_delta;
pub(in crate::app) use media_commands::PreviewMediaCommandQueue;
#[cfg(test)]
pub(in crate::app) use overlay::preview_overlay_keyboard_delta;
#[cfg(test)]
pub(in crate::app) use request::preview_runtime_dimensions;

mod canvas;
mod crop;
mod media_commands;
mod overlay;
mod playback;
mod request;
mod runtime;
mod scheduling;
mod trim_seek;

#[cfg(test)]
mod runtime_tests;
