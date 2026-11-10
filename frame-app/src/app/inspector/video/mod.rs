//! Video inspector tab, geometry controls, encoding controls and quality slider.

mod encoding;
mod geometry;
mod quality;
mod view;

pub(in crate::app) use geometry::{
    settings_video_resolution_section, settings_video_scaling_section,
};
pub(in crate::app) use view::settings_video_tab;
