use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::bounded_process::CleanupTask;
use crate::preview_engine::PreviewEngineError;

use super::process::ProcessHandles;
use super::startup_cleanup::StartupCleanup;

/// Pending retirements contain only already-admitted process owners. Once one
/// remains pending, admission closes until every retained owner is finished.
/// Existing sessions can still retire, without creating a worker per request or
/// imposing a new limit on the number of normally running preview sessions.
pub(super) struct CleanupBroker {
    queue: Mutex<Vec<StartupCleanup>>,
    changed: Condvar,
    pending: AtomicUsize,
}

impl CleanupBroker {
    pub(super) fn shared() -> Result<&'static Arc<Self>, PreviewEngineError> {
        static BROKER: OnceLock<Result<Arc<CleanupBroker>, String>> = OnceLock::new();
        BROKER
            .get_or_init(|| {
                let broker = Arc::new(Self {
                    queue: Mutex::new(Vec::new()),
                    changed: Condvar::new(),
                    pending: AtomicUsize::new(0),
                });
                let worker = Arc::clone(&broker);
                std::thread::Builder::new()
                    .name("frame-preview-cleanup".into())
                    .spawn(move || worker.supervise())
                    .map_err(|error| error.to_string())?;
                Ok(broker)
            })
            .as_ref()
            .map_err(|error| {
                PreviewEngineError::Ffmpeg(format!("could not initialize preview cleanup: {error}"))
            })
    }

    pub(super) fn check_admission(&self) -> Result<(), PreviewEngineError> {
        if self.pending.load(Ordering::Acquire) != 0 {
            return Err(PreviewEngineError::Ffmpeg(
                "preview cleanup is still in progress; retry shortly".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn retire(&self, handles: ProcessHandles) -> Result<(), PreviewEngineError> {
        let mut owner = StartupCleanup(vec![handles]);
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if owner.cleanup_step() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                self.pending.fetch_add(1, Ordering::AcqRel);
                self.queue
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(owner);
                self.changed.notify_one();
                return Err(PreviewEngineError::Ffmpeg(
                    "preview cleanup is still in progress; retry shortly".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn supervise(&self) {
        loop {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while queue.is_empty() {
                queue = self
                    .changed
                    .wait(queue)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            let mut owners = std::mem::take(&mut *queue);
            drop(queue);
            owners.retain_mut(|owner| {
                if owner.cleanup_step() {
                    self.pending.fetch_sub(1, Ordering::AcqRel);
                    false
                } else {
                    true
                }
            });
            self.queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend(owners);
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
