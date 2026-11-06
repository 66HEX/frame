use std::time::Duration;

pub(in crate::app) const LOG_LINE_NUMBER_WIDTH: f32 = 32.0;

pub(in crate::app) const LOG_LINE_HEIGHT: f32 = 24.0;

pub(in crate::app) const LOG_SCROLL_BUTTON_OFFSET: f32 = 10.0;

pub(in crate::app) const LOG_SCROLL_BUTTON_PADDING: f32 = 4.0;

pub(in crate::app) const LOG_SCROLL_BUTTON_SIZE: f32 = 24.0;

pub(in crate::app) const LOG_SCROLL_ICON_SIZE: f32 = 16.0;

pub(in crate::app) const LOG_TAB_ICON_SIZE: f32 = 16.0;

pub(in crate::app) const LOG_COPY_FEEDBACK_DURATION: Duration = Duration::from_millis(1_200);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::app) struct LogScrollTarget {
    pub(in crate::app) file_id: String,
    pub(in crate::app) line_count: usize,
}
