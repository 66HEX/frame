use super::*;
use crate::app::VisualFixture;
use crate::app::preview::panel::preview_playback_state;
use crate::preview::{MediaSnapshot, PreviewMediaKind, TimelineDragTarget};
use crate::preview_engine::PreviewCommand;
use gpui::{AppContext, TestAppContext};

fn request(
    command: PlaybackMediaCommand,
    precise_seek: bool,
    generation: u64,
) -> QueuedMediaCommand {
    QueuedMediaCommand {
        command,
        precise_seek,
        generation,
        origin: MediaCommandOrigin::Playback,
    }
}

fn seek(seconds: u32) -> QueuedMediaCommand {
    request(
        PlaybackMediaCommand::seek(f64::from(seconds)),
        true,
        u64::from(seconds),
    )
}

fn trim_seek(seconds: f64, generation: u64, pause: bool) -> QueuedMediaCommand {
    QueuedMediaCommand {
        command: PlaybackMediaCommand {
            seek_to: Some(seconds),
            pause,
            play: false,
        },
        precise_seek: true,
        generation: 0,
        origin: MediaCommandOrigin::Trim {
            generation,
            seconds,
        },
    }
}

fn prepare_preview(root: &mut FrameRoot, playing: bool) {
    root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
    let metadata = root.selected_source_metadata_entry();
    let config = root
        .selected_preview_runtime_request(&metadata)
        .unwrap()
        .config;
    root.preview_ui.session = Some(Arc::new(
        crate::preview_engine::PreviewSession::new_for_test(config),
    ));
    root.preview_ui.playback = preview_playback_state(PreviewMediaKind::Video, 90.0, None, None);
    root.preview_ui.playback.sync_media(MediaSnapshot {
        current_time: 30.0,
        duration: 90.0,
        paused: !playing,
    });
}

#[test]
fn cancelled_trim_cannot_execute_after_final_precise_seek_and_play() {
    for trim_already_running in [false, true] {
        let mut queue = PreviewMediaCommandQueue::default();
        queue.push(trim_seek(18.0, 5, true));
        let epoch = queue.start_worker().unwrap();
        let mut executed = Vec::new();
        if trim_already_running {
            let running = queue.take_next_current(epoch, 5).unwrap();
            executed.extend(preview_commands_from_playback(
                running.command,
                running.precise_seek,
            ));
        }
        queue.push(request(PlaybackMediaCommand::seek_and_play(72.0), true, 1));
        // Ending the scrub increments trim generation. Queued old work
        // is discarded; an already running trim completes before final.
        while let Some(next) = queue.take_next_current(epoch, 6) {
            executed.extend(preview_commands_from_playback(
                next.command,
                next.precise_seek,
            ));
        }
        let mut expected = Vec::new();
        if trim_already_running {
            expected.extend([PreviewCommand::Pause, PreviewCommand::SeekPrecise(18.0)]);
        }
        expected.push(PreviewCommand::SeekAndPlay {
            seconds: 72.0,
            precise: true,
        });
        assert_eq!(executed, expected);
    }
}

#[test]
fn trim_restore_keeps_its_completion_boundary_between_transport_seeks() {
    let mut queue = PreviewMediaCommandQueue::default();
    queue.push(seek(10));
    queue.push(trim_seek(30.0, 5, false));
    queue.push(seek(40));
    let epoch = queue.start_worker().unwrap();
    assert_eq!(queue.take_next_current(epoch, 5), Some(seek(10)));
    assert_eq!(
        queue.take_next_current(epoch, 5),
        Some(trim_seek(30.0, 5, false))
    );
    assert_eq!(queue.take_next_current(epoch, 5), Some(seek(40)));
}

#[test]
fn ending_scrub_cancels_queued_trim_and_routes_final_seek_through_shared_worker() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        prepare_preview(root, true);
        let old_trim_generation = root.preview_ui.trim_preview_seek.generation;
        assert!(root.apply_preview_timeline_drag_with_context(TimelineDragTarget::Scrub, 0.2, cx));
        assert!(root.apply_preview_timeline_drag_with_context(TimelineDragTarget::Scrub, 0.8, cx));
        assert!(root.end_preview_timeline_drag_with_context(cx));
        assert_ne!(
            root.preview_ui.trim_preview_seek.generation,
            old_trim_generation
        );
        assert!(!root.preview_ui.trim_preview_seek.is_active());
        let final_request = root.preview_ui.media_commands.pending.back().unwrap();
        assert_eq!(
            final_request.command,
            PlaybackMediaCommand::seek_and_play(72.0)
        );
        assert!(final_request.precise_seek);
        assert_eq!(final_request.origin, MediaCommandOrigin::Playback);
    });
    app.run_until_parked();
    root.read_with(&app, |root, _| {
        assert!(!root.preview_ui.media_commands.is_active());
        assert!(root.preview_ui.media_commands.pending.is_empty());
        assert!(root.preview_ui.playback.is_playing());
        assert!((root.preview_ui.playback.current_time() - 72.0).abs() < 0.000_001);
    });
}

#[test]
fn cancelled_trim_throttle_timer_cannot_requeue_after_final_seek() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        prepare_preview(root, false);
        assert!(root.apply_preview_timeline_drag_with_context(TimelineDragTarget::Scrub, 0.2, cx));
    });
    // Complete the first decode and leave the 50 ms trim timer pending.
    app.run_until_parked();
    root.update(&mut app, |root, cx| {
        assert!(root.preview_ui.trim_preview_seek.worker_active);
        assert!(root.apply_preview_timeline_drag_with_context(TimelineDragTarget::Scrub, 0.8, cx));
        assert!(root.end_preview_timeline_drag_with_context(cx));
    });
    app.run_until_parked();
    for _ in 0..5 {
        if !app.dispatcher.advance_clock_to_next_timer() {
            break;
        }
        app.run_until_parked();
    }
    root.read_with(&app, |root, _| {
        assert!(!root.preview_ui.trim_preview_seek.is_active());
        assert!(!root.preview_ui.media_commands.is_active());
        assert!(root.preview_ui.media_commands.pending.is_empty());
        assert!((root.preview_ui.playback.current_time() - 72.0).abs() < 0.000_001);
    });
}

#[test]
fn trim_handle_restore_completes_through_shared_worker_after_throttle() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        prepare_preview(root, false);
        assert!(root.apply_preview_timeline_drag_with_context(TimelineDragTarget::End, 0.75, cx));
        assert!(root.end_preview_timeline_drag_with_context(cx));
        assert_eq!(
            root.preview_ui.trim_preview_seek.pending_seconds,
            Some(30.0)
        );
    });
    app.run_until_parked();
    for _ in 0..8 {
        if !app.dispatcher.advance_clock_to_next_timer() {
            break;
        }
        app.run_until_parked();
    }
    root.read_with(&app, |root, _| {
        assert!(!root.preview_ui.trim_preview_seek.is_active());
        assert!(!root.preview_ui.media_commands.is_active());
        assert!((root.preview_ui.playback.current_time() - 30.0).abs() < 0.000_001);
        assert_eq!(
            root.selected_config().unwrap().end_time.as_deref(),
            Some("00:01:07.500")
        );
    });
}

#[test]
fn blocked_seek_burst_decodes_first_and_latest_instead_of_all_twenty() {
    // Reproduce a decoder busy on the first request while the UI receives
    // nineteen further targets. The previous dispatcher submitted all
    // twenty tasks to the session mutex, so none could be superseded.
    let baseline: Vec<_> = (1..=20).map(seek).collect();
    let mut queue = PreviewMediaCommandQueue::default();
    queue.push(baseline[0]);
    let epoch = queue.start_worker().unwrap();
    let mut executed = vec![queue.take_next(epoch).unwrap()];
    for next in baseline.iter().skip(1) {
        queue.push(*next);
        assert!(
            queue.start_worker().is_none(),
            "only one worker may execute commands"
        );
    }
    while let Some(next) = queue.take_next(epoch) {
        executed.push(next);
    }
    assert_eq!(baseline.len(), 20);
    assert_eq!(executed, [seek(1), seek(20)]);
    assert!(!queue.is_active());
}

#[test]
fn latest_precise_seek_replaces_pending_fast_seek_without_losing_precision() {
    let mut queue = PreviewMediaCommandQueue::default();
    queue.push(seek(1));
    let epoch = queue.start_worker().unwrap();
    assert_eq!(queue.take_next(epoch), Some(seek(1)));
    queue.push(request(PlaybackMediaCommand::seek(2.0), false, 2));
    queue.push(request(PlaybackMediaCommand::seek(3.125), true, 3));
    let next = queue.take_next(epoch).unwrap();
    assert_eq!(next.generation, 3);
    assert_eq!(
        preview_commands_from_playback(next.command, next.precise_seek),
        [PreviewCommand::SeekPrecise(3.125)]
    );
}

#[test]
fn transport_boundaries_keep_fifo_order_and_batch_order() {
    let mut queue = PreviewMediaCommandQueue::default();
    let commands = [
        seek(1),
        seek(2),
        request(PlaybackMediaCommand::play(), true, 3),
        seek(4),
        seek(5),
        request(PlaybackMediaCommand::pause_and_seek(6.0), true, 6),
        seek(7),
        request(PlaybackMediaCommand::seek_and_play(8.0), true, 8),
        request(PlaybackMediaCommand::pause(), true, 9),
    ];
    for command in commands {
        queue.push(command);
    }
    let epoch = queue.start_worker().unwrap();
    let mut executed = Vec::new();
    while let Some(next) = queue.take_next(epoch) {
        executed.extend(preview_commands_from_playback(
            next.command,
            next.precise_seek,
        ));
    }
    assert_eq!(
        executed,
        [
            PreviewCommand::SeekPrecise(2.0),
            PreviewCommand::Play,
            PreviewCommand::SeekPrecise(5.0),
            PreviewCommand::Pause,
            PreviewCommand::SeekPrecise(6.0),
            PreviewCommand::SeekPrecise(7.0),
            PreviewCommand::SeekAndPlay {
                seconds: 8.0,
                precise: true,
            },
            PreviewCommand::Pause,
        ]
    );
}

#[test]
fn seek_and_play_mapping_preserves_precision_and_preceding_pause() {
    for precise in [false, true] {
        let atomic_seek = PreviewCommand::SeekAndPlay {
            seconds: 12.5,
            precise,
        };
        assert_eq!(
            preview_commands_from_playback(PlaybackMediaCommand::seek_and_play(12.5), precise),
            [atomic_seek],
        );
        assert_eq!(
            preview_commands_from_playback(
                PlaybackMediaCommand {
                    seek_to: Some(12.5),
                    pause: true,
                    play: true
                },
                precise,
            ),
            [PreviewCommand::Pause, atomic_seek],
        );
        assert_eq!(
            preview_commands_from_playback(PlaybackMediaCommand::play(), precise),
            [PreviewCommand::Play],
        );
        assert_eq!(
            preview_commands_from_playback(PlaybackMediaCommand::pause(), precise),
            [PreviewCommand::Pause],
        );
        let plain_seek = if precise {
            PreviewCommand::SeekPrecise(12.5)
        } else {
            PreviewCommand::SeekFast(12.5)
        };
        assert_eq!(
            preview_commands_from_playback(PlaybackMediaCommand::pause_and_seek(12.5), precise),
            [PreviewCommand::Pause, plain_seek],
        );
    }
}

#[test]
fn source_reset_discards_pending_work_and_invalidates_old_worker() {
    let mut queue = PreviewMediaCommandQueue::default();
    queue.push(seek(1));
    let old_epoch = queue.start_worker().unwrap();
    assert_eq!(queue.take_next(old_epoch), Some(seek(1)));
    queue.push(seek(2));
    queue.reset();
    queue.push(seek(3));
    let new_epoch = queue.start_worker().unwrap();
    assert_ne!(old_epoch, new_epoch);
    assert!(queue.take_next(old_epoch).is_none());
    assert!(queue.is_current_worker(new_epoch));
    assert_eq!(queue.take_next(new_epoch), Some(seek(3)));
    assert!(queue.take_next(new_epoch).is_none());
}

#[test]
fn no_op_does_not_start_worker_or_replace_pending_seek() {
    let mut queue = PreviewMediaCommandQueue::default();
    queue.push(request(PlaybackMediaCommand::none(), true, 1));
    assert!(queue.start_worker().is_none());
    queue.push(seek(2));
    queue.push(request(PlaybackMediaCommand::none(), true, 3));
    let epoch = queue.start_worker().unwrap();
    assert_eq!(queue.take_next(epoch), Some(seek(2)));
}

/// Run in isolation with `FRAME_PREVIEW_BENCH_PATH`, optionally overriding
/// `FRAME_PREVIEW_BENCH_WIDTH/HEIGHT/DURATION` (defaults: 3840/2160/12).
#[test]
#[ignore = "real FFmpeg benchmark; requires a fixture and exclusive benchmark time"]
fn benchmark_real_ffmpeg_seek_backlog() {
    use crate::conversion_runner::core_config_from_gpui;
    use crate::preview_engine::{PreviewSessionConfig, PreviewSourceKind};
    use crate::settings::ConversionConfig;

    let config = PreviewSessionConfig {
        file_id: "seek-backlog-benchmark".into(),
        path: std::env::var("FRAME_PREVIEW_BENCH_PATH")
            .expect("set FRAME_PREVIEW_BENCH_PATH to a video fixture")
            .into(),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(benchmark_env("FRAME_PREVIEW_BENCH_WIDTH", 3840)),
        source_height: Some(benchmark_env("FRAME_PREVIEW_BENCH_HEIGHT", 2160)),
        source_is_palette: false,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        selected_audio_track: None,
        duration_seconds: benchmark_env("FRAME_PREVIEW_BENCH_DURATION", 12.0),
        max_width: 1280,
        max_height: 720,
        fps: 30,
        conversion_config: core_config_from_gpui(&ConversionConfig::default()),
    };
    let mut fifo_ms = Vec::new();
    let mut coalesced_ms = Vec::new();
    for trial in 0..3 {
        // Alternate order so the candidate does not always inherit a
        // warmer filesystem/decoder cache than the FIFO baseline.
        let (fifo, coalesced) = if trial % 2 == 0 {
            (
                measure_seek_backlog(&config, false),
                measure_seek_backlog(&config, true),
            )
        } else {
            let coalesced = measure_seek_backlog(&config, true);
            (measure_seek_backlog(&config, false), coalesced)
        };
        assert_eq!(fifo.1, 20, "baseline launches a decoder for every request");
        assert_eq!(coalesced.1, 2, "candidate launches only first and latest");
        assert_eq!(fifo.2, coalesced.2, "final frame timestamps must match");
        assert_eq!(
            fifo.3, coalesced.3,
            "final decoded pixels must match exactly"
        );
        eprintln!(
            "seek backlog trial {}: FIFO={:.2} ms/{} decodes; coalesced={:.2} ms/{} decodes; final timestamp={} us; pixels identical",
            trial + 1,
            fifo.0,
            fifo.1,
            coalesced.0,
            coalesced.1,
            fifo.2
        );
        fifo_ms.push(fifo.0);
        coalesced_ms.push(coalesced.0);
    }
    fifo_ms.sort_by(f64::total_cmp);
    coalesced_ms.sort_by(f64::total_cmp);
    eprintln!(
        "seek backlog median of 3: FIFO={:.2} ms; coalesced={:.2} ms; speedup={:.2}x (headless blocked-worker burst, excludes startup and GPUI event timing)",
        fifo_ms[1],
        coalesced_ms[1],
        fifo_ms[1] / coalesced_ms[1]
    );
}

fn benchmark_env<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name).ok().map_or(default, |value| {
        value.parse().unwrap_or_else(|_| panic!("invalid {name}"))
    })
}

fn measure_seek_backlog(
    config: &crate::preview_engine::PreviewSessionConfig,
    coalesce: bool,
) -> (f64, u64, u64, Vec<u8>) {
    use crate::preview_engine::PreviewSession;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let session = Arc::new(PreviewSession::start(config.clone()).expect("start fixture"));
    let initial = session.latest_frame().expect("initial frame");
    session.mark_frame_presented(initial.generation);
    let before = session.snapshot().runtime_metrics.video_process_spawns;
    let requests: Vec<_> = (1_u32..=20)
        .map(|index| {
            request(
                PlaybackMediaCommand::seek(config.duration_seconds * f64::from(index) / 21.0),
                true,
                u64::from(index),
            )
        })
        .collect();
    let mut queue = PreviewMediaCommandQueue::default();
    queue.push(requests[0]);
    let epoch = queue.start_worker().unwrap();
    let first = queue.take_next(epoch).unwrap();
    let (release, blocked) = mpsc::channel();
    let first_worker = std::thread::spawn({
        let session = Arc::clone(&session);
        move || {
            blocked.recv().unwrap();
            for command in preview_commands_from_playback(first.command, first.precise_seek) {
                session.command(command).expect("first seek");
            }
        }
    });
    // Hold the first submitted decoder while all remaining UI targets
    // arrive, making the same backlog deterministic for both policies.
    let mut pending = Vec::new();
    for next in requests.into_iter().skip(1) {
        if coalesce {
            queue.push(next);
        } else {
            pending.push(next);
        }
    }
    let started = Instant::now();
    release.send(()).unwrap();
    first_worker.join().expect("first worker");
    let frame = session.latest_frame().expect("first seek frame");
    session.mark_frame_presented(frame.generation);
    if coalesce {
        while let Some(next) = queue.take_next(epoch) {
            pending.push(next);
        }
    }
    for next in pending {
        for command in preview_commands_from_playback(next.command, next.precise_seek) {
            session.command(command).expect("queued seek");
        }
        let frame = session.latest_frame().expect("queued seek frame");
        session.mark_frame_presented(frame.generation);
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let frame = session.latest_frame().expect("final frame");
    let final_time = config.duration_seconds * 20.0 / 21.0;
    assert!((session.snapshot().playback.position_seconds - final_time).abs() < 0.000_001);
    assert!(
        (Duration::from_micros(frame.frame.timestamp_us).as_secs_f64() - final_time).abs()
            < 0.000_002
    );
    let pixels = frame
        .frame
        .render_image()
        .as_bytes(0)
        .expect("BGRA frame")
        .to_vec();
    let spawns = session.snapshot().runtime_metrics.video_process_spawns - before;
    session.stop().expect("stop fixture");
    (elapsed_ms, spawns, frame.frame.timestamp_us, pixels)
}
