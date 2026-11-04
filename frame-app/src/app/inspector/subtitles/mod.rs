//! Subtitle inspector: source tracks, burn-in styling, selects and color editing.

#[cfg(test)]
use crate::app::inspector::subtitles::tracks::{
    ExternalSubtitleMetadataVisibility, external_subtitle_metadata_visibility,
};
pub(in crate::app) use color::hex_to_subtitle_hsv;
#[cfg(test)]
pub(in crate::app) use color::subtitle_hsv_to_hex;
pub(in crate::app) use view::{SettingsSubtitlesTabState, settings_subtitles_tab};

mod actions;
mod burn_in;
mod color;
mod color_picker;
mod font_select;
mod popover;
mod size_select;
mod tracks;
mod view;

#[cfg(test)]
mod tests;

pub(in crate::app) mod state;

mod import;
