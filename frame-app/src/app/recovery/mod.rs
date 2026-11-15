mod dialog;
mod runtime;
#[cfg(test)]
mod ui_tests;

use crate::app::root::FrameRoot;
use crate::workspace_recovery::{RecoveryError, RecoverySnapshot, RecoveryStore};
use gpui::Context;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(in crate::app) struct WorkspaceRecovery {
    store: Option<Arc<Mutex<RecoveryStore>>>,
    pending: Option<RecoverySnapshot>,
    last_snapshot: Option<RecoverySnapshot>,
    notice: Option<String>,
    ready: bool,
    loading: bool,
    scheduled: bool,
    change_epoch: u64,
    revision: u64,
    session_id: String,
    pending_update: Option<crate::update_session::UpdateSessionSnapshot>,
}

impl FrameRoot {
    pub(in crate::app) fn retain_pending_update(
        &mut self,
        snapshot: crate::update_session::UpdateSessionSnapshot,
    ) {
        self.workspace_recovery.pending_update = Some(snapshot);
    }

    fn finish_recovered_update(&mut self) -> Result<(), RecoveryError> {
        // Installation enters this UI state before its background writer can
        // replace the pending session. Keep that writer's handoff untouched.
        if self.update_installation_in_progress() {
            return Ok(());
        }
        let Some(expected) = self.workspace_recovery.pending_update.as_ref() else {
            return Ok(());
        };
        let finalize = || -> Result<(), crate::update_session::UpdateSessionError> {
            let store = self.update_session_store()?;
            // Run this on the UI thread so installation cannot start between
            // the state check and consumption. Preserve any newer snapshot.
            if store.load()?.as_ref() == Some(expected) {
                store.consume()?;
            }
            Ok(())
        };
        finalize().map_err(|error| {
            eprintln!("Failed to finalize recovered update session: {error}");
            RecoveryError::Invalid("the pending update queue could not be cleared")
        })?;
        self.workspace_recovery.pending_update = None;
        Ok(())
    }

    pub(in crate::app) fn recovery_blocks_workspace(&self) -> bool {
        self.recovery_loading()
            || self.recovery_prompt_count().is_some()
            || self.recovery_notice().is_some()
    }
    pub(in crate::app) fn recovery_prompt_count(&self) -> Option<usize> {
        self.workspace_recovery
            .pending
            .as_ref()
            .map(RecoverySnapshot::file_count)
    }

    pub(in crate::app) fn recovery_notice(&self) -> Option<&str> {
        self.workspace_recovery.notice.as_deref()
    }

    pub(in crate::app) const fn recovery_loading(&self) -> bool {
        self.workspace_recovery.loading
    }

    pub(in crate::app) fn restore_workspace(&mut self, cx: &mut Context<Self>) {
        match self.resolve_workspace(true) {
            Ok(targets) => {
                for (id, path) in targets {
                    self.queue_restored_source_metadata_probe(id, path, cx);
                }
            }
            Err(error) => self.fail_workspace_recovery(&error),
        }
        cx.notify();
    }

    pub(in crate::app) fn discard_workspace(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.resolve_workspace(false) {
            self.fail_workspace_recovery(&error);
        }
        cx.notify();
    }

    pub(in crate::app) fn defer_workspace_recovery(&mut self, cx: &mut Context<Self>) {
        self.workspace_recovery.pending = None;
        self.workspace_recovery.ready = false;
        cx.notify();
    }

    pub(in crate::app) fn dismiss_recovery_notice(&mut self, cx: &mut Context<Self>) {
        self.workspace_recovery.notice = None;
        cx.notify();
    }

    fn accept_recovery_store(
        &mut self,
        mut store: RecoveryStore,
        snapshot: Option<RecoverySnapshot>,
        updater_restored: bool,
    ) -> Result<(), RecoveryError> {
        static SESSION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        self.workspace_recovery.loading = false;
        self.workspace_recovery.session_id = format!(
            "{}-{}-{}",
            std::process::id(),
            crate::update_runtime::unix_timestamp(),
            SESSION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        if updater_restored
            || snapshot
                .as_ref()
                .is_none_or(|snapshot| snapshot.file_count() == 0)
        {
            store.resolve_existing()?;
            self.workspace_recovery.ready = true;
        } else {
            self.workspace_recovery.pending = snapshot;
        }
        self.workspace_recovery.store = Some(Arc::new(Mutex::new(store)));
        Ok(())
    }

    fn resolve_workspace(&mut self, restore: bool) -> Result<Vec<(String, String)>, RecoveryError> {
        let Some(snapshot) = self.workspace_recovery.pending.as_ref() else {
            return Ok(Vec::new());
        };
        if !self.file_queue.files().is_empty() || self.is_processing {
            return Err(RecoveryError::Invalid(
                "the current queue is no longer empty",
            ));
        }
        let restored = if restore {
            Some(snapshot.clone().restore()?)
        } else {
            None
        };
        self.workspace_recovery
            .store
            .as_ref()
            .ok_or(RecoveryError::Unavailable)?
            .lock()
            .map_err(|_| RecoveryError::Invalid("recovery writer stopped"))?
            .resolve_existing()?;
        self.workspace_recovery.pending = None;
        self.workspace_recovery.ready = true;
        if let Some(restored) = restored {
            self.metadata_probes.cancel_all();
            self.file_queue = restored.queue;
            self.active_view = restored.active_view;
            self.default_output_directory = restored.default_output_directory;
            self.next_file_sequence = restored.next_file_sequence;
            self.source_metadata = crate::source_metadata::SourceMetadataStore::default();
            self.conversion_events = crate::conversion_events::ConversionEventState::new();
            self.active_conversion_task_ids.clear();
            self.is_processing = false;
            Ok(restored.probe_targets)
        } else {
            Ok(Vec::new())
        }
    }

    fn prepare_recovery_write(&mut self) -> Result<Option<RecoveryWrite>, RecoveryError> {
        self.capture_recovery_write(false)
    }

    fn capture_recovery_write(
        &mut self,
        force: bool,
    ) -> Result<Option<RecoveryWrite>, RecoveryError> {
        if !self.workspace_recovery.ready {
            return Ok(None);
        }
        let Some(store) = self.workspace_recovery.store.as_ref() else {
            return Ok(None);
        };
        let revision = self
            .workspace_recovery
            .revision
            .checked_add(1)
            .ok_or(RecoveryError::Invalid("recovery revision exhausted"))?;
        let snapshot = RecoverySnapshot::capture(
            &self.file_queue,
            self.active_view,
            self.default_output_directory.clone(),
            self.workspace_recovery.session_id.clone(),
            revision,
            crate::update_runtime::unix_timestamp(),
        )?;
        if !force
            && self
                .workspace_recovery
                .last_snapshot
                .as_ref()
                .is_some_and(|previous| previous.same_workspace_as(&snapshot))
        {
            return Ok(None);
        }
        self.workspace_recovery.revision = revision;
        self.workspace_recovery.last_snapshot = Some(snapshot.clone());
        Ok(Some(RecoveryWrite {
            store: Arc::clone(store),
            snapshot,
        }))
    }

    fn fail_workspace_recovery(&mut self, error: &RecoveryError) {
        eprintln!("Workspace recovery unavailable: {error}");
        self.workspace_recovery.ready = false;
        self.workspace_recovery.loading = false;
        self.workspace_recovery.pending = None;
        let message = match error {
            RecoveryError::ProfileInUse => {
                "Another Frame window is using this workspace. Close that window and restart Frame to save or restore the queue."
            }
            RecoveryError::UnsupportedVersion(_) => {
                "This queue was saved by a newer version of Frame. Open it with that version. The saved queue will be kept unchanged."
            }
            RecoveryError::Io(_) => {
                "Frame could not read or save the queue. Check access to Frame’s settings folder, then restart Frame. New changes will not be saved for recovery in this session."
            }
            RecoveryError::TooLarge => {
                "This queue is too large to save for recovery. A previously saved queue will be kept. New changes will not be saved for recovery in this session."
            }
            _ => {
                "Frame could not restore or save the queue. Any existing saved queue will be kept. New changes will not be saved for recovery in this session."
            }
        };
        self.workspace_recovery.notice = Some(message.into());
    }
}

struct RecoveryWrite {
    store: Arc<Mutex<RecoveryStore>>,
    snapshot: RecoverySnapshot,
}

impl RecoveryWrite {
    fn save(self) -> Result<(), RecoveryError> {
        let result = self
            .store
            .lock()
            .map_err(|_| RecoveryError::Invalid("recovery writer stopped"))?
            .save(&self.snapshot);
        match result {
            // A final close-time flush may overtake an older background write.
            Err(RecoveryError::StaleRevision) => Ok(()),
            result => result,
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod writer_tests;

#[cfg(test)]
mod lifecycle_tests;

#[cfg(test)]
mod updater_tests;
