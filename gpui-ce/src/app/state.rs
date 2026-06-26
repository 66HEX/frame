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
            settings_video_width_focus: None,
            settings_video_height_focus: None,
            settings_video_bitrate_focus: None,
            settings_gif_loop_focus: None,
            settings_metadata_title_focus: None,
            settings_metadata_artist_focus: None,
            settings_metadata_album_focus: None,
            settings_metadata_genre_focus: None,
            settings_metadata_date_focus: None,
            settings_metadata_comment_focus: None,
            settings_preset_name_focus: None,
            settings_subtitle_font_color_focus: None,
            settings_subtitle_outline_color_focus: None,
            active_text_input: None,
            max_concurrency_input: FrameTextInputRuntime::default(),
            output_name_input: FrameTextInputRuntime::default(),
            audio_bitrate_input: FrameTextInputRuntime::default(),
            video_width_input: FrameTextInputRuntime::default(),
            video_height_input: FrameTextInputRuntime::default(),
            video_bitrate_input: FrameTextInputRuntime::default(),
            gif_loop_input: FrameTextInputRuntime::default(),
            metadata_title_input: FrameTextInputRuntime::default(),
            metadata_artist_input: FrameTextInputRuntime::default(),
            metadata_album_input: FrameTextInputRuntime::default(),
            metadata_genre_input: FrameTextInputRuntime::default(),
            metadata_date_input: FrameTextInputRuntime::default(),
            metadata_comment_input: FrameTextInputRuntime::default(),
            preset_name_input: FrameTextInputRuntime::default(),
            subtitle_font_color_input: FrameTextInputRuntime::default(),
            subtitle_outline_color_input: FrameTextInputRuntime::default(),
            text_input_cursor_visible: false,
            text_input_cursor_paused: false,
            text_input_cursor_epoch: 0,
            text_input_cursor_task: Task::ready(()),
            source_metadata: SourceMetadataStore::default(),
            conversion_processes: ConversionProcessController::default(),
            available_encoders: AvailableEncoders::default(),
            subtitle_font_families: frame_core::fonts::list_system_font_families(),
            presets: default_presets(),
            preset_name_draft: String::new(),
            preset_notice: None,
            next_custom_preset_sequence: 0,
            settings_subtitle_popover: None,
            subtitle_font_color_draft: DEFAULT_SUBTITLE_FONT_COLOR.to_uppercase(),
            subtitle_outline_color_draft: DEFAULT_SUBTITLE_OUTLINE_COLOR.to_uppercase(),
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
