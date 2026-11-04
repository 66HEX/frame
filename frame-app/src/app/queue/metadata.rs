use crate::app::root::FrameRoot;
use crate::settings::{SourceMetadata, initialize_output_config};
use crate::source_metadata::{SourceMetadataEntry, probe_source_metadata};
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
                match result {
                    Ok(metadata) => {
                        if normalize_selected_config
                            && !root.update_installation_in_progress()
                            && let Some(file) = root.file_queue.file_by_id_mut(&file_id)
                        {
                            initialize_output_config(&mut file.config, Some(&metadata));
                        }
                        root.source_metadata.mark_ready(file_id.clone(), metadata);
                        if root.file_queue.selected_file_id() == Some(file_id.as_str()) {
                            let selected_metadata = root.selected_source_metadata();
                            root.resolve_selected_settings_tab(selected_metadata.as_ref());
                        }
                    }
                    Err(error) => {
                        root.source_metadata
                            .mark_error(file_id.clone(), error.to_string());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
