use super::*;

pub(super) struct FileDropLifecycleProbe {
    pub(super) owner: Entity<FrameRoot>,
}

impl IntoElement for FileDropLifecycleProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FileDropLifecycleProbe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            position: Position::Absolute,
            size: size(px(0.0).into(), px(0.0).into()),
            ..Style::default()
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let owner = self.owner.clone();
        window.on_mouse_event(move |event: &FileDropEvent, phase, _window, cx| {
            if phase != DispatchPhase::Capture || !matches!(event, FileDropEvent::Exited) {
                return;
            }

            owner.update(cx, |root, cx| {
                if root.close_drag_drop_overlay() {
                    cx.notify();
                }
            });
        });
    }
}

impl FrameRoot {
    pub(super) const fn open_drag_drop_overlay(&mut self) -> bool {
        let changed = !self.drag_drop_ui.is_open || !self.drag_drop_ui.is_present;
        self.drag_drop_ui.is_open = true;
        self.drag_drop_ui.is_present = true;
        changed
    }

    pub(super) const fn close_drag_drop_overlay(&mut self) -> bool {
        let changed = self.drag_drop_ui.is_open;
        self.drag_drop_ui.is_open = false;
        changed
    }

    pub(super) const fn finish_drag_drop_overlay_close(&mut self) -> bool {
        if self.drag_drop_ui.is_open || !self.drag_drop_ui.is_present {
            return false;
        }

        self.drag_drop_ui.is_present = false;
        true
    }

    pub(super) fn prompt_add_source(window: &Window, cx: &Context<Self>) {
        let dialog = source_file_dialog(window);
        cx.spawn(async move |this, cx| {
            let paths = pick_source_files(dialog).await;
            let Some(paths) = paths else {
                return;
            };
            if paths.is_empty() {
                return;
            }

            this.update(cx, |_root, cx| Self::import_source_paths(paths, cx))
                .ok();
        })
        .detach();
    }

    pub(super) fn prompt_add_source_folder(window: &Window, cx: &Context<Self>) {
        let dialog = source_folder_dialog(window);
        cx.spawn(async move |this, cx| {
            let Some(path) = pick_source_folder(dialog).await else {
                return;
            };

            this.update(cx, |_root, cx| {
                Self::import_source_paths(vec![path], cx);
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn import_source_paths(paths: Vec<PathBuf>, cx: &Context<Self>) {
        if paths.is_empty() {
            return;
        }

        cx.spawn(async move |this, cx| {
            let paths = cx
                .background_spawn(async move { discover_supported_source_paths(paths) })
                .await;
            let Ok(imports) = this.update(cx, |root, _cx| root.allocate_file_imports(paths)) else {
                return;
            };
            if imports.is_empty() {
                return;
            }

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
