use super::*;

impl FrameRoot {
    pub(super) fn prompt_add_source(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let paths = cx.background_spawn(async { pick_source_files() }).await;
            let Some(paths) = paths else {
                return;
            };
            if paths.is_empty() {
                return;
            }

            this.update(cx, |root, cx| root.import_source_paths(paths, cx))
                .ok();
        })
        .detach();
    }
    pub(super) fn import_source_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let imports = self.allocate_file_imports(paths);
        if imports.is_empty() {
            return;
        }

        cx.spawn(async move |this, cx| {
            let files = cx
                .background_spawn(async move {
                    imports
                        .into_iter()
                        .map(|(id, path)| FileItem::from_os_path(id, &path))
                        .collect::<Vec<_>>()
                })
                .await;
            let probe_targets = files
                .iter()
                .map(|file| (file.id.clone(), file.path.clone()))
                .collect::<Vec<_>>();

            this.update(cx, |root, cx| {
                if root.file_queue.add_files(files) > 0 {
                    for (file_id, file_path) in probe_targets {
                        root.queue_source_metadata_probe(file_id, file_path, cx);
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
    pub(super) fn allocate_file_imports(&mut self, paths: Vec<PathBuf>) -> Vec<(String, PathBuf)> {
        filter_supported_source_paths(paths)
            .into_iter()
            .map(|path| {
                let id = self.next_file_id();
                (id, path)
            })
            .collect()
    }
    pub(super) fn next_file_id(&mut self) -> String {
        self.next_file_sequence += 1;
        format!("file-{}", self.next_file_sequence)
    }
}
