//! Video filter inspector and its range controls.

mod controls;
mod range;
mod spec;
mod view;

#[cfg(test)]
mod tests;

pub(in crate::app) use view::settings_video_filters_tab;
