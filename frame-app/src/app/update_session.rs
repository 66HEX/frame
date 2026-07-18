use super::*;
use crate::update_session::{UpdateSessionError, UpdateSessionSnapshot, UpdateSessionStore};

impl FrameRoot {
    pub(super) fn update_session_store(&self) -> Result<UpdateSessionStore, UpdateSessionError> {
        let persistence = self
            .persistence
            .as_ref()
            .ok_or(UpdateSessionError::PersistenceUnavailable)?;
        Ok(UpdateSessionStore::from_settings_path(
            persistence.settings_path(),
        ))
    }

    pub(super) fn capture_update_session(
        &self,
        target_version: impl Into<String>,
    ) -> Result<UpdateSessionSnapshot, UpdateSessionError> {
        UpdateSessionSnapshot::capture(
            &self.file_queue,
            self.active_view,
            FRAME_APP_VERSION,
            target_version,
            unix_timestamp(),
        )
    }

    pub(super) fn restore_pending_update_session(&mut self, cx: &mut Context<Self>) {
        let Ok(store) = self.update_session_store() else {
            return;
        };
        let snapshot = match store.load() {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return,
            Err(error) => {
                eprintln!("Failed to load pending update session: {error}");
                return;
            }
        };
        let restored = match snapshot.restore() {
            Ok(restored) => restored,
            Err(error) => {
                eprintln!("Failed to restore pending update session: {error}");
                return;
            }
        };

        self.active_view = restored.active_view;
        self.file_queue = restored.queue;
        self.next_file_sequence = restored.next_file_sequence;
        self.conversion_events = ConversionEventState::new();
        self.source_metadata = SourceMetadataStore::default();
        self.active_conversion_task_ids.clear();
        self.is_processing = false;

        if let Err(error) = store.consume() {
            eprintln!("Failed to consume restored update session: {error}");
        }

        for (file_id, file_path) in restored.probe_targets {
            self.queue_restored_source_metadata_probe(file_id, file_path, cx);
        }
        cx.notify();
    }
}
