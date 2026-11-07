use std::time::{Duration, Instant};

pub(super) const AUDIO_DRAIN_NO_PROGRESS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AudioDrainPoll {
    Pending(Instant),
    Drained,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AudioDrainOutcome {
    Drained,
    Cancelled,
    Failed,
}

/// Protected by the PCM buffer mutex. Deadlines use callback-entry wall time,
/// not callback completion or padding, and never move backwards with jitter.
#[derive(Debug, Default)]
pub(super) struct AudioDrainState {
    pub(super) pipe_exhausted: bool,
    started: bool,
    progress_deadline: Option<Instant>,
    presentation_deadline: Option<Instant>,
    failure: Option<String>,
    failure_reported: bool,
}

impl AudioDrainState {
    pub(super) fn started(&mut self, now: Instant, queued_frames: usize) {
        self.started = true;
        if queued_frames > 0 {
            self.progress_deadline = Some(now + AUDIO_DRAIN_NO_PROGRESS_TIMEOUT);
        }
    }

    pub(super) fn queue_became_nonempty(&mut self, now: Instant) {
        if self.started {
            self.progress_deadline = Some(now + AUDIO_DRAIN_NO_PROGRESS_TIMEOUT);
        }
    }

    pub(super) fn consumed(
        &mut self,
        now: Instant,
        playback_delay: Duration,
        real_frames: usize,
        remaining_frames: usize,
        sample_rate: u32,
    ) {
        if real_frames == 0 {
            return;
        }
        let deadline = playback_delay
            .checked_add(frames_duration(real_frames, sample_rate))
            .and_then(|delay| now.checked_add(delay));
        let Some(deadline) = deadline else {
            self.fail("audio output playback timestamp is out of range".into());
            return;
        };
        self.presentation_deadline = Some(
            self.presentation_deadline
                .map_or(deadline, |previous| previous.max(deadline)),
        );
        self.progress_deadline = if remaining_frames == 0 {
            None
        } else {
            Some(now + AUDIO_DRAIN_NO_PROGRESS_TIMEOUT)
        };
    }

    pub(super) fn poll(&mut self, now: Instant, queued_frames: usize) -> AudioDrainPoll {
        if self.failed() {
            return AudioDrainPoll::Failed;
        }
        if !self.started {
            self.fail("audio output drain requested before output started".into());
            return AudioDrainPoll::Failed;
        }
        if queued_frames > 0 {
            let deadline = *self
                .progress_deadline
                .get_or_insert(now + AUDIO_DRAIN_NO_PROGRESS_TIMEOUT);
            if now >= deadline {
                self.fail("audio output made no progress for 2 seconds while draining".into());
                return AudioDrainPoll::Failed;
            }
            return AudioDrainPoll::Pending(deadline);
        }
        if let Some(deadline) = self.presentation_deadline
            && now < deadline
        {
            return AudioDrainPoll::Pending(deadline);
        }
        AudioDrainPoll::Drained
    }

    pub(super) fn fail(&mut self, message: String) {
        if self.failure.is_none() {
            self.failure = Some(message);
        }
    }

    pub(super) const fn failed(&self) -> bool {
        self.failure.is_some()
    }

    pub(super) fn take_failure_report(&mut self) -> Option<String> {
        if self.failure_reported {
            return None;
        }
        let failure = self.failure.clone()?;
        self.failure_reported = true;
        Some(failure)
    }
}

fn frames_duration(frames: usize, sample_rate: u32) -> Duration {
    let frames = u64::try_from(frames).unwrap_or(u64::MAX);
    let rate = u64::from(sample_rate.max(1));
    let nanos = ((frames % rate) * 1_000_000_000).div_ceil(rate);
    Duration::new(frames / rate, u32::try_from(nanos).unwrap_or(u32::MAX))
}
