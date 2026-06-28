use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use super::PreviewFrame;

#[derive(Clone, Debug, Default)]
pub struct LatestFrameStore {
    inner: Arc<Mutex<LatestFrameState>>,
}

#[derive(Debug, Default)]
struct LatestFrameState {
    generation: u64,
    latest: Option<Arc<PreviewFrame>>,
    dropped_frames: u64,
    last_publish: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatestFrameSnapshot {
    pub generation: u64,
    pub frame: Arc<PreviewFrame>,
    pub dropped_frames: u64,
}

impl LatestFrameStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(&self, frame: PreviewFrame) -> LatestFrameSnapshot {
        let mut state = lock_state(&self.inner);
        if state.latest.is_some() {
            state.dropped_frames = state.dropped_frames.saturating_add(1);
        }
        state.generation = state.generation.saturating_add(1);
        state.last_publish = Some(Instant::now());
        let frame = Arc::new(frame);
        state.latest = Some(Arc::clone(&frame));

        LatestFrameSnapshot {
            generation: state.generation,
            frame,
            dropped_frames: state.dropped_frames,
        }
    }

    #[must_use]
    pub fn latest(&self) -> Option<LatestFrameSnapshot> {
        let state = lock_state(&self.inner);
        let frame = state.latest.as_ref().cloned()?;
        Some(LatestFrameSnapshot {
            generation: state.generation,
            frame,
            dropped_frames: state.dropped_frames,
        })
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        lock_state(&self.inner).generation
    }

    pub fn clear(&self) {
        let mut state = lock_state(&self.inner);
        state.generation = state.generation.saturating_add(1);
        state.latest = None;
        state.last_publish = None;
    }
}

fn lock_state(state: &Mutex<LatestFrameState>) -> MutexGuard<'_, LatestFrameState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
