use super::*;

fn sample_file(id: &str, path: &str, size_bytes: u64) -> FileItem {
    FileItem::from_path(id, path, size_bytes)
}

mod file_status {
    use super::*;

    #[test]
    fn locks_settings_for_current_original_locked_statuses() {
        assert!(FileStatus::Converting.locks_settings());
        assert!(FileStatus::Queued.locks_settings());
        assert!(FileStatus::Completed.locks_settings());
    }

    #[test]
    fn keeps_paused_files_editable_like_original_ui() {
        assert!(!FileStatus::Paused.locks_settings());
    }

    #[test]
    fn completed_files_are_not_actionable_for_conversion() {
        assert!(!FileStatus::Completed.is_actionable_for_conversion());
    }

    #[test]
    fn only_idle_and_error_files_are_actionable_for_conversion() {
        assert!(FileStatus::Idle.is_actionable_for_conversion());
        assert!(FileStatus::Error.is_actionable_for_conversion());
        assert!(!FileStatus::Queued.is_actionable_for_conversion());
        assert!(!FileStatus::Converting.is_actionable_for_conversion());
        assert!(!FileStatus::Paused.is_actionable_for_conversion());
    }

    #[test]
    fn converting_files_are_not_removed_directly_from_list() {
        assert!(!FileStatus::Converting.can_be_removed_from_list());
    }

    #[test]
    fn queued_and_paused_files_can_be_removed_through_cancel_control() {
        assert!(FileStatus::Queued.can_be_removed_from_list());
        assert!(!FileStatus::Converting.can_be_removed_from_list());
        assert!(FileStatus::Paused.can_be_removed_from_list());
        assert!(FileStatus::Idle.can_be_removed_from_list());
        assert!(FileStatus::Completed.can_be_removed_from_list());
        assert!(FileStatus::Error.can_be_removed_from_list());
    }
}

mod file_item {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn from_path_derives_name_from_unix_path() {
        let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

        assert_eq!(file.name, "video.mp4");
    }

    #[test]
    fn from_path_derives_name_from_windows_path() {
        let file = FileItem::from_path("1", r"C:\Users\hex\video.mp4", 10);

        assert_eq!(file.name, "video.mp4");
    }

    #[test]
    fn from_path_initializes_conversion_selection_like_original_add_flow() {
        let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

        assert!(file.is_selected_for_conversion);
    }

    #[test]
    fn from_path_initializes_per_file_conversion_config() {
        let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

        assert_eq!(file.config, ConversionConfig::default());
    }

    #[test]
    fn from_os_path_reads_size_from_existing_file() {
        let path = temp_file_path("size-source.mp4");
        std::fs::write(&path, [1_u8, 2, 3]).expect("test file should be written");

        let file = FileItem::from_os_path("1", &path);

        std::fs::remove_file(&path).expect("test file should be removed");
        assert_eq!(file.size_bytes, 3);
    }

    #[test]
    fn from_os_path_uses_zero_size_when_metadata_is_unavailable() {
        let path = temp_file_path("missing-source.mp4");

        let file = FileItem::from_os_path("1", &path);

        assert_eq!(file.size_bytes, 0);
    }

    #[test]
    fn converting_row_state_uses_progress_percent() {
        let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
        file.status = FileStatus::Converting;
        file.progress_percent = 42;

        assert_eq!(file.row_state_label(), "42%");
        assert_eq!(file.row_state_tone(), FileStateTone::Amber);
    }

    #[test]
    fn completed_row_state_matches_ready_label() {
        let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
        file.status = FileStatus::Completed;

        assert_eq!(file.row_state_label(), "ready");
        assert_eq!(file.row_state_tone(), FileStateTone::Blue);
    }

    #[test]
    fn error_row_state_uses_red_tone() {
        let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
        file.status = FileStatus::Error;

        assert_eq!(file.row_state_label(), "error");
        assert_eq!(file.row_state_tone(), FileStateTone::Red);
    }

    #[test]
    fn converting_row_can_pause_but_not_delete_directly() {
        let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
        file.status = FileStatus::Converting;

        assert_eq!(
            file.row_actions(),
            RowActionAvailability {
                can_pause: true,
                can_resume: false,
                can_delete: false,
            }
        );
    }

    #[test]
    fn paused_row_can_resume_and_delete_through_cancel_control() {
        let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
        file.status = FileStatus::Paused;

        assert_eq!(
            file.row_actions(),
            RowActionAvailability {
                can_pause: false,
                can_resume: true,
                can_delete: true,
            }
        );
    }

    fn temp_file_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("frame-app-{}-{name}", std::process::id()))
    }
}

mod derive_output_name {
    use super::*;

    #[test]
    fn appends_converted_to_file_stem() {
        assert_eq!(derive_output_name("clip.mp4"), "clip_converted");
    }

    #[test]
    fn removes_only_final_extension() {
        assert_eq!(
            derive_output_name("archive.tar.gz"),
            "archive.tar_converted"
        );
    }

    #[test]
    fn falls_back_when_hidden_file_stem_is_empty() {
        assert_eq!(derive_output_name(".gitignore"), "output_converted");
    }
}

mod original_format_from_name {
    use super::*;

    #[test]
    fn uses_final_extension() {
        assert_eq!(original_format_from_name("archive.tar.gz"), "gz");
    }

    #[test]
    fn falls_back_when_trailing_dot_has_no_extension() {
        assert_eq!(original_format_from_name("clip."), "unknown");
    }
}

mod format_file_size {
    use super::*;

    #[test]
    fn returns_zero_bytes_label() {
        assert_eq!(format_file_size(0), "0 B");
    }

    #[test]
    fn trims_trailing_decimal_zeroes_like_javascript_parse_float() {
        assert_eq!(format_file_size(1536), "1.5 KB");
    }

    #[test]
    fn formats_megabytes_without_unneeded_decimals() {
        assert_eq!(format_file_size(1024 * 1024), "1 MB");
    }
}

mod file_queue {
    use super::*;

    #[test]
    fn add_file_selects_first_file_when_selection_is_empty() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));

        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn add_file_preserves_existing_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 1));

        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn selected_file_mut_updates_selected_file_only() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 1));
        queue.select_existing_file("second");

        queue
            .selected_file_mut()
            .expect("selected file should exist")
            .config
            .container = "webm".to_string();

        assert_eq!(
            queue
                .file_by_id("first")
                .map(|file| file.config.container.as_str()),
            Some("mp4")
        );
        assert_eq!(
            queue
                .file_by_id("second")
                .map(|file| file.config.container.as_str()),
            Some("webm")
        );
    }

    #[test]
    fn add_files_returns_added_count() {
        let mut queue = FileQueue::new();

        let added_count = queue.add_files([
            sample_file("first", "/tmp/one.mp4", 1),
            sample_file("second", "/tmp/two.mp4", 1),
        ]);

        assert_eq!(added_count, 2);
    }

    #[test]
    fn add_files_selects_first_import_when_queue_was_empty() {
        let mut queue = FileQueue::new();

        queue.add_files([
            sample_file("first", "/tmp/one.mp4", 1),
            sample_file("second", "/tmp/two.mp4", 1),
        ]);

        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn add_files_preserves_existing_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("existing", "/tmp/existing.mp4", 1));

        queue.add_files([
            sample_file("first", "/tmp/one.mp4", 1),
            sample_file("second", "/tmp/two.mp4", 1),
        ]);

        assert_eq!(queue.selected_file_id(), Some("existing"));
    }

    #[test]
    fn remove_file_selects_next_file_when_selected_file_is_removed() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 1));
        queue.remove_file("first");

        assert_eq!(queue.selected_file_id(), Some("second"));
    }

    #[test]
    fn remove_file_selects_previous_file_when_removed_file_was_last() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 1));
        queue.select_existing_file("second");
        queue.remove_file("second");

        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn remove_file_clears_selection_when_queue_becomes_empty() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.remove_file("first");

        assert_eq!(queue.selected_file_id(), None);
    }

    #[test]
    fn select_next_file_moves_selection_forward() {
        let mut queue = FileQueue::new();
        queue.add_files([
            sample_file("first", "/tmp/one.mp4", 1),
            sample_file("second", "/tmp/two.mp4", 1),
        ]);

        assert!(queue.select_next_file());
        assert_eq!(queue.selected_file_id(), Some("second"));
    }

    #[test]
    fn select_previous_file_moves_selection_backward() {
        let mut queue = FileQueue::new();
        queue.add_files([
            sample_file("first", "/tmp/one.mp4", 1),
            sample_file("second", "/tmp/two.mp4", 1),
        ]);
        queue.select_existing_file("second");

        assert!(queue.select_previous_file());
        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn remove_interactive_file_keeps_converting_file_in_queue() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.update_status("first", FileStatus::Converting, 20);

        assert!(queue.remove_interactive_file("first").is_none());
        assert_eq!(queue.files().len(), 1);
    }

    #[test]
    fn remove_interactive_file_allows_paused_file_after_cancel_control_exists() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.update_status("first", FileStatus::Paused, 20);

        assert!(queue.remove_interactive_file("first").is_some());
        assert!(queue.files().is_empty());
    }

    #[test]
    fn remove_interactive_file_allows_queued_file_after_cancel_control_exists() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.update_status("first", FileStatus::Queued, 0);

        assert!(queue.remove_interactive_file("first").is_some());
        assert!(queue.files().is_empty());
    }

    #[test]
    fn select_existing_file_ignores_unknown_ids() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));

        assert!(!queue.select_existing_file("missing"));
        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn select_existing_file_updates_selection_for_known_id() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 1));

        assert!(queue.select_existing_file("second"));
        assert_eq!(queue.selected_file_id(), Some("second"));
    }

    #[test]
    fn total_size_bytes_sums_all_files() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));

        assert_eq!(queue.total_size_bytes(), 25);
    }

    #[test]
    fn selected_count_counts_batch_selected_files() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
        queue.toggle_batch("second", false);

        assert_eq!(queue.selected_count(), 1);
    }

    #[test]
    fn toggle_batch_selection_inverts_single_file_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert_eq!(queue.toggle_batch_selection("first"), Some(false));
        assert_eq!(queue.selected_count(), 0);
    }

    #[test]
    fn toggle_batch_selection_returns_none_for_unknown_file() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert_eq!(queue.toggle_batch_selection("missing"), None);
    }

    #[test]
    fn has_actionable_files_ignores_completed_files() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Completed, 100);

        assert!(!queue.has_actionable_files());
    }

    #[test]
    fn has_actionable_files_ignores_processing_files() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Converting, 30);

        assert!(!queue.has_actionable_files());
    }

    #[test]
    fn all_checked_is_false_for_empty_queue() {
        let queue = FileQueue::new();

        assert!(!queue.all_checked());
    }

    #[test]
    fn is_indeterminate_matches_partial_batch_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
        queue.toggle_batch("second", false);

        assert!(queue.is_indeterminate());
    }

    #[test]
    fn batch_selection_state_is_disabled_for_empty_queue() {
        let queue = FileQueue::new();

        assert_eq!(
            queue.batch_selection_state(),
            BatchSelectionState {
                is_checked: false,
                is_indeterminate: false,
                is_enabled: false,
            }
        );
    }

    #[test]
    fn batch_selection_state_reports_all_files_checked() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));

        assert_eq!(
            queue.batch_selection_state(),
            BatchSelectionState {
                is_checked: true,
                is_indeterminate: false,
                is_enabled: true,
            }
        );
    }

    #[test]
    fn batch_selection_state_reports_partial_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
        queue.toggle_batch("second", false);

        assert_eq!(
            queue.batch_selection_state(),
            BatchSelectionState {
                is_checked: false,
                is_indeterminate: true,
                is_enabled: true,
            }
        );
    }

    #[test]
    fn toggle_all_batch_selection_selects_all_from_partial_state() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
        queue.toggle_batch("second", false);

        assert!(queue.toggle_all_batch_selection());
        assert!(queue.all_checked());
    }

    #[test]
    fn toggle_all_batch_selection_unchecks_all_when_all_are_checked() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 15));

        assert!(!queue.toggle_all_batch_selection());
        assert_eq!(queue.selected_count(), 0);
    }

    #[test]
    fn selected_file_locked_uses_selected_file_status() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Queued, 0);

        assert!(queue.selected_file_locked());
    }

    #[test]
    fn selected_file_locked_keeps_paused_file_editable_like_original_ui() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Paused, 30);

        assert!(!queue.selected_file_locked());
    }

    #[test]
    fn selected_file_locked_keeps_idle_file_editable() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert!(!queue.selected_file_locked());
    }

    #[test]
    fn file_status_locks_settings_for_processing_and_completed_states() {
        let locked_statuses = [
            FileStatus::Queued,
            FileStatus::Converting,
            FileStatus::Completed,
        ];

        assert!(locked_statuses.into_iter().all(FileStatus::locks_settings));
    }

    #[test]
    fn file_status_keeps_idle_paused_and_error_editable() {
        assert!(!FileStatus::Idle.locks_settings());
        assert!(!FileStatus::Paused.locks_settings());
        assert!(!FileStatus::Error.locks_settings());
    }

    #[test]
    fn update_status_clamps_progress_to_percent_range() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Converting, 250);

        assert_eq!(
            queue.selected_file().map(|file| file.progress_percent),
            Some(100)
        );
    }

    #[test]
    fn file_by_id_returns_matching_file_without_changing_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 10));

        assert_eq!(
            queue.file_by_id("second").map(|file| file.name.as_str()),
            Some("two.mp4")
        );
        assert_eq!(queue.selected_file_id(), Some("first"));
    }

    #[test]
    fn update_error_stores_conversion_error_message() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert!(queue.update_error("first", "ffmpeg failed"));

        let file = queue.file_by_id("first").expect("file should exist");
        assert_eq!(file.status, FileStatus::Error);
        assert_eq!(file.conversion_error.as_deref(), Some("ffmpeg failed"));
    }

    #[test]
    fn clear_error_removes_previous_conversion_error() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_error("first", "ffmpeg failed");

        assert!(queue.clear_error("first"));

        assert_eq!(
            queue
                .file_by_id("first")
                .and_then(|file| file.conversion_error.as_deref()),
            None
        );
    }

    #[test]
    fn update_selected_output_name_sanitizes_path_segments() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert!(queue.update_selected_output_name("/tmp/export/final.mov"));

        assert_eq!(
            queue.selected_file().map(|file| file.output_name.as_str()),
            Some("final.mov")
        );
    }

    #[test]
    fn update_selected_output_name_resets_empty_value_to_default() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_selected_output_name("custom");

        assert!(queue.update_selected_output_name(".."));

        assert_eq!(
            queue.selected_file().map(|file| file.output_name.as_str()),
            Some("one_converted")
        );
    }

    #[test]
    fn set_selected_output_name_from_input_allows_empty_value() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_selected_output_name("custom");

        assert!(queue.set_selected_output_name_from_input(""));

        assert_eq!(
            queue.selected_file().map(|file| file.output_name.as_str()),
            Some("")
        );
    }

    #[test]
    fn set_selected_output_name_from_input_sanitizes_path_segments() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert!(queue.set_selected_output_name_from_input("/tmp/export/final.mov"));

        assert_eq!(
            queue.selected_file().map(|file| file.output_name.as_str()),
            Some("final.mov")
        );
    }

    #[test]
    fn queue_selected_pending_conversions_marks_only_selected_pending_files() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.add_file(sample_file("second", "/tmp/two.mp4", 10));
        queue.add_file(sample_file("third", "/tmp/three.mp4", 10));
        queue.toggle_batch("second", false);
        queue.update_status("third", FileStatus::Completed, 100);

        let pending = queue.queue_selected_pending_conversions();

        assert_eq!(
            pending
                .iter()
                .map(|file| file.id.as_str())
                .collect::<Vec<_>>(),
            ["first"]
        );
        assert_eq!(
            queue.file_by_id("first").map(|file| file.status),
            Some(FileStatus::Queued)
        );
        assert_eq!(
            queue.file_by_id("second").map(|file| file.status),
            Some(FileStatus::Idle)
        );
        assert_eq!(
            queue.file_by_id("third").map(|file| file.status),
            Some(FileStatus::Completed)
        );
    }

    #[test]
    fn pause_file_changes_only_converting_file_to_paused() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Converting, 40);

        assert!(queue.pause_file("first"));
        assert_eq!(
            queue.selected_file().map(|file| file.status),
            Some(FileStatus::Paused)
        );
    }

    #[test]
    fn pause_file_ignores_non_converting_file() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert!(!queue.pause_file("first"));
        assert_eq!(
            queue.selected_file().map(|file| file.status),
            Some(FileStatus::Idle)
        );
    }

    #[test]
    fn resume_file_changes_only_paused_file_to_converting() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
        queue.update_status("first", FileStatus::Paused, 40);

        assert!(queue.resume_file("first"));
        assert_eq!(
            queue.selected_file().map(|file| file.status),
            Some(FileStatus::Converting)
        );
    }

    #[test]
    fn resume_file_ignores_non_paused_file() {
        let mut queue = FileQueue::new();
        queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

        assert!(!queue.resume_file("first"));
        assert_eq!(
            queue.selected_file().map(|file| file.status),
            Some(FileStatus::Idle)
        );
    }
}
