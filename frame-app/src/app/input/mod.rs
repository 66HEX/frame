use super::primitives::{color, input_highlight_shadows};
use super::*;

mod actions;
mod element;
mod entity;
mod runtime;
mod text;

pub(super) use element::{FrameTextInputSpec, frame_text_input};
#[cfg(test)]
pub(super) use element::{should_capture_text_input_drag, should_handle_text_input};
pub(super) use runtime::{
    FrameTextInputKind, FrameTextInputRuntime, FrameTextInputUiState, FrameTimecodeInputDraft,
    clamp_text_input_scroll_x, text_input_scroll_x_for_cursor,
};
