use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use super::PreviewRenderedFrame;

#[derive(Clone, Debug, Default)]
pub struct LatestFrameStore {
    inner: Arc<Mutex<LatestFrameState>>,
}

#[derive(Debug, Default)]
struct LatestFrameState {
    generation: u64,
    latest: Option<Arc<PreviewRenderedFrame>>,
    last_presented_generation: u64,
    stats: PreviewFrameStats,
    last_publish: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreviewFrameStats {
    pub published_frames: u64,
    pub presented_frames: u64,
    pub overwritten_before_present: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatestFrameSnapshot {
    pub generation: u64,
    pub frame: Arc<PreviewRenderedFrame>,
    pub stats: PreviewFrameStats,
}

impl LatestFrameStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn publish(&self, frame: PreviewRenderedFrame) -> LatestFrameSnapshot {
        let mut state = lock_state(&self.inner);
        if state.latest.is_some() && state.last_presented_generation < state.generation {
            state.stats.overwritten_before_present =
                state.stats.overwritten_before_present.saturating_add(1);
        }
        state.generation = state.generation.saturating_add(1);
        state.stats.published_frames = state.stats.published_frames.saturating_add(1);
        state.last_publish = Some(Instant::now());
        let frame = Arc::new(frame);
        state.latest = Some(Arc::clone(&frame));

        LatestFrameSnapshot {
            generation: state.generation,
            frame,
            stats: state.stats,
        }
    }

    #[must_use]
    pub fn latest(&self) -> Option<LatestFrameSnapshot> {
        let state = lock_state(&self.inner);
        let frame = state.latest.clone()?;
        Some(LatestFrameSnapshot {
            generation: state.generation,
            frame,
            stats: state.stats,
        })
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        lock_state(&self.inner).generation
    }

    #[must_use]
    pub fn stats(&self) -> PreviewFrameStats {
        lock_state(&self.inner).stats
    }

    #[must_use]
    pub fn has_unpresented_frame(&self) -> bool {
        let state = lock_state(&self.inner);
        state.latest.is_some() && state.last_presented_generation < state.generation
    }

    pub fn mark_presented(&self, generation: u64) {
        let mut state = lock_state(&self.inner);
        if generation == 0
            || generation > state.generation
            || generation <= state.last_presented_generation
        {
            return;
        }
        state.last_presented_generation = generation;
        state.stats.presented_frames = state.stats.presented_frames.saturating_add(1);
    }

    pub fn clear(&self) {
        let mut state = lock_state(&self.inner);
        state.generation = state.generation.saturating_add(1);
        state.latest = None;
        state.last_presented_generation = state.generation;
        state.last_publish = None;
    }
}

fn lock_state(state: &Mutex<LatestFrameState>) -> MutexGuard<'_, LatestFrameState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
