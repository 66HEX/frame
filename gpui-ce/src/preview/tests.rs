use super::*;

fn assert_rect_close(actual: CropRect, expected: CropRect) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.width, expected.width);
    assert_close(actual.height, expected.height);
}

fn assert_close(actual: f64, expected: f64) {
    const EPSILON: f64 = 0.000_001;
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be within {EPSILON} of {expected}"
    );
}

mod aspect_value {
    use super::*;

    #[test]
    fn returns_free_as_unconstrained() {
        assert_eq!(aspect_value("free"), None);
    }

    #[test]
    fn returns_original_wide_ratio() {
        assert_close(aspect_value("16:9").unwrap(), 16.0 / 9.0);
    }

    #[test]
    fn returns_none_for_unknown_ratio() {
        assert_eq!(aspect_value("2:1"), None);
    }
}

mod transform_crop_rect {
    use super::*;

    #[test]
    fn round_trips_zero_rotation_without_flips() {
        assert_round_trip(PreviewRotation::Deg0, false, false);
    }

    #[test]
    fn round_trips_side_rotation_with_both_flips() {
        assert_round_trip(PreviewRotation::Deg90, true, true);
    }

    #[test]
    fn round_trips_half_rotation_with_horizontal_flip() {
        assert_round_trip(PreviewRotation::Deg180, true, false);
    }

    #[test]
    fn round_trips_reverse_side_rotation_with_vertical_flip() {
        assert_round_trip(PreviewRotation::Deg270, false, true);
    }

    fn assert_round_trip(rotation: PreviewRotation, flip_horizontal: bool, flip_vertical: bool) {
        let rect = CropRect {
            x: 0.2,
            y: 0.15,
            width: 0.35,
            height: 0.45,
        };

        let transformed = super::super::transform_crop_rect(
            rect,
            rotation,
            flip_horizontal,
            flip_vertical,
            false,
        );
        let round_trip = super::super::transform_crop_rect(
            transformed,
            rotation,
            flip_horizontal,
            flip_vertical,
            true,
        );

        assert_rect_close(round_trip, rect);
    }
}

mod remap_drag_deltas {
    use super::*;

    #[test]
    fn leaves_zero_rotation_deltas_unchanged() {
        assert_eq!(
            super::super::remap_drag_deltas(0.2, 0.1, PreviewRotation::Deg0, false, false),
            DragDelta { dx: 0.2, dy: 0.1 }
        );
    }

    #[test]
    fn remaps_clockwise_side_rotation() {
        assert_eq!(
            super::super::remap_drag_deltas(0.2, 0.1, PreviewRotation::Deg90, false, false),
            DragDelta { dx: 0.1, dy: -0.2 }
        );
    }

    #[test]
    fn remaps_half_rotation() {
        assert_eq!(
            super::super::remap_drag_deltas(0.2, 0.1, PreviewRotation::Deg180, false, false),
            DragDelta { dx: -0.2, dy: -0.1 }
        );
    }

    #[test]
    fn remaps_reverse_side_rotation() {
        assert_eq!(
            super::super::remap_drag_deltas(0.2, 0.1, PreviewRotation::Deg270, false, false),
            DragDelta { dx: -0.1, dy: 0.2 }
        );
    }

    #[test]
    fn applies_flips_after_rotation() {
        assert_eq!(
            super::super::remap_drag_deltas(0.2, 0.1, PreviewRotation::Deg90, true, true),
            DragDelta { dx: -0.1, dy: 0.2 }
        );
    }
}

mod clamp_rect {
    use super::*;

    #[test]
    fn keeps_crop_inside_normalized_bounds() {
        assert_rect_close(
            super::super::clamp_rect(CropRect {
                x: -0.2,
                y: 0.9,
                width: 0.2,
                height: 0.3,
            }),
            CropRect {
                x: 0.0,
                y: 0.7,
                width: 0.2,
                height: 0.3,
            },
        );
    }

    #[test]
    fn enforces_original_minimum_crop_size() {
        assert_rect_close(
            super::super::clamp_rect(CropRect {
                x: 0.9,
                y: 0.9,
                width: 0.01,
                height: 0.01,
            }),
            CropRect {
                x: 0.9,
                y: 0.9,
                width: 0.05,
                height: 0.05,
            },
        );
    }
}

mod adjust_rect_to_ratio {
    use super::*;

    #[test]
    fn preserves_center_when_possible() {
        let adjusted = super::super::adjust_rect_to_ratio(
            CropRect {
                x: 0.2,
                y: 0.2,
                width: 0.6,
                height: 0.4,
            },
            1.0,
            1920.0,
            1080.0,
            false,
        );

        assert_close(adjusted.x + adjusted.width / 2.0, 0.5);
        assert_close(adjusted.y + adjusted.height / 2.0, 0.4);
        assert_close(adjusted.width / adjusted.height, 1.0 / (1920.0 / 1080.0));
    }
}

mod enforce_aspect {
    use super::*;

    #[test]
    fn anchors_the_dragged_edge() {
        let start_rect = CropRect {
            x: 0.2,
            y: 0.2,
            width: 0.4,
            height: 0.4,
        };
        let next = super::super::enforce_aspect(
            CropRect {
                x: 0.2,
                y: 0.2,
                width: 0.55,
                height: 0.4,
            },
            DragHandle::East,
            start_rect,
            16.0 / 9.0,
            1920.0,
            1080.0,
            false,
        );

        assert_eq!(next.x, start_rect.x);
        assert_close(
            next.y + next.height / 2.0,
            start_rect.y + start_rect.height / 2.0,
        );
        assert_close(next.width / next.height, 1.0);
    }
}

mod apply_visual_crop_drag {
    use super::*;

    #[test]
    fn keeps_drag_deltas_in_visual_space_for_side_rotations() {
        let next = super::super::apply_visual_crop_drag(VisualCropDrag {
            start_rect: CropRect {
                x: 0.2,
                y: 0.2,
                width: 0.4,
                height: 0.4,
            },
            handle: DragHandle::East,
            start_point: Point { x: 0.6, y: 0.4 },
            current_point: Point { x: 0.7, y: 0.4 },
            aspect_id: "free",
            source_width: 1920.0,
            source_height: 1080.0,
            is_side_rotation: true,
        });

        assert_rect_close(
            next,
            CropRect {
                x: 0.2,
                y: 0.2,
                width: 0.5,
                height: 0.4,
            },
        );
    }

    #[test]
    fn moves_crop_directly_in_visual_space_for_side_rotations() {
        let next = super::super::apply_visual_crop_drag(VisualCropDrag {
            start_rect: CropRect {
                x: 0.2,
                y: 0.2,
                width: 0.4,
                height: 0.4,
            },
            handle: DragHandle::Move,
            start_point: Point { x: 0.4, y: 0.4 },
            current_point: Point { x: 0.5, y: 0.45 },
            aspect_id: "free",
            source_width: 1920.0,
            source_height: 1080.0,
            is_side_rotation: true,
        });

        assert_rect_close(
            next,
            CropRect {
                x: 0.3,
                y: 0.25,
                width: 0.4,
                height: 0.4,
            },
        );
    }

    #[test]
    fn enforces_fixed_aspect_ratios_when_dragging_corner_handles() {
        let next = super::super::apply_visual_crop_drag(VisualCropDrag {
            start_rect: CropRect {
                x: 0.2,
                y: 0.2,
                width: 0.4,
                height: 0.4,
            },
            handle: DragHandle::SouthEast,
            start_point: Point { x: 0.6, y: 0.6 },
            current_point: Point { x: 0.8, y: 0.6 },
            aspect_id: "16:9",
            source_width: 1920.0,
            source_height: 1080.0,
            is_side_rotation: false,
        });

        assert_close(next.width / next.height, 1.0);
    }
}

mod handle_cursor {
    use super::*;

    #[test]
    fn swaps_cardinal_cursors_for_side_rotation() {
        assert_eq!(
            super::super::handle_cursor(DragHandle::North, false),
            "ns-resize"
        );
        assert_eq!(
            super::super::handle_cursor(DragHandle::North, true),
            "ew-resize"
        );
    }

    #[test]
    fn swaps_corner_cursors_for_side_rotation() {
        assert_eq!(
            super::super::handle_cursor(DragHandle::NorthEast, false),
            "nesw-resize"
        );
        assert_eq!(
            super::super::handle_cursor(DragHandle::NorthEast, true),
            "nwse-resize"
        );
    }
}

mod preview_rotation {
    use super::*;

    #[test]
    fn parses_original_rotation_strings() {
        assert_eq!(PreviewRotation::from("90"), PreviewRotation::Deg90);
        assert_eq!(PreviewRotation::from("180"), PreviewRotation::Deg180);
        assert_eq!(PreviewRotation::from("270"), PreviewRotation::Deg270);
    }

    #[test]
    fn treats_unknown_rotation_as_zero() {
        assert_eq!(PreviewRotation::from(""), PreviewRotation::Deg0);
        assert_eq!(PreviewRotation::from("45"), PreviewRotation::Deg0);
    }
}

mod preview_media_kind {
    use super::*;

    #[test]
    fn stays_unknown_until_metadata_is_ready() {
        assert_eq!(
            super::super::preview_media_kind(MetadataStatus::Loading, Some(SourceMediaKind::Video),),
            PreviewMediaKind::Unknown
        );
    }

    #[test]
    fn uses_ready_metadata_kind() {
        assert_eq!(
            super::super::preview_media_kind(MetadataStatus::Ready, Some(SourceMediaKind::Image)),
            PreviewMediaKind::Image
        );
    }
}

mod preview_control_availability {
    use super::*;

    #[test]
    fn disables_trim_for_unknown_metadata() {
        assert!(
            availability(MetadataStatus::Idle, Some(SourceMediaKind::Video), "mp4").trim_disabled
        );
    }

    #[test]
    fn disables_trim_for_image_sources() {
        assert!(
            availability(MetadataStatus::Ready, Some(SourceMediaKind::Image), "png").trim_disabled
        );
    }

    #[test]
    fn keeps_visual_controls_hidden_for_audio_sources() {
        assert!(
            availability(MetadataStatus::Ready, Some(SourceMediaKind::Audio), "mp3")
                .hide_visual_controls
        );
    }

    #[test]
    fn hides_visual_controls_for_video_to_audio_output() {
        assert!(
            availability(MetadataStatus::Ready, Some(SourceMediaKind::Video), "mp3")
                .hide_visual_controls
        );
    }

    #[test]
    fn enables_overlay_for_reencoded_video_output() {
        assert!(
            availability(MetadataStatus::Ready, Some(SourceMediaKind::Video), "mp4")
                .overlay_available
        );
    }

    #[test]
    fn disables_overlay_for_copy_mode() {
        let availability = super::super::preview_control_availability(PreviewControlInput {
            metadata_status: MetadataStatus::Ready,
            source_media_kind: Some(SourceMediaKind::Video),
            controls_disabled: false,
            processing_mode: ProcessingMode::Copy,
            container: Some("mp4"),
        });

        assert!(!availability.overlay_available);
    }

    #[test]
    fn disables_overlay_for_gif_output() {
        assert!(
            !availability(MetadataStatus::Ready, Some(SourceMediaKind::Video), "gif")
                .overlay_available
        );
    }

    #[test]
    fn respects_global_control_disabled_state_for_trim() {
        let availability = super::super::preview_control_availability(PreviewControlInput {
            metadata_status: MetadataStatus::Ready,
            source_media_kind: Some(SourceMediaKind::Video),
            controls_disabled: true,
            processing_mode: ProcessingMode::Reencode,
            container: Some("mp4"),
        });

        assert!(availability.trim_disabled);
    }

    fn availability(
        metadata_status: MetadataStatus,
        source_media_kind: Option<SourceMediaKind>,
        container: &str,
    ) -> PreviewControlAvailability {
        super::super::preview_control_availability(PreviewControlInput {
            metadata_status,
            source_media_kind,
            controls_disabled: false,
            processing_mode: ProcessingMode::Reencode,
            container: Some(container),
        })
    }
}

mod time_formatting {
    use super::*;

    #[test]
    fn parse_time_to_seconds_accepts_original_hh_mm_ss_fraction() {
        assert_close(super::super::parse_time_to_seconds("01:02:03.250"), 3723.25);
    }

    #[test]
    fn parse_time_to_seconds_returns_zero_for_partial_timecodes() {
        assert_eq!(super::super::parse_time_to_seconds("02:03"), 0.0);
    }

    #[test]
    fn format_time_matches_original_trim_precision() {
        assert_eq!(super::super::format_time(3723.25), "01:02:03.250");
    }

    #[test]
    fn format_time_pads_single_digit_seconds_like_svelte() {
        assert_eq!(super::super::format_time(61.2), "00:01:01.200");
    }
}

mod preview_playback_state {
    use super::*;

    #[test]
    fn sync_initial_values_reads_start_and_end_timecodes() {
        let mut playback = playback_with_media(120.0);

        playback.sync_initial_values(Some("00:00:05.000"), Some("00:01:00.500"));

        assert_close(playback.start_value(), 5.0);
        assert_close(playback.end_value(), 60.5);
    }

    #[test]
    fn sync_initial_values_uses_duration_when_end_is_missing() {
        let mut playback = playback_with_media(120.0);

        playback.sync_initial_values(Some("00:00:05.000"), None);

        assert_close(playback.end_value(), 120.0);
    }

    #[test]
    fn sync_from_media_clamps_trim_values_to_duration() {
        let mut playback = PreviewPlaybackState::new(false);
        playback.sync_initial_values(Some("00:05:00.000"), Some("00:10:00.000"));

        playback.sync_media(MediaSnapshot {
            current_time: 12.0,
            duration: 30.0,
            paused: true,
        });

        assert_close(playback.start_value(), 0.0);
        assert_close(playback.end_value(), 30.0);
    }

    #[test]
    fn clear_media_resets_timeline_state_like_detached_media_element() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some("00:00:05.000"), Some("00:00:20.000"));

        playback.clear_media();

        assert_close(playback.duration(), 0.0);
        assert_close(playback.start_value(), 0.0);
        assert_close(playback.end_value(), 0.0);
    }

    #[test]
    fn handle_time_update_loops_back_to_trim_start_at_end() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some("00:00:10.000"), Some("00:00:20.000"));

        let command = playback.handle_time_update(20.0);

        assert_eq!(command, PlaybackMediaCommand::pause_and_seek(10.0));
        assert_close(playback.current_time(), 10.0);
    }

    #[test]
    fn toggle_play_seeks_to_start_when_current_time_is_outside_trim() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some("00:00:10.000"), Some("00:00:20.000"));
        playback.sync_media(MediaSnapshot {
            current_time: 30.0,
            duration: 120.0,
            paused: true,
        });
        playback.handle_pause();

        let command = playback.toggle_play();

        assert_eq!(command, PlaybackMediaCommand::seek_and_play(10.0));
    }

    #[test]
    fn toggle_play_returns_pause_command_when_playing() {
        let mut playback = playback_with_media(120.0);
        playback.handle_play();

        assert_eq!(playback.toggle_play(), PlaybackMediaCommand::pause());
    }

    #[test]
    fn commit_trim_values_omits_zero_start_and_full_duration_end() {
        let playback = playback_with_media(120.0);

        assert_eq!(
            playback.commit_trim_values(),
            Some(TrimSelection {
                start_time: None,
                end_time: None,
            })
        );
    }

    #[test]
    fn commit_trim_values_formats_partial_trim_bounds() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some("00:00:05.000"), Some("00:00:30.250"));

        assert_eq!(
            playback.commit_trim_values(),
            Some(TrimSelection {
                start_time: Some("00:00:05.000".to_string()),
                end_time: Some("00:00:30.250".to_string()),
            })
        );
    }

    #[test]
    fn image_playback_ignores_trim_commit_and_play_toggle() {
        let mut playback = PreviewPlaybackState::new(true);
        playback.sync_media(MediaSnapshot {
            current_time: 5.0,
            duration: 120.0,
            paused: true,
        });

        assert_eq!(playback.commit_trim_values(), None);
        assert_eq!(playback.toggle_play(), PlaybackMediaCommand::none());
    }

    #[test]
    fn set_start_from_input_rejects_values_after_end() {
        let mut playback = playback_with_media(120.0);

        assert_eq!(playback.set_start_from_input(121.0), None);
    }

    #[test]
    fn set_end_from_input_accepts_values_within_duration() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some("00:00:05.000"), None);

        assert_eq!(
            playback.set_end_from_input(60.0),
            Some(PlaybackMediaCommand::seek(60.0))
        );
        assert_close(playback.end_value(), 60.0);
    }

    #[test]
    fn begin_handle_drag_ignores_image_sources() {
        let mut playback = PreviewPlaybackState::new(true);

        assert!(!playback.begin_handle_drag(TimelineDragTarget::Start));
    }

    #[test]
    fn drag_start_handle_uses_one_second_gap_before_end() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(None, Some("00:00:20.000"));
        assert!(playback.begin_handle_drag(TimelineDragTarget::Start));

        let update = playback.drag_to_percent(0.5);

        assert_eq!(update.command, PlaybackMediaCommand::seek(19.0));
        assert_close(playback.start_value(), 19.0);
        assert!(update.trim.is_some());
    }

    #[test]
    fn drag_end_handle_uses_one_second_gap_after_start() {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some("00:00:30.000"), None);
        assert!(playback.begin_handle_drag(TimelineDragTarget::End));

        let update = playback.drag_to_percent(0.1);

        assert_eq!(update.command, PlaybackMediaCommand::seek(31.0));
        assert_close(playback.end_value(), 31.0);
    }

    #[test]
    fn seek_to_percent_pauses_active_scrub_and_remembers_play_state() {
        let mut playback = playback_with_media(120.0);
        playback.handle_play();

        let command = playback.seek_to_percent(0.25);

        assert_eq!(command, PlaybackMediaCommand::pause_and_seek(30.0));
        assert_eq!(playback.dragging(), Some(TimelineDragTarget::Scrub));
    }

    #[test]
    fn end_drag_resumes_scrub_when_it_started_while_playing() {
        let mut playback = playback_with_media(120.0);
        playback.handle_play();
        let _ = playback.seek_to_percent(0.25);

        let end = playback.end_drag();

        assert_eq!(end.command, PlaybackMediaCommand::play());
        assert_eq!(end.trim, None);
        assert_eq!(playback.dragging(), None);
    }

    #[test]
    fn end_drag_commits_trim_when_handle_was_dragged() {
        let mut playback = playback_with_media(120.0);
        playback.begin_handle_drag(TimelineDragTarget::End);
        let _ = playback.drag_to_percent(0.5);

        let end = playback.end_drag();

        assert_eq!(end.command, PlaybackMediaCommand::none());
        assert_eq!(
            end.trim,
            Some(TrimSelection {
                start_time: None,
                end_time: Some("00:01:00.000".to_string()),
            })
        );
    }

    #[test]
    fn timeline_percent_returns_zero_without_positive_duration() {
        let playback = PreviewPlaybackState::new(false);

        assert_eq!(playback.to_timeline_percent(15.0), 0.0);
    }

    #[test]
    fn timeline_percent_matches_original_value_over_duration_formula() {
        let playback = playback_with_media(120.0);

        assert_close(playback.to_timeline_percent(30.0), 25.0);
    }

    fn playback_with_media(duration: f64) -> PreviewPlaybackState {
        let mut playback = PreviewPlaybackState::new(false);
        playback.sync_media(MediaSnapshot {
            current_time: 0.0,
            duration,
            paused: true,
        });
        playback
    }
}

mod preview_overlay_state {
    use super::*;

    #[test]
    fn clamp_overlay_center_keeps_overlay_inside_bounds() {
        let center = super::super::clamp_overlay_center(0.98, 0.01, 0.4, 0.2);

        assert_close(center.x, 0.8);
        assert_close(center.y, 0.1);
    }

    #[test]
    fn max_overlay_width_accounts_for_tall_overlay_aspect() {
        assert_close(super::super::max_overlay_width(2.0), 0.5);
    }

    #[test]
    fn clamp_overlay_width_uses_original_minimum_width() {
        assert_close(super::super::clamp_overlay_width(0.01, 1.0), 0.03);
    }

    #[test]
    fn create_default_overlay_matches_original_centered_overlay() {
        let overlay = super::super::create_default_overlay("/tmp/logo.png");

        assert_eq!(overlay.path, "/tmp/logo.png");
        assert_close(overlay.x, 0.5);
        assert_close(overlay.y, 0.5);
        assert_close(overlay.width, DEFAULT_OVERLAY_WIDTH);
        assert_close(overlay.opacity, 1.0);
    }

    #[test]
    fn normalize_overlay_clamps_position_size_and_opacity() {
        let overlay = super::super::normalize_overlay(&PreviewOverlay {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.99,
            y: -0.1,
            width: 2.0,
            opacity: 2.0,
            anchor: "ignored".to_string(),
        });

        assert_close(overlay.x, 0.6);
        assert_close(overlay.y, 0.4);
        assert_close(overlay.width, MAX_OVERLAY_WIDTH);
        assert_close(overlay.opacity, 1.0);
        assert_eq!(overlay.anchor, "custom");
    }

    #[test]
    fn sync_initial_overlay_discards_disabled_or_empty_overlay() {
        let mut state = PreviewOverlayState::new();
        state.set_overlay_from_path("/tmp/logo.png", false);

        state.sync_initial_overlay(Some(&PreviewOverlay {
            enabled: false,
            path: "/tmp/logo.png".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 1.0,
            anchor: "custom".to_string(),
        }));

        assert_eq!(state.overlay(), None);
        assert!(!state.overlay_mode());
    }

    #[test]
    fn sync_initial_overlay_ignores_external_changes_while_dragging() {
        let mut state = state_with_overlay();
        assert!(state.begin_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.5,
                y: 0.5,
                width: Some(0.18),
                height: Some(0.18),
            },
            false,
        ));

        state.sync_initial_overlay(None);

        assert!(state.overlay().is_some());
        assert!(state.is_dragging());
    }

    #[test]
    fn set_overlay_from_path_is_blocked_when_controls_are_disabled() {
        let mut state = PreviewOverlayState::new();

        assert_eq!(state.set_overlay_from_path("/tmp/logo.png", true), None);
        assert_eq!(state.overlay(), None);
    }

    #[test]
    fn set_overlay_from_path_persists_overlay_and_enters_overlay_mode() {
        let mut state = PreviewOverlayState::new();

        let overlay = state.set_overlay_from_path("/tmp/logo.png", false).unwrap();

        assert_eq!(overlay.path, "/tmp/logo.png");
        assert!(state.overlay_mode());
        assert!(state.overlay().is_some());
    }

    #[test]
    fn toggle_overlay_mode_requests_crop_deactivation_when_enabling() {
        let mut state = state_with_overlay();
        state.set_overlay_mode(false, false);

        let change = state.toggle_overlay_mode(false);

        assert!(change.changed);
        assert!(change.should_deactivate_crop);
        assert!(state.overlay_mode());
    }

    #[test]
    fn toggle_overlay_mode_does_nothing_without_overlay() {
        let mut state = PreviewOverlayState::new();

        let change = state.toggle_overlay_mode(false);

        assert!(!change.changed);
        assert!(!change.should_deactivate_crop);
    }

    #[test]
    fn set_overlay_mode_refuses_enabling_when_controls_are_disabled() {
        let mut state = state_with_overlay();
        state.set_overlay_mode(false, false);

        let change = state.set_overlay_mode(true, true);

        assert!(!change.changed);
        assert!(!state.overlay_mode());
    }

    #[test]
    fn begin_overlay_drag_requires_overlay_mode() {
        let mut state = state_with_overlay();
        state.set_overlay_mode(false, false);

        assert!(!state.begin_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.5,
                y: 0.5,
                width: Some(0.18),
                height: Some(0.18),
            },
            false,
        ));
    }

    #[test]
    fn move_drag_updates_center_in_normalized_overlay_space() {
        let mut state = state_with_overlay();
        state.begin_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.5,
                y: 0.5,
                width: Some(0.2),
                height: Some(0.1),
            },
            false,
        );

        let overlay = state
            .update_overlay_drag(OverlayDragPoint {
                x: 0.6,
                y: 0.45,
                width: Some(0.2),
                height: Some(0.1),
            })
            .unwrap();

        assert_close(overlay.x, 0.6);
        assert_close(overlay.y, 0.45);
    }

    #[test]
    fn resize_drag_preserves_overlay_aspect_from_pointer_metrics() {
        let mut state = state_with_overlay();
        state.sync_initial_overlay(Some(&PreviewOverlay {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 1.0,
            anchor: "custom".to_string(),
        }));
        state.set_overlay_mode(true, false);
        state.begin_overlay_drag(
            OverlayDragHandle::SouthEast,
            OverlayDragPoint {
                x: 0.6,
                y: 0.55,
                width: Some(0.2),
                height: Some(0.1),
            },
            false,
        );

        let overlay = state
            .update_overlay_drag(OverlayDragPoint {
                x: 0.8,
                y: 0.7,
                width: Some(0.2),
                height: Some(0.1),
            })
            .unwrap();

        assert_close(overlay.width, 0.5);
        assert_close(overlay.x, 0.65);
        assert_close(overlay.y, 0.575);
    }

    #[test]
    fn resize_drag_requires_pointer_dimensions() {
        let mut state = state_with_overlay();
        state.begin_overlay_drag(
            OverlayDragHandle::SouthEast,
            OverlayDragPoint {
                x: 0.6,
                y: 0.6,
                width: None,
                height: Some(0.1),
            },
            false,
        );

        assert_eq!(
            state.update_overlay_drag(OverlayDragPoint {
                x: 0.8,
                y: 0.7,
                width: None,
                height: Some(0.1),
            }),
            None
        );
    }

    #[test]
    fn end_overlay_drag_clears_dragging_state() {
        let mut state = state_with_overlay();
        state.begin_overlay_drag(
            OverlayDragHandle::Move,
            OverlayDragPoint {
                x: 0.5,
                y: 0.5,
                width: Some(0.18),
                height: Some(0.18),
            },
            false,
        );

        state.end_overlay_drag();

        assert!(!state.is_dragging());
    }

    #[test]
    fn set_opacity_clamps_to_original_zero_one_range() {
        let mut state = state_with_overlay();

        let overlay = state.set_opacity(2.0, false).unwrap();

        assert_close(overlay.opacity, 1.0);
    }

    #[test]
    fn nudge_size_uses_original_step_and_recenters_inside_bounds() {
        let mut state = PreviewOverlayState::new();
        state.sync_initial_overlay(Some(&PreviewOverlay {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.99,
            y: 0.99,
            width: 0.79,
            opacity: 1.0,
            anchor: "custom".to_string(),
        }));

        let overlay = state
            .nudge_size(OverlaySizeDirection::Increase, Some(1.0), false)
            .unwrap();

        assert_close(overlay.width, 0.8);
        assert_close(overlay.x, 0.6);
        assert_close(overlay.y, 0.6);
    }

    #[test]
    fn nudge_size_respects_height_ratio_max_width() {
        let mut state = state_with_overlay();

        let overlay = state
            .nudge_size(OverlaySizeDirection::Increase, Some(4.0), false)
            .unwrap();

        assert_close(overlay.width, 0.205);
    }

    #[test]
    fn remove_overlay_clears_overlay_and_mode() {
        let mut state = state_with_overlay();

        assert_eq!(state.remove_overlay(false), Some(None));
        assert_eq!(state.overlay(), None);
        assert!(!state.overlay_mode());
    }

    #[test]
    fn remove_overlay_is_blocked_when_controls_are_disabled() {
        let mut state = state_with_overlay();

        assert_eq!(state.remove_overlay(true), None);
        assert!(state.overlay().is_some());
    }

    fn state_with_overlay() -> PreviewOverlayState {
        let mut state = PreviewOverlayState::new();
        state.set_overlay_from_path("/tmp/logo.png", false);
        state
    }
}
