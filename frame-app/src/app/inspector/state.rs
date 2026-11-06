use crate::app::components::popover_state::PopoverState;
use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorInputFocuses, SettingsSubtitleFocuses, SettingsSubtitleHsv,
    SettingsSubtitleMode, SettingsSubtitlePopover,
};
use crate::settings::{
    ConversionConfig, OutputSection, PresetDefinition, SettingsTab, SourceMetadata,
};
use crate::source_metadata::MetadataStatus;
use crate::theme;
use frame_core::capabilities::{AvailableEncoders, AvailableFilters};
use frame_core::types::DEFAULT_MAX_CONCURRENCY;
use gpui::{FocusHandle, ScrollHandle};

pub(in crate::app) struct SettingsUiState {
    pub(in crate::app) is_open: bool,
    pub(in crate::app) is_present: bool,
    pub(in crate::app) active_tab: SettingsTab,
    pub(in crate::app) expanded_output_sections: std::collections::BTreeSet<OutputSection>,
    pub(in crate::app) max_concurrency_draft: String,
    pub(in crate::app) max_concurrency_error: Option<String>,
    pub(in crate::app) output_directory_error: Option<String>,
    pub(in crate::app) appearance_error: Option<String>,
    pub(in crate::app) theme_popover: PopoverState,
    pub(in crate::app) ui_scale_popover: PopoverState,
    pub(in crate::app) app_settings_scroll_handle: ScrollHandle,
    pub(in crate::app) theme_scroll_handle: ScrollHandle,
    pub(in crate::app) ui_scale_scroll_handle: ScrollHandle,
    pub(in crate::app) preset_name_draft: String,
    pub(in crate::app) next_custom_preset_sequence: u64,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_present: false,
            active_tab: SettingsTab::Metadata,
            expanded_output_sections: std::collections::BTreeSet::new(),
            max_concurrency_draft: DEFAULT_MAX_CONCURRENCY.to_string(),
            max_concurrency_error: None,
            output_directory_error: None,
            appearance_error: None,
            theme_popover: PopoverState::Hidden,
            ui_scale_popover: PopoverState::Hidden,
            app_settings_scroll_handle: ScrollHandle::new(),
            theme_scroll_handle: ScrollHandle::new(),
            ui_scale_scroll_handle: ScrollHandle::new(),
            preset_name_draft: String::new(),
            next_custom_preset_sequence: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsRenderState<'a> {
    pub(in crate::app) palette: &'static theme::ThemePalette,
    pub(in crate::app) active_tab: SettingsTab,
    pub(in crate::app) expanded_output_sections: &'a std::collections::BTreeSet<OutputSection>,
    pub(in crate::app) config: &'a ConversionConfig,
    pub(in crate::app) crop_editing: bool,
    pub(in crate::app) metadata: Option<&'a SourceMetadata>,
    pub(in crate::app) metadata_status: MetadataStatus,
    pub(in crate::app) metadata_error: Option<&'a str>,
    pub(in crate::app) settings_disabled: bool,
    pub(in crate::app) output_name: &'a str,
    pub(in crate::app) output_name_focus: Option<&'a FocusHandle>,
    pub(in crate::app) audio_bitrate_focus: Option<&'a FocusHandle>,
    pub(in crate::app) video_width_focus: Option<&'a FocusHandle>,
    pub(in crate::app) video_height_focus: Option<&'a FocusHandle>,
    pub(in crate::app) video_bitrate_focus: Option<&'a FocusHandle>,
    pub(in crate::app) gif_loop_focus: Option<&'a FocusHandle>,
    pub(in crate::app) metadata_focuses: SettingsMetadataInputFocuses<'a>,
    pub(in crate::app) subtitle_focuses: SettingsSubtitleFocuses<'a>,
    pub(in crate::app) external_subtitle_language_focus: Option<&'a FocusHandle>,
    pub(in crate::app) external_subtitle_title_focus: Option<&'a FocusHandle>,
    pub(in crate::app) external_subtitle_track_index: Option<usize>,
    pub(in crate::app) subtitle_mode: SettingsSubtitleMode,
    pub(in crate::app) subtitle_color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    pub(in crate::app) subtitle_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) subtitle_rendered_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) subtitle_font_select_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) subtitle_font_size_select_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) subtitle_font_color_draft: &'a str,
    pub(in crate::app) subtitle_outline_color_draft: &'a str,
    pub(in crate::app) subtitle_font_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) subtitle_outline_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) preset_name: &'a str,
    pub(in crate::app) preset_name_focus: Option<&'a FocusHandle>,
    pub(in crate::app) presets: &'a [PresetDefinition],
    pub(in crate::app) subtitle_fonts: &'a [String],
    pub(in crate::app) available_encoders: &'a AvailableEncoders,
    pub(in crate::app) available_filters: &'a AvailableFilters,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsVideoInputFocuses<'a> {
    pub(in crate::app) resolution_locked: bool,
    pub(in crate::app) width: Option<&'a FocusHandle>,
    pub(in crate::app) height: Option<&'a FocusHandle>,
    pub(in crate::app) bitrate: Option<&'a FocusHandle>,
    pub(in crate::app) gif_loop: Option<&'a FocusHandle>,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsMetadataInputFocuses<'a> {
    pub(in crate::app) title: Option<&'a FocusHandle>,
    pub(in crate::app) artist: Option<&'a FocusHandle>,
    pub(in crate::app) album: Option<&'a FocusHandle>,
    pub(in crate::app) genre: Option<&'a FocusHandle>,
    pub(in crate::app) date: Option<&'a FocusHandle>,
    pub(in crate::app) comment: Option<&'a FocusHandle>,
    pub(in crate::app) service_name: Option<&'a FocusHandle>,
    pub(in crate::app) service_provider: Option<&'a FocusHandle>,
}

impl crate::app::root::FrameRoot {
    pub(in crate::app) fn sync_inspector_availability(
        &mut self,
        metadata: Option<&SourceMetadata>,
    ) {
        if self.selected_config().is_none() {
            self.settings_ui.active_tab = SettingsTab::Metadata;
        }
        let external_tracks = self
            .selected_config()
            .map_or(0, |config| config.external_subtitle_tracks.len());
        self.subtitle_ui.external_track_index = if external_tracks == 0 {
            None
        } else {
            Some(
                self.subtitle_ui
                    .external_track_index
                    .unwrap_or(0)
                    .min(external_tracks - 1),
            )
        };
        let subtitles_visible = self.settings_ui.active_tab == SettingsTab::Output
            && self
                .settings_ui
                .expanded_output_sections
                .contains(&OutputSection::Subtitles)
            && self.selected_config().is_some_and(|config| {
                crate::settings::visible_output_sections(config, metadata)
                    .contains(&OutputSection::Subtitles)
            });
        if !subtitles_visible {
            self.subtitle_ui.popover = None;
            self.subtitle_ui.rendered_popover = None;
        }
    }
}
