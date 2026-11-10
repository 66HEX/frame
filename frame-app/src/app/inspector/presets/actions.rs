use crate::app::root::FrameRoot;
use crate::settings::{PresetDefinition, apply_preset, create_custom_preset};
use gpui::{Context, PromptButton, PromptLevel, Window};

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
        self.presets.push(create_custom_preset(id, name, &config));
        if self.persist_app_settings().is_err() {
            self.presets.pop();
            return false;
        }

        self.settings_ui.next_custom_preset_sequence = next_sequence;
        self.settings_ui.preset_name_draft.clear();
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
        let metadata = self.selected_source_metadata();
        if !crate::settings::preset_is_compatible(&preset, metadata.as_ref()) {
            return false;
        }
        self.update_selected_config(|config| apply_preset(config, &preset, metadata.as_ref()))
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

        let detail = format!(
            "This will apply \"{}\" to all pending files in the queue. Existing settings will be overwritten.",
            preset.name
        );
        let receiver = window.prompt(
            PromptLevel::Warning,
            "Apply to all?",
            Some(&detail),
            &[PromptButton::ok("Apply"), PromptButton::cancel("Cancel")],
            cx,
        );

        cx.spawn(async move |this, cx| {
            let Ok(answer) = receiver.await else {
                return;
            };
            if answer != 0 {
                return;
            }

            this.update(cx, |root, cx| {
                if root.apply_preset_to_all_pending(&preset) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(in crate::app) fn apply_preset_to_all_pending(
        &mut self,
        preset: &PresetDefinition,
    ) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let mut changed = false;
        for file in self.file_queue.files_mut() {
            if !file.status.is_actionable_for_conversion() {
                continue;
            }
            let metadata = self.source_metadata.metadata_for(&file.id).cloned();
            if !crate::settings::preset_is_compatible(preset, metadata.as_ref()) {
                continue;
            }
            if apply_preset(&mut file.config, preset, metadata.as_ref()) {
                changed = true;
            }
        }

        changed
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
