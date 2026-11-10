use crate::theme;
use std::time::Duration;

pub(in crate::app) const FRAME_TEXT_INPUT_CONTEXT: &str = "FrameTextInput";

pub(in crate::app) const FRAME_TIMECODE_INPUT_CONTEXT: &str = "FrameTimecodeInput";

pub(in crate::app) const FRAME_TIMECODE_TEXT_INPUT_CONTEXT: &str =
    "FrameTextInput FrameTimecodeInput";

pub(in crate::app) const TEXT_INPUT_CARET_WIDTH: f32 = 1.5;

pub(in crate::app) const TEXT_INPUT_CARET_BASE_HEIGHT: f32 = theme::TEXT_INPUT_CARET_BASE_HEIGHT;

pub(in crate::app) const TEXT_INPUT_BLINK_INTERVAL: Duration = Duration::from_millis(500);

pub(in crate::app) const TEXT_INPUT_BLINK_PAUSE: Duration = Duration::from_millis(300);

pub(in crate::app) const TIMECODE_PRECISION_SECONDS: f64 = 0.001;
