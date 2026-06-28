use super::input::{
    should_capture_text_input_drag, should_handle_text_input, text_input_scroll_x_for_cursor,
};
use super::preview_actions::{
    lerp_preview_canvas_value, preview_canvas_initial_zoom, preview_canvas_layout_metrics,
    preview_canvas_pan_limits, preview_canvas_transform_settled,
};
use super::preview_panel::{
    centered_offset, preview_crop_visual_rect, preview_shell_state, preview_timeline_labels,
    preview_trim_enabled, preview_visual_controls_visible, timeline_fraction_from_percent,
    timeline_slider_percent_from_bounds,
};
use super::primitives::{ButtonVariant, button_colors};
use super::settings_panel::{hex_to_subtitle_hsv, subtitle_hsv_to_hex};
use super::*;
use crate::app_persistence::{AppPersistence, AppSettings};
use crate::notifications::{AppNotifier, ConversionNotificationSummary};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

static TEST_SETTINGS_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

mod frame_root_imports {
    use super::*;

    #[test]
    fn allocate_file_imports_assigns_incrementing_ids() {
        let mut root = FrameRoot::new();

        let imports = root.allocate_file_imports(vec![
            PathBuf::from("/tmp/one.mp4"),
            PathBuf::from("/tmp/two.mp4"),
        ]);

        assert_eq!(imports[0].0, "file-1");
        assert_eq!(imports[1].0, "file-2");
    }

    #[test]
    fn drag_drop_overlay_stays_present_until_close_motion_finishes() {
        let mut root = FrameRoot::new();
        root.drag_drop_ui.is_open = true;
        root.drag_drop_ui.is_present = true;

        assert!(root.close_drag_drop_overlay());

        assert!(!root.drag_drop_ui.is_open);
        assert!(root.drag_drop_ui.is_present);
        assert!(root.finish_drag_drop_overlay_close());
        assert!(!root.drag_drop_ui.is_present);
        assert!(!root.finish_drag_drop_overlay_close());
    }

    #[test]
    fn drag_drop_overlay_open_is_stable_without_pointer_motion() {
        let mut root = FrameRoot::new();

        assert!(root.open_drag_drop_overlay());
        assert!(!root.open_drag_drop_overlay());
        assert!(root.drag_drop_ui.is_open);
        assert!(root.drag_drop_ui.is_present);
    }

    #[test]
    fn allocate_file_imports_continues_after_previous_batch() {
        let mut root = FrameRoot::new();
        root.allocate_file_imports(vec![PathBuf::from("/tmp/one.mp4")]);

        let imports = root.allocate_file_imports(vec![PathBuf::from("/tmp/two.mp4")]);

        assert_eq!(imports[0].0, "file-2");
    }

    #[test]
    fn allocate_file_imports_returns_empty_for_empty_drop() {
        let mut root = FrameRoot::new();

        let imports = root.allocate_file_imports(Vec::new());

        assert!(imports.is_empty());
    }

    #[test]
    fn allocate_file_imports_skips_unsupported_source_extensions() {
        let mut root = FrameRoot::new();

        let imports = root.allocate_file_imports(vec![
            PathBuf::from("/tmp/one.mp4"),
            PathBuf::from("/tmp/readme.txt"),
            PathBuf::from("/tmp/two.PNG"),
        ]);

        assert_eq!(
            imports,
            [
                ("file-1".to_string(), PathBuf::from("/tmp/one.mp4")),
                ("file-2".to_string(), PathBuf::from("/tmp/two.PNG")),
            ]
        );
    }

    #[test]
    fn allocate_file_imports_does_not_advance_ids_for_unsupported_sources() {
        let mut root = FrameRoot::new();
        root.allocate_file_imports(vec![PathBuf::from("/tmp/readme.txt")]);

        let imports = root.allocate_file_imports(vec![PathBuf::from("/tmp/clip.mov")]);

        assert_eq!(imports[0].0, "file-1");
    }
}

mod frame_root_conversion {
    use super::*;

    #[test]
    fn queue_selected_conversion_tasks_marks_pending_file_as_queued() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
        root.file_queue.toggle_batch("second", false);

        let tasks = root.queue_selected_conversion_tasks();

        assert_eq!(
            tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            ["first"]
        );
        assert_eq!(
            root.file_queue.file_by_id("first").map(|file| file.status),
            Some(FileStatus::Queued)
        );
        assert_eq!(tasks[0].output_name.as_deref(), Some("one_converted"));
    }

    #[test]
    fn queue_selected_conversion_tasks_normalizes_each_file_from_own_metadata() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.file_queue
            .add_file(FileItem::from_path("image", "/tmp/two.png", 1));
        root.source_metadata.mark_ready(
            "image",
            SourceMetadata {
                media_kind: Some(SourceKind::Image),
                ..SourceMetadata::default()
            },
        );

        let tasks = root.queue_selected_conversion_tasks();

        let image_task = tasks
            .iter()
            .find(|task| task.id == "image")
            .expect("image task should be queued");
        assert_eq!(image_task.config.container, "png");
        assert_eq!(image_task.config.video_codec, "png");
    }

    #[test]
    fn queue_selected_conversion_tasks_infers_image_config_from_extension_without_metadata() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("image", "/tmp/two.PNG", 1));

        let tasks = root.queue_selected_conversion_tasks();

        assert_eq!(tasks[0].config.container, "png");
        assert_eq!(tasks[0].config.video_codec, "png");
    }

    #[test]
    fn apply_conversion_event_updates_processing_state_from_queue() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.queue_selected_conversion_tasks();
        root.is_processing = true;

        root.apply_conversion_event(ConversionEvent::completed("first", "/tmp/one.mp4"));

        assert!(!root.is_processing);
        assert_eq!(
            root.file_queue.file_by_id("first").map(|file| file.status),
            Some(FileStatus::Completed)
        );
    }

    #[test]
    fn apply_conversion_event_notifies_when_active_batch_settles() {
        let notifications = std::sync::Arc::new(Mutex::new(Vec::new()));
        let received_notifications = notifications.clone();
        let mut root = FrameRoot::new_with_notifier(AppNotifier::from_conversion_finished_handler(
            move |summary| {
                received_notifications
                    .lock()
                    .expect("notifications should be writable")
                    .push(summary);
            },
        ));
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
        root.queue_selected_conversion_tasks();
        root.active_conversion_task_ids = vec!["first".to_string(), "second".to_string()];
        root.is_processing = true;

        root.apply_conversion_event(ConversionEvent::completed("first", "/tmp/one.mp4"));
        root.apply_conversion_event(ConversionEvent::error("second", "ffmpeg failed"));

        assert_eq!(
            notifications
                .lock()
                .expect("notifications should be readable")
                .as_slice(),
            [ConversionNotificationSummary {
                completed_count: 1,
                error_count: 1,
            }]
        );
    }

    #[test]
    fn apply_conversion_event_does_not_notify_when_active_batch_has_no_results() {
        let notifications = std::sync::Arc::new(Mutex::new(Vec::new()));
        let received_notifications = notifications.clone();
        let mut root = FrameRoot::new_with_notifier(AppNotifier::from_conversion_finished_handler(
            move |summary| {
                received_notifications
                    .lock()
                    .expect("notifications should be writable")
                    .push(summary);
            },
        ));
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.queue_selected_conversion_tasks();
        root.active_conversion_task_ids = vec!["first".to_string()];
        root.is_processing = true;

        root.apply_conversion_event(ConversionEvent::cancelled("first"));

        assert!(
            notifications
                .lock()
                .expect("notifications should be readable")
                .is_empty()
        );
    }

    #[test]
    fn remove_file_from_queue_cancels_and_removes_paused_file() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .update_status("first", FileStatus::Paused, 30);
        root.conversion_events
            .apply_conversion_event(&mut root.file_queue, ConversionEvent::log("first", "line"));

        assert!(root.remove_file_from_queue("first"));

        assert!(root.file_queue.file_by_id("first").is_none());
        assert!(root.conversion_events.logs_for("first").is_empty());
    }

    #[test]
    fn pause_conversion_task_keeps_status_when_process_is_missing() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .update_status("first", FileStatus::Converting, 30);

        assert!(!root.pause_conversion_task("first"));

        assert_eq!(
            root.file_queue.file_by_id("first").map(|file| file.status),
            Some(FileStatus::Converting)
        );
        assert!(
            root.conversion_events
                .logs_for("first")
                .iter()
                .any(|line| line.contains("Failed to pause"))
        );
    }

    #[test]
    fn max_concurrency_defaults_to_shared_backend_limit() {
        let root = FrameRoot::new();

        assert_eq!(root.max_concurrency, DEFAULT_MAX_CONCURRENCY);
        assert_eq!(
            root.conversion_processes
                .current_max_concurrency()
                .expect("max concurrency should be readable"),
            DEFAULT_MAX_CONCURRENCY
        );
    }

    #[test]
    fn new_with_persistence_hydrates_max_concurrency_and_custom_presets() {
        let persistence = AppPersistence::from_settings_path(test_settings_path());
        persistence
            .save(&AppSettings {
                max_concurrency: 6,
                custom_presets: vec![PresetDefinition::custom(
                    "custom-preset-7".to_string(),
                    "Review MP4".to_string(),
                    ConversionConfig::default(),
                )],
                ..AppSettings::default()
            })
            .expect("settings should be saved");

        let root = FrameRoot::new_with_persistence(persistence);

        assert_eq!(root.max_concurrency, 6);
        assert_eq!(
            root.conversion_processes
                .current_max_concurrency()
                .expect("max concurrency should be readable"),
            6
        );
        assert!(
            root.presets
                .iter()
                .any(|preset| preset.name == "Review MP4")
        );
        assert_eq!(root.settings_ui.next_custom_preset_sequence, 7);
    }

    #[test]
    fn apply_max_concurrency_draft_updates_live_controller_limit() {
        let mut root = FrameRoot::new();
        root.settings_ui.max_concurrency_draft = "4".to_string();

        assert!(root.apply_max_concurrency_draft());

        assert_eq!(root.max_concurrency, 4);
        assert_eq!(
            root.conversion_processes
                .current_max_concurrency()
                .expect("max concurrency should be readable"),
            4
        );
    }

    #[test]
    fn app_settings_close_keeps_sheet_present_until_motion_finishes() {
        let mut root = FrameRoot::new();

        root.open_app_settings();
        root.close_app_settings();

        assert!(!root.settings_ui.is_open);
        assert!(root.settings_ui.is_present);

        assert!(root.finish_app_settings_close());
        assert!(!root.settings_ui.is_present);
    }

    #[test]
    fn app_settings_sheet_motion_keeps_final_edge_inset() {
        assert_eq!(settings_sheet_right_inset(1.0), 8.0);
        assert_eq!(settings_sheet_right_inset(0.0), -16.0);
    }

    #[test]
    fn apply_max_concurrency_draft_persists_updated_limit() {
        let persistence = AppPersistence::from_settings_path(test_settings_path());
        let mut root = FrameRoot::new_with_persistence(persistence.clone());
        root.settings_ui.max_concurrency_draft = "5".to_string();

        assert!(root.apply_max_concurrency_draft());

        assert_eq!(
            persistence
                .load()
                .expect("settings should be readable")
                .max_concurrency,
            5
        );
    }

    #[test]
    fn apply_max_concurrency_draft_rejects_zero() {
        let mut root = FrameRoot::new();
        root.settings_ui.max_concurrency_draft = "0".to_string();

        assert!(!root.apply_max_concurrency_draft());

        assert_eq!(root.max_concurrency, DEFAULT_MAX_CONCURRENCY);
        assert!(root.settings_ui.max_concurrency_error.is_some());
    }

    #[test]
    fn max_concurrency_input_inserts_digits_at_selection() {
        let mut root = FrameRoot::new();
        root.settings_ui.max_concurrency_draft = "12".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::MaxConcurrency)
            .selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::MaxConcurrency,
            None,
            "9",
            None,
            false,
        ));

        assert_eq!(root.settings_ui.max_concurrency_draft, "192");
        assert_eq!(
            root.text_input_runtime(FrameTextInputKind::MaxConcurrency)
                .selected_range,
            2..2
        );
    }

    #[test]
    fn max_concurrency_input_deletes_selected_range() {
        let mut root = FrameRoot::new();
        root.settings_ui.max_concurrency_draft = "12".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::MaxConcurrency)
            .selected_range = 1..2;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::MaxConcurrency,
            None,
            "",
            None,
            false,
        ));

        assert_eq!(root.settings_ui.max_concurrency_draft, "1");
        assert_eq!(
            root.text_input_runtime(FrameTextInputKind::MaxConcurrency)
                .selected_range,
            1..1
        );
    }

    #[test]
    fn max_concurrency_apply_updates_live_controller_limit() {
        let mut root = FrameRoot::new();
        root.settings_ui.max_concurrency_draft = "4".to_string();

        assert!(root.apply_max_concurrency_draft());

        assert_eq!(root.max_concurrency, 4);
        assert_eq!(
            root.conversion_processes
                .current_max_concurrency()
                .expect("max concurrency should be readable"),
            4
        );
    }

    #[test]
    fn output_name_input_appends_text_at_selection() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        let len = root
            .file_queue
            .selected_file()
            .map_or(0, |file| file.output_name.len());
        root.text_input_runtime_mut(FrameTextInputKind::OutputName)
            .selected_range = len..len;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::OutputName,
            None,
            "x",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.output_name.as_str()),
            Some("one_convertedx")
        );
    }

    #[test]
    fn output_name_input_delete_can_leave_field_empty() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue.update_selected_output_name("a");
        root.text_input_runtime_mut(FrameTextInputKind::OutputName)
            .selected_range = 0..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::OutputName,
            None,
            "",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.output_name.as_str()),
            Some("")
        );
    }

    #[test]
    fn preview_start_time_input_normalizes_seconds_to_timecode() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.preview_ui.playback =
            preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
        root.text_input_runtime_mut(FrameTextInputKind::PreviewStartTime)
            .selected_range = 0..12;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::PreviewStartTime,
            None,
            "12.5",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.start_time.as_deref()),
            Some("00:00:12.500")
        );
        assert_eq!(
            root.text_input_value(FrameTextInputKind::PreviewStartTime),
            "00:00:12.500"
        );
    }

    #[test]
    fn preview_end_time_input_can_clear_existing_bound() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.update_selected_config(|config| {
            config.end_time = Some("00:00:30.000".to_string());
            true
        });
        root.preview_ui.playback =
            preview_playback_state(PreviewMediaKind::Video, 90.0, None, Some("00:00:30.000"));
        root.text_input_runtime_mut(FrameTextInputKind::PreviewEndTime)
            .selected_range = 0..12;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::PreviewEndTime,
            None,
            "",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.end_time.as_deref()),
            None
        );
        assert_eq!(
            root.text_input_value(FrameTextInputKind::PreviewEndTime),
            "00:01:30.000"
        );
    }

    #[test]
    fn metadata_title_input_inserts_free_text_at_selection() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .selected_file_mut()
            .unwrap()
            .config
            .metadata
            .title = Some("Render".to_string());
        root.text_input_runtime_mut(FrameTextInputKind::MetadataTitle)
            .selected_range = 6..6;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::MetadataTitle,
            None,
            " Title",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.metadata.title.as_deref()),
            Some("Render Title")
        );
    }

    #[test]
    fn preset_name_input_inserts_free_text_at_selection() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.settings_ui.preset_name_draft = "Review".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::PresetName)
            .selected_range = 6..6;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::PresetName,
            None,
            " MP4",
            None,
            false,
        ));

        assert_eq!(root.settings_ui.preset_name_draft, "Review MP4");
    }

    #[test]
    fn subtitle_font_color_hex_input_expands_short_hex_and_updates_config() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.subtitle_ui.font_color_draft = "#".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::SubtitleFontColorHex)
            .selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::SubtitleFontColorHex,
            None,
            "abc",
            None,
            false,
        ));

        assert_eq!(root.subtitle_ui.font_color_draft, "#AABBCC");
        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.subtitle_font_color.as_deref()),
            Some("#aabbcc")
        );
    }

    #[test]
    fn subtitle_outline_color_hex_input_keeps_incomplete_draft_without_committing() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.subtitle_ui.outline_color_draft = "#".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::SubtitleOutlineColorHex)
            .selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::SubtitleOutlineColorHex,
            None,
            "f",
            None,
            false,
        ));

        assert_eq!(root.subtitle_ui.outline_color_draft, "#F");
        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.subtitle_outline_color.as_deref()),
            None
        );
    }

    #[test]
    fn subtitle_color_hsv_commit_updates_selected_config_and_draft() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));

        assert!(root.commit_subtitle_hsv_color(
            SettingsSubtitleColorTarget::Font,
            SettingsSubtitleHsv {
                h: 60.0,
                s: 1.0,
                v: 1.0,
            },
        ));

        assert_eq!(root.subtitle_ui.font_color_draft, "#FFFF00");
        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.subtitle_font_color.as_deref()),
            Some("#ffff00")
        );
    }

    #[test]
    fn subtitle_color_click_commits_from_picker_bounds() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.set_subtitle_color_picker_bounds(
            SettingsSubtitleColorTarget::Font,
            SettingsSubtitleColorDragKind::SaturationValue,
            Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(100.0))),
        );

        assert!(root.commit_subtitle_color_at_position(
            SettingsSubtitleColorTarget::Font,
            SettingsSubtitleColorDragKind::SaturationValue,
            point(px(10.0), px(20.0)),
        ));

        assert_eq!(root.subtitle_ui.font_color_draft, "#FFFFFF");
        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.subtitle_font_color.as_deref()),
            Some("#ffffff")
        );
    }

    #[test]
    fn subtitle_color_sv_drag_keeps_start_hue_after_white() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.set_subtitle_color_picker_bounds(
            SettingsSubtitleColorTarget::Font,
            SettingsSubtitleColorDragKind::SaturationValue,
            Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(100.0))),
        );
        let drag = SettingsSubtitleColorDrag {
            target: SettingsSubtitleColorTarget::Font,
            kind: SettingsSubtitleColorDragKind::SaturationValue,
            base_hsv: SettingsSubtitleHsv {
                h: 270.0,
                s: 1.0,
                v: 1.0,
            },
        };

        assert!(root.commit_subtitle_color_drag_at_position(drag, point(px(10.0), px(20.0))));
        assert_eq!(root.subtitle_ui.font_color_draft, "#FFFFFF");
        assert_eq!(
            root.subtitle_ui.font_color_hsv_draft,
            SettingsSubtitleHsv {
                h: 270.0,
                s: 0.0,
                v: 1.0,
            }
        );
        assert!(root.commit_subtitle_color_drag_at_position(drag, point(px(110.0), px(20.0))));

        assert_eq!(root.subtitle_ui.font_color_draft, "#8000FF");
        assert_eq!(root.subtitle_ui.font_color_hsv_draft.h, 270.0);
        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.subtitle_font_color.as_deref()),
            Some("#8000ff")
        );
    }

    #[test]
    fn subtitle_popover_toggle_keeps_only_one_open_panel() {
        let mut root = FrameRoot::new();

        root.toggle_subtitle_popover(SettingsSubtitlePopover::FontName);
        assert_eq!(
            root.subtitle_ui.popover,
            Some(SettingsSubtitlePopover::FontName)
        );
        assert_eq!(
            root.subtitle_ui.rendered_popover,
            Some(SettingsSubtitlePopover::FontName)
        );

        root.toggle_subtitle_popover(SettingsSubtitlePopover::FontSize);
        assert_eq!(
            root.subtitle_ui.popover,
            Some(SettingsSubtitlePopover::FontSize)
        );
        assert_eq!(
            root.subtitle_ui.rendered_popover,
            Some(SettingsSubtitlePopover::FontSize)
        );

        root.toggle_subtitle_popover(SettingsSubtitlePopover::FontSize);
        assert_eq!(root.subtitle_ui.popover, None);
        assert_eq!(
            root.subtitle_ui.rendered_popover,
            Some(SettingsSubtitlePopover::FontSize)
        );
        assert!(root.finish_subtitle_popover_close(SettingsSubtitlePopover::FontSize));
        assert_eq!(root.subtitle_ui.rendered_popover, None);
    }

    #[test]
    fn save_preset_from_draft_adds_custom_preset() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.settings_ui.preset_name_draft = "Review MP4".to_string();

        assert!(root.save_preset_from_draft());

        assert!(
            root.presets
                .iter()
                .any(|preset| preset.name == "Review MP4")
        );
        assert!(root.settings_ui.preset_name_draft.is_empty());
    }

    #[test]
    fn save_preset_from_draft_persists_custom_preset_with_unique_id() {
        let persistence = AppPersistence::from_settings_path(test_settings_path());
        persistence
            .save(&AppSettings {
                max_concurrency: DEFAULT_MAX_CONCURRENCY,
                custom_presets: vec![PresetDefinition::custom(
                    "custom-preset-3".to_string(),
                    "Existing".to_string(),
                    ConversionConfig::default(),
                )],
                ..AppSettings::default()
            })
            .expect("settings should be saved");
        let mut root = FrameRoot::new_with_persistence(persistence.clone());
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.settings_ui.preset_name_draft = "Review MP4".to_string();

        assert!(root.save_preset_from_draft());

        let settings = persistence.load().expect("settings should be readable");
        assert!(
            settings
                .custom_presets
                .iter()
                .any(|preset| preset.id == "custom-preset-4" && preset.name == "Review MP4")
        );
    }

    #[test]
    fn delete_preset_persists_removed_custom_preset() {
        let persistence = AppPersistence::from_settings_path(test_settings_path());
        persistence
            .save(&AppSettings {
                max_concurrency: DEFAULT_MAX_CONCURRENCY,
                custom_presets: vec![PresetDefinition::custom(
                    "custom-preset-1".to_string(),
                    "Review MP4".to_string(),
                    ConversionConfig::default(),
                )],
                ..AppSettings::default()
            })
            .expect("settings should be saved");
        let mut root = FrameRoot::new_with_persistence(persistence.clone());

        assert!(root.delete_preset("custom-preset-1"));

        assert!(
            persistence
                .load()
                .expect("settings should be readable")
                .custom_presets
                .is_empty()
        );
    }

    #[test]
    fn audio_bitrate_input_inserts_digits_at_selection() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .selected_file_mut()
            .unwrap()
            .config
            .audio_bitrate = "12".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::AudioBitrate)
            .selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::AudioBitrate,
            None,
            "9",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.config.audio_bitrate.as_str()),
            Some("192")
        );
        assert_eq!(
            root.text_input_runtime(FrameTextInputKind::AudioBitrate)
                .selected_range,
            2..2
        );
    }

    #[test]
    fn audio_bitrate_input_rejects_non_digits() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .selected_file_mut()
            .unwrap()
            .config
            .audio_bitrate = "128".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::AudioBitrate)
            .selected_range = 3..3;

        assert!(!root.replace_text_input_range(
            FrameTextInputKind::AudioBitrate,
            None,
            "k",
            None,
            false,
        ));

        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.config.audio_bitrate.as_str()),
            Some("128")
        );
    }

    #[test]
    fn text_input_handler_is_scoped_to_the_active_focused_field() {
        assert!(!should_handle_text_input(false, false, false));
        assert!(!should_handle_text_input(false, true, false));
        assert!(!should_handle_text_input(false, false, true));
        assert!(should_handle_text_input(false, true, true));
        assert!(!should_handle_text_input(true, true, true));
    }

    #[test]
    fn text_input_outside_mouse_up_captures_only_while_selecting() {
        assert!(!should_capture_text_input_drag(false));
        assert!(should_capture_text_input_drag(true));
    }

    #[test]
    fn text_input_scroll_reveals_cursor_past_right_edge() {
        let scroll_x = text_input_scroll_x_for_cursor(px(0.0), px(180.0), px(240.0), px(120.0));

        assert!(scroll_x > px(0.0));
    }

    #[test]
    fn text_input_scroll_reveals_cursor_past_left_edge() {
        let scroll_x = text_input_scroll_x_for_cursor(px(80.0), px(40.0), px(240.0), px(120.0));

        assert_eq!(scroll_x, px(40.0));
    }

    #[test]
    fn text_input_scroll_stays_zero_when_content_fits() {
        let scroll_x = text_input_scroll_x_for_cursor(px(0.0), px(60.0), px(90.0), px(120.0));

        assert_eq!(scroll_x, px(0.0));
    }
}

mod frame_root_config {
    use super::*;

    #[test]
    fn update_selected_config_mutates_only_selected_file() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
        root.file_queue.select_existing_file("second");

        root.update_selected_config(|config| {
            config.container = "webm".to_string();
            true
        });

        assert_eq!(
            root.file_queue
                .file_by_id("first")
                .map(|file| file.config.container.as_str()),
            Some("mp4")
        );
        assert_eq!(
            root.file_queue
                .file_by_id("second")
                .map(|file| file.config.container.as_str()),
            Some("webm")
        );
    }

    #[test]
    fn normalize_selected_config_clears_trim_for_selected_image_only() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.file_queue
            .add_file(FileItem::from_path("image", "/tmp/two.png", 1));

        for id in ["video", "image"] {
            root.file_queue.select_existing_file(id);
            root.update_selected_config(|config| {
                config.start_time = Some("00:00:05.000".to_string());
                config.end_time = Some("00:00:30.000".to_string());
                true
            });
        }
        root.file_queue.select_existing_file("image");

        root.normalize_selected_config(Some(&SourceMetadata {
            media_kind: Some(SourceKind::Image),
            ..SourceMetadata::default()
        }));

        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.start_time.as_deref()),
            Some("00:00:05.000")
        );
        assert_eq!(
            root.file_queue
                .file_by_id("image")
                .and_then(|file| file.config.start_time.as_deref()),
            None
        );
    }

    #[test]
    fn apply_preview_timeline_drag_updates_selected_file_start_time() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.0".to_string()),
                ..SourceMetadata::default()
            },
        );
        root.preview_ui.playback =
            preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);

        let changed = root.apply_preview_timeline_drag(TimelineDragTarget::Start, 0.25);

        assert!(changed);
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.start_time.as_deref()),
            Some("00:00:22.500")
        );
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.end_time.as_deref()),
            None
        );
    }

    #[test]
    fn apply_preview_timeline_handle_drag_pauses_active_playback() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.0".to_string()),
                ..SourceMetadata::default()
            },
        );
        root.preview_ui.playback =
            preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
        root.preview_ui.playback.handle_play();

        assert!(root.apply_preview_timeline_drag(TimelineDragTarget::End, 0.75));

        assert!(!root.preview_ui.playback.is_playing());
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.end_time.as_deref()),
            Some("00:01:07.500")
        );
    }

    #[test]
    fn apply_preview_timeline_drag_preserves_gap_when_end_moves_before_start() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.0".to_string()),
                ..SourceMetadata::default()
            },
        );
        root.update_selected_config(|config| {
            config.start_time = Some("00:00:20.000".to_string());
            true
        });
        root.preview_ui.playback =
            preview_playback_state(PreviewMediaKind::Video, 90.0, Some("00:00:20.000"), None);

        let changed = root.apply_preview_timeline_drag(TimelineDragTarget::End, 0.10);

        assert!(changed);
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.end_time.as_deref()),
            Some("00:00:21.000")
        );
    }

    #[test]
    fn apply_preview_timeline_drag_ignores_image_sources() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("image", "/tmp/one.png", 1));
        root.source_metadata.mark_ready(
            "image".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Image),
                duration: Some("90.0".to_string()),
                ..SourceMetadata::default()
            },
        );

        let changed = root.apply_preview_timeline_drag(TimelineDragTarget::Start, 0.25);

        assert!(!changed);
        assert_eq!(
            root.file_queue
                .file_by_id("image")
                .and_then(|file| file.config.start_time.as_deref()),
            None
        );
    }

    #[test]
    fn commit_preview_timeline_seek_at_position_updates_local_playhead() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.0".to_string()),
                ..SourceMetadata::default()
            },
        );
        root.preview_ui.playback =
            preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
        root.set_preview_timeline_track_bounds(Bounds {
            origin: point(px(10.0), px(0.0)),
            size: size(px(100.0), px(30.0)),
        });

        let changed = root.commit_preview_timeline_seek_at_position(point(px(60.0), px(0.0)));

        assert!(changed);
        assert_eq!(root.preview_ui.playback.current_time(), 45.0);
    }

    #[test]
    fn toggle_selected_crop_mode_initializes_default_video_draft() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );

        let changed = root.toggle_selected_crop_mode();

        assert!(changed);
        assert!(root.preview_ui.crop_mode);
        assert_eq!(root.preview_ui.draft_crop, Some(default_crop_rect()));
        assert_eq!(root.preview_ui.crop_aspect, "free");
    }

    #[test]
    fn select_preview_crop_aspect_keeps_side_rotation_preview_square() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );

        assert!(root.rotate_selected_preview());
        assert!(root.toggle_selected_crop_mode());
        assert!(root.select_preview_crop_aspect("1:1"));

        let config = &root.file_queue.file_by_id("video").unwrap().config;
        let metadata_entry = root.source_metadata.entry_for("video");
        let crop_state = root.preview_crop_render_state(metadata_entry.metadata.as_ref(), config);
        let visual_rect = preview_crop_visual_rect(&crop_state);
        let visual_width = visual_rect.width * 1080.0;
        let visual_height = visual_rect.height * 1920.0;

        assert!((visual_width - visual_height).abs() < 0.000_001);
        assert!(root.apply_selected_crop());
        let crop = root
            .file_queue
            .file_by_id("video")
            .and_then(|file| file.config.crop.as_ref())
            .expect("crop settings");
        assert_eq!(crop.width, crop.height);
    }

    #[test]
    fn apply_selected_crop_writes_selected_file_crop_pixels() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.file_queue
            .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
        root.file_queue.select_existing_file("second");
        root.source_metadata.mark_ready(
            "second".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );
        root.preview_ui.crop_mode = true;
        root.preview_ui.draft_crop = Some(CropRect {
            x: 0.25,
            y: 0.25,
            width: 0.5,
            height: 0.5,
        });
        root.preview_ui.crop_aspect = "16:9".to_string();

        let changed = root.apply_selected_crop();

        assert!(changed);
        assert_eq!(
            root.file_queue
                .file_by_id("first")
                .and_then(|file| file.config.crop.as_ref()),
            None
        );
        assert_eq!(
            root.file_queue
                .file_by_id("second")
                .and_then(|file| file.config.crop.as_ref()),
            Some(&CropSettings {
                enabled: true,
                x: 480,
                y: 270,
                width: 960,
                height: 540,
                source_width: Some(1920),
                source_height: Some(1080),
                aspect_ratio: Some("16:9".to_string()),
            })
        );
    }

    #[test]
    fn apply_selected_full_crop_clears_existing_crop() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );
        root.update_selected_config(|config| {
            config.crop = Some(CropSettings {
                enabled: true,
                x: 100,
                y: 100,
                width: 1000,
                height: 600,
                source_width: Some(1920),
                source_height: Some(1080),
                aspect_ratio: None,
            });
            true
        });
        root.preview_ui.crop_mode = true;
        root.preview_ui.draft_crop = Some(full_crop_rect());

        let changed = root.apply_selected_crop();

        assert!(changed);
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.crop.as_ref()),
            None
        );
    }

    #[test]
    fn rotate_and_flip_preview_update_selected_config() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );

        assert!(root.rotate_selected_preview());
        assert!(root.toggle_selected_flip(FlipAxis::Horizontal));

        let config = &root.file_queue.file_by_id("video").unwrap().config;
        assert_eq!(config.rotation, "90");
        assert!(config.flip_horizontal);
        assert!(!config.flip_vertical);
    }

    #[test]
    fn preview_runtime_key_ignores_presentation_only_transform_changes() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                duration: Some("12.5".to_string()),
                ..SourceMetadata::default()
            },
        );
        let metadata_entry = root.source_metadata.entry_for("video");
        let initial_request = root
            .selected_preview_runtime_request(&metadata_entry)
            .expect("initial request");

        assert!(root.rotate_selected_preview());
        assert!(root.toggle_selected_flip(FlipAxis::Horizontal));
        root.preview_ui.crop_mode = true;
        root.preview_ui.draft_crop = Some(CropRect {
            x: 0.25,
            y: 0.25,
            width: 0.5,
            height: 0.5,
        });
        assert!(root.apply_selected_crop());
        let transformed_request = root
            .selected_preview_runtime_request(&metadata_entry)
            .expect("transformed request");

        assert_eq!(transformed_request.key, initial_request.key);
        assert_ne!(
            transformed_request.presentation,
            initial_request.presentation
        );
        assert_eq!(
            transformed_request.config.transform,
            PreviewTransform::default()
        );
        assert_eq!(transformed_request.config.crop, None);
        assert_eq!(
            transformed_request.presentation.crop,
            Some(EnginePreviewCrop {
                x: 270,
                y: 480,
                width: 540,
                height: 960,
            })
        );
    }

    #[test]
    fn apply_preview_crop_drag_updates_draft_without_persisting_config() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );
        root.preview_ui.crop_mode = true;
        root.preview_ui.draft_crop = Some(CropRect {
            x: 0.10,
            y: 0.10,
            width: 0.50,
            height: 0.50,
        });

        assert!(
            !root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.50, y: 0.50 },)
        );
        assert!(root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.60, y: 0.55 },));

        let draft = root.preview_ui.draft_crop.unwrap();
        assert!((draft.x - 0.20).abs() < 0.000_001);
        assert!((draft.y - 0.15).abs() < 0.000_001);
        assert_eq!(draft.width, 0.50);
        assert_eq!(draft.height, 0.50);
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.crop.as_ref()),
            None
        );
    }

    #[test]
    fn apply_preview_crop_drag_moves_visual_rect_after_side_rotation() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );

        assert!(root.rotate_selected_preview());
        assert!(root.toggle_selected_crop_mode());
        assert!(root.select_preview_crop_aspect("1:1"));

        let config = &root.file_queue.file_by_id("video").unwrap().config;
        let metadata_entry = root.source_metadata.entry_for("video");
        let crop_state = root.preview_crop_render_state(metadata_entry.metadata.as_ref(), config);
        let before = preview_crop_visual_rect(&crop_state);

        assert!(!root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.50, y: 0.50 }));
        assert!(root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.60, y: 0.50 }));

        let config = &root.file_queue.file_by_id("video").unwrap().config;
        let metadata_entry = root.source_metadata.entry_for("video");
        let crop_state = root.preview_crop_render_state(metadata_entry.metadata.as_ref(), config);
        let after = preview_crop_visual_rect(&crop_state);

        assert!((after.x - before.x - 0.10).abs() < 0.000_001);
        assert!((after.y - before.y).abs() < 0.000_001);
    }

    #[test]
    fn sync_preview_canvas_for_selection_resets_canvas_when_file_changes() {
        let mut root = FrameRoot::new();
        root.sync_preview_canvas_for_selection(Some("first"));
        root.preview_ui.canvas.current_zoom = 2.0;
        root.preview_ui.canvas.target_zoom = 3.0;
        root.preview_ui.canvas.current_pan_x = 0.5;
        root.preview_ui.canvas.target_pan_y = -0.5;
        root.preview_ui.canvas.auto_fit_pending = false;

        root.sync_preview_canvas_for_selection(Some("first"));

        assert_eq!(root.preview_ui.canvas.current_zoom, 2.0);

        root.sync_preview_canvas_for_selection(Some("second"));

        assert_eq!(root.preview_ui.canvas, PreviewCanvasState::default());
    }

    #[test]
    fn lerp_preview_canvas_value_moves_toward_target_without_overshooting() {
        let next = lerp_preview_canvas_value(1.0, 2.0);

        assert!(next > 1.0 && next < 2.0);
    }

    #[test]
    fn preview_canvas_transform_waits_for_all_axes_to_settle() {
        assert!(!preview_canvas_transform_settled(
            1.0, 1.000_01, 10.0, 10.005, -10.0, -10.005,
        ));
        assert!(preview_canvas_transform_settled(
            1.0,
            1.000_000_5,
            10.0,
            10.005,
            -10.0,
            -10.005,
        ));
    }

    #[test]
    fn tick_preview_canvas_animation_keeps_pan_moving_until_zoom_settles() {
        let mut root = FrameRoot::new();
        root.preview_ui.canvas.current_zoom = 1.0;
        root.preview_ui.canvas.target_zoom = 1.000_01;
        root.preview_ui.canvas.current_pan_x = 10.0;
        root.preview_ui.canvas.target_pan_x = 10.005;
        root.preview_ui.canvas.current_pan_y = -10.0;
        root.preview_ui.canvas.target_pan_y = -10.005;

        assert!(root.tick_preview_canvas_animation());
        assert!(root.preview_ui.canvas.current_zoom > 1.0);
        assert!(root.preview_ui.canvas.current_pan_x > 10.0);
        assert!(root.preview_ui.canvas.current_pan_y < -10.0);
    }

    #[test]
    fn apply_preview_overlay_drag_persists_selected_file_overlay_position() {
        let mut root = root_with_overlay();

        assert!(!root.apply_preview_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.50,
                y: 0.50,
                width: Some(0.20),
                height: Some(0.20),
            },
        ));
        assert!(root.apply_preview_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.60,
                y: 0.55,
                width: Some(0.20),
                height: Some(0.20),
            },
        ));

        let overlay = root
            .file_queue
            .file_by_id("video")
            .and_then(|file| file.config.overlay.as_ref())
            .unwrap();
        assert!((overlay.x - 0.60).abs() < 0.000_001);
        assert!((overlay.y - 0.55).abs() < 0.000_001);
        assert_eq!(overlay.anchor, "custom");
    }

    #[test]
    fn commit_preview_overlay_opacity_at_position_persists_slider_value() {
        let mut root = root_with_overlay();
        root.set_preview_overlay_opacity_slider_bounds(Bounds {
            origin: point(px(10.0), px(0.0)),
            size: size(px(100.0), px(30.0)),
        });

        assert!(root.commit_preview_overlay_opacity_at_position(point(px(60.0), px(0.0))));

        let overlay = root
            .file_queue
            .file_by_id("video")
            .and_then(|file| file.config.overlay.as_ref())
            .unwrap();
        assert!((overlay.opacity - 0.50).abs() < 0.000_001);
    }

    #[test]
    fn remove_selected_overlay_clears_selected_file_overlay_config() {
        let mut root = root_with_overlay();

        assert!(root.remove_selected_overlay());

        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.overlay.as_ref()),
            None
        );
        assert!(root.preview_ui.overlay.overlay().is_none());
    }

    fn root_with_overlay() -> FrameRoot {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
        root.source_metadata.mark_ready(
            "video".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );
        let overlay = PreviewOverlay {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.50,
            y: 0.50,
            width: 0.20,
            opacity: 1.0,
            anchor: "custom".to_string(),
        };
        root.preview_ui.overlay.sync_initial_overlay(Some(&overlay));
        root.preview_ui.overlay.set_overlay_mode(true, false);
        root.update_selected_config(|config| {
            config.overlay = Some(OverlaySettings {
                enabled: overlay.enabled,
                path: overlay.path.clone(),
                x: overlay.x,
                y: overlay.y,
                width: overlay.width,
                opacity: overlay.opacity,
                anchor: overlay.anchor.clone(),
            });
            true
        });
        root
    }
}

mod frame_window_options {
    use super::*;

    #[test]
    fn keeps_transparent_titlebar_without_positioning_native_controls() {
        let options = frame_window_options(Bounds::default());
        let titlebar = options
            .titlebar
            .as_ref()
            .expect("custom Frame controls still need a transparent native titlebar host");

        assert!(titlebar.appears_transparent);
        assert_eq!(titlebar.traffic_light_position, None);
    }

    #[test]
    fn preserves_original_minimum_window_size() {
        let options = frame_window_options(Bounds::default());

        assert_eq!(
            options.window_min_size,
            Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)))
        );
    }

    #[test]
    fn sets_the_frame_application_id() {
        let options = frame_window_options(Bounds::default());

        assert_eq!(options.app_id.as_deref(), Some(FRAME_APP_ID));
    }
}

mod visual_fixtures {
    use super::*;

    #[test]
    fn app_settings_fixture_opens_runtime_settings_sheet() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::AppSettings));

        assert!(root.settings_ui.is_open);
        assert_eq!(
            root.settings_ui.max_concurrency_draft,
            root.max_concurrency.to_string()
        );
    }

    #[test]
    fn preview_ready_fixture_seeds_selected_video_metadata() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));

        assert_eq!(root.active_view, ActiveView::Workspace);
        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.name.as_str()),
            Some("source_render.mov")
        );
        assert_eq!(
            root.selected_source_metadata()
                .map(|metadata| metadata.source_kind()),
            Some(SourceKind::Video)
        );
    }

    #[test]
    fn preview_crop_fixture_enters_crop_mode() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::PreviewCrop));

        assert!(root.preview_ui.crop_mode);
        assert!(root.preview_ui.draft_crop.is_some());
        assert_eq!(root.preview_ui.crop_aspect, "1:1");
    }

    #[test]
    fn workspace_empty_fixture_clears_workspace_state() {
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));

        root.apply_visual_fixture(Some(VisualFixture::WorkspaceEmpty));

        assert_eq!(root.active_view, ActiveView::Workspace);
        assert!(root.file_queue.files().is_empty());
        assert_eq!(
            root.selected_source_metadata_entry().status,
            MetadataStatus::Idle
        );
    }

    #[test]
    fn workspace_audio_fixture_seeds_selected_audio_source() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::WorkspaceAudio));

        assert_eq!(root.active_view, ActiveView::Workspace);
        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.name.as_str()),
            Some("source_mix.wav")
        );
        assert_eq!(
            root.selected_source_metadata()
                .map(|metadata| metadata.source_kind()),
            Some(SourceKind::Audio)
        );
    }

    #[test]
    fn workspace_image_fixture_seeds_selected_image_source() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::WorkspaceImage));

        assert_eq!(root.active_view, ActiveView::Workspace);
        assert_eq!(
            root.selected_source_metadata()
                .map(|metadata| metadata.source_kind()),
            Some(SourceKind::Image)
        );
    }

    #[test]
    fn settings_source_fixture_opens_source_tab_with_ready_metadata() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsSource));

        let metadata = root
            .selected_source_metadata()
            .expect("source fixture should seed ready metadata");
        assert_eq!(root.settings_ui.active_tab, SettingsTab::Source);
        assert_eq!(
            source_info_sections(&metadata)
                .iter()
                .map(|section| match section {
                    SourceInfoSection::Rows { title, .. }
                    | SourceInfoSection::Tracks { title, .. } => *title,
                })
                .collect::<Vec<_>>(),
            ["FILE INFORMATION", "VIDEO STREAM"]
        );
    }

    #[test]
    fn settings_output_fixture_opens_output_tab_with_output_name() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsOutput));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| file.output_name.as_str()),
            Some("source_render_review.mov")
        );
    }

    #[test]
    fn settings_audio_fixture_opens_audio_tab_with_tracks_and_controls() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsAudio));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Audio);
        assert_eq!(
            root.selected_source_metadata()
                .map(|metadata| metadata.audio_tracks.len()),
            Some(2)
        );
        assert_eq!(
            root.file_queue
                .selected_file()
                .map(|file| (file.config.audio_codec.as_str(), file.config.audio_volume)),
            Some(("mp3", 145))
        );
    }

    #[test]
    fn settings_metadata_fixture_opens_metadata_tab_with_source_tags() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsMetadata));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Metadata);
        assert_eq!(
            root.selected_source_metadata()
                .and_then(|metadata| metadata.tags)
                .and_then(|tags| tags.title),
            Some("Original Scene 24A".to_string())
        );
    }

    #[test]
    fn settings_video_fixture_opens_video_tab() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsVideo));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Video);
        assert_eq!(
            root.file_queue
                .selected_file()
                .and_then(|file| file.config.custom_width.as_deref()),
            Some("1920")
        );
    }

    #[test]
    fn settings_images_fixture_opens_images_tab_for_image_source() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsImages));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Images);
        assert_eq!(
            root.selected_source_metadata()
                .map(|metadata| metadata.source_kind()),
            Some(SourceKind::Image)
        );
    }

    #[test]
    fn settings_subtitles_fixture_opens_subtitles_tab_with_tracks() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsSubtitles));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Subtitles);
        assert_eq!(
            root.selected_source_metadata()
                .map(|metadata| metadata.subtitle_tracks.len()),
            Some(2)
        );
    }

    #[test]
    fn settings_subtitles_popover_fixture_opens_font_color_picker() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsSubtitlesPopover));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Subtitles);
        assert_eq!(
            root.subtitle_ui.popover,
            Some(SettingsSubtitlePopover::FontColor)
        );
        assert_eq!(root.subtitle_ui.font_color_draft, "#FFD166");
    }

    #[test]
    fn settings_presets_fixture_opens_presets_tab_with_custom_draft() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsPresets));

        assert_eq!(root.settings_ui.active_tab, SettingsTab::Presets);
        assert_eq!(root.settings_ui.preset_name_draft, "Client Review MP4");
        assert!(
            root.presets
                .iter()
                .any(|preset| preset.id == "custom-review")
        );
    }
}

mod button_state_colors {
    use super::*;

    #[test]
    fn default_button_hover_matches_original_frame_gray_400_90() {
        let colors = button_colors(ButtonVariant::Default, false, true);

        assert_eq!(
            colors.hover_background,
            theme::FRAME_GRAY_400.with_alpha(0.18)
        );
        assert_eq!(colors.active_background, colors.hover_background);
    }

    #[test]
    fn secondary_button_hover_matches_original_frame_gray_200() {
        let colors = button_colors(ButtonVariant::Secondary, false, true);

        assert_eq!(colors.hover_background, theme::FRAME_GRAY_200);
    }

    #[test]
    fn disabled_default_button_uses_original_half_alpha_background() {
        let colors = button_colors(ButtonVariant::Default, false, false);

        assert_eq!(colors.background, theme::FRAME_GRAY_400.with_alpha(0.10));
        assert_eq!(colors.opacity, 1.0);
    }

    #[test]
    fn disabled_secondary_button_keeps_original_whole_button_opacity() {
        let colors = button_colors(ButtonVariant::Secondary, false, false);

        assert_eq!(colors.background, theme::FRAME_GRAY_100);
        assert_eq!(colors.opacity, 0.5);
    }

    #[test]
    fn ghost_button_matches_original_transparent_icon_button_states() {
        let colors = button_colors(ButtonVariant::Ghost, false, true);

        assert_eq!(colors.background, theme::TRANSPARENT);
        assert_eq!(colors.hover_background, theme::FRAME_GRAY_100);
        assert_eq!(colors.active_background, theme::FRAME_GRAY_200);
        assert_eq!(colors.foreground, theme::FRAME_GRAY_600);
        assert_eq!(colors.hover_foreground, theme::FOREGROUND);
    }
}

mod preview_shell {
    use super::*;

    fn empty_encoders() -> &'static AvailableEncoders {
        static ENCODERS: AvailableEncoders = AvailableEncoders {
            h264_videotoolbox: false,
            h264_nvenc: false,
            hevc_videotoolbox: false,
            hevc_nvenc: false,
            av1_nvenc: false,
            libfdk_aac: false,
            libmp3lame: false,
        };
        &ENCODERS
    }

    fn settings_state<'a>(
        config: &'a ConversionConfig,
        metadata: Option<&'a SourceMetadata>,
        status: MetadataStatus,
    ) -> SettingsRenderState<'a> {
        let subtitle_font_select_scroll_handle = Box::leak(Box::new(ScrollHandle::new()));
        let subtitle_font_size_select_scroll_handle = Box::leak(Box::new(ScrollHandle::new()));

        SettingsRenderState {
            active_tab: SettingsTab::Source,
            config,
            metadata,
            metadata_status: status,
            metadata_error: None,
            settings_disabled: false,
            output_name: "",
            output_name_focus: None,
            audio_bitrate_focus: None,
            video_width_focus: None,
            video_height_focus: None,
            video_bitrate_focus: None,
            gif_loop_focus: None,
            metadata_focuses: SettingsMetadataInputFocuses {
                title: None,
                artist: None,
                album: None,
                genre: None,
                date: None,
                comment: None,
            },
            subtitle_color_focuses: SettingsSubtitleColorInputFocuses {
                font: None,
                outline: None,
            },
            subtitle_popover: None,
            subtitle_rendered_popover: None,
            subtitle_font_select_scroll_handle,
            subtitle_font_size_select_scroll_handle,
            subtitle_font_color_draft: "",
            subtitle_outline_color_draft: "",
            subtitle_font_color_hsv_draft: hex_to_subtitle_hsv(DEFAULT_SUBTITLE_FONT_COLOR),
            subtitle_outline_color_hsv_draft: hex_to_subtitle_hsv(DEFAULT_SUBTITLE_OUTLINE_COLOR),
            preset_name: "",
            preset_name_focus: None,
            presets: &[],
            preset_notice: None,
            subtitle_fonts: &[],
            available_encoders: empty_encoders(),
        }
    }

    fn crop_state() -> PreviewCropRenderState {
        PreviewCropRenderState {
            crop_mode: false,
            draft_crop: None,
            applied_crop: None,
            crop_aspect: "free".to_string(),
            has_crop_dimensions: false,
            rotation: "0".to_string(),
            flip_horizontal: false,
            flip_vertical: false,
        }
    }

    #[test]
    fn ready_video_metadata_populates_timeline_labels() {
        let config = ConversionConfig::default();
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.4".to_string()),
            ..SourceMetadata::default()
        };
        let file = FileItem::from_path("video", "/tmp/render.mov", 1024);

        let state = preview_shell_state(
            Some(&file),
            settings_state(&config, Some(&metadata), MetadataStatus::Ready),
            crop_state(),
            PreviewOverlayRenderState::empty(),
            PreviewCanvasRenderState::default(),
            preview_playback_state(PreviewMediaKind::Video, 90.4, None, None),
            None,
            None,
        );
        let labels = preview_timeline_labels(&state);

        assert_eq!(state.availability.media_kind, PreviewMediaKind::Video);
        assert!(preview_trim_enabled(&state));
        assert_eq!(labels.start, "00:00:00.000");
        assert_eq!(labels.end, "00:01:30.400");
        assert_eq!(labels.duration, "00:01:30.400");
    }

    #[test]
    fn ready_video_metadata_uses_configured_trim_bounds() {
        let config = ConversionConfig {
            start_time: Some("00:00:05.000".to_string()),
            end_time: Some("00:00:30.250".to_string()),
            ..ConversionConfig::default()
        };
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.4".to_string()),
            ..SourceMetadata::default()
        };
        let file = FileItem::from_path("video", "/tmp/render.mov", 1024);

        let state = preview_shell_state(
            Some(&file),
            settings_state(&config, Some(&metadata), MetadataStatus::Ready),
            crop_state(),
            PreviewOverlayRenderState::empty(),
            PreviewCanvasRenderState::default(),
            preview_playback_state(
                PreviewMediaKind::Video,
                90.4,
                config.start_time.as_deref(),
                config.end_time.as_deref(),
            ),
            None,
            None,
        );
        let labels = preview_timeline_labels(&state);

        assert_eq!(labels.start, "00:00:05.000");
        assert_eq!(labels.end, "00:00:30.250");
        assert_eq!(labels.duration, "00:00:25.250");
    }

    #[test]
    fn image_metadata_uses_placeholder_timeline_labels() {
        let config = ConversionConfig::default();
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Image),
            duration: Some("10.0".to_string()),
            ..SourceMetadata::default()
        };
        let file = FileItem::from_path("image", "/tmp/still.png", 1024);

        let state = preview_shell_state(
            Some(&file),
            settings_state(&config, Some(&metadata), MetadataStatus::Ready),
            crop_state(),
            PreviewOverlayRenderState::empty(),
            PreviewCanvasRenderState::default(),
            PreviewPlaybackState::new(false),
            None,
            None,
        );
        let labels = preview_timeline_labels(&state);

        assert_eq!(state.availability.media_kind, PreviewMediaKind::Image);
        assert!(state.availability.trim_disabled);
        assert_eq!(labels.start, "--:--:--.---");
        assert_eq!(labels.end, "--:--:--.---");
        assert_eq!(labels.duration, "--:--:--.---");
    }

    #[test]
    fn audio_metadata_hides_visual_controls() {
        let config = ConversionConfig::default();
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Audio),
            duration: Some("00:00:12.500".to_string()),
            ..SourceMetadata::default()
        };

        let state = preview_shell_state(
            None,
            settings_state(&config, Some(&metadata), MetadataStatus::Ready),
            crop_state(),
            PreviewOverlayRenderState::empty(),
            PreviewCanvasRenderState::default(),
            PreviewPlaybackState::new(false),
            None,
            None,
        );

        assert_eq!(state.availability.media_kind, PreviewMediaKind::Audio);
        assert!(state.availability.hide_visual_controls);
        assert!(!preview_visual_controls_visible(&state));
        assert_eq!(preview_duration_seconds(Some(&metadata)), 12.5);
    }

    #[test]
    fn loading_metadata_keeps_preview_unknown() {
        let config = ConversionConfig::default();
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("90.0".to_string()),
            ..SourceMetadata::default()
        };

        let state = preview_shell_state(
            None,
            settings_state(&config, Some(&metadata), MetadataStatus::Loading),
            crop_state(),
            PreviewOverlayRenderState::empty(),
            PreviewCanvasRenderState::default(),
            PreviewPlaybackState::new(false),
            None,
            None,
        );

        assert_eq!(state.availability.media_kind, PreviewMediaKind::Unknown);
        assert!(state.availability.trim_disabled);
    }

    #[test]
    fn centered_offset_never_returns_negative_values() {
        assert_eq!(centered_offset(30.0, 6.0), 12.0);
        assert_eq!(centered_offset(6.0, 30.0), 0.0);
    }

    #[test]
    fn timeline_fraction_from_percent_clamps_to_track_range() {
        assert_eq!(timeline_fraction_from_percent(-25.0), 0.0);
        assert_eq!(timeline_fraction_from_percent(50.0), 0.5);
        assert_eq!(timeline_fraction_from_percent(125.0), 1.0);
    }

    #[test]
    fn timeline_slider_percent_from_bounds_clamps_pointer_to_track() {
        let bounds = Bounds {
            origin: point(px(10.0), px(0.0)),
            size: size(px(100.0), px(30.0)),
        };

        assert_eq!(
            timeline_slider_percent_from_bounds(point(px(60.0), px(0.0)), bounds),
            0.5
        );
        assert_eq!(
            timeline_slider_percent_from_bounds(point(px(-10.0), px(0.0)), bounds),
            0.0
        );
        assert_eq!(
            timeline_slider_percent_from_bounds(point(px(140.0), px(0.0)), bounds),
            1.0
        );
    }

    #[test]
    fn preview_canvas_layout_metrics_preserve_media_aspect_when_zooming() {
        let metrics = preview_canvas_layout_metrics(1000.0, 500.0, 1920.0, 1080.0, 1.18, 0.0, 0.0)
            .expect("metrics");

        assert!(((metrics.width / metrics.height) - (16.0 / 9.0)).abs() < 0.000_001);
        assert!(metrics.width > 1000.0);
        assert!((metrics.top - -45.0).abs() < 0.000_001);
    }

    #[test]
    fn preview_canvas_pan_limits_allow_original_overscroll_window() {
        let (max_x, max_y) =
            preview_canvas_pan_limits(1000.0, 500.0, 1920.0, 1080.0, 0.25).expect("limits");

        assert_eq!(max_x, 1000.0);
        assert_eq!(max_y, 500.0);
    }

    #[test]
    fn preview_canvas_initial_zoom_starts_just_below_object_cover() {
        let zoom =
            preview_canvas_initial_zoom(1000.0, 500.0, 1920.0, 1080.0).expect("initial zoom");

        assert!((zoom - 1.08).abs() < 0.000_001);
    }

    #[test]
    fn subtitle_hsv_helpers_round_trip_primary_colors() {
        assert_eq!(subtitle_hsv_to_hex(0.0, 1.0, 1.0), "#ff0000");
        assert_eq!(subtitle_hsv_to_hex(120.0, 1.0, 1.0), "#00ff00");

        let hsv = hex_to_subtitle_hsv("#00f");
        assert_eq!(hsv.h, 240.0);
        assert_eq!(hsv.s, 1.0);
        assert_eq!(hsv.v, 1.0);
    }
}

mod visual_contract {
    use super::*;

    #[test]
    fn file_list_controls_match_design_sizes() {
        assert_eq!(components::FRAME_ICON_BUTTON_SM_SIZE, 24.0);
        assert_eq!(components::FRAME_ICON_SM_SIZE, 16.0);
        assert_eq!(components::FRAME_CHECKBOX_SIZE, 14.0);
        assert_eq!(components::FRAME_CHECK_ICON_SIZE, 12.0);
    }

    #[test]
    fn max_concurrency_runtime_settings_has_no_stepper_actions() {
        let mut root = FrameRoot::new();
        root.settings_ui.max_concurrency_draft = "1".to_string();
        root.text_input_runtime_mut(FrameTextInputKind::MaxConcurrency)
            .selected_range = 1..1;

        assert!(!root.replace_text_input_range(
            FrameTextInputKind::MaxConcurrency,
            None,
            "-",
            None,
            false,
        ));
        assert_eq!(root.settings_ui.max_concurrency_draft, "1");
    }

    #[test]
    fn audio_slider_helpers_map_values_to_original_range() {
        assert_eq!(settings_panel::range_fraction(100, 0, 200), 0.5);
        assert_eq!(settings_panel::range_value_from_fraction(0.5, 0, 200), 100);
    }

    #[test]
    fn preview_left_toolbar_centering_uses_full_stack_height() {
        assert_eq!(preview_panel::preview_toolbar_height(), 190.0);
        assert_eq!(preview_panel::preview_toolbar_center_margin(), -95.0);
    }

    #[test]
    fn preview_crop_handles_use_screen_space_cursors() {
        assert_eq!(
            preview_panel::crop_handle_screen_cursor(DragHandle::NorthEast),
            "nesw-resize"
        );
        assert_eq!(
            preview_panel::crop_handle_screen_cursor(DragHandle::NorthWest),
            "nwse-resize"
        );
    }
}

fn test_settings_path() -> PathBuf {
    let sequence = TEST_SETTINGS_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();

    std::env::temp_dir()
        .join("frame-root-persistence-tests")
        .join(format!("{}-{millis}-{sequence}", std::process::id()))
        .join("settings.json")
}
