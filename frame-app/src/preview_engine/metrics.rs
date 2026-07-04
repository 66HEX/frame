use std::{
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreviewRuntimeMetrics {
    pub playback_generation: u64,
    pub video_process_spawns: u64,
    pub audio_process_spawns: u64,
    pub video_frames_read: u64,
    pub video_frame_bytes_read: u64,
    pub video_frame_read_total_us: u64,
    pub video_frames_published: u64,
    pub video_frames_dropped: u64,
    pub render_image_conversions: u64,
    pub render_image_bytes: u64,
    pub render_image_conversion_total_us: u64,
    pub rendered_frames_presented: u64,
    pub audio_pcm_chunks: u64,
    pub audio_pcm_bytes_read: u64,
    pub audio_samples_queued: u64,
    pub audio_output_callbacks: u64,
    pub audio_output_underruns: u64,
    pub first_video_frame_read_ms: Option<u64>,
    pub first_video_frame_published_ms: Option<u64>,
    pub first_audio_pcm_ms: Option<u64>,
    pub first_audio_callback_ms: Option<u64>,
    pub first_render_image_ms: Option<u64>,
    pub first_presented_ms: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PreviewRuntimeMetricsStore {
    inner: Arc<Mutex<PreviewRuntimeMetricsState>>,
}

#[derive(Debug, Default)]
struct PreviewRuntimeMetricsState {
    generation_started_at: Option<Instant>,
    metrics: PreviewRuntimeMetrics,
}

impl PreviewRuntimeMetricsStore {
    #[must_use]
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn begin_generation(&self, generation: u64) {
        self.begin_generation_at(generation, Instant::now());
    }

    pub(super) fn begin_generation_at(&self, generation: u64, started_at: Instant) {
        let mut state = lock_metrics(&self.inner);
        state.generation_started_at = Some(started_at);
        state.metrics.playback_generation = generation;
        state.metrics.first_video_frame_read_ms = None;
        state.metrics.first_video_frame_published_ms = None;
        state.metrics.first_audio_pcm_ms = None;
        state.metrics.first_audio_callback_ms = None;
        state.metrics.first_render_image_ms = None;
        state.metrics.first_presented_ms = None;
    }

    pub(super) fn record_video_process_spawn(&self) {
        let mut state = lock_metrics(&self.inner);
        state.metrics.video_process_spawns = state.metrics.video_process_spawns.saturating_add(1);
    }

    pub(super) fn record_audio_process_spawn(&self) {
        let mut state = lock_metrics(&self.inner);
        state.metrics.audio_process_spawns = state.metrics.audio_process_spawns.saturating_add(1);
    }

    pub(super) fn record_video_frame_read(
        &self,
        generation: u64,
        bytes: usize,
        read_elapsed: Duration,
    ) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation {
            return;
        }
        state.metrics.video_frames_read = state.metrics.video_frames_read.saturating_add(1);
        state.metrics.video_frame_bytes_read = state
            .metrics
            .video_frame_bytes_read
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        state.metrics.video_frame_read_total_us = state
            .metrics
            .video_frame_read_total_us
            .saturating_add(duration_us(read_elapsed));
        set_first_elapsed_ms(
            state.generation_started_at,
            &mut state.metrics.first_video_frame_read_ms,
        );
    }

    pub(super) fn record_video_first_frame_read_duration(&self, generation: u64, elapsed_ms: u64) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation
            || state.metrics.first_video_frame_read_ms.is_some()
        {
            return;
        }
        state.metrics.first_video_frame_read_ms = Some(elapsed_ms);
    }

    pub(super) fn record_video_frame_published(&self, generation: u64) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation {
            return;
        }
        state.metrics.video_frames_published =
            state.metrics.video_frames_published.saturating_add(1);
        set_first_elapsed_ms(
            state.generation_started_at,
            &mut state.metrics.first_video_frame_published_ms,
        );
    }

    pub(super) fn record_video_frame_dropped(&self, generation: u64) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation {
            return;
        }
        state.metrics.video_frames_dropped = state.metrics.video_frames_dropped.saturating_add(1);
    }

    pub(super) fn record_render_image_converted(
        &self,
        generation: u64,
        bytes: usize,
        elapsed: Duration,
    ) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation {
            return;
        }
        state.metrics.render_image_conversions =
            state.metrics.render_image_conversions.saturating_add(1);
        state.metrics.render_image_bytes = state
            .metrics
            .render_image_bytes
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        state.metrics.render_image_conversion_total_us = state
            .metrics
            .render_image_conversion_total_us
            .saturating_add(duration_us(elapsed));
        set_first_elapsed_ms(
            state.generation_started_at,
            &mut state.metrics.first_render_image_ms,
        );
    }

    pub(super) fn record_frame_presented(&self) {
        let mut state = lock_metrics(&self.inner);
        state.metrics.rendered_frames_presented =
            state.metrics.rendered_frames_presented.saturating_add(1);
        set_first_elapsed_ms(
            state.generation_started_at,
            &mut state.metrics.first_presented_ms,
        );
    }

    pub(super) fn record_audio_pcm_chunk(&self, generation: u64, bytes: usize, samples: usize) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation {
            return;
        }
        state.metrics.audio_pcm_chunks = state.metrics.audio_pcm_chunks.saturating_add(1);
        state.metrics.audio_pcm_bytes_read = state
            .metrics
            .audio_pcm_bytes_read
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        state.metrics.audio_samples_queued = state
            .metrics
            .audio_samples_queued
            .saturating_add(u64::try_from(samples).unwrap_or(u64::MAX));
        set_first_elapsed_ms(
            state.generation_started_at,
            &mut state.metrics.first_audio_pcm_ms,
        );
    }

    pub(super) fn record_audio_output_callback(&self, generation: u64, underrun: bool) {
        let mut state = lock_metrics(&self.inner);
        if state.metrics.playback_generation != generation {
            return;
        }
        state.metrics.audio_output_callbacks =
            state.metrics.audio_output_callbacks.saturating_add(1);
        if underrun {
            state.metrics.audio_output_underruns =
                state.metrics.audio_output_underruns.saturating_add(1);
        }
        set_first_elapsed_ms(
            state.generation_started_at,
            &mut state.metrics.first_audio_callback_ms,
        );
    }

    #[must_use]
    pub(super) fn snapshot(&self) -> PreviewRuntimeMetrics {
        lock_metrics(&self.inner).metrics
    }
}

#[must_use]
pub(super) fn preview_runtime_metrics_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("FRAME_PREVIEW_METRICS").is_some())
}

pub(super) fn log_preview_runtime_metrics(label: &str, metrics: PreviewRuntimeMetrics) {
    if !preview_runtime_metrics_enabled() {
        return;
    }

    eprintln!(
        "frame preview metrics [{label}]: generation={} video_spawns={} audio_spawns={} frames_read={} frame_bytes={} read_total_us={} frames_published={} frames_dropped={} render_conversions={} render_bytes={} render_total_us={} presented={} audio_chunks={} audio_bytes={} audio_samples={} audio_callbacks={} audio_underruns={} first_video_read_ms={:?} first_video_published_ms={:?} first_audio_pcm_ms={:?} first_audio_callback_ms={:?} first_render_ms={:?} first_presented_ms={:?}",
        metrics.playback_generation,
        metrics.video_process_spawns,
        metrics.audio_process_spawns,
        metrics.video_frames_read,
        metrics.video_frame_bytes_read,
        metrics.video_frame_read_total_us,
        metrics.video_frames_published,
        metrics.video_frames_dropped,
        metrics.render_image_conversions,
        metrics.render_image_bytes,
        metrics.render_image_conversion_total_us,
        metrics.rendered_frames_presented,
        metrics.audio_pcm_chunks,
        metrics.audio_pcm_bytes_read,
        metrics.audio_samples_queued,
        metrics.audio_output_callbacks,
        metrics.audio_output_underruns,
        metrics.first_video_frame_read_ms,
        metrics.first_video_frame_published_ms,
        metrics.first_audio_pcm_ms,
        metrics.first_audio_callback_ms,
        metrics.first_render_image_ms,
        metrics.first_presented_ms,
    );
}

fn set_first_elapsed_ms(started_at: Option<Instant>, slot: &mut Option<u64>) {
    if slot.is_some() {
        return;
    }
    *slot = started_at.map(elapsed_ms);
}

fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn duration_us(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn lock_metrics(
    state: &Mutex<PreviewRuntimeMetricsState>,
) -> MutexGuard<'_, PreviewRuntimeMetricsState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
