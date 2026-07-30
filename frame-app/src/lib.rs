//! Shared state and layout contracts for the native GPUI-CE app.

pub mod app;
pub mod app_info;
pub mod app_persistence;
pub mod appearance;
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
pub(crate) mod runtime_environment;
pub mod settings;
pub mod source_metadata;
pub mod theme;
pub mod update_runtime;
pub(crate) mod update_session;

use file_queue::FileQueue;
use numeric::u64_to_f64;

pub const WINDOW_MIN_WIDTH: f32 = 1200.0;
pub const WINDOW_MIN_HEIGHT: f32 = 800.0;
pub const LINUX_WINDOW_FRAME_INSET: f32 = 24.0;
pub const CONTENT_PADDING: f32 = 16.0;
pub const TITLEBAR_HEIGHT: f32 = 40.0;
pub const TITLEBAR_TOP_PADDING: f32 = 8.0;
pub const TITLEBAR_TRAFFIC_LIGHT_SIZE: f32 = 24.0;
pub const TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_INSET: f32 = 4.8;
pub const TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_X: f32 =
    CONTENT_PADDING + TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_INSET;
pub const TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_Y: f32 =
    TITLEBAR_HEIGHT - TITLEBAR_TRAFFIC_LIGHT_SIZE + TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_INSET;
pub const TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH: f32 =
    TITLEBAR_TRAFFIC_LIGHT_SIZE * 3.0;
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
    AppSettingsThemeOpen,
    AppSettingsUiOpen,
    LogsActive,
    PreviewCrop,
    PreviewReady,
    SettingsAudio,
    SettingsAudioFilters,
    SettingsImages,
    SettingsMetadata,
    SettingsOutput,
    SettingsPresets,
    SettingsSource,
    SettingsSubtitles,
    SettingsSubtitlesPopover,
    SettingsVideo,
    SettingsVideoFilters,
    UpdateAvailable,
    WorkspaceAudio,
    WorkspaceEmpty,
    WorkspaceImage,
    WorkspaceLargeQueue,
}

#[must_use]
pub fn visual_fixture_from_env_value(value: Option<&str>) -> Option<VisualFixture> {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("app-settings") => Some(VisualFixture::AppSettings),
        Some("app-settings-theme-open") => Some(VisualFixture::AppSettingsThemeOpen),
        Some("app-settings-ui-open") => Some(VisualFixture::AppSettingsUiOpen),
        Some("logs-active") => Some(VisualFixture::LogsActive),
        Some("preview-crop") => Some(VisualFixture::PreviewCrop),
        Some("preview-ready") => Some(VisualFixture::PreviewReady),
        Some("settings-audio") => Some(VisualFixture::SettingsAudio),
        Some("settings-audio-filters") => Some(VisualFixture::SettingsAudioFilters),
        Some("settings-images") => Some(VisualFixture::SettingsImages),
        Some("settings-metadata") => Some(VisualFixture::SettingsMetadata),
        Some("settings-output") => Some(VisualFixture::SettingsOutput),
        Some("settings-presets") => Some(VisualFixture::SettingsPresets),
        Some("settings-source") => Some(VisualFixture::SettingsSource),
        Some("settings-subtitles") => Some(VisualFixture::SettingsSubtitles),
        Some("settings-subtitles-popover") => Some(VisualFixture::SettingsSubtitlesPopover),
        Some("settings-video") => Some(VisualFixture::SettingsVideo),
        Some("settings-video-filters") => Some(VisualFixture::SettingsVideoFilters),
        Some("update-available") => Some(VisualFixture::UpdateAvailable),
        Some("workspace-audio") => Some(VisualFixture::WorkspaceAudio),
        Some("workspace-empty") => Some(VisualFixture::WorkspaceEmpty),
        Some("workspace-image") => Some(VisualFixture::WorkspaceImage),
        Some("workspace-large-queue") => Some(VisualFixture::WorkspaceLargeQueue),
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
    pub has_default_output_directory: bool,
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
            has_default_output_directory: false,
            total_size_bytes: 0,
        }
    }
}

impl FrameAppState {
    #[must_use]
    pub const fn can_start_conversion(self) -> bool {
        !self.is_processing
            && self.selected_count > 0
            && self.has_actionable_files
            && self.has_default_output_directory
    }

    #[must_use]
    pub fn from_file_queue(
        active_view: ActiveView,
        is_processing: bool,
        has_default_output_directory: bool,
        file_queue: &FileQueue,
    ) -> Self {
        Self {
            active_view,
            is_processing,
            file_count: file_queue.files().len(),
            selected_count: file_queue.selected_count(),
            has_actionable_files: file_queue.has_actionable_files(),
            has_default_output_directory,
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
    use super::*;

    mod frame_app_state {
        use super::*;

        #[test]
        fn can_start_conversion_returns_true_when_selection_has_pending_work() {
            let state = FrameAppState {
                selected_count: 1,
                has_actionable_files: true,
                has_default_output_directory: true,
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
        fn can_start_conversion_returns_false_without_default_output_directory() {
            let state = FrameAppState {
                selected_count: 1,
                has_actionable_files: true,
                ..FrameAppState::default()
            };

            assert!(!state.can_start_conversion());
        }

        #[test]
        fn from_file_queue_maps_the_complete_queue_snapshot() {
            let mut queue = FileQueue::new();
            queue.add_file(file_queue::FileItem::from_path("first", "/tmp/one.mp4", 10));
            queue.add_file(file_queue::FileItem::from_path(
                "second",
                "/tmp/two.mp4",
                25,
            ));
            assert_eq!(queue.toggle_batch_selection("second"), Some(false));

            let state = FrameAppState::from_file_queue(ActiveView::Logs, true, true, &queue);

            assert_eq!(
                state,
                FrameAppState {
                    active_view: ActiveView::Logs,
                    is_processing: true,
                    file_count: 2,
                    selected_count: 1,
                    has_actionable_files: true,
                    has_default_output_directory: true,
                    total_size_bytes: 35,
                }
            );
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
        fn every_supported_value_enables_its_visual_fixture() {
            let cases = [
                ("app-settings", VisualFixture::AppSettings),
                (
                    "app-settings-theme-open",
                    VisualFixture::AppSettingsThemeOpen,
                ),
                ("app-settings-ui-open", VisualFixture::AppSettingsUiOpen),
                ("logs-active", VisualFixture::LogsActive),
                ("preview-crop", VisualFixture::PreviewCrop),
                ("preview-ready", VisualFixture::PreviewReady),
                ("settings-audio", VisualFixture::SettingsAudio),
                (
                    "settings-audio-filters",
                    VisualFixture::SettingsAudioFilters,
                ),
                ("settings-images", VisualFixture::SettingsImages),
                ("settings-metadata", VisualFixture::SettingsMetadata),
                ("settings-output", VisualFixture::SettingsOutput),
                ("settings-presets", VisualFixture::SettingsPresets),
                ("settings-source", VisualFixture::SettingsSource),
                ("settings-subtitles", VisualFixture::SettingsSubtitles),
                (
                    "settings-subtitles-popover",
                    VisualFixture::SettingsSubtitlesPopover,
                ),
                ("settings-video", VisualFixture::SettingsVideo),
                (
                    "settings-video-filters",
                    VisualFixture::SettingsVideoFilters,
                ),
                ("update-available", VisualFixture::UpdateAvailable),
                ("workspace-audio", VisualFixture::WorkspaceAudio),
                ("workspace-empty", VisualFixture::WorkspaceEmpty),
                ("workspace-image", VisualFixture::WorkspaceImage),
                ("workspace-large-queue", VisualFixture::WorkspaceLargeQueue),
            ];

            for (value, expected) in cases {
                assert_eq!(
                    visual_fixture_from_env_value(Some(value)),
                    Some(expected),
                    "fixture value {value:?} mapped incorrectly"
                );
            }
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
    }
}
