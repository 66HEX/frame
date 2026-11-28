//! Native GPUI conversion runner backed by the shared Frame ffmpeg argument builder.

mod alac;
mod completion;
mod config;
mod controller;
mod gif;
mod opus;
mod output_paths;
mod pass_directory;
mod process;
mod publication;
mod runner;
mod stabilization;
mod subtitles;
#[cfg(test)]
mod tests;
pub(crate) mod video_route;

#[cfg(test)]
use crate::file_queue::FileItem;
#[cfg(test)]
use crate::settings::ConversionConfig as GpuiConversionConfig;
pub(crate) use alac::staging::StagedOutput;
pub use config::*;
pub use controller::*;
#[cfg(test)]
use frame_core::events::ConversionEvent;
#[cfg(test)]
use frame_core::types::{ConversionTask, DEFAULT_MAX_CONCURRENCY};
pub use output_paths::*;
pub(crate) use publication::{PublicationDecision, publish_staged_output};
pub(crate) use runner::run_planned_batch;
#[cfg(test)]
use runner::{expected_duration_seconds, ffmpeg_progress_from_line, next_batch_launch_count};
pub use runner::{
    run_conversion_batch_with_control, run_conversion_task, run_conversion_task_with_control,
};
pub use subtitles::check_subtitle_trim_support;
