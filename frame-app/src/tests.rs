use super::*;
use crate::file_queue::FileQueue;

mod frame_app_state {
    use super::*;

    #[test]
    fn can_start_conversion_returns_true_when_selection_has_pending_work() {
        let state = FrameAppState {
            file_count: 1,
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
            file_count: 1,
            selected_count: 1,
            has_actionable_files: true,
            ..FrameAppState::default()
        };

        assert!(!state.can_start_conversion());
    }

    #[test]
    fn start_availability_reports_each_blocker_in_priority_order() {
        let ready = FrameAppState {
            file_count: 1,
            selected_count: 1,
            has_actionable_files: true,
            has_default_output_directory: true,
            ..FrameAppState::default()
        };

        assert_eq!(ready.start_availability(), StartAvailability::Ready);
        assert_eq!(
            FrameAppState {
                is_processing: true,
                ..ready
            }
            .start_availability(),
            StartAvailability::Processing
        );
        assert_eq!(
            FrameAppState::default().start_availability(),
            StartAvailability::NoFiles
        );
        assert_eq!(
            FrameAppState {
                selected_count: 0,
                ..ready
            }
            .start_availability(),
            StartAvailability::NoSelectedFiles
        );
        assert_eq!(
            FrameAppState {
                has_actionable_files: false,
                ..ready
            }
            .start_availability(),
            StartAvailability::NoActionableFiles
        );
        assert_eq!(
            FrameAppState {
                has_default_output_directory: false,
                ..ready
            }
            .start_availability(),
            StartAvailability::MissingOutputDirectory
        );
    }

    #[test]
    fn missing_output_directory_is_an_actionable_start_state() {
        let availability = StartAvailability::MissingOutputDirectory;

        assert!(availability.button_enabled());
        assert_eq!(availability.button_label(), "Choose output");
        assert_eq!(
            availability.accessibility_label(),
            "Choose an output folder before starting"
        );
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
            ("preview-crop-options", VisualFixture::PreviewCropOptions),
            ("preview-crop-custom", VisualFixture::PreviewCropCustom),
            ("settings-video-av1", VisualFixture::SettingsVideoAv1),
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
