use super::audio_drain::AudioDrainState;
use super::audio_output::{
    AudioCallbackContext, AudioCallbackTiming, AudioOutputSpec, AudioSampleBuffer, f32_to_i16,
    f32_to_u16, fill_audio_output, lock_audio_buffer,
};
use super::audio_queue::{AudioQueueOutcome, AudioQueuePoll};
use super::clock::{AudioPlaybackStart, PreviewPlaybackClock, lock_playback};
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

const WATCHDOG: Duration = Duration::from_secs(4);

fn context(capacity: usize, samples: &[f32]) -> AudioCallbackContext {
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: false,
        ended: false,
        failed: false,
    }));
    let metrics = PreviewRuntimeMetricsStore::new();
    metrics.begin_generation(1);
    AudioCallbackContext {
        buffer: Arc::new(Mutex::new(AudioSampleBuffer {
            samples: samples.iter().copied().collect(),
            capacity,
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

fn wait_until(context: &AudioCallbackContext, predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + WATCHDOG;
    while !predicate() {
        assert!(Instant::now() < deadline, "producer test watchdog");
        thread::sleep(Duration::from_millis(1));
    }
    // The clock handoff proves any wait counter increment reached Condvar.
    drop(lock_playback(&context.playback));
}

fn check_fifo<T: Copy + Debug + PartialEq>(convert: fn(f32) -> T) {
    let context = context(6, &[]);
    let samples = [
        -1.0, 1.0, -0.75, 0.75, -0.5, 0.5, -0.25, 0.25, 0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4,
    ];
    let producer_context = context.clone();
    let (done_tx, done_rx) = mpsc::channel();
    let producer = thread::spawn(move || {
        let stop = AtomicBool::new(false);
        let mut admitted = 0;
        while admitted < samples.len() {
            let AudioQueueOutcome::Admitted(count) =
                producer_context.admit_samples(&samples[admitted..], &stop)
            else {
                panic!("FIFO producer cancelled")
            };
            producer_context
                .metrics
                .record_audio_samples_queued(1, count);
            admitted += count;
        }
        let _ = done_tx.send(admitted);
    });
    wait_until(&context, || {
        context.start.wait_count.load(Ordering::SeqCst) >= 1
    });
    let waits = context.start.wait_count.load(Ordering::SeqCst);
    thread::sleep(Duration::from_millis(30));
    assert_eq!(
        context.start.wait_count.load(Ordering::SeqCst),
        waits,
        "prewarm must have no periodic wakeups"
    );
    context.record_play_result(Ok(()), Instant::now()).unwrap();
    lock_playback(&context.playback).start_generation(1, 0.0);
    context.start.notify_output_changed();
    let mut actual = Vec::new();
    while actual.len() < samples.len() {
        let target = 6.min(samples.len() - actual.len());
        wait_until(&context, || {
            lock_audio_buffer(&context.buffer).samples.len() >= target
        });
        assert!(lock_audio_buffer(&context.buffer).samples.len() <= 6);
        let mut output = [convert(0.0); 5];
        fill_audio_output(
            &mut output,
            &context,
            AudioCallbackTiming {
                entered_at: Instant::now(),
                playback_delay: Duration::ZERO,
            },
            convert,
        );
        actual.extend_from_slice(&output[..4]);
        assert_eq!(
            output[4],
            convert(0.0),
            "never emit half an interleaved frame"
        );
    }
    assert_eq!(done_rx.recv_timeout(WATCHDOG).unwrap(), samples.len());
    producer.join().unwrap();
    assert_eq!(actual, samples.map(convert));
    assert_eq!(context.metrics.snapshot().audio_samples_queued, 16);
    assert_eq!(context.metrics.snapshot().audio_samples_consumed, 16);
    let buffer = lock_audio_buffer(&context.buffer);
    assert!(buffer.samples.is_empty());
    assert!(!buffer.producer_waiting);
    drop(buffer);
}

#[test]
fn bounded_queue_preserves_oversized_fifo_chunks_and_wraparound_in_all_formats() {
    check_fifo::<f32>(std::convert::identity);
    check_fifo::<f64>(f64::from);
    check_fifo::<i16>(f32_to_i16);
    check_fifo::<u16>(f32_to_u16);
}

#[test]
fn cancellation_before_or_during_capacity_wait_never_appends_remaining_samples() {
    for before_wait in [false, true] {
        for event in ["stop", "generation", "ended", "failed", "output failure"] {
            let context = context(2, &[0.25, -0.25]);
            let stop = Arc::new(AtomicBool::new(false));
            let cancel = || match event {
                "stop" => context.start.request_stop(&stop),
                "generation" => {
                    lock_playback(&context.playback).prepare_generation(1.0);
                    context.start.notify_output_changed();
                }
                "ended" => context.start.mark_ended(1),
                "failed" => {
                    lock_playback(&context.playback).failed = true;
                    context.start.notify_output_changed();
                }
                "output failure" => context.start.mark_output_failed(None),
                _ => unreachable!(),
            };
            if before_wait {
                cancel();
            }
            let producer_context = context.clone();
            let producer_stop = Arc::clone(&stop);
            let (done_tx, done_rx) = mpsc::channel();
            let producer = thread::spawn(move || {
                let _ =
                    done_tx.send(producer_context.admit_samples(&[0.75, -0.75], &producer_stop));
            });
            if !before_wait {
                wait_until(&context, || {
                    context.start.wait_count.load(Ordering::SeqCst) > 0
                });
                // A spurious notification must recheck fullness and repark.
                context.start.notify_output_changed();
                wait_until(&context, || {
                    context.start.wait_count.load(Ordering::SeqCst) > 1
                });
                cancel();
            }
            let outcome = done_rx
                .recv_timeout(WATCHDOG)
                .expect("cancelled producer watchdog");
            producer.join().unwrap();
            assert!(matches!(
                outcome,
                AudioQueueOutcome::Cancelled | AudioQueueOutcome::Failed
            ));
            let buffer = lock_audio_buffer(&context.buffer);
            assert_eq!(buffer.samples, VecDeque::from([0.25, -0.25]), "{event}");
            assert!(!buffer.producer_waiting, "{event}");
            drop(buffer);
        }
    }
}

#[test]
fn producer_watchdog_ignores_spurious_wakes_and_admission_but_tracks_consumption() {
    let context = context(4, &[0.1, -0.1, 0.2, -0.2]);
    let base = Instant::now();
    {
        let mut buffer = lock_audio_buffer(&context.buffer);
        assert_eq!(
            buffer.admit_samples(&[0.3, -0.3], base),
            AudioQueuePoll::Pending(None)
        );
        assert_eq!(
            buffer.admit_samples(&[0.3, -0.3], base + Duration::from_mins(1)),
            AudioQueuePoll::Pending(None)
        );
        drop(buffer);
    }
    context.record_play_result(Ok(()), base).unwrap();
    lock_playback(&context.playback).start_generation(1, 0.0);
    {
        let mut buffer = lock_audio_buffer(&context.buffer);
        for milliseconds in [0, 500, 1_000, 1_999] {
            assert_eq!(
                buffer.admit_samples(&[0.3, -0.3], base + Duration::from_millis(milliseconds)),
                AudioQueuePoll::Pending(Some(base + Duration::from_secs(2)))
            );
        }
        drop(buffer);
    }
    fill_audio_output(
        &mut [0.0_f32; 2],
        &context,
        AudioCallbackTiming {
            entered_at: base + Duration::from_millis(1_900),
            playback_delay: Duration::ZERO,
        },
        std::convert::identity,
    );
    let mut buffer = lock_audio_buffer(&context.buffer);
    assert_eq!(
        buffer.admit_samples(&[0.3, -0.3], base + Duration::from_millis(1_950)),
        AudioQueuePoll::Admitted(2)
    );
    assert_eq!(
        buffer.admit_samples(&[0.4, -0.4], base + Duration::from_millis(3_899)),
        AudioQueuePoll::Pending(Some(base + Duration::from_millis(3_900)))
    );
    assert_eq!(
        buffer.admit_samples(&[0.4, -0.4], base + Duration::from_millis(3_900)),
        AudioQueuePoll::Failed
    );
    assert!(!buffer.producer_waiting);
    drop(buffer);
}

#[test]
fn producer_watchdog_reports_failure_with_audio_only_or_av_clock_semantics() {
    for audio_only in [false, true] {
        let mut context = context(2, &[0.25, -0.25]);
        context.failure_ends_playback = audio_only;
        lock_playback(&context.playback).start_generation(1, 0.0);
        context
            .record_play_result(
                Ok(()),
                if audio_only {
                    Instant::now()
                } else {
                    Instant::now().checked_sub(Duration::from_secs(2)).unwrap()
                },
            )
            .unwrap();
        let producer_context = context.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let producer = thread::spawn(move || {
            let _ =
                done_tx.send(producer_context.admit_samples(&[0.5, -0.5], &AtomicBool::new(false)));
        });
        assert_eq!(
            done_rx
                .recv_timeout(WATCHDOG)
                .expect("missing callback watchdog"),
            AudioQueueOutcome::Failed
        );
        producer.join().unwrap();
        assert_eq!(lock_playback(&context.playback).failed, audio_only);
        assert_eq!(lock_playback(&context.playback).playing, !audio_only);
        assert!(
            context
                .stderr
                .lock()
                .unwrap()
                .iter()
                .any(|line| line.contains("while buffering"))
        );
        assert_eq!(
            lock_audio_buffer(&context.buffer).samples,
            VecDeque::from([0.25, -0.25])
        );
    }
}

#[test]
fn bounded_queue_rounds_capacity_to_complete_frames_and_counts_only_admission() {
    let context = context(3, &[]);
    context.metrics.record_audio_pcm_chunk(1, 16);
    let stop = AtomicBool::new(false);
    let outcome = context.admit_samples(&[0.25, -0.25, 0.5, -0.5], &stop);
    assert_eq!(outcome, AudioQueueOutcome::Admitted(2));
    context.metrics.record_audio_samples_queued(1, 2);
    context.start.request_stop(&stop);
    assert_eq!(
        context.admit_samples(&[0.5, -0.5], &stop),
        AudioQueueOutcome::Cancelled
    );
    let metrics = context.metrics.snapshot();
    assert_eq!(metrics.audio_pcm_chunks, 1);
    assert_eq!(metrics.audio_pcm_bytes_read, 16);
    assert_eq!(metrics.audio_samples_queued, 2);
}

#[test]
fn capacity_notification_cannot_be_lost_before_inspection_or_before_sleep() {
    for before_inspection in [false, true] {
        let context = context(2, &[0.25, -0.25]);
        let producer_context = context.clone();
        let consumer_context = context.clone();
        let (inspect_tx, inspect_rx) = mpsc::channel();
        let (freed_tx, freed_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let consumer = thread::spawn(move || {
            inspect_rx.recv_timeout(WATCHDOG).unwrap();
            // Reproduce the callback's buffer mutation after its playback
            // check. Its later clock acquisition must wait for the producer.
            {
                let mut buffer = lock_audio_buffer(&consumer_context.buffer);
                assert!(buffer.producer_waiting || before_inspection);
                buffer.samples.pop_front();
                buffer.samples.pop_front();
            }
            freed_tx.send(()).unwrap();
            consumer_context.start.notify_output_changed();
        });
        let producer = thread::spawn(move || {
            let mut first_poll = true;
            let outcome =
                producer_context
                    .start
                    .wait_for_audio_capacity(&AtomicBool::new(false), 1, |now| {
                        if first_poll && before_inspection {
                            inspect_tx.send(()).unwrap();
                            freed_rx.recv_timeout(WATCHDOG).unwrap();
                        }
                        let poll = lock_audio_buffer(&producer_context.buffer)
                            .admit_samples(&[0.5, -0.5], now);
                        if first_poll && !before_inspection {
                            assert_eq!(poll, AudioQueuePoll::Pending(None));
                            inspect_tx.send(()).unwrap();
                            freed_rx.recv_timeout(WATCHDOG).unwrap();
                        }
                        first_poll = false;
                        poll
                    });
            done_tx.send(outcome).unwrap();
        });
        assert_eq!(
            done_rx
                .recv_timeout(WATCHDOG)
                .expect("lost capacity notification"),
            AudioQueueOutcome::Admitted(2)
        );
        producer.join().unwrap();
        consumer.join().unwrap();
        assert_eq!(
            context.start.wait_count.load(Ordering::SeqCst),
            usize::from(!before_inspection)
        );
        let buffer = lock_audio_buffer(&context.buffer);
        assert_eq!(buffer.samples, VecDeque::from([0.5, -0.5]));
        assert!(!buffer.producer_waiting);
        drop(buffer);
    }
}

#[test]
fn video_end_cancels_a_full_producer_with_an_exact_emitted_prefix() {
    let context = context(4, &[]);
    let samples = [0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4, 0.5, -0.5];
    let producer_context = context.clone();
    let (done_tx, done_rx) = mpsc::channel();
    let producer = thread::spawn(move || {
        let mut admitted = 0;
        loop {
            match producer_context.admit_samples(&samples[admitted..], &AtomicBool::new(false)) {
                AudioQueueOutcome::Admitted(count) => {
                    admitted += count;
                    producer_context
                        .metrics
                        .record_audio_samples_queued(1, count);
                }
                outcome => {
                    done_tx.send((outcome, admitted)).unwrap();
                    break;
                }
            }
        }
    });
    wait_until(&context, || {
        context.start.wait_count.load(Ordering::SeqCst) > 0
    });
    context.record_play_result(Ok(()), Instant::now()).unwrap();
    lock_playback(&context.playback).start_generation(1, 0.0);
    let mut first = [0.0_f32; 3];
    fill_audio_output(
        &mut first,
        &context,
        AudioCallbackTiming {
            entered_at: Instant::now(),
            playback_delay: Duration::ZERO,
        },
        std::convert::identity,
    );
    assert_eq!(
        first.map(f32::to_bits),
        [samples[0], samples[1], 0.0].map(f32::to_bits)
    );
    wait_until(&context, || {
        context.metrics.snapshot().audio_samples_queued == 6
            && lock_audio_buffer(&context.buffer).producer_waiting
    });
    context.start.mark_ended(1);
    assert_eq!(
        done_rx
            .recv_timeout(WATCHDOG)
            .expect("video boundary cancellation"),
        (AudioQueueOutcome::Cancelled, 6)
    );
    producer.join().unwrap();
    let before = context.metrics.snapshot();
    for _ in 0..3 {
        let mut padding = [1.0_f32; 4];
        fill_audio_output(
            &mut padding,
            &context,
            AudioCallbackTiming {
                entered_at: Instant::now(),
                playback_delay: Duration::ZERO,
            },
            std::convert::identity,
        );
        assert_eq!(padding.map(f32::to_bits), [0.0_f32; 4].map(f32::to_bits));
    }
    let after = context.metrics.snapshot();
    assert_eq!(after.audio_samples_consumed, 2);
    assert_eq!(after.audio_output_callbacks, before.audio_output_callbacks);
    let buffer = lock_audio_buffer(&context.buffer);
    assert_eq!(buffer.samples, VecDeque::from(samples[2..6].to_vec()));
    assert!(!buffer.producer_waiting);
    assert!(
        !buffer.drain.pipe_exhausted,
        "video cancellation must not publish audio EOF"
    );
    drop(buffer);
}
