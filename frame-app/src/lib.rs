//! Shared state and layout contracts for the native GPUI-CE app.

pub mod app;
pub mod app_info;
pub mod app_persistence;
pub mod assets;
pub mod capabilities;
pub mod conversion_events;
pub mod conversion_runner;
pub mod file_filters;
pub mod file_queue;
pub mod native_dialogs;
pub mod notifications;
pub(crate) mod numeric;
pub mod preview;
pub mod preview_engine;
pub mod runtime_binaries;
pub mod settings;
pub mod source_metadata;
pub mod theme;
pub mod update_runtime;

use file_queue::FileQueue;
use numeric::u64_to_f64;

pub const WINDOW_MIN_WIDTH: f32 = 1200.0;
pub const WINDOW_MIN_HEIGHT: f32 = 800.0;
pub const LINUX_WINDOW_FRAME_INSET: f32 = 24.0;
pub const CONTENT_PADDING: f32 = 16.0;
pub const TITLEBAR_HEIGHT: f32 = 40.0;
pub const TITLEBAR_TOP_PADDING: f32 = 8.0;
pub const TITLEBAR_TRAFFIC_LIGHT_SIZE: f32 = 24.0;
pub const TITLEBAR_TRAFFIC_LIGHT_DOT_SIZE: f32 = 14.4;
pub const TITLEBAR_TRAFFIC_LIGHT_STROKE_WIDTH: f32 = 0.72;
pub const TITLEBAR_LOGO_SIZE: f32 = 20.0;
pub const TITLEBAR_DIVIDER_HEIGHT: f32 = 24.0;
pub const TITLEBAR_SEGMENT_HEIGHT: f32 = 30.0;
pub const TITLEBAR_BUTTON_HEIGHT: f32 = 30.0;
pub const TITLEBAR_ICON_BUTTON_SIZE: f32 = 30.0;
pub const TITLEBAR_NAV_BUTTON_HEIGHT: f32 = 24.0;
pub const TITLEBAR_ICON_SIZE: f32 = 14.0;
pub const TITLEBAR_ACTION_ICON_SIZE: f32 = 16.0;
pub const TITLEBAR_PLATFORM_DIVIDER_HEIGHT: f32 = 20.0;
pub const TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH: f32 = 46.0;
pub const TITLEBAR_WINDOWS_WINDOW_ICON_SIZE: f32 = 18.0;
pub const TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE: f32 = 14.0;
pub const TITLEBAR_LINUX_WINDOW_BUTTON_SIZE: f32 = 28.0;
pub const TITLEBAR_LINUX_WINDOW_CONTROLS_GAP: f32 = 2.0;
pub const TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X: f32 = 8.0;
pub const WORKSPACE_COLUMNS: u16 = 12;
pub const WORKSPACE_GAP: f32 = 16.0;
pub const LEFT_COLUMN_SPAN: u16 = 8;
pub const RIGHT_COLUMN_SPAN: u16 = 4;
pub const LEFT_GRID_ROWS: u16 = 12;
pub const PREVIEW_ROW_SPAN: u16 = 8;
pub const FILE_LIST_ROW_SPAN: u16 = 4;
pub const PANEL_HEADER_HEIGHT: f32 = TITLEBAR_HEIGHT;
pub const FILE_ROW_HEIGHT: f32 = 40.0;
pub const SETTINGS_PANEL_PADDING: f32 = 16.0;
pub const SETTINGS_TAB_BUTTON_SIZE: f32 = 24.0;
pub const SETTINGS_TAB_ICON_SIZE: f32 = 16.0;
pub const SETTINGS_CONTROL_HEIGHT: f32 = 30.0;
pub const PREVIEW_PANEL_PADDING: f32 = CONTENT_PADDING;
pub const PREVIEW_TIMELINE_TOP_MARGIN: f32 = 16.0;
pub const PREVIEW_TIMELINE_CONTROL_HEIGHT: f32 = SETTINGS_CONTROL_HEIGHT;
pub const PREVIEW_TIMELINE_HANDLE_WIDTH: f32 = 20.0;
pub const PREVIEW_TOOLBAR_OFFSET: f32 = 16.0;
pub const PREVIEW_TOOLBAR_BUTTON_SIZE: f32 = 30.0;
pub const PREVIEW_TOOLBAR_ICON_SIZE: f32 = 16.0;
pub const PREVIEW_TRACK_HEIGHT: f32 = 6.0;
pub const PREVIEW_PLAYHEAD_HEIGHT: f32 = 16.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveView {
    Workspace,
    Logs,
}

#[must_use]
pub fn active_view_from_env_value(value: Option<&str>) -> ActiveView {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("logs") => ActiveView::Logs,
        _ => ActiveView::Workspace,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualFixture {
    AppSettings,
    LogsActive,
    PreviewCrop,
    PreviewReady,
    SettingsAudio,
    SettingsImages,
    SettingsMetadata,
    SettingsOutput,
    SettingsPresets,
    SettingsSource,
    SettingsSubtitles,
    SettingsSubtitlesPopover,
    SettingsVideo,
    UpdateAvailable,
    WorkspaceAudio,
    WorkspaceEmpty,
    WorkspaceImage,
}

#[must_use]
pub fn visual_fixture_from_env_value(value: Option<&str>) -> Option<VisualFixture> {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("app-settings") => Some(VisualFixture::AppSettings),
        Some("logs-active") => Some(VisualFixture::LogsActive),
        Some("preview-crop") => Some(VisualFixture::PreviewCrop),
        Some("preview-ready") => Some(VisualFixture::PreviewReady),
        Some("settings-audio") => Some(VisualFixture::SettingsAudio),
        Some("settings-images") => Some(VisualFixture::SettingsImages),
        Some("settings-metadata") => Some(VisualFixture::SettingsMetadata),
        Some("settings-output") => Some(VisualFixture::SettingsOutput),
        Some("settings-presets") => Some(VisualFixture::SettingsPresets),
        Some("settings-source") => Some(VisualFixture::SettingsSource),
        Some("settings-subtitles") => Some(VisualFixture::SettingsSubtitles),
        Some("settings-subtitles-popover") => Some(VisualFixture::SettingsSubtitlesPopover),
        Some("settings-video") => Some(VisualFixture::SettingsVideo),
        Some("update-available") => Some(VisualFixture::UpdateAvailable),
        Some("workspace-audio") => Some(VisualFixture::WorkspaceAudio),
        Some("workspace-empty") => Some(VisualFixture::WorkspaceEmpty),
        Some("workspace-image") => Some(VisualFixture::WorkspaceImage),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameAppState {
    pub active_view: ActiveView,
    pub is_processing: bool,
    pub file_count: usize,
    pub selected_count: usize,
    pub has_actionable_files: bool,
    pub total_size_bytes: u64,
}

impl Default for FrameAppState {
    fn default() -> Self {
        Self {
            active_view: ActiveView::Workspace,
            is_processing: false,
            file_count: 0,
            selected_count: 0,
            has_actionable_files: false,
            total_size_bytes: 0,
        }
    }
}

impl FrameAppState {
    #[must_use]
    pub const fn can_start_conversion(self) -> bool {
        !self.is_processing && self.selected_count > 0 && self.has_actionable_files
    }

    #[must_use]
    pub fn from_file_queue(
        active_view: ActiveView,
        is_processing: bool,
        file_queue: &FileQueue,
    ) -> Self {
        Self {
            active_view,
            is_processing,
            file_count: file_queue.files().len(),
            selected_count: file_queue.selected_count(),
            has_actionable_files: file_queue.has_actionable_files(),
            total_size_bytes: file_queue.total_size_bytes(),
        }
    }
}

#[must_use]
pub fn format_total_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 KB".to_string();
    }

    let mb = u64_to_f64(bytes) / (1024.0 * 1024.0);
    if mb > 1000.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{mb:.1} MB")
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::float_cmp,
        reason = "Layout tests compare exact public sizing constants."
    )]

    use super::*;

    mod frame_app_state {
        use super::*;

        #[test]
        fn can_start_conversion_returns_true_when_selection_has_pending_work() {
            let state = FrameAppState {
                selected_count: 1,
                has_actionable_files: true,
                ..FrameAppState::default()
            };

            assert!(state.can_start_conversion());
        }

        #[test]
        fn can_start_conversion_returns_false_when_app_is_processing() {
            let state = FrameAppState {
                is_processing: true,
                selected_count: 1,
                has_actionable_files: true,
                ..FrameAppState::default()
            };

            assert!(!state.can_start_conversion());
        }

        #[test]
        fn from_file_queue_uses_queue_derived_counts() {
            let mut queue = FileQueue::new();
            queue.add_file(file_queue::FileItem::from_path("first", "/tmp/one.mp4", 10));

            let state = FrameAppState::from_file_queue(ActiveView::Workspace, false, &queue);

            assert_eq!(state.file_count, 1);
        }
    }

    mod active_view_env {
        use super::*;

        #[test]
        fn logs_value_opens_logs_view_for_visual_checks() {
            assert_eq!(active_view_from_env_value(Some("logs")), ActiveView::Logs);
            assert_eq!(active_view_from_env_value(Some(" LOGS ")), ActiveView::Logs);
        }

        #[test]
        fn missing_or_unknown_value_keeps_workspace_default() {
            assert_eq!(active_view_from_env_value(None), ActiveView::Workspace);
            assert_eq!(
                active_view_from_env_value(Some("workspace")),
                ActiveView::Workspace
            );
        }
    }

    mod visual_fixture_env {
        use super::*;

        #[test]
        fn app_settings_value_enables_settings_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("app-settings")),
                Some(VisualFixture::AppSettings)
            );
        }

        #[test]
        fn logs_active_value_enables_logs_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("logs-active")),
                Some(VisualFixture::LogsActive)
            );
        }

        #[test]
        fn preview_ready_value_enables_workspace_preview_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("preview-ready")),
                Some(VisualFixture::PreviewReady)
            );
        }

        #[test]
        fn preview_crop_value_enables_workspace_crop_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("preview-crop")),
                Some(VisualFixture::PreviewCrop)
            );
        }

        #[test]
        fn workspace_empty_value_enables_empty_workspace_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("workspace-empty")),
                Some(VisualFixture::WorkspaceEmpty)
            );
        }

        #[test]
        fn workspace_audio_value_enables_selected_audio_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("workspace-audio")),
                Some(VisualFixture::WorkspaceAudio)
            );
        }

        #[test]
        fn workspace_image_value_enables_selected_image_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("workspace-image")),
                Some(VisualFixture::WorkspaceImage)
            );
        }

        #[test]
        fn settings_source_value_enables_source_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-source")),
                Some(VisualFixture::SettingsSource)
            );
        }

        #[test]
        fn settings_output_value_enables_output_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-output")),
                Some(VisualFixture::SettingsOutput)
            );
        }

        #[test]
        fn settings_audio_value_enables_audio_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-audio")),
                Some(VisualFixture::SettingsAudio)
            );
        }

        #[test]
        fn settings_metadata_value_enables_metadata_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-metadata")),
                Some(VisualFixture::SettingsMetadata)
            );
        }

        #[test]
        fn settings_presets_value_enables_presets_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-presets")),
                Some(VisualFixture::SettingsPresets)
            );
        }

        #[test]
        fn settings_subtitles_value_enables_subtitles_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-subtitles")),
                Some(VisualFixture::SettingsSubtitles)
            );
        }

        #[test]
        fn settings_subtitles_popover_value_enables_subtitles_popover_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-subtitles-popover")),
                Some(VisualFixture::SettingsSubtitlesPopover)
            );
        }

        #[test]
        fn settings_video_value_enables_video_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-video")),
                Some(VisualFixture::SettingsVideo)
            );
        }

        #[test]
        fn settings_images_value_enables_images_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("settings-images")),
                Some(VisualFixture::SettingsImages)
            );
        }

        #[test]
        fn update_available_value_enables_update_fixture() {
            assert_eq!(
                visual_fixture_from_env_value(Some("update-available")),
                Some(VisualFixture::UpdateAvailable)
            );
        }

        #[test]
        fn missing_or_unknown_value_disables_visual_fixtures() {
            assert_eq!(visual_fixture_from_env_value(None), None);
            assert_eq!(visual_fixture_from_env_value(Some("workspace")), None);
        }
    }

    mod format_total_size {
        use super::*;

        #[test]
        fn returns_zero_kilobytes_when_size_is_empty() {
            assert_eq!(format_total_size(0), "0 KB");
        }

        #[test]
        fn returns_megabytes_below_browser_threshold() {
            assert_eq!(format_total_size(512 * 1024 * 1024), "512.0 MB");
        }

        #[test]
        fn returns_gigabytes_above_browser_threshold() {
            assert_eq!(format_total_size(2 * 1024 * 1024 * 1024), "2.00 GB");
        }
    }

    mod layout_contract {
        use super::*;

        #[test]
        fn workspace_columns_preserve_original_left_right_split() {
            assert_eq!(LEFT_COLUMN_SPAN + RIGHT_COLUMN_SPAN, WORKSPACE_COLUMNS);
        }

        #[test]
        fn left_workspace_rows_preserve_original_preview_file_list_split() {
            assert_eq!(PREVIEW_ROW_SPAN + FILE_LIST_ROW_SPAN, LEFT_GRID_ROWS);
        }

        #[test]
        fn titlebar_height_matches_shared_panel_header_height() {
            assert_eq!(TITLEBAR_HEIGHT, PANEL_HEADER_HEIGHT);
        }

        #[test]
        fn macos_traffic_lights_preserve_original_hit_area() {
            assert_eq!(TITLEBAR_TRAFFIC_LIGHT_SIZE, 24.0);
        }

        #[test]
        fn macos_traffic_lights_preserve_original_svg_circle_geometry() {
            assert_eq!(TITLEBAR_TRAFFIC_LIGHT_DOT_SIZE, 14.4);
            assert_eq!(TITLEBAR_TRAFFIC_LIGHT_STROKE_WIDTH, 0.72);
        }

        #[test]
        fn titlebar_segment_matches_original_thirty_pixel_control() {
            assert_eq!(TITLEBAR_SEGMENT_HEIGHT, 30.0);
        }

        #[test]
        fn platform_titlebar_divider_matches_windows_linux_original_height() {
            assert_eq!(TITLEBAR_PLATFORM_DIVIDER_HEIGHT, 20.0);
        }

        #[test]
        fn windows_titlebar_controls_match_original_hit_areas() {
            assert_eq!(TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH, 46.0);
            assert_eq!(TITLEBAR_WINDOWS_WINDOW_ICON_SIZE, 18.0);
            assert_eq!(TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE, 14.0);
        }

        #[test]
        fn linux_titlebar_controls_match_original_compact_buttons() {
            assert_eq!(TITLEBAR_LINUX_WINDOW_BUTTON_SIZE, 28.0);
            assert_eq!(TITLEBAR_LINUX_WINDOW_CONTROLS_GAP, 2.0);
            assert_eq!(TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X, 8.0);
        }

        #[test]
        fn settings_tab_button_matches_original_icon_button_size() {
            assert_eq!(SETTINGS_TAB_BUTTON_SIZE, 24.0);
        }

        #[test]
        fn settings_panel_padding_matches_original_content_padding() {
            assert_eq!(SETTINGS_PANEL_PADDING, CONTENT_PADDING);
        }

        #[test]
        fn settings_controls_match_original_default_button_height() {
            assert_eq!(SETTINGS_CONTROL_HEIGHT, 30.0);
        }

        #[test]
        fn preview_panel_padding_matches_original_card_padding() {
            assert_eq!(PREVIEW_PANEL_PADDING, CONTENT_PADDING);
        }

        #[test]
        fn preview_timeline_controls_match_original_timecode_height() {
            assert_eq!(PREVIEW_TIMELINE_CONTROL_HEIGHT, 30.0);
        }

        #[test]
        fn preview_toolbar_buttons_match_original_icon_button_size() {
            assert_eq!(PREVIEW_TOOLBAR_BUTTON_SIZE, 30.0);
        }

        #[test]
        fn preview_timeline_handle_matches_original_hit_width() {
            assert_eq!(PREVIEW_TIMELINE_HANDLE_WIDTH, 20.0);
        }
    }
}
