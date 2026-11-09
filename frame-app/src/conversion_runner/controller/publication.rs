//! Cancellation cutoff for a verified, staged output.

use super::ConversionProcessController;
use frame_core::error::ConversionError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PublicationPhase {
    Committing,
    Terminal,
}

pub(in crate::conversion_runner) struct PublicationGuard {
    controller: ConversionProcessController,
    id: String,
    finished: bool,
}

impl ConversionProcessController {
    /// Registers queued attempts before their cancellation controls are exposed.
    ///
    /// Clears only prior terminal publications, preserving cancellation markers.
    /// Repeating registration at batch/worker entry is safe and idempotent.
    ///
    /// # Errors
    /// Returns an error if the controller lock is poisoned or any requested task
    /// is still publishing. No task state is changed on that failure.
    pub fn begin_queued_attempts(&self, ids: &[String]) -> Result<(), ConversionError> {
        let mut state = self.lock_state()?;
        if ids
            .iter()
            .any(|id| state.publications.get(id) == Some(&PublicationPhase::Committing))
        {
            return Err(ConversionError::Worker(
                "Wait for this export to finish before starting it again.".into(),
            ));
        }
        for id in ids {
            if state.publications.get(id) == Some(&PublicationPhase::Terminal) {
                state.publications.remove(id);
            }
        }
        drop(state);
        Ok(())
    }

    /// Single-task and worker defense after the earlier queue registration.
    pub(in crate::conversion_runner) fn begin_attempt(
        &self,
        id: &str,
    ) -> Result<(), ConversionError> {
        self.begin_queued_attempts(&[id.to_owned()])
    }

    #[cfg(test)]
    pub(crate) fn complete_publication_for_test(&self, id: &str) -> Result<(), ConversionError> {
        self.begin_attempt(id)?;
        self.begin_publication(id, false)?
            .ok_or_else(|| ConversionError::Worker("test publication was cancelled".into()))?
            .finish()
    }

    /// Serialized cancellation cutoff. Publication and callbacks happen outside
    /// this lock. After success, cancellation cannot change its terminal decision.
    pub(in crate::conversion_runner) fn begin_publication(
        &self,
        id: &str,
        previously_cancelled: bool,
    ) -> Result<Option<PublicationGuard>, ConversionError> {
        let mut state = self.lock_state()?;
        // Always remove the marker, including with an earlier saved cancellation.
        let late_cancelled = state.cancelled_tasks.remove(id);
        if previously_cancelled || late_cancelled {
            return Ok(None);
        }
        if state.active_processes.contains_key(id) || state.publications.contains_key(id) {
            return Err(ConversionError::Worker(
                "Export is not ready for publication".into(),
            ));
        }
        state
            .publications
            .insert(id.to_owned(), PublicationPhase::Committing);
        drop(state);
        Ok(Some(PublicationGuard {
            controller: self.clone(),
            id: id.to_owned(),
            finished: false,
        }))
    }

    fn record_publication_decision(&self, id: &str) -> Result<(), ConversionError> {
        let mut state = self.lock_state()?;
        if state.publications.get(id) != Some(&PublicationPhase::Committing) {
            return Err(ConversionError::Worker(
                "Export publication ownership changed".into(),
            ));
        }
        state
            .publications
            .insert(id.to_owned(), PublicationPhase::Terminal);
        drop(state);
        Ok(())
    }
}

impl PublicationGuard {
    pub fn finish(mut self) -> Result<(), ConversionError> {
        self.controller.record_publication_decision(&self.id)?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for PublicationGuard {
    fn drop(&mut self) {
        if !self.finished {
            // A panic must not leave this id permanently Committing or turn a
            // late cancellation into a marker that poisons an explicit retry.
            let _ = self.controller.record_publication_decision(&self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_before_cutoff_prevents_publication_and_is_always_consumed() {
        let controller = ConversionProcessController::default();
        controller.cancel_task("id").unwrap();
        controller.begin_attempt("id").unwrap();
        assert!(controller.begin_publication("id", false).unwrap().is_none());
        assert!(!controller.take_cancelled("id").unwrap());
        controller.cancel_task("id").unwrap();
        assert!(controller.begin_publication("id", true).unwrap().is_none());
        assert!(!controller.take_cancelled("id").unwrap());
    }

    #[test]
    fn publication_requires_the_final_child_to_be_unregistered() {
        let controller = ConversionProcessController::default();
        controller.register_started_process("id", 0).unwrap();
        assert!(controller.begin_publication("id", false).is_err());
        controller.finish_task("id").unwrap();
        assert_eq!(controller.active_process_count().unwrap(), 0);
        let owner = controller.begin_publication("id", false).unwrap().unwrap();
        owner.finish().unwrap();
    }

    #[test]
    fn cancellation_after_cutoff_does_not_poison_same_id_retry() {
        let controller = ConversionProcessController::default();
        controller.begin_attempt("id").unwrap();
        let owner = controller.begin_publication("id", false).unwrap().unwrap();
        assert!(controller.begin_attempt("id").is_err());
        let other = controller.clone();
        std::thread::spawn(move || other.cancel_task("id").unwrap())
            .join()
            .unwrap();
        assert!(!controller.is_cancelled("id"));
        owner.finish().unwrap();
        controller.cancel_task("id").unwrap();
        assert!(!controller.take_cancelled("id").unwrap());
        controller.begin_attempt("id").unwrap();
        controller.cancel_task("id").unwrap();
        assert!(controller.take_cancelled("id").unwrap());
    }

    #[test]
    fn publication_owner_unwind_closes_the_cutoff_without_leaving_a_running_phase() {
        let controller = ConversionProcessController::default();
        let other = controller.clone();
        assert!(
            std::panic::catch_unwind(move || {
                let _owner = other.begin_publication("id", false).unwrap().unwrap();
                panic!("publication failure after cutoff");
            })
            .is_err()
        );
        controller.cancel_task("id").unwrap();
        assert!(!controller.take_cancelled("id").unwrap());
        controller.begin_attempt("id").unwrap();
        assert!(controller.begin_publication("id", false).unwrap().is_some());
    }

    #[test]
    fn bulk_queue_registration_is_atomic_and_never_clears_pending_cancellation() {
        let controller = ConversionProcessController::default();
        controller.complete_publication_for_test("first").unwrap();
        let committing = controller
            .begin_publication("second", false)
            .unwrap()
            .unwrap();
        let ids = ["first".to_owned(), "second".to_owned()];
        assert!(controller.begin_queued_attempts(&ids).is_err());
        controller.cancel_task("first").unwrap();
        assert!(
            !controller.is_cancelled("first"),
            "failed bulk registration must leave the first Terminal intact"
        );
        committing.finish().unwrap();
        controller.begin_queued_attempts(&ids).unwrap();
        controller.cancel_task("second").unwrap();
        // UI registration, public batch registration, then a delayed worker.
        controller.begin_queued_attempts(&ids).unwrap();
        controller.begin_attempt("second").unwrap();
        assert!(controller.take_cancelled("second").unwrap());
        controller.cancel_task("first").unwrap();
        controller.begin_attempt("first").unwrap();
        assert!(controller.take_cancelled("first").unwrap());
    }
}
