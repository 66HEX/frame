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
    FrameTextInputKind, FrameTextInputMetrics, FrameTextInputRuntime, FrameTextInputUiState,
    FrameTimecodeInputDraft, clamp_text_input_scroll_x, text_input_scroll_x_for_cursor,
};

// Inputs are single-line. GPUI's text backend reports caret positions in two
// dimensions; only the horizontal coordinate affects scrolling and selection.
fn text_line_x_for_index(line: &ShapedLine, index: usize) -> Pixels {
    line.platform_layout
        .caret_bounds(
            gpui::CaretPosition::attached_to_next_cluster(index),
            line.font_size,
        )
        .map_or_else(|| line.width(), |bounds| bounds.origin.x)
}

fn text_line_closest_index_for_x(line: &ShapedLine, x: Pixels) -> usize {
    line.platform_layout
        .caret_from_pixel_point(point(x, Pixels::ZERO), line.font_size)
        .unwrap_or_else(|caret| caret)
        .index
}
