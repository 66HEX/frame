use super::{
    AppearanceSettings, Context, DecreaseUiScale, FrameRoot, FrameTextInputKind, IncreaseUiScale,
    PresetDefinition, PresetNotice, PresetNoticeTone, PromptButton, PromptLevel, ResetUiScale,
    ScalePreset, UiScalePopoverState, Window, apply_preset, apply_subtitle_burn_path,
    create_custom_preset, is_supported_subtitle_path, output_folder_dialog, pick_output_folder,
    pick_subtitle_file, subtitle_file_dialog,
};

impl FrameRoot {
    pub(super) fn open_app_settings(&mut self) {
        self.settings_ui.is_open = true;
        self.settings_ui.is_present = true;
        self.settings_ui.max_concurrency_draft = self.max_concurrency.to_string();
        self.settings_ui.max_concurrency_error = None;
        self.settings_ui.output_directory_error = None;
        self.settings_ui.appearance_error = None;
        self.settings_ui.ui_scale_popover = UiScalePopoverState::Hidden;
    }

    pub(super) fn close_app_settings(&mut self) {
        self.settings_ui.is_open = false;
        self.settings_ui.max_concurrency_error = None;
        self.settings_ui.output_directory_error = None;
        self.settings_ui.appearance_error = None;
        self.close_app_settings_ui_scale_popover();
        self.text_input_ui
            .focuses
            .clear(FrameTextInputKind::MaxConcurrency);
        if self.text_input_ui.active == Some(FrameTextInputKind::MaxConcurrency) {
            self.stop_text_input_cursor();
        }
    }

    pub(super) const fn toggle_app_settings_ui_scale_popover(&mut self) {
        self.settings_ui.ui_scale_popover = match self.settings_ui.ui_scale_popover {
            UiScalePopoverState::Open => UiScalePopoverState::Closing,
            UiScalePopoverState::Hidden | UiScalePopoverState::Closing => UiScalePopoverState::Open,
        };
    }

    pub(super) const fn close_app_settings_ui_scale_popover(&mut self) {
        if matches!(self.settings_ui.ui_scale_popover, UiScalePopoverState::Open) {
            self.settings_ui.ui_scale_popover = UiScalePopoverState::Closing;
        }
    }

    pub(super) const fn finish_app_settings_ui_scale_popover_close(&mut self) -> bool {
        if !matches!(
            self.settings_ui.ui_scale_popover,
            UiScalePopoverState::Closing
        ) {
            return false;
        }
        self.settings_ui.ui_scale_popover = UiScalePopoverState::Hidden;
        true
    }

    pub(super) const fn finish_app_settings_close(&mut self) -> bool {
        if self.settings_ui.is_open || !self.settings_ui.is_present {
            return false;
        }
        self.settings_ui.is_present = false;
        true
    }

    pub(super) fn apply_max_concurrency_draft(&mut self) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let Some(value) = self.parsed_max_concurrency_draft() else {
            self.settings_ui.max_concurrency_error =
                Some("Enter a whole number greater than zero.".to_string());
            return false;
        };

        match self.conversion_processes.update_max_concurrency(value) {
            Ok(()) => {
                self.max_concurrency = value;
                self.settings_ui.max_concurrency_draft = value.to_string();
                self.settings_ui.max_concurrency_error = None;
                if let Err(error) = self.persist_app_settings() {
                    self.settings_ui.max_concurrency_error =
                        Some(format!("Failed to save settings: {error}"));
                }
                true
            }
            Err(error) => {
                self.settings_ui.max_concurrency_error = Some(error.to_string());
                false
            }
        }
    }
    pub(super) fn parsed_max_concurrency_draft(&self) -> Option<usize> {
        let trimmed = self.settings_ui.max_concurrency_draft.trim();
        let value = trimmed.parse::<usize>().ok()?;
        (value > 0).then_some(value)
    }

    pub(super) fn set_ui_scale(&mut self, scale: ScalePreset) -> bool {
        let mut appearance = self.appearance;
        appearance.ui_scale = scale;
        self.set_appearance(appearance)
    }

    pub(super) fn increase_ui_scale(
        &mut self,
        _: &IncreaseUiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_ui_scale_shortcut(self.appearance.ui_scale.next(), window, cx);
    }

    pub(super) fn decrease_ui_scale(
        &mut self,
        _: &DecreaseUiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_ui_scale_shortcut(self.appearance.ui_scale.previous(), window, cx);
    }

    pub(super) fn reset_ui_scale(
        &mut self,
        _: &ResetUiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_ui_scale_shortcut(ScalePreset::Percent100, window, cx);
    }

    fn apply_ui_scale_shortcut(
        &mut self,
        scale: ScalePreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.set_ui_scale(scale) {
            window.set_rem_size(gpui::px(crate::appearance::BASE_REM_PX * scale.factor()));
            cx.notify();
        }
    }

    fn set_appearance(&mut self, appearance: AppearanceSettings) -> bool {
        if self.update_installation_in_progress() || self.appearance == appearance {
            return false;
        }

        let previous = self.appearance;
        self.appearance = appearance;
        if let Err(error) = self.persist_app_settings() {
            self.appearance = previous;
            self.settings_ui.appearance_error = Some(format!("Failed to save settings: {error}"));
            return false;
        }

        self.settings_ui.appearance_error = None;
        true
    }

    pub(super) fn prompt_default_output_folder(window: &Window, cx: &Context<Self>) {
        let dialog = output_folder_dialog(window);
        cx.spawn(async move |this, cx| {
            let Some(path) = pick_output_folder(dialog).await else {
                return;
            };

            this.update(cx, |root, cx| {
                root.settings_ui.output_directory_error = root
                    .set_default_output_directory(path)
                    .err()
                    .map(|error| format!("Failed to save settings: {error}"));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn set_default_output_directory(
        &mut self,
        path: std::path::PathBuf,
    ) -> Result<(), crate::app_persistence::AppPersistenceError> {
        if self.update_installation_in_progress() {
            return Err(crate::app_persistence::AppPersistenceError::InstallationInProgress);
        }
        let previous = self.default_output_directory.replace(path);
        if let Err(error) = self.persist_app_settings() {
            self.default_output_directory = previous;
            return Err(error);
        }

        Ok(())
    }

    pub(super) fn prompt_subtitle_burn_file(&self, window: &Window, cx: &Context<Self>) {
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

    pub(super) fn save_preset_from_draft(&mut self) -> bool {
        if self.update_installation_in_progress() || self.file_queue.selected_file_locked() {
            return false;
        }
        let name = self.settings_ui.preset_name_draft.trim();
        if name.is_empty() {
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: "Name required".to_string(),
                tone: PresetNoticeTone::Error,
            });
            return false;
        }

        let Some(config) = self.selected_config().cloned() else {
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: "Preset not saved".to_string(),
                tone: PresetNoticeTone::Error,
            });
            return false;
        };

        let (id, next_sequence) = self.next_custom_preset_identity();
        self.presets.push(create_custom_preset(id, name, &config));
        if let Err(error) = self.persist_app_settings() {
            self.presets.pop();
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: format!("Preset not saved: {error}"),
                tone: PresetNoticeTone::Error,
            });
            return false;
        }

        self.settings_ui.next_custom_preset_sequence = next_sequence;
        self.settings_ui.preset_name_draft.clear();
        self.settings_ui.preset_notice = Some(PresetNotice {
            text: "Preset saved".to_string(),
            tone: PresetNoticeTone::Success,
        });
        true
    }

    pub(super) fn delete_preset(&mut self, preset_id: &str) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let Some(index) = self
            .presets
            .iter()
            .position(|preset| preset.id == preset_id && !preset.built_in)
        else {
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: "Unable to delete".to_string(),
                tone: PresetNoticeTone::Error,
            });
            return false;
        };

        let removed = self.presets.remove(index);
        if let Err(error) = self.persist_app_settings() {
            self.presets.insert(index, removed);
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: format!("Unable to delete: {error}"),
                tone: PresetNoticeTone::Error,
            });
            return false;
        }

        self.settings_ui.preset_notice = Some(PresetNotice {
            text: "Preset removed".to_string(),
            tone: PresetNoticeTone::Success,
        });
        true
    }

    pub(super) fn apply_preset_to_selected(&mut self, preset_id: &str) -> bool {
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
        let changed =
            self.update_selected_config(|config| apply_preset(config, &preset, metadata.as_ref()));
        if changed {
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: format!("Applied {}", preset.name),
                tone: PresetNoticeTone::Success,
            });
        }
        changed
    }

    pub(super) fn confirm_apply_preset_to_all(
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

    pub(super) fn apply_preset_to_all_pending(&mut self, preset: &PresetDefinition) -> bool {
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

        if changed {
            self.settings_ui.preset_notice = Some(PresetNotice {
                text: "Applied to all items".to_string(),
                tone: PresetNoticeTone::Success,
            });
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
