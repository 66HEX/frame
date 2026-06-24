use super::*;

impl FrameRoot {
    pub fn new() -> Self {
        let mut root = Self {
            active_view: active_view_from_env_value(
                std::env::var("FRAME_GPUI_INITIAL_VIEW").ok().as_deref(),
            ),
            file_queue: FileQueue::new(),
            conversion_events: ConversionEventState::new(),
            logs_scroll_handle: UniformListScrollHandle::new(),
            last_log_scroll_target: None,
            is_processing: false,
            is_settings_open: false,
            settings_active_tab: SettingsTab::Source,
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            max_concurrency_draft: DEFAULT_MAX_CONCURRENCY.to_string(),
            max_concurrency_error: None,
            app_settings_value_focus: None,
            settings_output_name_focus: None,
            settings_audio_bitrate_focus: None,
            active_text_input: None,
            max_concurrency_input: FrameTextInputRuntime::default(),
            output_name_input: FrameTextInputRuntime::default(),
            audio_bitrate_input: FrameTextInputRuntime::default(),
            text_input_cursor_visible: false,
            text_input_cursor_paused: false,
            text_input_cursor_epoch: 0,
            text_input_cursor_task: Task::ready(()),
            source_metadata: SourceMetadataStore::default(),
            conversion_processes: ConversionProcessController::default(),
            preview_crop_file_id: None,
            preview_crop_mode: false,
            preview_draft_crop: None,
            preview_crop_aspect: "free".to_string(),
            preview_crop_drag: None,
            native_titlebar_controls_hidden: false,
            next_file_sequence: 0,
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
}

impl Default for FrameRoot {
    fn default() -> Self {
        Self::new()
    }
}
