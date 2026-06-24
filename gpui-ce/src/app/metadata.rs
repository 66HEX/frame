use super::*;

impl FrameRoot {
    pub(super) fn selected_source_metadata_entry(&self) -> SourceMetadataEntry {
        self.source_metadata.selected_entry(&self.file_queue)
    }

    pub(super) fn selected_source_metadata(&self) -> Option<SourceMetadata> {
        self.file_queue
            .selected_file_id()
            .and_then(|id| self.source_metadata.metadata_for(id))
            .cloned()
    }
    pub(super) fn queue_source_metadata_probe(
        &mut self,
        file_id: String,
        file_path: String,
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
                        root.source_metadata.mark_ready(file_id.clone(), metadata);
                        if root.file_queue.selected_file_id() == Some(file_id.as_str()) {
                            let selected_metadata = root.selected_source_metadata();
                            root.normalize_selected_config(selected_metadata.as_ref());
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
