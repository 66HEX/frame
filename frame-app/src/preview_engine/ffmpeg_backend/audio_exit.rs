use super::process::{RunningProcessState, lock_process, push_stderr_line};
use std::collections::VecDeque;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::{Duration, Instant};

pub(super) const AUDIO_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const AUDIO_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum AudioExitOutcome {
    Pending,
    Exited(ExitStatus),
    Cancelled,
    Unknown(String),
}

pub(super) struct AudioExitUpdate {
    changed: bool,
    diagnostic: Option<String>,
}

/// Bound to one child by its stop-token identity. When both locks are needed,
/// process state is always acquired before this cache. Publishing never holds
/// the process lock, and cached-state reads never retain a guard across polling.
#[derive(Debug)]
pub(super) struct AudioProcessExit {
    pub(super) stop_requested: Arc<AtomicBool>,
    outcome: Mutex<AudioExitOutcome>,
    changed: Condvar,
    stderr: Arc<Mutex<VecDeque<String>>>,
}

impl AudioProcessExit {
    pub(super) const fn new(
        stop_requested: Arc<AtomicBool>,
        stderr: Arc<Mutex<VecDeque<String>>>,
    ) -> Self {
        Self {
            stop_requested,
            outcome: Mutex::new(AudioExitOutcome::Pending),
            changed: Condvar::new(),
            stderr,
        }
    }

    pub(super) fn record_exit(&self, status: ExitStatus) -> AudioExitUpdate {
        let mut outcome = self
            .outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(
            *outcome,
            AudioExitOutcome::Exited(_) | AudioExitOutcome::Cancelled
        ) {
            return AudioExitUpdate {
                changed: false,
                diagnostic: None,
            };
        }
        *outcome = AudioExitOutcome::Exited(status);
        drop(outcome);
        AudioExitUpdate {
            changed: true,
            diagnostic: (!status.success())
                .then(|| format!("audio preview process exited with {status}")),
        }
    }

    pub(super) fn publish(&self, update: AudioExitUpdate) {
        if update.changed {
            self.changed.notify_all();
        }
        if let Some(message) = update.diagnostic {
            push_stderr_line(&self.stderr, message);
        }
    }

    pub(super) fn cancel(&self) {
        let mut outcome = self
            .outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !matches!(*outcome, AudioExitOutcome::Exited(_)) {
            *outcome = AudioExitOutcome::Cancelled;
        }
        drop(outcome);
        self.changed.notify_all();
    }

    pub(super) fn observe_after_registration(
        &self,
        process: &Mutex<RunningProcessState>,
        registered: &mpsc::Receiver<()>,
    ) {
        if registered.recv().is_ok() {
            self.wait_for_exit(process, AUDIO_EXIT_TIMEOUT);
        } else {
            self.cancel();
        }
    }

    fn unknown(&self, message: String) -> AudioExitOutcome {
        let mut outcome = self
            .outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let report = if matches!(*outcome, AudioExitOutcome::Pending) {
            *outcome = AudioExitOutcome::Unknown(message.clone());
            Some(message)
        } else {
            None
        };
        let result = outcome.clone();
        drop(outcome);
        self.changed.notify_all();
        if let Some(message) = report {
            push_stderr_line(&self.stderr, message);
        }
        result
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "The cache guard is consumed by Condvar::wait_timeout, which atomically releases it while waiting."
    )]
    pub(super) fn wait_for_exit(
        &self,
        process: &Mutex<RunningProcessState>,
        timeout: Duration,
    ) -> AudioExitOutcome {
        let deadline = Instant::now() + timeout;
        loop {
            if self.stop_requested.load(Ordering::SeqCst) {
                self.cancel();
                return AudioExitOutcome::Cancelled;
            }
            match self.poll(process) {
                AudioExitOutcome::Pending => {}
                outcome => return outcome,
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return self.unknown(
                    "audio preview process exit is unknown: timed out after pipe closure".into(),
                );
            }
            let outcome = self
                .outcome
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // Stop publishes cancellation and wakes this wait immediately. An
            // observer only enters this bounded loop after stderr/PCM EOF.
            if self.stop_requested.load(Ordering::SeqCst)
                || !matches!(*outcome, AudioExitOutcome::Pending)
            {
                drop(outcome);
                continue;
            }
            let waited = self
                .changed
                .wait_timeout(outcome, remaining.min(AUDIO_EXIT_POLL_INTERVAL))
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            drop(waited);
        }
    }

    fn poll(&self, process: &Mutex<RunningProcessState>) -> AudioExitOutcome {
        let mut process = lock_process(process);
        // Reapers classify the child before removing it. Read the cached
        // classification under the same lock so removal cannot erase proof.
        let cached = self
            .outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if !matches!(cached, AudioExitOutcome::Pending) {
            return cached;
        }
        if !process
            .stop_requested
            .as_ref()
            .is_some_and(|token| Arc::ptr_eq(token, &self.stop_requested))
        {
            drop(process);
            self.cancel();
            return AudioExitOutcome::Cancelled;
        }
        let result = process.child.as_mut().map(std::process::Child::try_wait);
        match result {
            Some(Ok(Some(status))) => {
                let update = self.record_exit(status);
                drop(process);
                self.publish(update);
                AudioExitOutcome::Exited(status)
            }
            Some(Ok(None)) => AudioExitOutcome::Pending,
            Some(Err(error)) => {
                drop(process);
                self.unknown(format!("audio preview process exit is unknown: {error}"))
            }
            None => {
                drop(process);
                self.cancel();
                AudioExitOutcome::Cancelled
            }
        }
    }
}
