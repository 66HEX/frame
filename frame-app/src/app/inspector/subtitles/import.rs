use crate::app::root::FrameRoot;
use crate::file_filters::{is_supported_selectable_subtitle_path, is_supported_subtitle_path};
use crate::native_dialogs::{
    external_subtitle_file_dialog, pick_external_subtitle_files, pick_subtitle_file,
    subtitle_file_dialog,
};
use crate::settings::{add_external_subtitle_tracks, apply_subtitle_burn_path};
use gpui::{Context, Window};

impl FrameRoot {
    pub(in crate::app) fn prompt_subtitle_burn_file(&self, window: &Window, cx: &Context<Self>) {
        if self.file_queue.selected_file_locked() {
            return;
        }

        let dialog = subtitle_file_dialog(window);
        cx.spawn(async move |this, cx| {
            let Some(path) = pick_subtitle_file(dialog).await else {
                return;
            };
            if !is_supported_subtitle_path(&path) {
                return;
            }
            let path = path.to_string_lossy().to_string();

            this.update(cx, |root, cx| {
                if root
                    .update_selected_config(|config| apply_subtitle_burn_path(config, Some(path)))
                {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(in crate::app) fn prompt_external_subtitle_files(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) {
        if self.file_queue.selected_file_locked() {
            return;
        }

        let Some(container) = self
            .selected_config()
            .map(|config| config.container.clone())
        else {
            return;
        };
        let dialog = external_subtitle_file_dialog(window, &container);
        cx.spawn(async move |this, cx| {
            let Some(paths) = pick_external_subtitle_files(dialog).await else {
                return;
            };
            let paths = paths
                .into_iter()
                .filter(|path| is_supported_selectable_subtitle_path(path))
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>();

            this.update(cx, |root, cx| {
                let mut selected_index = None;
                let changed = root.update_selected_config(|config| {
                    selected_index = add_external_subtitle_tracks(config, paths);
                    selected_index.is_some()
                });
                if changed {
                    root.subtitle_ui.external_track_index = selected_index;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}
