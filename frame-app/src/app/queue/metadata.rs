use crate::app::root::FrameRoot;
use crate::settings::{SourceMetadata, initialize_output_config};
use crate::source_metadata::{SourceMetadataEntry, probe_source_metadata};
use frame_core::error::ConversionError;
use gpui::Context;
use gpui::prelude::*;

impl FrameRoot {
    pub(in crate::app) fn selected_source_metadata_entry(&self) -> SourceMetadataEntry {
        self.source_metadata.selected_entry(&self.file_queue)
    }

    pub(in crate::app) fn selected_source_metadata(&self) -> Option<SourceMetadata> {
        self.file_queue
            .selected_file_id()
            .and_then(|id| self.source_metadata.metadata_for(id))
            .cloned()
    }
    pub(in crate::app) fn queue_source_metadata_probe(
        &mut self,
        file_id: String,
        file_path: String,
        cx: &mut Context<Self>,
    ) {
        self.queue_source_metadata_probe_inner(file_id, file_path, true, cx);
    }

    pub(in crate::app) fn queue_restored_source_metadata_probe(
        &mut self,
        file_id: String,
        file_path: String,
        cx: &mut Context<Self>,
    ) {
        self.queue_source_metadata_probe_inner(file_id, file_path, false, cx);
    }

    fn queue_source_metadata_probe_inner(
        &mut self,
        file_id: String,
        file_path: String,
        normalize_selected_config: bool,
        cx: &mut Context<Self>,
    ) {
        self.source_metadata.mark_loading(file_id.clone());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { probe_source_metadata(&file_path) })
                .await;

            this.update(cx, |root, cx| {
                root.complete_source_metadata_probe(
                    &file_id,
                    result,
                    normalize_selected_config,
                    || cx.notify(),
                );
            })
            .ok();
        })
        .detach();
    }

    fn complete_source_metadata_probe(
        &mut self,
        file_id: &str,
        result: Result<SourceMetadata, ConversionError>,
        normalize_selected_config: bool,
        notify: impl FnOnce(),
    ) {
        if self.file_queue.file_by_id(file_id).is_none() {
            return;
        }
        match result {
            Ok(metadata) => {
                if normalize_selected_config
                    && !self.update_installation_in_progress()
                    && let Some(file) = self.file_queue.file_by_id_mut(file_id)
                {
                    initialize_output_config(&mut file.config, Some(&metadata));
                }
                self.source_metadata.mark_ready(file_id, metadata);
                if self.file_queue.selected_file_id() == Some(file_id) {
                    let selected_metadata = self.selected_source_metadata();
                    self.sync_inspector_availability(selected_metadata.as_ref());
                }
            }
            Err(error) => {
                self.source_metadata.mark_error(file_id, error.to_string());
            }
        }
        notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::updates::state::UpdateStatus;
    use crate::file_queue::FileItem;
    use crate::settings::SourceKind;
    use crate::source_metadata::MetadataStatus;

    fn metadata() -> SourceMetadata {
        SourceMetadata {
            media_kind: Some(SourceKind::Audio),
            ..SourceMetadata::default()
        }
    }

    fn root_with_file(id: &str) -> FrameRoot {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path(id, "/tmp/reimport.wav", 1));
        root.source_metadata.mark_loading(id);
        root
    }

    #[test]
    fn removed_probe_success_and_error_do_not_restore_metadata_or_notify() {
        for result in [
            Ok(metadata()),
            Err(ConversionError::Probe("late failure".into())),
        ] {
            let mut root = root_with_file("old");
            assert!(root.remove_file_from_queue("old"));
            let before = root.source_metadata.clone();
            let mut notifications = 0;
            root.complete_source_metadata_probe("old", result, true, || notifications += 1);
            assert_eq!(root.source_metadata, before);
            assert!(root.file_queue.files().is_empty());
            assert_eq!(notifications, 0);
        }
    }

    #[test]
    fn old_probe_completion_cannot_mutate_a_reimport_with_a_new_id() {
        let mut root = root_with_file("old");
        assert!(root.remove_file_from_queue("old"));
        root.file_queue
            .add_file(FileItem::from_path("new", "/tmp/reimport.wav", 1));
        root.source_metadata.mark_loading("new");
        let before = root.file_queue.file_by_id("new").unwrap().config.clone();
        let mut notifications = 0;
        for result in [
            Ok(metadata()),
            Err(ConversionError::Probe("late failure".into())),
        ] {
            root.complete_source_metadata_probe("old", result, true, || notifications += 1);
        }
        assert_eq!(
            root.source_metadata.entry_for("old").status,
            MetadataStatus::Idle
        );
        assert_eq!(
            root.source_metadata.entry_for("new").status,
            MetadataStatus::Loading
        );
        assert_eq!(root.file_queue.file_by_id("new").unwrap().config, before);
        assert_eq!(notifications, 0);
        root.complete_source_metadata_probe("new", Ok(metadata()), true, || notifications += 1);
        assert_eq!(
            root.source_metadata.entry_for("new").status,
            MetadataStatus::Ready
        );
        assert_eq!(notifications, 1);
    }

    #[test]
    fn selected_and_unselected_completions_normalize_only_their_file_and_sync_selected_inspector() {
        for id in ["selected", "unselected"] {
            let mut root = root_with_file("selected");
            root.file_queue
                .add_file(FileItem::from_path("unselected", "/tmp/other.wav", 1));
            root.subtitle_ui.external_track_index = Some(9);
            let mut notifications = 0;
            root.complete_source_metadata_probe(id, Ok(metadata()), true, || notifications += 1);
            assert_eq!(
                root.file_queue.file_by_id(id).unwrap().config.container,
                "mp3"
            );
            let untouched = if id == "selected" {
                "unselected"
            } else {
                "selected"
            };
            assert_eq!(
                root.file_queue
                    .file_by_id(untouched)
                    .unwrap()
                    .config
                    .container,
                "mp4"
            );
            assert_eq!(
                root.subtitle_ui.external_track_index,
                if id == "selected" { None } else { Some(9) }
            );
            assert_eq!(
                root.source_metadata.entry_for(id).status,
                MetadataStatus::Ready
            );
            assert_eq!(notifications, 1);
        }
    }

    #[test]
    fn restored_and_installing_completions_keep_config_but_publish_metadata_once() {
        for (normalize, installing) in [(false, false), (true, true)] {
            let mut root = root_with_file("existing");
            let before = root
                .file_queue
                .file_by_id("existing")
                .unwrap()
                .config
                .clone();
            if installing {
                root.update_ui.status = UpdateStatus::Installing;
            }
            let mut notifications = 0;
            root.complete_source_metadata_probe("existing", Ok(metadata()), normalize, || {
                notifications += 1;
            });
            assert_eq!(
                root.file_queue.file_by_id("existing").unwrap().config,
                before
            );
            assert_eq!(
                root.source_metadata.entry_for("existing").status,
                MetadataStatus::Ready
            );
            assert_eq!(notifications, 1);
        }
    }

    #[test]
    fn existing_probe_error_still_publishes_error_and_notifies_once() {
        let mut root = root_with_file("existing");
        let mut notifications = 0;
        root.complete_source_metadata_probe(
            "existing",
            Err(ConversionError::Probe("failure".into())),
            false,
            || notifications += 1,
        );
        let entry = root.source_metadata.entry_for("existing");
        assert_eq!(entry.status, MetadataStatus::Error);
        assert!(entry.error.unwrap().contains("failure"));
        assert_eq!(notifications, 1);
    }
}
