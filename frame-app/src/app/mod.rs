//! Native application entry points and stable action identifiers.

mod accessibility;
mod chrome;
mod components;
mod fixtures;
mod input;
mod inspector;
pub(crate) mod layout;
mod logs;
pub(crate) mod model;
mod motion;
mod preferences;
mod preview;
mod primitives;
mod queue;
mod render;
mod root;
mod runtime;
mod system_motion;
mod updates;
mod workspace;

#[cfg(test)]
mod accessibility_ui_tests;
#[cfg(test)]
mod tests;

use gpui::actions;

pub use root::FrameRoot;
pub use runtime::{frame_window_options, init_app, open_frame_window};

#[cfg(test)]
use tests::support::*;

actions!(
    frame_app,
    [
        Quit,
        TextInputBackspace,
        TextInputDelete,
        TextInputLeft,
        TextInputRight,
        TextInputSelectLeft,
        TextInputSelectRight,
        TextInputHome,
        TextInputEnd,
        TextInputSelectAll,
        TextInputCopy,
        TextInputCut,
        TextInputPaste,
        TextInputCommit,
        TextInputCancel,
        IncreaseUiScale,
        DecreaseUiScale,
        ResetUiScale,
    ]
);
