use crate::app::accessibility::FrameFocusRegistry;
use crate::app::chrome::state::TitlebarDragState;
use crate::app::input::FrameTextInputUiState;
use crate::app::inspector::state::SettingsUiState;
use crate::app::inspector::subtitles::state::SubtitleUiState;
use crate::app::logs::state::LogScrollTarget;
use crate::app::model::{
    ActiveView, FrameAppState, active_view_from_env_value, visual_fixture_from_env_value,
};
use crate::app::preview::state::PreviewUiState;
use crate::app::queue::state::DragDropUiState;
use crate::app::updates::state::UpdateUiState;
use crate::app_persistence::{AppPersistence, AppSettings};
use crate::appearance::AppearanceSettings;
use crate::capabilities::{detect_available_encoders, detect_available_filters};
use crate::conversion_events::ConversionEventState;
use crate::conversion_runner::ConversionProcessController;
use crate::file_queue::FileQueue;
use crate::notifications::AppNotifier;
use crate::settings::{
    ConversionConfig, PresetDefinition, SourceMetadata, default_presets, normalize_output_config,
};
use crate::source_metadata::SourceMetadataStore;
use frame_core::capabilities::{AvailableEncoders, AvailableFilters};
use frame_core::types::DEFAULT_MAX_CONCURRENCY;
use frame_updater::UpdateChannel;
use gpui::prelude::*;
use gpui::{Context, UniformListScrollHandle};

pub struct FrameRoot {
    pub(in crate::app) active_view: ActiveView,
    pub(in crate::app) appearance: AppearanceSettings,
    pub(in crate::app) titlebar_drag: TitlebarDragState,
    pub(in crate::app) focus_registry: FrameFocusRegistry,
    pub(in crate::app) file_queue: FileQueue,
    pub(in crate::app) file_list_scroll_handle: UniformListScrollHandle,
    pub(in crate::app) conversion_events: ConversionEventState,
    pub(in crate::app) logs_scroll_handle: UniformListScrollHandle,
    pub(in crate::app) last_log_scroll_target: Option<LogScrollTarget>,
    pub(in crate::app) logs_keyboard_scroll_top: usize,
    pub(in crate::app) logs_follow_tail: bool,
    pub(in crate::app) copied_log_file_id: Option<String>,
    pub(in crate::app) log_copy_feedback_epoch: usize,
    pub(in crate::app) is_processing: bool,
    pub(in crate::app) settings_ui: SettingsUiState,
    pub(in crate::app) drag_drop_ui: DragDropUiState,
    pub(in crate::app) max_concurrency: usize,
    pub(in crate::app) default_output_directory: Option<std::path::PathBuf>,
    pub(in crate::app) text_input_ui: FrameTextInputUiState,
    pub(in crate::app) source_metadata: SourceMetadataStore,
    pub(in crate::app) conversion_processes: ConversionProcessController,
    pub(in crate::app) available_encoders: AvailableEncoders,
    pub(in crate::app) available_filters: AvailableFilters,
    pub(in crate::app) active_conversion_task_ids: Vec<String>,
    pub(in crate::app) notifier: AppNotifier,
    pub(in crate::app) subtitle_font_families: Vec<String>,
    pub(in crate::app) presets: Vec<PresetDefinition>,
    pub(in crate::app) subtitle_ui: SubtitleUiState,
    pub(in crate::app) preview_ui: PreviewUiState,
    pub(in crate::app) next_file_sequence: u64,
    pub(in crate::app) persistence: Option<AppPersistence>,
    pub(in crate::app) auto_update_check: bool,
    pub(in crate::app) update_channel: UpdateChannel,
    pub(in crate::app) skipped_update_version: Option<String>,
    pub(in crate::app) last_update_check_at: Option<u64>,
    pub(in crate::app) update_ui: UpdateUiState,
}

impl FrameRoot {
    #[must_use]
    pub fn new() -> Self {
        Self::new_inner(None, AppSettings::default(), AppNotifier::disabled())
    }

    #[must_use]
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
                .background_spawn(async {
                    (detect_available_encoders(), detect_available_filters())
                })
                .await;

            this.update(cx, |root, cx| {
                match detected.0 {
                    Ok(encoders) => root.available_encoders = encoders,
                    Err(error) => {
                        eprintln!("Failed to detect FFmpeg encoder capabilities: {error}");
                    }
                }
                match detected.1 {
                    Ok(filters) => root.available_filters = filters,
                    Err(error) => eprintln!("Failed to detect FFmpeg filter capabilities: {error}"),
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
        mut persisted_settings: AppSettings,
        notifier: AppNotifier,
    ) -> Self {
        if let Some(color_theme) = crate::appearance::color_theme_from_env_value(
            std::env::var("FRAME_VISUAL_THEME").ok().as_deref(),
        ) {
            persisted_settings.appearance.color_theme = color_theme;
        }
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
            appearance: persisted_settings.appearance,
            titlebar_drag: TitlebarDragState::default(),
            focus_registry: FrameFocusRegistry::default(),
            file_queue: FileQueue::new(),
            file_list_scroll_handle: UniformListScrollHandle::new(),
            conversion_events: ConversionEventState::new(),
            logs_scroll_handle: UniformListScrollHandle::new(),
            last_log_scroll_target: None,
            logs_keyboard_scroll_top: 0,
            logs_follow_tail: true,
            copied_log_file_id: None,
            log_copy_feedback_epoch: 0,
            is_processing: false,
            settings_ui,
            drag_drop_ui: DragDropUiState::default(),
            max_concurrency,
            default_output_directory: persisted_settings.default_output_directory,
            text_input_ui: FrameTextInputUiState::default(),
            source_metadata: SourceMetadataStore::default(),
            conversion_processes,
            available_encoders: AvailableEncoders::default(),
            available_filters: AvailableFilters::default(),
            active_conversion_task_ids: Vec::new(),
            notifier,
            subtitle_font_families: frame_core::fonts::list_system_font_families(),
            presets,
            subtitle_ui: SubtitleUiState::default(),
            preview_ui: PreviewUiState::default(),
            next_file_sequence: 0,
            persistence,
            auto_update_check: persisted_settings.auto_update_check,
            update_channel: persisted_settings.update_channel,
            skipped_update_version: persisted_settings.skipped_update_version,
            last_update_check_at: persisted_settings.last_update_check_at,
            update_ui: UpdateUiState::default(),
        };

        root.apply_visual_fixture(visual_fixture_from_env_value(
            std::env::var("FRAME_GPUI_VISUAL_FIXTURE").ok().as_deref(),
        ));
        if let Some(ui_scale) = crate::appearance::scale_preset_from_env_value(
            std::env::var("FRAME_GPUI_UI_SCALE").ok().as_deref(),
        ) {
            root.appearance.ui_scale = ui_scale;
        }
        root
    }
    pub(super) fn app_state(&self) -> FrameAppState {
        FrameAppState::from_file_queue(
            self.active_view,
            self.is_processing,
            self.default_output_directory.is_some(),
            &self.file_queue,
        )
    }
    pub(super) fn selected_config(&self) -> Option<&ConversionConfig> {
        self.file_queue.selected_file().map(|file| &file.config)
    }
    pub(super) fn update_selected_config(
        &mut self,
        update: impl FnOnce(&mut ConversionConfig) -> bool,
    ) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let changed = self
            .file_queue
            .selected_file_mut()
            .is_some_and(|file| update(&mut file.config));
        changed | self.suspend_unsupported_preview_overlay()
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
            self.appearance,
            self.max_concurrency,
            self.default_output_directory.clone(),
            &self.presets,
            self.auto_update_check,
            self.update_channel,
            self.skipped_update_version.clone(),
            self.last_update_check_at,
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
