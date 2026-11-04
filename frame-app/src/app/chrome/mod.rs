//! Titlebar, main-view navigation and platform window controls.

#[cfg(test)]
use crate::app::chrome::navigation::titlebar_view_for_key;
#[cfg(test)]
use crate::app::chrome::titlebar::titlebar_shows_workspace_controls;
#[cfg(test)]
use crate::app::model::{ActiveView, FrameAppState};
#[cfg(test)]
use crate::app::primitives::color;
#[cfg(test)]
use crate::app::root::FrameRoot;
#[cfg(test)]
use crate::app::updates::dialog::update_dialog_summary;
#[cfg(test)]
use crate::app::updates::preferences::update_status_text;
#[cfg(test)]
use crate::app::updates::release_notes::parse_update_release_note_emphasis;
#[cfg(test)]
use crate::app::updates::state::{UPDATE_INSTALL_WAIT_MESSAGE, UpdateStatus};
#[cfg(test)]
use crate::appearance::ColorTheme;
#[cfg(test)]
use crate::theme;
#[cfg(test)]
use gpui::{Context, FocusHandle, InteractiveElement, IntoElement, MouseButton, Window, div, px};
pub(super) use titlebar::titlebar;
#[cfg(test)]
pub(super) use titlebar::{FrameTitlebarPlatform, titlebar_for_platform};

mod navigation;
mod titlebar;
mod window_controls;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod titlebar_drag_tests;

pub(super) mod state;
