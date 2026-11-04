//! Native GPUI conversion runner backed by the shared Frame ffmpeg argument builder.

mod config;
mod controller;
mod output_paths;
mod process;
mod runner;
#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::file_queue::FileItem;
#[cfg(test)]
use crate::settings::ConversionConfig as GpuiConversionConfig;
pub use config::*;
pub use controller::*;
#[cfg(test)]
use frame_core::events::ConversionEvent;
#[cfg(test)]
use frame_core::types::{ConversionTask, DEFAULT_MAX_CONCURRENCY};
pub use output_paths::*;
pub use runner::*;
#[cfg(test)]
use runner::{ffmpeg_progress_from_line, next_batch_launch_count};
