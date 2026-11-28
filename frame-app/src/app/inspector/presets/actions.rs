use crate::app::root::FrameRoot;
use crate::settings::{apply_preset, create_custom_preset};
use gpui::{Context, Window};

impl FrameRoot {
    pub(in crate::app) fn save_preset_from_draft(&mut self) -> bool {
        if self.update_installation_in_progress() || self.file_queue.selected_file_locked() {
            return false;
        }
        let name = self.settings_ui.preset_name_draft.trim();
        if name.is_empty() {
            return false;
        }

        let Some(config) = self.selected_config().cloned() else {
            return false;
        };

        let (id, next_sequence) = self.next_custom_preset_identity();
        let mut preset = create_custom_preset(id, name, &config);
        preset.enable_source_audio = crate::app::inspector::state::preset_draft_enables_audio(
            self.settings_ui.preset_audio_draft,
            &config,
        );
        self.presets.push(preset);
        if self.persist_app_settings().is_err() {
            self.presets.pop();
            return false;
        }

        self.settings_ui.next_custom_preset_sequence = next_sequence;
        self.settings_ui.preset_name_draft.clear();
        self.settings_ui.preset_audio_draft = None;
        true
    }

    pub(in crate::app) fn delete_preset(&mut self, preset_id: &str) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let Some(index) = self
            .presets
            .iter()
            .position(|preset| preset.id == preset_id && !preset.built_in)
        else {
            return false;
        };

        let removed = self.presets.remove(index);
        if self.persist_app_settings().is_err() {
            self.presets.insert(index, removed);
            return false;
        }

        self.prune_applied_presets();

        true
    }

    pub(in crate::app) fn apply_preset_to_selected(&mut self, preset_id: &str) -> bool {
        if self.update_installation_in_progress() || self.file_queue.selected_file_locked() {
            return false;
        }
        let Some(preset) = self
            .presets
            .iter()
            .find(|preset| preset.id == preset_id)
            .cloned()
        else {
            return false;
        };
        let Some(file_id) = self.file_queue.selected_file_id().map(str::to_string) else {
            return false;
        };
        let Some(metadata) = self.source_metadata.ready_metadata_for(&file_id).cloned() else {
            return false;
        };
        if !crate::settings::preset_is_compatible(&preset, Some(&metadata))
            || self.selected_config().is_none_or(|target| {
                crate::settings::preset_audio_unavailable(&preset, target, Some(&metadata))
                    .is_some()
            })
        {
            return false;
        }
        let changed =
            self.update_selected_config(|config| apply_preset(config, &preset, Some(&metadata)));
        self.remember_applied_preset(&file_id, preset_id) | changed
    }

    pub(in crate::app) fn selected_applied_preset_id(&self) -> Option<&str> {
        let file = self.file_queue.selected_file()?;
        self.settings_ui
            .applied_presets
            .get(&file.id)
            .filter(|applied| applied.path == file.path)
            .map(|applied| applied.preset_id.as_str())
    }

    pub(in crate::app) fn prune_applied_presets(&mut self) {
        if self.settings_ui.applied_presets.is_empty() {
            return;
        }
        let files: std::collections::HashSet<_> = self
            .file_queue
            .files()
            .iter()
            .map(|file| (file.id.as_str(), file.path.as_str()))
            .collect();
        self.settings_ui.applied_presets.retain(|id, applied| {
            files.contains(&(id.as_str(), applied.path.as_str()))
                && self
                    .presets
                    .iter()
                    .any(|preset| preset.id == applied.preset_id)
        });
    }

    pub(in crate::app) fn remember_applied_preset(
        &mut self,
        file_id: &str,
        preset_id: &str,
    ) -> bool {
        self.prune_applied_presets();
        let Some(file) = self.file_queue.file_by_id(file_id) else {
            return false;
        };
        let changed = self
            .settings_ui
            .applied_presets
            .get(file_id)
            .is_none_or(|applied| applied.path != file.path || applied.preset_id != preset_id);
        if changed {
            self.settings_ui.applied_presets.insert(
                file.id.clone(),
                crate::app::inspector::state::AppliedPreset {
                    path: file.path.clone(),
                    preset_id: preset_id.to_string(),
                },
            );
        }
        changed
    }

    pub(in crate::app) fn confirm_apply_preset_to_all(
        &self,
        preset_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.file_queue.selected_file_locked() {
            return;
        }
        let Some(preset) = self
            .presets
            .iter()
            .find(|preset| preset.id == preset_id)
            .cloned()
        else {
            return;
        };

        self.confirm_preset_batch(&preset, window, cx);
    }

    fn next_custom_preset_identity(&self) -> (String, u64) {
        let mut sequence = self.settings_ui.next_custom_preset_sequence;

        loop {
            sequence += 1;
            let id = format!("custom-preset-{sequence}");
            if !self.presets.iter().any(|preset| preset.id == id) {
                return (id, sequence);
            }
        }
    }
}
