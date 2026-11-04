use crate::numeric::f64_to_u64;
use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::process::RunningPreviewProcess;
use crate::preview_engine::types::{
    PreviewPlaybackSnapshot, PreviewRenderedFrame, PreviewSessionConfig,
};
use frame_core::utils::parse_time;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

pub(super) const SEEK_END_EPSILON: f64 = 0.001;

pub(super) const VIDEO_START_WAIT_INTERVAL: Duration = Duration::from_millis(2);

pub(super) const VIDEO_FRAME_TIMING_EPSILON_SECONDS: f64 = 0.002;

#[derive(Clone, Copy, Debug)]
pub(super) struct PreviewPlaybackClock {
    pub(super) generation: u64,
    pub(super) base_seconds: f64,
    pub(super) last_frame_seconds: f64,
    pub(super) started_at: Instant,
    pub(super) playing: bool,
    pub(super) ended: bool,
}

/// Read-only clock access for UI snapshots. It never owns or reaps `FFmpeg` processes.
#[derive(Clone)]
pub(in crate::preview_engine) struct PreviewPlaybackReader {
    pub(super) clock: Arc<Mutex<PreviewPlaybackClock>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlaybackStreamState {
    Ready,
    Waiting,
    Stale,
}

impl RunningPreviewProcess {
    #[must_use]
    pub(in crate::preview_engine) fn playback_reader(&self) -> PreviewPlaybackReader {
        PreviewPlaybackReader {
            clock: Arc::clone(&self.playback),
        }
    }

    #[must_use]
    pub fn position(&self) -> f64 {
        self.reap_finished_processes();
        let playback = lock_playback(&self.playback);
        if playback.playing {
            let clock_seconds = playback.base_seconds + playback.started_at.elapsed().as_secs_f64();
            clock_seconds.max(playback.last_frame_seconds)
        } else {
            playback.last_frame_seconds
        }
    }

    #[must_use]
    pub const fn duration(&self) -> f64 {
        self.config.duration_seconds
    }

    #[must_use]
    pub fn ended(&self) -> bool {
        self.reap_finished_processes();
        lock_playback(&self.playback).ended
    }

    pub(super) fn prepare_playback_generation(&self, seconds: f64) -> u64 {
        let generation = {
            let mut playback = lock_playback(&self.playback);
            playback.prepare_generation(seconds)
        };
        self.metrics.begin_generation(generation);
        generation
    }

    pub(super) fn prepare_playback_generation_at(&self, seconds: f64, started_at: Instant) -> u64 {
        let generation = {
            let mut playback = lock_playback(&self.playback);
            playback.prepare_generation(seconds)
        };
        self.metrics.begin_generation_at(generation, started_at);
        generation
    }

    pub(super) fn start_playback_generation(&self, generation: u64, seconds: f64) {
        lock_playback(&self.playback).start_generation(generation, seconds);
    }

    pub(super) fn park_playback_generation(&self, generation: u64, seconds: f64) {
        lock_playback(&self.playback).park_generation(generation, seconds);
    }

    pub(super) fn parked_generation_at(&self, seconds: f64) -> Option<u64> {
        let playback = lock_playback(&self.playback);
        if playback.playing || playback.ended {
            return None;
        }
        if (playback.last_frame_seconds - seconds).abs() > SEEK_END_EPSILON {
            return None;
        }
        Some(playback.generation)
    }

    pub(super) fn playback_generation_matches(&self, generation: u64) -> bool {
        lock_playback(&self.playback).generation_matches(generation)
    }

    pub(super) fn publish_frame_for_generation(
        &self,
        frame: PreviewRenderedFrame,
        render_elapsed: Duration,
        generation: u64,
    ) {
        if !self.playback_generation_matches(generation) {
            return;
        }
        self.metrics
            .record_render_image_converted(generation, frame.byte_len, render_elapsed);
        let _ = self.frame_store.publish(frame);
        self.metrics.record_video_frame_published(generation);
    }
}

pub(super) fn clock_generation_matches(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> bool {
    lock_playback(playback).generation_matches(generation)
}

pub(super) fn playback_stream_state(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> PlaybackStreamState {
    let playback = lock_playback(playback);
    if !playback.generation_matches(generation) || playback.ended {
        return PlaybackStreamState::Stale;
    }
    if playback.playing {
        PlaybackStreamState::Ready
    } else {
        PlaybackStreamState::Waiting
    }
}

pub(super) fn frame_presentation_state(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    timestamp_seconds: f64,
) -> PlaybackStreamState {
    let playback = lock_playback(playback);
    if !playback.generation_matches(generation) || playback.ended {
        return PlaybackStreamState::Stale;
    }
    if !playback.playing {
        return PlaybackStreamState::Waiting;
    }
    let clock_seconds = playback.base_seconds + playback.started_at.elapsed().as_secs_f64();
    if clock_seconds + VIDEO_FRAME_TIMING_EPSILON_SECONDS >= timestamp_seconds {
        PlaybackStreamState::Ready
    } else {
        PlaybackStreamState::Waiting
    }
}

pub(super) fn wait_for_playback_start(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    stop_requested: &AtomicBool,
    generation: u64,
) -> PlaybackStreamState {
    loop {
        if stop_requested.load(Ordering::SeqCst) {
            return PlaybackStreamState::Stale;
        }
        match playback_stream_state(playback, generation) {
            PlaybackStreamState::Ready => return PlaybackStreamState::Ready,
            PlaybackStreamState::Stale => return PlaybackStreamState::Stale,
            PlaybackStreamState::Waiting => thread::sleep(VIDEO_START_WAIT_INTERVAL),
        }
    }
}

pub(super) fn wait_for_frame_presentation_time(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    stop_requested: &AtomicBool,
    generation: u64,
    timestamp_seconds: f64,
) -> PlaybackStreamState {
    loop {
        if stop_requested.load(Ordering::SeqCst) {
            return PlaybackStreamState::Stale;
        }
        match frame_presentation_state(playback, generation, timestamp_seconds) {
            PlaybackStreamState::Ready => return PlaybackStreamState::Ready,
            PlaybackStreamState::Stale => return PlaybackStreamState::Stale,
            PlaybackStreamState::Waiting => thread::sleep(VIDEO_START_WAIT_INTERVAL),
        }
    }
}

pub(super) fn update_last_frame_seconds(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    timestamp_seconds: f64,
) -> bool {
    let mut playback = lock_playback(playback);
    if !playback.generation_matches(generation) {
        return false;
    }
    playback.last_frame_seconds = timestamp_seconds;
    playback.ended = false;
    true
}

pub(super) fn parsed_time(value: Option<&str>) -> Option<f64> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(parse_time)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

pub(super) fn initial_start_seconds(config: &PreviewSessionConfig) -> f64 {
    parsed_time(config.conversion_config.start_time.as_deref()).unwrap_or(0.0)
}

pub(super) fn clamp_seek_seconds(seconds: f64, duration: f64) -> Result<f64, PreviewEngineError> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(PreviewEngineError::InvalidInput(
            "seek position must be a positive finite number".to_string(),
        ));
    }

    if !duration.is_finite() || duration <= 0.0 {
        return Ok(seconds);
    }

    Ok(seconds.min((duration - SEEK_END_EPSILON).max(0.0)))
}

pub(super) fn seconds_to_us(seconds: f64) -> u64 {
    f64_to_u64(seconds.max(0.0) * 1_000_000.0)
}

pub(super) fn lock_playback(
    state: &Mutex<PreviewPlaybackClock>,
) -> MutexGuard<'_, PreviewPlaybackClock> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl PreviewPlaybackReader {
    pub(in crate::preview_engine) fn snapshot(
        &self,
        duration_seconds: f64,
        playing: bool,
    ) -> PreviewPlaybackSnapshot {
        let clock = *lock_playback(&self.clock);
        let position_seconds = if clock.ended {
            duration_seconds
        } else if clock.playing {
            (clock.base_seconds + clock.started_at.elapsed().as_secs_f64())
                .max(clock.last_frame_seconds)
        } else {
            clock.last_frame_seconds
        };
        PreviewPlaybackSnapshot {
            position_seconds,
            duration_seconds,
            // Rebuilding briefly parks the clock while retaining playback intent.
            playing: playing && !clock.ended,
        }
    }
}

impl PreviewPlaybackClock {
    pub(super) fn prepare_generation(&mut self, seconds: f64) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = false;
        self.ended = false;
        self.generation
    }

    pub(super) fn start_generation(&mut self, generation: u64, seconds: f64) {
        if self.generation != generation {
            return;
        }
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = true;
        self.ended = false;
    }

    pub(super) fn park_generation(&mut self, generation: u64, seconds: f64) {
        if self.generation != generation {
            return;
        }
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = false;
        self.ended = false;
    }

    pub(super) const fn generation_matches(&self, generation: u64) -> bool {
        self.generation == generation
    }
}
