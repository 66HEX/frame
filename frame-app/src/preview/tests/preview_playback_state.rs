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
    playback.handle_play();

    let command = playback.handle_time_update(20.0);

    assert_eq!(command, PlaybackMediaCommand::pause_and_seek(10.0));
    assert_close(playback.current_time(), 10.0);
}

#[test]
fn handle_time_update_loops_back_when_time_is_just_before_end() {
    let mut playback = playback_with_media(120.0);
    playback.handle_play();

    let command = playback.handle_time_update(119.96);

    assert_eq!(command, PlaybackMediaCommand::pause_and_seek(0.0));
    assert_close(playback.current_time(), 0.0);
}

#[test]
fn handle_time_update_keeps_paused_playhead_at_trim_end() {
    let mut playback = playback_with_media(120.0);
    playback.sync_initial_values(Some("00:00:10.000"), Some("00:00:20.000"));

    let command = playback.handle_time_update(20.0);

    assert_eq!(command, PlaybackMediaCommand::none());
    assert_close(playback.current_time(), 20.0);
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
fn toggle_play_restarts_when_current_time_is_just_before_duration_end() {
    let mut playback = playback_with_media(120.0);
    playback.sync_media(MediaSnapshot {
        current_time: 119.96,
        duration: 120.0,
        paused: true,
    });

    let command = playback.toggle_play();

    assert_eq!(command, PlaybackMediaCommand::seek_and_play(0.0));
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
fn drag_start_handle_uses_one_second_gap_before_end_without_moving_playhead() {
    let mut playback = playback_with_media(120.0);
    playback.sync_initial_values(None, Some("00:00:20.000"));
    assert!(playback.begin_handle_drag(TimelineDragTarget::Start));

    let update = playback.drag_to_percent(0.5);

    assert_eq!(update.command, PlaybackMediaCommand::none());
    assert_close(playback.start_value(), 19.0);
    assert_close(playback.current_time(), 0.0);
    assert_eq!(update.preview_seek_to, Some(19.0));
    assert!(update.trim.is_some());
}

#[test]
fn drag_end_handle_uses_one_second_gap_after_start_without_moving_playhead() {
    let mut playback = playback_with_media(120.0);
    playback.sync_initial_values(Some("00:00:30.000"), None);
    assert!(playback.begin_handle_drag(TimelineDragTarget::End));

    let update = playback.drag_to_percent(0.1);

    assert_eq!(update.command, PlaybackMediaCommand::none());
    assert_close(playback.end_value(), 31.0);
    assert_close(playback.current_time(), 0.0);
    assert_eq!(update.preview_seek_to, Some(31.0));
}

#[test]
fn repeated_trim_drags_keep_one_second_gap_after_timecode_round_trip() {
    let mut playback = playback_with_media(90.4);
    playback.sync_initial_values(Some("00:00:00.400"), None);
    for target in [
        TimelineDragTarget::End,
        TimelineDragTarget::Start,
        TimelineDragTarget::End,
        TimelineDragTarget::Start,
    ] {
        assert!(playback.begin_handle_drag(target));
        let percent = if target == TimelineDragTarget::Start {
            1.0
        } else {
            0.0
        };
        let _ = playback.drag_to_percent(percent);
        assert_close(playback.start_value(), 0.4);
        assert_close(playback.end_value(), 1.4);

        let trim = playback.end_drag().trim.unwrap();
        playback.sync_initial_values(trim.start_time.as_deref(), trim.end_time.as_deref());
    }
}

#[test]
fn subsecond_source_trim_handles_remain_inside_the_source() {
    let mut playback = playback_with_media(0.5);
    assert!(playback.begin_handle_drag(TimelineDragTarget::Start));
    let update = playback.drag_to_percent(1.0);
    assert_close(playback.start_value(), 0.499);
    assert_close(playback.end_value(), 0.5);
    assert_close(update.preview_seek_to.unwrap(), 0.499);
    let _ = playback.end_drag();

    assert!(playback.begin_handle_drag(TimelineDragTarget::End));
    let update = playback.drag_to_percent(0.0);
    assert_close(playback.end_value(), 0.5);
    assert_close(update.preview_seek_to.unwrap(), 0.5);
    let _ = playback.end_drag();

    let mut playback = playback_with_media(0.5);
    assert!(playback.begin_handle_drag(TimelineDragTarget::End));
    let _ = playback.drag_to_percent(0.0);
    assert_close(playback.end_value(), 0.001);
    let _ = playback.end_drag();
    assert!(playback.begin_handle_drag(TimelineDragTarget::Start));
    let _ = playback.drag_to_percent(1.0);
    assert_close(playback.start_value(), 0.0);
    assert_close(playback.end_value(), 0.001);
}

#[test]
fn precise_subsecond_trim_stays_editable_near_both_source_edges() {
    for (start, end) in [
        ("00:00:00.100", "00:00:00.250"),
        ("00:00:00.100", "00:00:01.099"),
        ("00:01:59.750", "00:01:59.900"),
    ] {
        let mut playback = playback_with_media(120.0);
        playback.sync_initial_values(Some(start), Some(end));
        let initial_start = playback.start_value();
        let initial_end = playback.end_value();
        assert!(playback.begin_handle_drag(TimelineDragTarget::Start));
        let _ = playback.drag_to_percent(initial_start / playback.duration());
        assert_close(playback.start_value(), initial_start);
        assert_close(playback.end_value(), initial_end);
        let _ = playback.end_drag();

        assert!(playback.begin_handle_drag(TimelineDragTarget::End));
        let _ = playback.drag_to_percent(initial_end / playback.duration());
        assert_close(playback.start_value(), initial_start);
        assert_close(playback.end_value(), initial_end);
    }
}

#[test]
fn widening_a_subsecond_trim_does_not_change_its_minimum_span_mid_drag() {
    let mut playback = playback_with_media(120.0);
    playback.sync_initial_values(Some("00:00:10.000"), Some("00:00:10.250"));
    assert!(playback.begin_handle_drag(TimelineDragTarget::Start));
    let _ = playback.drag_to_percent(8.0 / 120.0);
    assert_close(playback.start_value(), 8.0);
    let _ = playback.drag_to_percent(10.0 / 120.0);
    assert_close(playback.start_value(), 10.0);
    assert_close(playback.end_value(), 10.25);
}

#[test]
fn sources_shorter_than_timecode_precision_cannot_trim_outside_the_source() {
    let mut playback = playback_with_media(0.000_5);
    assert!(playback.begin_handle_drag(TimelineDragTarget::Start));
    let _ = playback.drag_to_percent(1.0);
    assert_close(playback.start_value(), 0.0);
    let _ = playback.end_drag();
    assert!(playback.begin_handle_drag(TimelineDragTarget::End));
    let _ = playback.drag_to_percent(0.0);
    assert_close(playback.end_value(), 0.000_5);
}

#[test]
fn drag_start_handle_started_while_playing_preserves_playhead() {
    let mut playback = playback_with_media(120.0);
    playback.sync_media(MediaSnapshot {
        current_time: 42.0,
        duration: 120.0,
        paused: false,
    });
    assert!(playback.begin_handle_drag(TimelineDragTarget::Start));
    playback.handle_pause();

    let update = playback.drag_to_percent(0.25);
    let end = playback.end_drag();

    assert_eq!(update.command, PlaybackMediaCommand::none());
    assert_close(playback.start_value(), 30.0);
    assert_close(playback.current_time(), 42.0);
    assert_eq!(update.preview_seek_to, Some(30.0));
    assert_eq!(end.command, PlaybackMediaCommand::none());
    assert_eq!(
        end.trim,
        Some(TrimSelection {
            start_time: Some("00:00:30.000".to_string()),
            end_time: None,
        })
    );
}

#[test]
fn drag_end_handle_started_while_playing_preserves_playhead() {
    let mut playback = playback_with_media(120.0);
    playback.sync_media(MediaSnapshot {
        current_time: 42.0,
        duration: 120.0,
        paused: false,
    });
    assert!(playback.begin_handle_drag(TimelineDragTarget::End));
    playback.handle_pause();

    let update = playback.drag_to_percent(0.5);
    let end = playback.end_drag();

    assert_eq!(update.command, PlaybackMediaCommand::none());
    assert_close(playback.end_value(), 60.0);
    assert_close(playback.current_time(), 42.0);
    assert_eq!(update.preview_seek_to, Some(60.0));
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

    assert_eq!(end.command, PlaybackMediaCommand::seek_and_play(30.0));
    assert_eq!(end.trim, None);
    assert_eq!(playback.dragging(), None);
}

#[test]
fn drag_scrub_requests_preview_seek_without_trim() {
    let mut playback = playback_with_media(120.0);
    let _ = playback.seek_to_percent(0.25);

    let update = playback.drag_to_percent(0.5);

    assert_eq!(update.command, PlaybackMediaCommand::seek(60.0));
    assert_eq!(update.preview_seek_to, Some(60.0));
    assert_eq!(update.trim, None);
}

#[test]
fn end_drag_commits_precise_seek_when_scrub_started_paused() {
    let mut playback = playback_with_media(120.0);
    let _ = playback.seek_to_percent(0.25);

    let end = playback.end_drag();

    assert_eq!(end.command, PlaybackMediaCommand::seek(30.0));
    assert_eq!(end.trim, None);
}

#[test]
fn seek_once_to_percent_does_not_enter_drag_state() {
    let mut playback = playback_with_media(120.0);

    let command = playback.seek_once_to_percent(0.25);

    assert_eq!(command, PlaybackMediaCommand::seek(30.0));
    assert_eq!(playback.dragging(), None);
}

#[test]
fn end_drag_commits_trim_without_seeking_when_handle_was_dragged() {
    let mut playback = playback_with_media(120.0);
    playback.begin_handle_drag(TimelineDragTarget::End);
    let _ = playback.drag_to_percent(0.5);

    let end = playback.end_drag();

    assert_eq!(end.command, PlaybackMediaCommand::none());
    assert_close(playback.current_time(), 0.0);
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
