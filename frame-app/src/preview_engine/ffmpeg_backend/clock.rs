use crate::numeric::f64_to_u64;
use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_drain::{AudioDrainOutcome, AudioDrainPoll};
use crate::preview_engine::ffmpeg_backend::audio_queue::{AudioQueueOutcome, AudioQueuePoll};
use crate::preview_engine::ffmpeg_backend::process::RunningPreviewProcess;
use crate::preview_engine::types::{
    PreviewPlaybackSnapshot, PreviewRenderedFrame, PreviewSessionConfig,
};
use frame_core::utils::parse_time;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
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
    pub(super) failed: bool,
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

/// One audio child's prestart wait, sharing the playback clock's notification.
/// Every wake predicate is changed while holding `playback`, so checking the
/// predicate and entering the condition-variable wait cannot lose a wakeup.
#[derive(Debug)]
pub(super) struct AudioPlaybackStart {
    playback: Arc<Mutex<PreviewPlaybackClock>>,
    changed: Arc<Condvar>,
    output_failed: AtomicBool,
    #[cfg(test)]
    pub(super) wait_count: std::sync::atomic::AtomicUsize,
}

impl AudioPlaybackStart {
    pub(super) const fn new(
        playback: Arc<Mutex<PreviewPlaybackClock>>,
        changed: Arc<Condvar>,
    ) -> Self {
        Self {
            playback,
            changed,
            output_failed: AtomicBool::new(false),
            #[cfg(test)]
            wait_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub(super) fn request_stop(&self, stop_requested: &AtomicBool) {
        let _playback = lock_playback(&self.playback);
        stop_requested.store(true, Ordering::SeqCst);
        self.changed.notify_all();
    }

    pub(super) fn mark_ended(&self, generation: u64) {
        mark_playback_ended(&self.playback, &self.changed, generation);
    }

    /// Buffer changes are published before this lock is acquired. Drain waits
    /// hold the clock while inspecting the buffer, so this cannot lose a wake.
    pub(super) fn notify_output_changed(&self) {
        let _playback = lock_playback(&self.playback);
        self.changed.notify_all();
    }

    pub(super) fn mark_output_failed(&self, end_generation: Option<u64>) {
        let mut playback = lock_playback(&self.playback);
        if let Some(generation) = end_generation {
            playback.fail_generation(generation);
        }
        self.output_failed.store(true, Ordering::SeqCst);
        drop(playback);
        self.changed.notify_all();
    }

    pub(super) fn wait_for_audio_drain(
        &self,
        stop_requested: &AtomicBool,
        generation: u64,
        mut poll: impl FnMut(Instant) -> AudioDrainPoll,
    ) -> AudioDrainOutcome {
        let mut playback = lock_playback(&self.playback);
        loop {
            if stop_requested.load(Ordering::SeqCst)
                || !playback.generation_matches(generation)
                || playback.ended
            {
                return AudioDrainOutcome::Cancelled;
            }
            if self.output_failed.load(Ordering::SeqCst) {
                return AudioDrainOutcome::Failed;
            }
            if !playback.playing {
                return AudioDrainOutcome::Cancelled;
            }
            match poll(Instant::now()) {
                AudioDrainPoll::Drained => return AudioDrainOutcome::Drained,
                AudioDrainPoll::Failed => return AudioDrainOutcome::Failed,
                AudioDrainPoll::Pending(deadline) => {
                    #[cfg(test)]
                    self.wait_count.fetch_add(1, Ordering::SeqCst);
                    let (guard, _) = self
                        .changed
                        .wait_timeout(playback, deadline.saturating_duration_since(Instant::now()))
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    playback = guard;
                }
            }
        }
    }

    /// The admission callback runs with the clock held, then takes the PCM
    /// buffer. Stop/generation changes cannot race a successful admission.
    pub(super) fn wait_for_audio_capacity(
        &self,
        stop_requested: &AtomicBool,
        generation: u64,
        mut poll: impl FnMut(Instant) -> AudioQueuePoll,
    ) -> AudioQueueOutcome {
        let mut playback = lock_playback(&self.playback);
        loop {
            if stop_requested.load(Ordering::SeqCst)
                || !playback.generation_matches(generation)
                || playback.ended
            {
                return AudioQueueOutcome::Cancelled;
            }
            if playback.failed || self.output_failed.load(Ordering::SeqCst) {
                return AudioQueueOutcome::Failed;
            }
            let deadline = match poll(Instant::now()) {
                AudioQueuePoll::Admitted(samples) => return AudioQueueOutcome::Admitted(samples),
                AudioQueuePoll::Failed => return AudioQueueOutcome::Failed,
                AudioQueuePoll::Pending(deadline) => deadline,
            };
            #[cfg(test)]
            self.wait_count.fetch_add(1, Ordering::SeqCst);
            playback = if let Some(deadline) = deadline {
                self.changed
                    .wait_timeout(playback, deadline.saturating_duration_since(Instant::now()))
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .0
            } else {
                self.changed
                    .wait(playback)
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
            };
        }
    }
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

    pub(in crate::preview_engine) fn failed(&self) -> bool {
        lock_playback(&self.playback).failed
    }

    pub(super) fn prepare_playback_generation(&self, seconds: f64) -> u64 {
        let generation = {
            let mut playback = lock_playback(&self.playback);
            playback.prepare_generation(seconds)
        };
        self.playback_changed.notify_all();
        self.metrics.begin_generation(generation);
        generation
    }

    pub(super) fn prepare_playback_generation_at(&self, seconds: f64, started_at: Instant) -> u64 {
        let generation = {
            let mut playback = lock_playback(&self.playback);
            playback.prepare_generation(seconds)
        };
        self.playback_changed.notify_all();
        self.metrics.begin_generation_at(generation, started_at);
        generation
    }

    pub(super) fn start_playback_generation(&self, generation: u64, seconds: f64) {
        lock_playback(&self.playback).start_generation(generation, seconds);
        self.playback_changed.notify_all();
    }

    pub(super) fn park_playback_generation(&self, generation: u64, seconds: f64) {
        lock_playback(&self.playback).park_generation(generation, seconds);
    }

    pub(super) fn parked_generation_at(&self, seconds: f64) -> Option<u64> {
        let playback = lock_playback(&self.playback);
        if playback.playing || playback.ended || playback.failed {
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

pub(super) const fn playback_stream_state(
    playback: &PreviewPlaybackClock,
    generation: u64,
) -> PlaybackStreamState {
    if !playback.generation_matches(generation) || playback.ended || playback.failed {
        return PlaybackStreamState::Stale;
    }
    if playback.playing {
        PlaybackStreamState::Ready
    } else {
        PlaybackStreamState::Waiting
    }
}

pub(super) fn mark_playback_ended(
    playback: &Mutex<PreviewPlaybackClock>,
    changed: &Condvar,
    generation: u64,
) {
    let mut playback = lock_playback(playback);
    if playback.generation_matches(generation) && !playback.failed {
        playback.playing = false;
        playback.ended = true;
        drop(playback);
        changed.notify_all();
    }
}

pub(super) fn frame_presentation_state(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    timestamp_seconds: f64,
) -> PlaybackStreamState {
    let playback = lock_playback(playback);
    if !playback.generation_matches(generation) || playback.ended || playback.failed {
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
    start: &AudioPlaybackStart,
    stop_requested: &AtomicBool,
    generation: u64,
) -> PlaybackStreamState {
    let mut playback = lock_playback(&start.playback);
    loop {
        if stop_requested.load(Ordering::SeqCst) || start.output_failed.load(Ordering::SeqCst) {
            return PlaybackStreamState::Stale;
        }
        match playback_stream_state(&playback, generation) {
            PlaybackStreamState::Ready => return PlaybackStreamState::Ready,
            PlaybackStreamState::Stale => return PlaybackStreamState::Stale,
            PlaybackStreamState::Waiting => {}
        }
        #[cfg(test)]
        start.wait_count.fetch_add(1, Ordering::SeqCst);
        playback = start
            .changed
            .wait(playback)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    if !playback.generation_matches(generation) || playback.failed {
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
    config
        .conversion_config
        .start_time
        .as_deref()
        .and_then(frame_core::trim::RawFfmpegTime::parse)
        .map_or(0.0, |time| time.seconds().max(0.0))
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
            playing: playing && !clock.ended && !clock.failed,
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
        self.failed = false;
        self.generation
    }

    pub(super) fn start_generation(&mut self, generation: u64, seconds: f64) {
        if self.generation != generation || self.failed {
            return;
        }
        self.base_seconds = seconds;
        self.last_frame_seconds = seconds;
        self.started_at = Instant::now();
        self.playing = true;
        self.ended = false;
    }

    pub(super) fn park_generation(&mut self, generation: u64, seconds: f64) {
        if self.generation != generation || self.failed {
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

    pub(super) fn fail_generation(&mut self, generation: u64) {
        if !self.generation_matches(generation) || self.failed || self.ended {
            return;
        }
        if self.playing {
            self.last_frame_seconds = (self.base_seconds + self.started_at.elapsed().as_secs_f64())
                .max(self.last_frame_seconds);
        }
        self.playing = false;
        self.failed = true;
    }
}
