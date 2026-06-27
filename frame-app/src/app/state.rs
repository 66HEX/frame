use super::*;

impl FrameRoot {
    pub fn new() -> Self {
        Self::new_inner(None, AppSettings::default(), AppNotifier::disabled())
    }

    pub fn new_with_platform_persistence() -> Self {
        let notifier = AppNotifier::system();
        match AppPersistence::platform() {
            Ok(persistence) => Self::new_with_persistence_and_notifier(persistence, notifier),
            Err(_) => Self::new_inner(None, AppSettings::default(), notifier),
        }
    }

    pub fn load_runtime_capabilities(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let detected = cx
                .background_spawn(async { detect_available_encoders() })
                .await;

            this.update(cx, |root, cx| {
                match detected {
                    Ok(encoders) => root.available_encoders = encoders,
                    Err(error) => eprintln!("Failed to detect FFmpeg capabilities: {error}"),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    #[cfg(test)]
    pub(crate) fn new_with_notifier(notifier: AppNotifier) -> Self {
        Self::new_inner(None, AppSettings::default(), notifier)
    }

    #[cfg(test)]
    pub(crate) fn new_with_persistence(persistence: AppPersistence) -> Self {
        Self::new_with_persistence_and_notifier(persistence, AppNotifier::disabled())
    }

    fn new_with_persistence_and_notifier(
        persistence: AppPersistence,
        notifier: AppNotifier,
    ) -> Self {
        let settings = persistence.load().unwrap_or_default();
        Self::new_inner(Some(persistence), settings, notifier)
    }

    fn new_inner(
        persistence: Option<AppPersistence>,
        persisted_settings: AppSettings,
        notifier: AppNotifier,
    ) -> Self {
        let conversion_processes = ConversionProcessController::default();
        let max_concurrency = if conversion_processes
            .update_max_concurrency(persisted_settings.max_concurrency)
            .is_ok()
        {
            persisted_settings.max_concurrency
        } else {
            DEFAULT_MAX_CONCURRENCY
        };
        let presets = merged_presets(persisted_settings.custom_presets);
        let settings_ui = SettingsUiState {
            max_concurrency_draft: max_concurrency.to_string(),
            next_custom_preset_sequence: next_custom_preset_sequence(&presets),
            ..SettingsUiState::default()
        };

        let mut root = Self {
            active_view: active_view_from_env_value(
                std::env::var("FRAME_GPUI_INITIAL_VIEW").ok().as_deref(),
            ),
            file_queue: FileQueue::new(),
            conversion_events: ConversionEventState::new(),
            logs_scroll_handle: UniformListScrollHandle::new(),
            last_log_scroll_target: None,
            logs_follow_tail: true,
            is_processing: false,
            settings_ui,
            max_concurrency,
            text_input_ui: FrameTextInputUiState::default(),
            source_metadata: SourceMetadataStore::default(),
            conversion_processes,
            available_encoders: AvailableEncoders::default(),
            active_conversion_task_ids: Vec::new(),
            notifier,
            subtitle_font_families: frame_core::fonts::list_system_font_families(),
            presets,
            subtitle_ui: SubtitleUiState::default(),
            preview_ui: PreviewUiState::default(),
            native_titlebar_controls_hidden: false,
            next_file_sequence: 0,
            persistence,
        };

        root.apply_visual_fixture(visual_fixture_from_env_value(
            std::env::var("FRAME_GPUI_VISUAL_FIXTURE").ok().as_deref(),
        ));
        root
    }
    pub(super) fn app_state(&self) -> FrameAppState {
        FrameAppState::from_file_queue(self.active_view, self.is_processing, &self.file_queue)
    }
    pub(super) fn selected_config(&self) -> Option<&ConversionConfig> {
        self.file_queue.selected_file().map(|file| &file.config)
    }
    pub(super) fn update_selected_config(
        &mut self,
        update: impl FnOnce(&mut ConversionConfig) -> bool,
    ) -> bool {
        self.file_queue
            .selected_file_mut()
            .is_some_and(|file| update(&mut file.config))
    }
    pub(super) fn normalize_selected_config(&mut self, metadata: Option<&SourceMetadata>) -> bool {
        self.update_selected_config(|config| normalize_output_config(config, metadata))
    }

    pub(super) fn persist_app_settings(
        &self,
    ) -> Result<(), crate::app_persistence::AppPersistenceError> {
        let Some(persistence) = &self.persistence else {
            return Ok(());
        };

        persistence.save(&AppSettings::from_runtime(
            self.max_concurrency,
            &self.presets,
        ))
    }
}

impl Default for FrameRoot {
    fn default() -> Self {
        Self::new()
    }
}

fn merged_presets(custom_presets: Vec<PresetDefinition>) -> Vec<PresetDefinition> {
    let mut presets = default_presets();

    for preset in custom_presets {
        if !presets.iter().any(|existing| existing.id == preset.id) {
            presets.push(preset);
        }
    }

    presets
}

fn next_custom_preset_sequence(presets: &[PresetDefinition]) -> u64 {
    presets
        .iter()
        .filter(|preset| !preset.built_in)
        .filter_map(|preset| {
            preset
                .id
                .strip_prefix("custom-preset-")
                .and_then(|suffix| suffix.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0)
}
