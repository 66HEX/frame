//! `FFmpeg` process lifecycle, playback clock, video decoding and audio output.

#[cfg(test)]
use crate::preview_engine::ffmpeg_backend::audio_output::{
    AudioOutputSpec, AudioSampleBuffer, lock_audio_buffer, push_audio_sample_bytes,
};
#[cfg(test)]
use crate::preview_engine::ffmpeg_backend::audio_stream::audio_prebuffer_samples;
#[cfg(test)]
use crate::preview_engine::ffmpeg_backend::clock::{
    PlaybackStreamState, PreviewPlaybackClock, clamp_seek_seconds, frame_presentation_state,
    lock_playback, playback_stream_state, update_last_frame_seconds, wait_for_playback_start,
};
#[cfg(test)]
use crate::preview_engine::ffmpeg_backend::plan::{
    insert_frame_limit, preview_audio_plan, preview_plan,
};
#[cfg(test)]
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
#[cfg(test)]
use crate::preview_engine::types::{PreviewSessionConfig, PreviewSourceKind};
pub(super) use clock::PreviewPlaybackReader;
pub use process::RunningPreviewProcess;
pub(super) use process::start_ffmpeg_preview_process;
#[cfg(test)]
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::{Duration, Instant};

mod audio_drain;
mod audio_exit;
mod audio_output;
mod audio_stream;
mod clock;
mod decode;
mod plan;
mod process;
mod video_stream;

#[cfg(test)]
mod audio_drain_tests;
#[cfg(all(test, unix))]
mod audio_exit_tests;
#[cfg(test)]
mod playback_start_tests;
#[cfg(test)]
mod tests;
