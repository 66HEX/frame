//! Native GPUI conversion runner backed by the shared Frame ffmpeg argument builder.

mod config;
mod controller;
mod process;
mod runner;
#[cfg(test)]
mod tests;

pub use config::*;
pub use controller::*;
pub use runner::*;

#[cfg(test)]
use crate::file_queue::FileItem;
#[cfg(test)]
use crate::settings::ConversionConfig as GpuiConversionConfig;
#[cfg(test)]
use frame_core::{
    events::ConversionEvent,
    types::{ConversionTask, DEFAULT_MAX_CONCURRENCY},
};
#[cfg(test)]
use runner::{ffmpeg_progress_from_line, next_batch_launch_count};
