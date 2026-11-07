use super::audio_drain::{AudioDrainOutcome, AudioDrainPoll, AudioDrainState};
use super::audio_output::{
    AudioCallbackContext, AudioCallbackTiming, AudioOutputSpec, AudioSampleBuffer, f32_to_i16,
    f32_to_u16, fill_audio_output, lock_audio_buffer, push_audio_sample_bytes,
};
use super::clock::{
    AudioPlaybackStart, PlaybackStreamState, PreviewPlaybackClock, lock_playback,
    wait_for_playback_start,
};
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

fn new_context(samples: &[f32]) -> AudioCallbackContext {
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: true,
        ended: false,
        failed: false,
    }));
    let metrics = PreviewRuntimeMetricsStore::new();
    metrics.begin_generation(1);
    AudioCallbackContext {
        buffer: Arc::new(Mutex::new(AudioSampleBuffer {
            samples: samples.iter().copied().collect(),
            capacity: 32,
            spec: AudioOutputSpec {
                sample_rate: 1_000,
                channels: 2,
            },
            drain: AudioDrainState::default(),
            producer_waiting: false,
        })),
        metrics,
        start: Arc::new(AudioPlaybackStart::new(
            Arc::clone(&playback),
            Arc::new(Condvar::new()),
        )),
        playback,
        generation: 1,
        stderr: Arc::new(Mutex::new(VecDeque::new())),
        failure_ends_playback: true,
    }
}

fn timing(base: Instant, entered_ms: u64, delay_ms: u64) -> AudioCallbackTiming {
    AudioCallbackTiming {
        entered_at: base + Duration::from_millis(entered_ms),
        playback_delay: Duration::from_millis(delay_ms),
    }
}

fn check_format<T: Copy + PartialEq + Debug>(convert: fn(f32) -> T, expected: [T; 6], silence: T) {
    let context = new_context(&[-1.0, 1.0, 0.5, -0.5, 0.25, -0.25]);
    let base = Instant::now();
    context.record_play_result(Ok(()), base).unwrap();
    lock_audio_buffer(&context.buffer).drain.pipe_exhausted = true;
    let mut first = [silence; 4];
    fill_audio_output(&mut first, &context, timing(base, 100, 30), convert);
    assert_eq!(first, expected[..4]);
    let mut last = [silence; 8];
    fill_audio_output(&mut last, &context, timing(base, 110, 20), convert);
    assert_eq!(&last[..2], &expected[4..]);
    assert!(last[2..].iter().all(|sample| *sample == silence));
    assert_eq!(context.metrics.snapshot().audio_samples_consumed, 6);
    assert_eq!(context.metrics.snapshot().audio_output_underruns, 0);
    let mut padding = [silence; 8];
    fill_audio_output(&mut padding, &context, timing(base, 500, 100), convert);
    assert!(padding.iter().all(|sample| *sample == silence));
    assert_eq!(context.metrics.snapshot().audio_samples_consumed, 6);
    let mut buffer = lock_audio_buffer(&context.buffer);
    assert!(buffer.samples.is_empty());
    assert_eq!(
        buffer.drain.poll(base + Duration::from_millis(131), 0),
        AudioDrainPoll::Pending(base + Duration::from_millis(132))
    );
    assert_eq!(
        buffer.drain.poll(base + Duration::from_millis(132), 0),
        AudioDrainPoll::Drained
    );
    drop(buffer);
}

#[test]
fn audio_drain_preserves_pcm_order_and_partial_callbacks_in_all_formats() {
    check_format::<f32>(
        std::convert::identity,
        [-1.0, 1.0, 0.5, -0.5, 0.25, -0.25],
        0.0,
    );
    check_format::<f64>(f64::from, [-1.0, 1.0, 0.5, -0.5, 0.25, -0.25], 0.0);
    check_format::<i16>(f32_to_i16, [-32767, 32767, 16384, -16384, 8192, -8192], 0);
    check_format::<u16>(f32_to_u16, [0, 65535, 49151, 16384, 40959, 24576], 32768);
}

#[test]
fn audio_drain_waits_until_135ms_for_five_real_frames_without_counting_padding() {
    let context = new_context(&[0.25; 10]);
    let base = Instant::now();
    context.record_play_result(Ok(()), base).unwrap();
    lock_audio_buffer(&context.buffer).drain.pipe_exhausted = true;
    fill_audio_output(
        &mut [0.0_f32; 16],
        &context,
        timing(base, 100, 30),
        std::convert::identity,
    );
    let mut buffer = lock_audio_buffer(&context.buffer);
    assert_eq!(
        buffer.drain.poll(base + Duration::from_millis(134), 0),
        AudioDrainPoll::Pending(base + Duration::from_millis(135))
    );
    assert_eq!(
        buffer.drain.poll(base + Duration::from_millis(135), 0),
        AudioDrainPoll::Drained
    );
    drop(buffer);
}

#[test]
fn audio_drain_saturates_negative_device_delay_and_never_consumes_half_an_interleaved_frame() {
    let context = new_context(&[0.25, -0.25, 0.5, -0.5]);
    let now = Instant::now();
    let timing = AudioCallbackTiming::from_timestamp(
        now,
        cpal::OutputStreamTimestamp {
            callback: cpal::StreamInstant::from_nanos(20),
            playback: cpal::StreamInstant::from_nanos(10),
        },
    );
    assert_eq!(timing.playback_delay, Duration::ZERO);
    let mut data = [1.0_f32; 3];
    fill_audio_output(&mut data, &context, timing, std::convert::identity);
    assert_eq!(
        data.map(f32::to_bits),
        [0.25_f32, -0.25, 0.0].map(f32::to_bits)
    );
    assert_eq!(
        lock_audio_buffer(&context.buffer).samples,
        VecDeque::from([0.5, -0.5])
    );
    assert_eq!(context.metrics.snapshot().audio_samples_consumed, 2);
}

#[test]
fn audio_drain_counts_starvation_only_while_reading_and_never_consumes_partial_frames() {
    let context = new_context(&[]);
    let bytes = [0.25_f32, -0.5, 0.75]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect::<Vec<_>>();
    assert_eq!(push_audio_sample_bytes(&context.buffer, &bytes), 2);
    let base = Instant::now();
    let mut data = [0.0_f32; 4];
    fill_audio_output(
        &mut data,
        &context,
        timing(base, 0, 0),
        std::convert::identity,
    );
    assert_eq!(
        data.map(f32::to_bits),
        [0.25_f32, -0.5, 0.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(context.metrics.snapshot().audio_output_underruns, 1);
    lock_audio_buffer(&context.buffer).drain.pipe_exhausted = true;
    fill_audio_output(
        &mut data,
        &context,
        timing(base, 1, 0),
        std::convert::identity,
    );
    assert_eq!(context.metrics.snapshot().audio_output_underruns, 1);
    assert_eq!(context.metrics.snapshot().audio_samples_consumed, 2);
}

#[test]
fn audio_drain_watchdog_tracks_real_progress_instead_of_total_drain_duration() {
    let base = Instant::now();
    let mut stalled = AudioDrainState::default();
    stalled.started(base, 10);
    stalled.consumed(
        base + Duration::from_millis(1_900),
        Duration::ZERO,
        0,
        10,
        1_000,
    );
    assert_eq!(
        stalled.poll(base + Duration::from_millis(1_999), 10),
        AudioDrainPoll::Pending(base + Duration::from_secs(2))
    );
    assert_eq!(
        stalled.poll(base + Duration::from_secs(2), 10),
        AudioDrainPoll::Failed
    );

    let mut healthy = AudioDrainState::default();
    healthy.started(base, 10_000);
    for step in 1..=8_u64 {
        let now = base + Duration::from_millis(step * 500);
        healthy.consumed(now, Duration::ZERO, 100, 9_000, 1_000);
        assert_eq!(
            healthy.poll(now, 9_000),
            AudioDrainPoll::Pending(now + Duration::from_secs(2))
        );
    }
    // A device tail is independent of the no-progress budget once PCM is empty.
    healthy.consumed(
        base + Duration::from_secs(5),
        Duration::from_secs(4),
        1,
        0,
        1_000,
    );
    assert_eq!(
        healthy.poll(base + Duration::from_secs(8), 0),
        AudioDrainPoll::Pending(base + Duration::from_millis(9_001))
    );
}

fn wait_until_parked(context: &AudioCallbackContext) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while context.start.wait_count.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "drain did not block");
        thread::sleep(Duration::from_millis(1));
    }
    drop(lock_playback(&context.playback));
}

#[test]
fn audio_drain_cancellation_and_failure_wake_both_queue_and_hardware_tail_waits() {
    for hardware_tail in [false, true] {
        for event in ["stop", "stale", "video-ended", "output-error"] {
            let context = new_context(&[0.25, -0.25]);
            let base = Instant::now();
            context.record_play_result(Ok(()), base).unwrap();
            if hardware_tail {
                fill_audio_output(
                    &mut [0.0_f32; 2],
                    &context,
                    timing(base, 0, 10_000),
                    std::convert::identity,
                );
            }
            let stop = Arc::new(AtomicBool::new(false));
            let worker_context = context.clone();
            let worker_stop = Arc::clone(&stop);
            let (done_tx, done) = mpsc::channel();
            let worker = thread::spawn(move || {
                let _ = done_tx.send(worker_context.drain_after_pipe_exhausted(&worker_stop));
            });
            wait_until_parked(&context);
            assert!(matches!(done.try_recv(), Err(mpsc::TryRecvError::Empty)));
            match event {
                "stop" => context.start.request_stop(&stop),
                "stale" => {
                    lock_playback(&context.playback).generation = 2;
                    context.start.notify_output_changed();
                }
                "video-ended" => context.start.mark_ended(1),
                "output-error" => context.fail("injected callback failure".into()),
                _ => unreachable!(),
            }
            let expected = if event == "output-error" {
                AudioDrainOutcome::Failed
            } else {
                AudioDrainOutcome::Cancelled
            };
            assert_eq!(
                done.recv_timeout(Duration::from_secs(1))
                    .expect("drain cancellation watchdog"),
                expected
            );
            worker.join().unwrap();
            assert_eq!(
                context.metrics.snapshot().audio_samples_consumed,
                if hardware_tail { 2 } else { 0 }
            );
        }
    }
}

#[test]
fn audio_drain_play_and_callback_failures_are_reported_and_release_prestart_waits() {
    let context = new_context(&[0.25, -0.25]);
    assert!(
        context
            .record_play_result(Err("injected Play failure".into()), Instant::now())
            .is_err()
    );
    assert_eq!(
        context.drain_after_pipe_exhausted(&AtomicBool::new(false)),
        AudioDrainOutcome::Failed
    );
    assert_eq!(context.stderr.lock().unwrap().len(), 1);

    let context = new_context(&[0.25, -0.25]);
    lock_playback(&context.playback).playing = false;
    let worker_context = context.clone();
    let (done_tx, done) = mpsc::channel();
    let worker = thread::spawn(move || {
        let result = wait_for_playback_start(&worker_context.start, &AtomicBool::new(false), 1);
        let _ = done_tx.send(result);
    });
    wait_until_parked(&context);
    context.fail("injected callback failure before Play".into());
    assert_eq!(
        done.recv_timeout(Duration::from_secs(1))
            .expect("failed prewarm watchdog"),
        PlaybackStreamState::Stale
    );
    worker.join().unwrap();
    assert_eq!(context.stderr.lock().unwrap().len(), 1);
}

#[test]
fn audio_failure_freezes_position_and_remains_sticky_until_a_new_generation() {
    let context = new_context(&[]);
    {
        let mut clock = lock_playback(&context.playback);
        clock.base_seconds = 1.0;
        clock.last_frame_seconds = 1.25;
        clock.started_at = Instant::now()
            .checked_sub(Duration::from_millis(500))
            .unwrap();
    }
    context.fail("injected audio failure".into());
    let reader = super::clock::PreviewPlaybackReader {
        clock: Arc::clone(&context.playback),
    };
    let failed = reader.snapshot(10.0, true);
    assert!(!failed.playing);
    assert!(failed.position_seconds >= 1.5 && failed.position_seconds < 2.0);
    {
        let mut clock = lock_playback(&context.playback);
        clock.start_generation(1, 8.0);
        clock.park_generation(1, 9.0);
        assert!(clock.failed);
        drop(clock);
    }
    context.start.mark_ended(1);
    let after_late_events = reader.snapshot(10.0, true);
    assert!(!after_late_events.playing);
    assert_eq!(
        after_late_events.position_seconds.to_bits(),
        failed.position_seconds.to_bits()
    );
    assert!(!lock_playback(&context.playback).ended);
    let generation = lock_playback(&context.playback).prepare_generation(0.25);
    lock_playback(&context.playback).start_generation(generation, 0.25);
    context.start.mark_output_failed(Some(1));
    assert!(
        reader.snapshot(10.0, true).playing,
        "old callback cannot fail a fresh generation"
    );
    assert!(!lock_playback(&context.playback).failed);
}

#[test]
fn audio_failure_in_av_keeps_video_playing() {
    let mut context = new_context(&[0.25, -0.25]);
    context.failure_ends_playback = false;
    context.fail("injected AV audio failure".into());
    let clock = lock_playback(&context.playback);
    assert!(clock.playing);
    assert!(!clock.failed);
    drop(clock);
    assert_eq!(
        context.drain_after_pipe_exhausted(&AtomicBool::new(false)),
        AudioDrainOutcome::Failed
    );
}

#[test]
fn audio_callbacks_before_play_returns_keep_their_deadline_or_failure() {
    let context = new_context(&[0.25, -0.25]);
    let base = Instant::now();
    fill_audio_output(
        &mut [0.0_f32; 2],
        &context,
        timing(base, 100, 30),
        std::convert::identity,
    );
    context
        .record_play_result(Ok(()), base + Duration::from_millis(200))
        .unwrap();
    assert_eq!(
        lock_audio_buffer(&context.buffer)
            .drain
            .poll(base + Duration::from_millis(130), 0),
        AudioDrainPoll::Pending(base + Duration::from_millis(131))
    );

    let context = new_context(&[0.25, -0.25]);
    context.fail("callback failed during Play".into());
    assert!(context.record_play_result(Ok(()), Instant::now()).is_err());
    assert!(lock_playback(&context.playback).failed);
    assert!(!lock_playback(&context.playback).playing);
}

#[test]
fn audio_drain_rounds_fractional_frame_deadlines_up() {
    let base = Instant::now();
    let mut drain = AudioDrainState::default();
    drain.started(base, 1);
    drain.consumed(base, Duration::ZERO, 1, 0, 48_000);
    assert_eq!(
        drain.poll(base + Duration::from_nanos(20_833), 0),
        AudioDrainPoll::Pending(base + Duration::from_nanos(20_834))
    );
}

#[test]
fn audio_output_stall_reports_failure_and_stops_the_audio_only_snapshot() {
    let context = new_context(&[0.25, -0.25]);
    let started = Instant::now().checked_sub(Duration::from_secs(3)).unwrap();
    context.record_play_result(Ok(()), started).unwrap();
    assert_eq!(
        context.drain_after_pipe_exhausted(&AtomicBool::new(false)),
        AudioDrainOutcome::Failed
    );
    let reader = super::clock::PreviewPlaybackReader {
        clock: Arc::clone(&context.playback),
    };
    assert!(!reader.snapshot(10.0, true).playing);
    assert!(!lock_playback(&context.playback).ended);
    assert_eq!(lock_audio_buffer(&context.buffer).samples.len(), 2);
    assert!(
        context
            .stderr
            .lock()
            .unwrap()
            .front()
            .unwrap()
            .contains("no progress")
    );
}

struct FailingReader {
    prebuffer: Cursor<Vec<u8>>,
    after_interrupt: Cursor<Vec<u8>>,
    interrupted: bool,
    error_kind: io::ErrorKind,
    playback: Arc<Mutex<PreviewPlaybackClock>>,
    failed_while_playing: Arc<AtomicBool>,
}

impl Read for FailingReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.prebuffer.read(bytes)?;
        if count > 0 {
            return Ok(count);
        }
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        let count = self.after_interrupt.read(bytes)?;
        if count > 0 {
            return Ok(count);
        }
        self.failed_while_playing
            .store(lock_playback(&self.playback).playing, Ordering::SeqCst);
        Err(io::Error::new(self.error_kind, "injected PCM read failure"))
    }
}

#[test]
#[ignore = "requires a default audio output device; run with --ignored"]
fn audio_stdout_read_errors_fail_after_ready_without_publishing_eof() {
    use super::audio_stream::{
        AudioStdoutWorkerConfig, AudioWorkerStatus, audio_prebuffer_samples,
        spawn_audio_stdout_worker,
    };

    for audio_only in [true, false] {
        for error_kind in [
            io::ErrorKind::Other,
            io::ErrorKind::UnexpectedEof,
            io::ErrorKind::BrokenPipe,
        ] {
            let context = new_context(&[]);
            lock_playback(&context.playback).playing = false;
            let output_spec = AudioOutputSpec::default_output().expect("audio output device");
            let prebuffer_samples = audio_prebuffer_samples(output_spec);
            let extra_samples = usize::from(output_spec.channels) * 2;
            let failed_while_playing = Arc::new(AtomicBool::new(false));
            let stop = Arc::new(AtomicBool::new(false));
            let (ready_tx, ready_rx) = mpsc::channel();
            let worker = spawn_audio_stdout_worker(
                FailingReader {
                    prebuffer: Cursor::new(vec![0; prebuffer_samples * 4]),
                    after_interrupt: Cursor::new(vec![0; extra_samples * 4]),
                    interrupted: false,
                    error_kind,
                    playback: Arc::clone(&context.playback),
                    failed_while_playing: Arc::clone(&failed_while_playing),
                },
                AudioStdoutWorkerConfig {
                    stderr_lines: Arc::clone(&context.stderr),
                    playback: Arc::clone(&context.playback),
                    metrics: context.metrics.clone(),
                    stop_requested: Arc::clone(&stop),
                    audio_start: Arc::clone(&context.start),
                    mark_ended_on_eof: audio_only,
                    generation: 1,
                    prebuffer_samples,
                    ready_tx,
                },
            )
            .expect("audio stdout worker");
            let (done_tx, done) = mpsc::channel();
            let joiner = thread::spawn(move || {
                let _ = done_tx.send(worker.join());
            });
            assert_eq!(
                ready_rx
                    .recv_timeout(Duration::from_secs(3))
                    .expect("ready watchdog"),
                AudioWorkerStatus::Ready
            );
            lock_playback(&context.playback).start_generation(1, 0.0);
            context.start.notify_output_changed();
            let completion = done.recv_timeout(Duration::from_secs(3));
            if completion.is_err() {
                context.start.request_stop(&stop);
            }
            completion
                .expect("read failure worker watchdog")
                .expect("audio worker panicked");
            joiner.join().unwrap();
            assert!(
                failed_while_playing.load(Ordering::SeqCst),
                "read failure must follow successful output startup"
            );
            assert_eq!(
                context.metrics.snapshot().audio_samples_queued,
                u64::try_from(prebuffer_samples + extra_samples).unwrap(),
                "Interrupted must retry without losing subsequent PCM"
            );
            let reader = super::clock::PreviewPlaybackReader {
                clock: Arc::clone(&context.playback),
            };
            let snapshot = reader.snapshot(10.0, true);
            assert_eq!(snapshot.playing, !audio_only);
            assert_eq!(lock_playback(&context.playback).failed, audio_only);
            assert!(
                !lock_playback(&context.playback).ended,
                "read errors must not publish natural EOF"
            );
            assert!(snapshot.position_seconds < snapshot.duration_seconds);
            assert!(
                context
                    .stderr
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|line| line.contains("injected PCM read failure"))
            );
        }
    }
}
