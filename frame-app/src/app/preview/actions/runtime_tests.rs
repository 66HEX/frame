use super::*;
use crate::app::*;
use crate::preview::PlaybackMediaCommand;
use gpui::{AppContext, TestAppContext};
use std::time::{Duration, Instant};

#[test]
fn paused_preview_stops_polling_after_presenting_the_final_frame() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let metadata = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&metadata).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(request.config));
        let frame = crate::preview_engine::rendered_frame_from_bgra_payload(1, 1, 4, 0, vec![0; 4])
            .unwrap();
        let _ = session.frame_store().publish(frame);
        root.preview_ui.session = Some(session);
        root.schedule_preview_frame_tick(cx);
    });
    app.run_until_parked();
    let mut wakeups = 0;
    for _ in 0..600 {
        if !app.dispatcher.advance_clock_to_next_timer() {
            break;
        }
        app.run_until_parked();
        wakeups += 1;
    }
    eprintln!("paused preview timer wakeups over 600 potential ticks: {wakeups}");
    assert_eq!(
        wakeups, 1,
        "an idle frame must not keep a 16 ms timer alive"
    );
    root.read_with(&app, |root, _| {
        assert_eq!(root.preview_ui.render_generation, 1);
        assert!(!root.preview_ui.frame_tick_active);
    });
}

#[test]
fn paused_preview_wakes_for_seek_and_play_then_stops_after_pause() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let metadata = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&metadata).unwrap();
        root.preview_ui.session = Some(Arc::new(PreviewSession::new_for_test(request.config)));
        root.schedule_preview_frame_tick(cx);
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    assert!(!app.dispatcher.advance_clock_to_next_timer());

    root.update(&mut app, |root, cx| {
        let frame =
            crate::preview_engine::rendered_frame_from_bgra_payload(1, 1, 4, 1_000, vec![0; 4])
                .unwrap();
        let _ = root
            .preview_ui
            .session
            .as_ref()
            .unwrap()
            .frame_store()
            .publish(frame);
        root.apply_preview_media_command(PlaybackMediaCommand::seek(1.0), true, Some(cx));
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert_eq!(root.preview_ui.render_generation, 1);
        assert!(!root.preview_ui.frame_tick_active);
    });
    assert!(!app.dispatcher.advance_clock_to_next_timer());

    root.update(&mut app, |root, cx| {
        root.apply_preview_media_command(PlaybackMediaCommand::play(), true, Some(cx));
    });
    app.run_until_parked();
    for _ in 0..3 {
        assert!(app.dispatcher.advance_clock_to_next_timer());
        app.run_until_parked();
        root.read_with(&app, |root, _| assert!(root.preview_ui.frame_tick_active));
    }

    root.update(&mut app, |root, cx| {
        root.apply_preview_media_command(PlaybackMediaCommand::pause(), true, Some(cx));
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    root.read_with(&app, |root, _| assert!(!root.preview_ui.frame_tick_active));
    assert!(!app.dispatcher.advance_clock_to_next_timer());
}

#[test]
fn preview_timer_keeps_debounces_alive_and_stops_after_their_deadlines() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.preview_ui.preview_dimensions_debounce_until =
            Some(Instant::now() + Duration::from_mins(1));
        root.preview_ui.filter_preview_debounce_until =
            Some(Instant::now() + Duration::from_mins(1));
        root.schedule_preview_frame_tick(cx);
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    root.read_with(&app, |root, _| assert!(root.preview_ui.frame_tick_active));

    root.update(&mut app, |root, _| {
        let expired = Some(
            Instant::now()
                .checked_sub(Duration::from_secs(1))
                .expect("test deadline fits the monotonic clock"),
        );
        root.preview_ui.preview_dimensions_debounce_until = expired;
        root.preview_ui.filter_preview_debounce_until = expired;
    });
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert!(root.preview_ui.preview_dimensions_debounce_until.is_none());
        assert!(root.preview_ui.filter_preview_debounce_until.is_none());
        assert!(!root.preview_ui.frame_tick_active);
    });
    assert!(!app.dispatcher.advance_clock_to_next_timer());
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "The completed animation must snap exactly to its deterministic target."
)]
fn preview_timer_keeps_canvas_animation_alive_and_settles_before_stopping() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.preview_ui.canvas.target_zoom = 2.0;
        root.schedule_preview_frame_tick(cx);
    });
    app.run_until_parked();
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert!(root.preview_ui.canvas.current_zoom > 1.0);
        assert!(root.preview_ui.canvas.current_zoom < 2.0);
        assert!(root.preview_ui.frame_tick_active);
    });
    root.update(&mut app, |root, _| {
        root.preview_ui.canvas.last_animation_tick = Some(
            Instant::now()
                .checked_sub(Duration::from_secs(10))
                .expect("test animation timestamp fits the monotonic clock"),
        );
    });
    assert!(app.dispatcher.advance_clock_to_next_timer());
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert_eq!(root.preview_ui.canvas.current_zoom, 2.0);
        assert!(root.preview_ui.canvas.last_animation_tick.is_none());
        assert!(!root.preview_ui.frame_tick_active);
    });
    assert!(!app.dispatcher.advance_clock_to_next_timer());
}

#[test]
fn stale_preview_completion_cannot_replace_a_new_request_for_the_same_key() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let metadata = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&metadata).unwrap();
        let old_generation = root.preview_ui.runtime_request_generation;
        root.preview_ui.pending_runtime_key = Some(request.key.clone());
        let old_command_generation = root.preview_ui.media_command_generation;

        root.clear_preview_runtime(cx).detach();
        assert_ne!(
            root.preview_ui.media_command_generation,
            old_command_generation
        );
        root.preview_ui.pending_runtime_key = Some(request.key.clone());
        let new_generation = root.preview_ui.runtime_request_generation;
        let result = Ok(Arc::new(PreviewSession::new_for_test(request.config)));
        assert!(!root.finish_preview_runtime_request(old_generation, &request.key, &result, cx));
        assert!(root.preview_ui.session.is_none());
        assert_eq!(
            root.preview_ui.pending_runtime_key.as_ref(),
            Some(&request.key)
        );

        let error = Err(PreviewEngineError::Ffmpeg("stale failure".to_string()));
        assert!(!root.finish_preview_runtime_request(old_generation, &request.key, &error, cx));
        assert!(root.preview_ui.runtime_error.is_none());

        root.preview_ui.frame_tick_active = true;
        assert!(root.finish_preview_runtime_request(new_generation, &request.key, &result, cx));
        assert_eq!(root.preview_ui.runtime_key, Some(request.key));
        assert!(root.preview_ui.pending_runtime_key.is_none());
        assert!(Arc::ptr_eq(
            root.preview_ui.session.as_ref().unwrap(),
            result.as_ref().unwrap()
        ));
    });
}

#[test]
fn preset_changes_coalesce_while_a_rebuild_is_pending_and_keep_the_old_frame() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let metadata = root.selected_source_metadata_entry();
        let current = root.selected_preview_runtime_request(&metadata).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(current.config));
        let frame = crate::preview_engine::rendered_frame_from_bgra_payload(1, 1, 4, 0, vec![0; 4])
            .unwrap();
        let _ = session.frame_store().publish(frame);
        root.preview_ui.session = Some(Arc::clone(&session));
        root.refresh_preview_render_image(cx);
        let image = root.preview_ui.render_image.clone().unwrap();
        root.preview_ui.runtime_key = Some(current.key.clone());
        let mut pending_key = current.key.clone();
        pending_key.visual_hash = current.key.visual_hash.wrapping_add(1);
        root.preview_ui.pending_runtime_key = Some(pending_key.clone());

        let mut latest = root.selected_preview_runtime_request(&metadata).unwrap();
        latest.key.visual_hash = current.key.visual_hash.wrapping_add(2);
        root.sync_preview_runtime_for_selection(Some(latest), cx);

        assert_eq!(root.preview_ui.runtime_key, Some(current.key));
        assert_eq!(root.preview_ui.pending_runtime_key, Some(pending_key));
        assert!(Arc::ptr_eq(
            root.preview_ui.session.as_ref().unwrap(),
            &session
        ));
        assert!(Arc::ptr_eq(
            root.preview_ui.render_image.as_ref().unwrap(),
            &image
        ));
    });
}
