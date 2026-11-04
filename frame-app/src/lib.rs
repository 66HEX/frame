//! Shared state and layout contracts for the native GPUI-CE app.

pub mod app;
pub mod app_info;
pub mod app_persistence;
pub mod appearance;
pub mod assets;
pub mod capabilities;
pub mod conversion_events;
pub mod conversion_runner;
pub mod file_filters;
pub mod file_queue;
pub mod native_dialogs;
pub mod notifications;
pub(crate) mod numeric;
pub mod preview;
pub mod preview_engine;
pub mod runtime_binaries;
pub(crate) mod runtime_environment;
pub mod settings;
pub mod source_metadata;
pub mod theme;
pub mod update_runtime;
pub(crate) mod update_session;

pub use app::layout::*;
pub use app::model::{
    ActiveView, FrameAppState, StartAvailability, VisualFixture, active_view_from_env_value,
    format_total_size, visual_fixture_from_env_value,
};

#[cfg(test)]
mod tests;
