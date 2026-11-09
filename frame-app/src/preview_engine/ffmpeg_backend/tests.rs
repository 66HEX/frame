use super::audio_drain::AudioDrainState;
use super::audio_output::{AudioCallbackContext, AudioCallbackTiming, fill_audio_output};
use super::*;

fn callback_context(
    buffer: Arc<Mutex<AudioSampleBuffer>>,
    metrics: PreviewRuntimeMetricsStore,
    playback: Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> AudioCallbackContext {
    AudioCallbackContext {
        buffer,
        metrics,
        start: Arc::new(super::clock::AudioPlaybackStart::new(
            Arc::clone(&playback),
            Arc::new(std::sync::Condvar::new()),
        )),
        playback,
        generation,
        stderr: Arc::new(Mutex::new(VecDeque::new())),
        failure_ends_playback: true,
    }
}
use crate::conversion_runner::core_config_from_gpui;
use crate::settings::ConversionConfig;
use std::path::PathBuf;

pub(super) fn preview_config() -> PreviewSessionConfig {
    PreviewSessionConfig {
        file_id: "video-1".to_string(),
        path: PathBuf::from("/tmp/video.mp4"),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(1920),
        source_height: Some(1080),
        source_is_palette: false,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        selected_audio_track: None,
        duration_seconds: 12.5,
        max_width: 1280,
        max_height: 720,
        fps: 30,
        conversion_config: core_config_from_gpui(&ConversionConfig::default()),
    }
}

#[test]
fn initial_preview_seek_uses_raw_microseconds_while_explicit_playhead_keeps_rounding() {
    for (start, expected_seek) in [
        ("00:00:00.123456789", Some("0.123456")),
        (".123456789", Some("0.123456")),
        ("0.999999999", Some("0.999999")),
        ("-00:00:00.5", None),
        ("-.5", None),
        ("0.0000009", None),
    ] {
        let mut config = preview_config();
        config.conversion_config.start_time = Some(start.into());
        config.conversion_config.end_time = Some("1:60".into());
        config.conversion_config.subtitle_burn_path = Some("/tmp/sub.srt".into());
        let seconds = super::clock::initial_start_seconds(&config);
        let plan = preview_plan(&config, seconds, false, true).unwrap();
        assert_eq!(
            plan.args
                .windows(2)
                .find(|pair| pair[0] == "-ss")
                .map(|pair| pair[1].as_str()),
            expected_seek,
            "{start}"
        );
        assert_eq!(config.conversion_config.start_time.as_deref(), Some(start));
        let graph = plan
            .args
            .windows(2)
            .find(|pair| pair[0] == "-filter_complex")
            .unwrap();
        if let Some(seek) = expected_seek {
            assert!(
                graph[1].contains(&format!("setpts=PTS+(round({seek}/TB))")),
                "{start}"
            );
        } else {
            assert!(!graph[1].contains("setpts=PTS"), "{start}");
        }
        // END still follows the established arithmetic policy, not raw -ss grammar.
        assert!(
            super::clock::parsed_time(config.conversion_config.end_time.as_deref())
                .is_some_and(|value| (value - 120.0).abs() < f64::EPSILON)
        );
        config.selected_audio_track = Some(1);
        let audio = preview_audio_plan(
            &config,
            seconds,
            false,
            true,
            AudioOutputSpec {
                sample_rate: 48_000,
                channels: 2,
            },
        )
        .unwrap();
        assert_eq!(
            audio
                .args
                .windows(2)
                .find(|pair| pair[0] == "-ss")
                .map(|pair| pair[1].as_str()),
            expected_seek,
            "audio: {start}"
        );
    }
    let plan = preview_plan(&preview_config(), 0.123_456_789, false, true).unwrap();
    assert!(plan.args.windows(2).any(|pair| pair == ["-ss", "0.123457"]));
}

#[test]
fn tiff_preview_plan_uses_the_session_palette_fact_before_visual_effects() {
    let mut config = preview_config();
    config.source_is_palette = true;
    config.conversion_config.container = "tiff".into();
    config.conversion_config.flip_horizontal = true;
    let plan = super::preview_plan(&config, 0.0, false, true).unwrap();
    let graph = plan
        .args
        .windows(2)
        .find(|pair| pair[0] == "-filter_complex")
        .unwrap();
    assert!(graph[1].starts_with("[0:v:0]format=pix_fmts=rgba|rgba64le,hflip"));
    assert!(graph[1].contains(
        "format=pix_fmts=rgb24|rgb48le|rgba|rgba64le|gray|ya8|gray16le|ya16le|monob|monow"
    ));
    config.conversion_config.container = "png".into();
    let plan = super::preview_plan(&config, 0.0, false, true).unwrap();
    assert!(
        !plan
            .args
            .iter()
            .any(|arg| arg.contains("format=pix_fmts=rgba|rgba64le"))
    );
}

#[test]
fn insert_frame_limit_adds_single_frame_before_pipe_output() {
    let mut args = vec![
        "-f".to_string(),
        "rawvideo".to_string(),
        "pipe:1".to_string(),
    ];

    insert_frame_limit(&mut args, 1);

    assert_eq!(args[2], "-frames:v");
    assert_eq!(args[3], "1");
    assert_eq!(args.last(), Some(&"pipe:1".to_string()));
}

#[test]
fn preview_plan_uses_configured_trim_end_as_duration_bound() {
    let mut config = preview_config();
    config.conversion_config.end_time = Some("00:00:10.000".to_string());

    let plan = preview_plan(&config, 2.0, true, true).expect("plan");

    assert!(plan.args.windows(2).any(|args| args == ["-t", "8.000"]));
}

#[test]
fn preview_audio_plan_streams_selected_track_as_device_pcm() {
    let mut config = preview_config();
    config.selected_audio_track = Some(2);
    config.conversion_config.end_time = Some("00:00:05.000".to_string());

    let plan = preview_audio_plan(
        &config,
        1.0,
        true,
        true,
        AudioOutputSpec {
            sample_rate: 44_100,
            channels: 2,
        },
    )
    .expect("audio plan");

    assert!(plan.args.windows(2).any(|args| args == ["-map", "0:2"]));
    assert!(plan.args.windows(2).any(|args| args == ["-t", "4.000"]));
    assert!(plan.args.windows(2).any(|args| args == ["-ar", "44100"]));
    assert!(plan.args.windows(2).any(|args| args == ["-ac", "2"]));
    assert!(plan.args.windows(2).any(|args| args == ["-f", "f32le"]));
}

#[test]
fn preview_audio_plan_rejects_missing_explicit_track_selection() {
    let error = preview_audio_plan(
        &preview_config(),
        1.0,
        true,
        true,
        AudioOutputSpec {
            sample_rate: 44_100,
            channels: 2,
        },
    )
    .expect_err("audio preview should require an explicit source track");

    assert!(error.to_string().contains("explicitly selected"));
}

#[test]
fn preview_playback_clock_prepares_distinct_generations() {
    let mut clock = PreviewPlaybackClock {
        generation: 0,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: true,
        ended: true,
        failed: false,
    };

    let generation = clock.prepare_generation(2.5);

    assert_eq!(generation, 1);
    assert!((clock.base_seconds - 2.5).abs() <= f64::EPSILON);
    assert!((clock.last_frame_seconds - 2.5).abs() <= f64::EPSILON);
    assert!(!clock.playing);
    assert!(!clock.ended);
}

#[test]
fn playback_reader_tracks_pause_play_and_eof_without_process_access() {
    let reader = PreviewPlaybackReader {
        clock: Arc::new(Mutex::new(PreviewPlaybackClock {
            generation: 0,
            base_seconds: 2.5,
            last_frame_seconds: 2.5,
            started_at: Instant::now()
                .checked_sub(Duration::from_millis(500))
                .unwrap(),
            playing: false,
            ended: false,
            failed: false,
        })),
    };
    let paused = reader.snapshot(12.5, false);
    assert!((paused.position_seconds - 2.5).abs() <= f64::EPSILON);
    assert!(!paused.playing);

    lock_playback(&reader.clock).playing = true;
    let playing = reader.snapshot(12.5, true);
    assert!(playing.position_seconds >= 3.0);
    assert!(playing.playing);

    lock_playback(&reader.clock).playing = false;
    assert!(
        reader.snapshot(12.5, true).playing,
        "a parked rebuild keeps playback intent"
    );

    lock_playback(&reader.clock).ended = true;
    let ended = reader.snapshot(12.5, true);
    assert!((ended.position_seconds - 12.5).abs() <= f64::EPSILON);
    assert!(!ended.playing);
}

#[test]
fn playback_stream_state_waits_until_generation_is_started() {
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 0,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: true,
        ended: false,
        failed: false,
    }));
    let generation = lock_playback(&playback).prepare_generation(2.5);

    assert_eq!(
        playback_stream_state(&lock_playback(&playback), generation),
        PlaybackStreamState::Waiting
    );

    lock_playback(&playback).start_generation(generation, 2.5);

    assert_eq!(
        playback_stream_state(&lock_playback(&playback), generation),
        PlaybackStreamState::Ready
    );
}

#[test]
fn frame_presentation_state_waits_for_future_video_timestamp() {
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 10.0,
        last_frame_seconds: 10.0,
        started_at: Instant::now(),
        playing: true,
        ended: false,
        failed: false,
    }));

    assert_eq!(
        frame_presentation_state(&playback, 1, 10.5),
        PlaybackStreamState::Waiting
    );
}

#[test]
fn frame_presentation_state_allows_due_video_timestamp() {
    let started_at = Instant::now()
        .checked_sub(Duration::from_millis(40))
        .expect("test timestamp should be representable");
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 10.0,
        last_frame_seconds: 10.0,
        started_at,
        playing: true,
        ended: false,
        failed: false,
    }));

    assert_eq!(
        frame_presentation_state(&playback, 1, 10.033),
        PlaybackStreamState::Ready
    );
}

#[test]
fn wait_for_playback_start_returns_stale_when_stop_is_requested() {
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: false,
        ended: false,
        failed: false,
    }));
    let stop_requested = AtomicBool::new(true);
    let start =
        super::clock::AudioPlaybackStart::new(playback, Arc::new(std::sync::Condvar::new()));

    assert_eq!(
        wait_for_playback_start(&start, &stop_requested, 1),
        PlaybackStreamState::Stale
    );
}

#[test]
fn update_last_frame_seconds_rejects_stale_generations() {
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 2,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: true,
        ended: false,
        failed: false,
    }));

    let updated = update_last_frame_seconds(&playback, 1, 4.0);

    assert!(!updated);
    assert!(lock_playback(&playback).last_frame_seconds.abs() <= f64::EPSILON);
}

#[test]
fn audio_prebuffer_samples_uses_sample_rate_and_channels() {
    let samples = audio_prebuffer_samples(AudioOutputSpec {
        sample_rate: 48_000,
        channels: 2,
    });

    assert_eq!(samples, 2_880);
}

#[test]
fn push_audio_sample_bytes_clamps_samples_and_returns_count() {
    let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
        samples: VecDeque::new(),
        capacity: 4,
        spec: AudioOutputSpec {
            sample_rate: 48_000,
            channels: 1,
        },
        drain: AudioDrainState::default(),
        producer_waiting: false,
    }));
    let samples = [
        2.0_f32.to_le_bytes(),
        (-2.0_f32).to_le_bytes(),
        0.25_f32.to_le_bytes(),
    ]
    .concat();

    let count = push_audio_sample_bytes(&buffer, &samples);

    let queued = {
        let buffer = lock_audio_buffer(&buffer);
        buffer.samples.iter().copied().collect::<Vec<_>>()
    };
    assert_eq!(count, 3);
    assert!((queued[0] - 1.0).abs() <= f32::EPSILON);
    assert!((queued[1] + 1.0).abs() <= f32::EPSILON);
    assert!((queued[2] - 0.25).abs() <= f32::EPSILON);
}

#[test]
fn fill_audio_output_f32_mutes_when_playback_is_paused() {
    let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
        samples: VecDeque::from([0.5, -0.5]),
        capacity: 4,
        spec: AudioOutputSpec {
            sample_rate: 48_000,
            channels: 1,
        },
        drain: AudioDrainState::default(),
        producer_waiting: false,
    }));
    let metrics = PreviewRuntimeMetricsStore::new();
    metrics.begin_generation(1);
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: false,
        ended: false,
        failed: false,
    }));
    let mut data = [1.0_f32, 1.0];

    let context = callback_context(Arc::clone(&buffer), metrics.clone(), playback, 1);
    fill_audio_output(
        &mut data,
        &context,
        AudioCallbackTiming {
            entered_at: Instant::now(),
            playback_delay: Duration::ZERO,
        },
        std::convert::identity,
    );

    assert!(data.iter().all(|sample| sample.abs() <= f32::EPSILON));
    assert_eq!(lock_audio_buffer(&buffer).samples.len(), 2);
    assert_eq!(metrics.snapshot().audio_output_callbacks, 0);
}

#[test]
fn fill_audio_output_f32_drains_samples_when_playback_is_playing() {
    let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
        samples: VecDeque::from([0.25]),
        capacity: 4,
        spec: AudioOutputSpec {
            sample_rate: 48_000,
            channels: 1,
        },
        drain: AudioDrainState::default(),
        producer_waiting: false,
    }));
    let metrics = PreviewRuntimeMetricsStore::new();
    metrics.begin_generation(1);
    let playback = Arc::new(Mutex::new(PreviewPlaybackClock {
        generation: 1,
        base_seconds: 0.0,
        last_frame_seconds: 0.0,
        started_at: Instant::now(),
        playing: true,
        ended: false,
        failed: false,
    }));
    let mut data = [0.0_f32, 0.0];

    let context = callback_context(Arc::clone(&buffer), metrics.clone(), playback, 1);
    fill_audio_output(
        &mut data,
        &context,
        AudioCallbackTiming {
            entered_at: Instant::now(),
            playback_delay: Duration::ZERO,
        },
        std::convert::identity,
    );

    assert!((data[0] - 0.25).abs() <= f32::EPSILON);
    assert!(data[1].abs() <= f32::EPSILON);
    assert!(lock_audio_buffer(&buffer).samples.is_empty());
    assert_eq!(metrics.snapshot().audio_output_callbacks, 1);
}

#[test]
fn clamp_seek_seconds_keeps_seek_inside_duration() {
    let seconds = clamp_seek_seconds(12.5, 12.5).expect("seconds");

    assert!((seconds - 12.499).abs() <= f64::EPSILON);
}
