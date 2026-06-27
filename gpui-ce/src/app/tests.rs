use super::input::{should_capture_text_input_drag, should_handle_text_input};
use super::preview_panel::{
    centered_offset, preview_shell_state, preview_timeline_labels, preview_trim_enabled,
    preview_visual_controls_visible, timeline_fraction_from_percent,
    timeline_slider_percent_from_bounds,
};
use super::primitives::{ButtonVariant, button_colors};
use super::settings_panel::{hex_to_subtitle_hsv, subtitle_hsv_to_hex};
use super::*;

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
    fn apply_max_concurrency_draft_updates_live_controller_limit() {
        let mut root = FrameRoot::new();
        root.max_concurrency_draft = "4".to_string();

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
    fn apply_max_concurrency_draft_rejects_zero() {
        let mut root = FrameRoot::new();
        root.max_concurrency_draft = "0".to_string();

        assert!(!root.apply_max_concurrency_draft());

        assert_eq!(root.max_concurrency, DEFAULT_MAX_CONCURRENCY);
        assert!(root.max_concurrency_error.is_some());
    }

    #[test]
    fn max_concurrency_input_inserts_digits_at_selection() {
        let mut root = FrameRoot::new();
        root.max_concurrency_draft = "12".to_string();
        root.max_concurrency_input.selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::MaxConcurrency,
            None,
            "9",
            None,
            false,
        ));

        assert_eq!(root.max_concurrency_draft, "192");
        assert_eq!(root.max_concurrency_input.selected_range, 2..2);
    }

    #[test]
    fn max_concurrency_input_deletes_selected_range() {
        let mut root = FrameRoot::new();
        root.max_concurrency_draft = "12".to_string();
        root.max_concurrency_input.selected_range = 1..2;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::MaxConcurrency,
            None,
            "",
            None,
            false,
        ));

        assert_eq!(root.max_concurrency_draft, "1");
        assert_eq!(root.max_concurrency_input.selected_range, 1..1);
    }

    #[test]
    fn max_concurrency_apply_updates_live_controller_limit() {
        let mut root = FrameRoot::new();
        root.max_concurrency_draft = "4".to_string();

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
        root.output_name_input.selected_range = len..len;

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
        root.output_name_input.selected_range = 0..1;

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
        root.metadata_title_input.selected_range = 6..6;

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
        root.preset_name_draft = "Review".to_string();
        root.preset_name_input.selected_range = 6..6;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::PresetName,
            None,
            " MP4",
            None,
            false,
        ));

        assert_eq!(root.preset_name_draft, "Review MP4");
    }

    #[test]
    fn subtitle_font_color_hex_input_expands_short_hex_and_updates_config() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.subtitle_font_color_draft = "#".to_string();
        root.subtitle_font_color_input.selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::SubtitleFontColorHex,
            None,
            "abc",
            None,
            false,
        ));

        assert_eq!(root.subtitle_font_color_draft, "#AABBCC");
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
        root.subtitle_outline_color_draft = "#".to_string();
        root.subtitle_outline_color_input.selected_range = 1..1;

        assert!(root.replace_text_input_range(
            FrameTextInputKind::SubtitleOutlineColorHex,
            None,
            "f",
            None,
            false,
        ));

        assert_eq!(root.subtitle_outline_color_draft, "#F");
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

        assert_eq!(root.subtitle_font_color_draft, "#FFFF00");
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

        assert_eq!(root.subtitle_font_color_draft, "#FFFFFF");
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
        assert!(root.commit_subtitle_color_drag_at_position(drag, point(px(110.0), px(20.0))));

        assert_eq!(root.subtitle_font_color_draft, "#8000FF");
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
            root.settings_subtitle_popover,
            Some(SettingsSubtitlePopover::FontName)
        );

        root.toggle_subtitle_popover(SettingsSubtitlePopover::FontSize);
        assert_eq!(
            root.settings_subtitle_popover,
            Some(SettingsSubtitlePopover::FontSize)
        );

        root.toggle_subtitle_popover(SettingsSubtitlePopover::FontSize);
        assert_eq!(root.settings_subtitle_popover, None);
    }

    #[test]
    fn save_preset_from_draft_adds_custom_preset() {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        root.preset_name_draft = "Review MP4".to_string();

        assert!(root.save_preset_from_draft());

        assert!(
            root.presets
                .iter()
                .any(|preset| preset.name == "Review MP4")
        );
        assert!(root.preset_name_draft.is_empty());
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
        root.audio_bitrate_input.selected_range = 1..1;

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
        assert_eq!(root.audio_bitrate_input.selected_range, 2..2);
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
        root.audio_bitrate_input.selected_range = 3..3;

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
    fn apply_selected_trim_drag_updates_selected_file_start_time() {
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

        let changed = root.apply_selected_trim_drag(TimelineDragTarget::Start, 0.25);

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
    fn apply_selected_trim_drag_preserves_gap_when_end_moves_before_start() {
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

        let changed = root.apply_selected_trim_drag(TimelineDragTarget::End, 0.10);

        assert!(changed);
        assert_eq!(
            root.file_queue
                .file_by_id("video")
                .and_then(|file| file.config.end_time.as_deref()),
            Some("00:00:21.000")
        );
    }

    #[test]
    fn apply_selected_trim_drag_ignores_image_sources() {
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

        let changed = root.apply_selected_trim_drag(TimelineDragTarget::Start, 0.25);

        assert!(!changed);
        assert_eq!(
            root.file_queue
                .file_by_id("image")
                .and_then(|file| file.config.start_time.as_deref()),
            None
        );
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
        assert!(root.preview_crop_mode);
        assert_eq!(root.preview_draft_crop, Some(default_crop_rect()));
        assert_eq!(root.preview_crop_aspect, "free");
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
        root.preview_crop_mode = true;
        root.preview_draft_crop = Some(CropRect {
            x: 0.25,
            y: 0.25,
            width: 0.5,
            height: 0.5,
        });
        root.preview_crop_aspect = "16:9".to_string();

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
        root.preview_crop_mode = true;
        root.preview_draft_crop = Some(full_crop_rect());

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
        root.preview_crop_mode = true;
        root.preview_draft_crop = Some(CropRect {
            x: 0.10,
            y: 0.10,
            width: 0.50,
            height: 0.50,
        });

        assert!(
            !root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.50, y: 0.50 },)
        );
        assert!(root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.60, y: 0.55 },));

        let draft = root.preview_draft_crop.unwrap();
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
}

mod visual_fixtures {
    use super::*;

    #[test]
    fn app_settings_fixture_opens_runtime_settings_sheet() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::AppSettings));

        assert!(root.is_settings_open);
        assert_eq!(root.max_concurrency_draft, root.max_concurrency.to_string());
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

        assert!(root.preview_crop_mode);
        assert!(root.preview_draft_crop.is_some());
        assert_eq!(root.preview_crop_aspect, "1:1");
    }

    #[test]
    fn settings_metadata_fixture_opens_metadata_tab_with_source_tags() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsMetadata));

        assert_eq!(root.settings_active_tab, SettingsTab::Metadata);
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

        assert_eq!(root.settings_active_tab, SettingsTab::Video);
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

        assert_eq!(root.settings_active_tab, SettingsTab::Images);
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

        assert_eq!(root.settings_active_tab, SettingsTab::Subtitles);
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

        assert_eq!(root.settings_active_tab, SettingsTab::Subtitles);
        assert_eq!(
            root.settings_subtitle_popover,
            Some(SettingsSubtitlePopover::FontColor)
        );
        assert_eq!(root.subtitle_font_color_draft, "#FFD166");
    }

    #[test]
    fn settings_presets_fixture_opens_presets_tab_with_custom_draft() {
        let mut root = FrameRoot::new();

        root.apply_visual_fixture(Some(VisualFixture::SettingsPresets));

        assert_eq!(root.settings_active_tab, SettingsTab::Presets);
        assert_eq!(root.preset_name_draft, "Client Review MP4");
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
            ml_upscale: false,
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
            subtitle_font_color_draft: "",
            subtitle_outline_color_draft: "",
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
    fn file_list_controls_match_original_svelte_sizes() {
        assert_eq!(components::FRAME_ICON_BUTTON_SM_SIZE, 24.0);
        assert_eq!(components::FRAME_ICON_SM_SIZE, 16.0);
        assert_eq!(components::FRAME_CHECKBOX_SIZE, 14.0);
        assert_eq!(components::FRAME_CHECK_ICON_SIZE, 12.0);
    }

    #[test]
    fn max_concurrency_runtime_settings_has_no_stepper_actions() {
        let mut root = FrameRoot::new();
        root.max_concurrency_draft = "1".to_string();
        root.max_concurrency_input.selected_range = 1..1;

        assert!(!root.replace_text_input_range(
            FrameTextInputKind::MaxConcurrency,
            None,
            "-",
            None,
            false,
        ));
        assert_eq!(root.max_concurrency_draft, "1");
    }

    #[test]
    fn audio_slider_helpers_map_values_to_original_range() {
        assert_eq!(settings_panel::range_fraction(100, 0, 200), 0.5);
        assert_eq!(settings_panel::range_value_from_fraction(0.5, 0, 200), 100);
    }

    #[test]
    fn preview_left_toolbar_centering_uses_full_stack_height() {
        assert_eq!(preview_panel::preview_toolbar_height(), 199.0);
        assert_eq!(preview_panel::preview_toolbar_center_margin(), -99.5);
    }
}
