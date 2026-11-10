//! Measures the real `FFmpeg` preview pipeline without a window or GPU presentation.
//! Run with --help for arguments; JSON includes individual samples and percentiles.

use frame_app::conversion_runner::core_config_from_gpui;
use frame_app::preview_engine::{
    PreviewCommand, PreviewRenderedFrame, PreviewRuntimeMetrics, PreviewSession,
    PreviewSessionConfig, PreviewSourceKind,
};
use frame_app::runtime_binaries::ffmpeg_executable;
use frame_app::settings::ConversionConfig;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type BenchmarkResult<T> = Result<T, Box<dyn Error>>;

struct Options {
    path: PathBuf,
    width: u32,
    height: u32,
    duration: f64,
    iterations: u32,
    warmup: u32,
    max_width: u32,
    max_height: u32,
    fps: u32,
    playback_seconds: f64,
    timeout_seconds: u64,
    audio_track: Option<u32>,
    output: Option<PathBuf>,
}

struct IterationMeasurement {
    timings: BTreeMap<&'static str, f64>,
    playback: Value,
    seek_and_play: Value,
}

fn main() -> BenchmarkResult<()> {
    let Some(options) = parse_options()? else {
        return Ok(());
    };
    let tool_deadline = Deadline::new(options.timeout_seconds, "FFmpeg version query");
    let executable = ffmpeg_executable();
    let version = Command::new(&executable).arg("-version").output()?;
    if !version.status.success() {
        return Err("FFmpeg version query failed".into());
    }
    let version = String::from_utf8_lossy(&version.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    drop(tool_deadline);
    let config = session_config(&options);
    let mut samples: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    let mut iterations = Vec::new();
    for iteration in 0..options.warmup + options.iterations {
        let warmup = iteration < options.warmup;
        eprintln!(
            "preview benchmark iteration {} (warmup={warmup})",
            iteration + 1
        );
        let _deadline = Deadline::new(options.timeout_seconds, "preview iteration");
        let measurement = measure_iteration(&config, &options, iteration)?;
        if !warmup {
            for (name, duration) in &measurement.timings {
                samples.entry(name).or_default().push(*duration);
            }
            iterations.push(json!({
                "timings_ms": measurement.timings,
                "playback": measurement.playback,
                "seek_and_play": measurement.seek_and_play,
            }));
        }
    }
    let summary: BTreeMap<_, _> = samples
        .into_iter()
        .map(|(name, samples)| (name, summarize(&samples)))
        .collect();
    let report = json!({
        "schema_version": 1,
        "ffmpeg": {"executable": executable, "version": version},
        "host": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH},
        "config": {
            "path": options.path, "source_width": options.width,
            "source_height": options.height, "duration_seconds": options.duration,
            "max_width": options.max_width, "max_height": options.max_height,
            "fps": options.fps, "audio_track": options.audio_track,
            "iterations": options.iterations, "warmup_iterations": options.warmup,
            "playback_seconds": options.playback_seconds, "poll_interval_ms": 2,
        },
        "summary": summary,
        "iterations": iterations,
        "scope": "Headless engine wall time; frames are acknowledged by a 2 ms polling consumer. No GPUI window or GPU upload is measured. Optional audio runs through the real device with volume muted.",
    });
    let output = serde_json::to_string_pretty(&report)?;
    if let Some(path) = options.output {
        std::fs::write(path, &output)?;
    }
    println!("{output}");
    Ok(())
}

fn session_config(options: &Options) -> PreviewSessionConfig {
    let mut conversion_config = core_config_from_gpui(&ConversionConfig::default());
    if options.audio_track.is_some() {
        conversion_config.audio_volume = 0.0;
    }
    PreviewSessionConfig {
        file_id: "preview-benchmark".to_owned(),
        path: options.path.clone(),
        source_kind: PreviewSourceKind::Video,
        source_width: Some(options.width),
        source_height: Some(options.height),
        source_is_palette: false,
        source_sample_aspect_ratio: None,
        request_token: 0,
        source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
        preview_burn_clock: None,
        selected_audio_track: options.audio_track,
        duration_seconds: options.duration,
        max_width: options.max_width,
        max_height: options.max_height,
        fps: options.fps,
        conversion_config,
    }
}

fn measure_iteration(
    config: &PreviewSessionConfig,
    options: &Options,
    iteration: u32,
) -> BenchmarkResult<IterationMeasurement> {
    let mut timings = BTreeMap::new();
    let start = Instant::now();
    let session = PreviewSession::start(config.clone())?;
    timings.insert("startup_first_frame", elapsed_ms(start));
    acknowledge_new_frame(&session, 0)?;
    for (name, fraction, precise) in [
        ("seek_precise_20pct", 0.2, true),
        ("seek_precise_70pct", 0.7, true),
        ("seek_precise_40pct", 0.4, true),
        ("seek_fast_20pct", 0.2, false),
        ("seek_fast_70pct", 0.7, false),
        ("seek_fast_40pct", 0.4, false),
    ] {
        let before = session.frame_store().generation();
        let seconds = config.duration_seconds * fraction;
        let command = if precise {
            PreviewCommand::SeekPrecise(seconds)
        } else {
            PreviewCommand::SeekFast(seconds)
        };
        let start = Instant::now();
        session.command(command)?;
        timings.insert(name, elapsed_ms(start));
        acknowledge_new_frame(&session, before)?;
    }
    for (name, brightness) in [("filter_brightness_10", 10), ("filter_brightness_20", 20)] {
        let mut filtered = config.clone();
        filtered
            .conversion_config
            .video_filters
            .color
            .brightness
            .enabled = true;
        filtered
            .conversion_config
            .video_filters
            .color
            .brightness
            .value = brightness;
        let before = session.frame_store().generation();
        let start = Instant::now();
        session.reconfigure(filtered)?;
        timings.insert(name, elapsed_ms(start));
        acknowledge_new_frame(&session, before)?;
    }
    session.reconfigure(config.clone())?;
    session.command(PreviewCommand::SeekPrecise(0.0))?;
    let previous_generation = acknowledge_new_frame(&session, 0)?;
    let before_playback = session.snapshot().runtime_metrics;
    let start = Instant::now();
    session.command(PreviewCommand::Play)?;
    acknowledge_new_frame(&session, previous_generation)?;
    timings.insert("play_first_frame", elapsed_ms(start));
    let playback = observe_playback(&session, options.playback_seconds, before_playback)?;
    let start = Instant::now();
    session.command(PreviewCommand::Pause)?;
    timings.insert("pause", elapsed_ms(start));
    let start = Instant::now();
    session.stop()?;
    timings.insert("stop", elapsed_ms(start));
    // Keep the original observation and teardown samples unchanged. Both
    // paired paths share a fresh session, with identical unmeasured resets.
    let seek_and_play = measure_seek_and_play_pair(config, iteration)?;
    Ok(IterationMeasurement {
        timings,
        playback,
        seek_and_play,
    })
}

fn measure_seek_and_play_pair(
    config: &PreviewSessionConfig,
    iteration: u32,
) -> BenchmarkResult<Value> {
    let session = PreviewSession::start(config.clone())?;
    acknowledge_new_frame(&session, 0)?;
    let seconds = config.duration_seconds * 0.2;
    let order = if iteration.is_multiple_of(2) {
        [false, true]
    } else {
        [true, false]
    };
    let mut samples = BTreeMap::new();
    let mut reference_frame: Option<Arc<PreviewRenderedFrame>> = None;
    for atomic in order {
        let (sample, frame) = measure_seek_and_play_variant(&session, seconds, atomic)?;
        if let Some(reference) = &reference_frame {
            if reference.timestamp_us != frame.timestamp_us
                || reference.dimensions() != frame.dimensions()
                || reference.render_image().as_bytes(0) != frame.render_image().as_bytes(0)
            {
                return Err("separate and atomic seek/play produced different first frames".into());
            }
        } else {
            reference_frame = Some(frame);
        }
        samples.insert(if atomic { "atomic" } else { "separate" }, sample);
    }
    session.stop()?;
    Ok(json!({
        "target_seconds": seconds,
        "order": order.map(|atomic| if atomic { "atomic" } else { "separate" }),
        "reset": "Pause then precise seek to zero, outside each measurement; audio prewarm costs excluded symmetrically",
        "first_frame_timestamp_and_bgra_equal": true,
        "samples": samples,
    }))
}

fn measure_seek_and_play_variant(
    session: &PreviewSession,
    seconds: f64,
    atomic: bool,
) -> BenchmarkResult<(Value, Arc<PreviewRenderedFrame>)> {
    session.command(PreviewCommand::Pause)?;
    let before_reset = session.frame_store().generation();
    session.command(PreviewCommand::SeekPrecise(0.0))?;
    let previous_generation = acknowledge_new_frame(session, before_reset)?;
    let before = session.snapshot().runtime_metrics;
    let started = Instant::now();
    if atomic {
        session.command(PreviewCommand::SeekAndPlay {
            seconds,
            precise: true,
        })?;
    } else {
        session.command(PreviewCommand::SeekPrecise(seconds))?;
        session.command(PreviewCommand::Play)?;
    }
    let duration_ms = elapsed_ms(started);
    // Do not acknowledge the first frame until it is captured and checked:
    // the streaming worker drops subsequent frames while this one is pending.
    let frame = session
        .latest_frame()
        .ok_or("seek/play returned without a frame")?;
    if frame.generation <= previous_generation || frame.frame.byte_len == 0 {
        return Err("seek/play did not publish a new nonempty frame".into());
    }
    let image = frame.frame.render_image();
    if image.as_bytes(0).is_none() {
        return Err("seek/play frame has no BGRA data".into());
    }
    let snapshot = session.snapshot();
    let timestamp_seconds = Duration::from_micros(frame.frame.timestamp_us).as_secs_f64();
    if !snapshot.playback.playing || (timestamp_seconds - seconds).abs() > 0.000_002 {
        return Err("seek/play did not start playing from the requested timestamp".into());
    }
    let after = snapshot.runtime_metrics;
    let started = Instant::now();
    session.command(PreviewCommand::Play)?;
    let repeated_play_ms = elapsed_ms(started);
    let repeated = session.snapshot();
    if repeated.runtime_metrics.video_process_spawns != after.video_process_spawns
        || repeated.runtime_metrics.audio_process_spawns != after.audio_process_spawns
        || !repeated.playback.playing
        || session.frame_store().generation() != frame.generation
    {
        return Err("repeated Play restarted the active stream or stopped playback".into());
    }
    session.mark_frame_presented(frame.generation);
    Ok((
        json!({
            "elapsed_ms": duration_ms,
            "video_process_spawns": after.video_process_spawns.saturating_sub(before.video_process_spawns),
            "audio_process_spawns": after.audio_process_spawns.saturating_sub(before.audio_process_spawns),
            "first_frame_timestamp_us": frame.frame.timestamp_us,
            "first_frame_bytes": frame.frame.byte_len,
            "playing": true,
            "repeated_play_ms": repeated_play_ms,
            "repeated_play_extra_process_spawns": 0,
        }),
        frame.frame,
    ))
}

fn acknowledge_new_frame(session: &PreviewSession, previous: u64) -> BenchmarkResult<u64> {
    let frame = session
        .latest_frame()
        .ok_or("engine returned without a frame")?;
    if frame.generation <= previous || frame.frame.byte_len == 0 {
        return Err("engine did not publish a new nonempty frame".into());
    }
    session.mark_frame_presented(frame.generation);
    Ok(frame.generation)
}

fn observe_playback(
    session: &PreviewSession,
    seconds: f64,
    before: PreviewRuntimeMetrics,
) -> BenchmarkResult<Value> {
    let start = Instant::now();
    let mut last_generation = session.frame_store().generation();
    let mut last_observed = start;
    let mut intervals = Vec::new();
    while start.elapsed() < Duration::from_secs_f64(seconds) {
        if let Some(frame) = session.latest_frame()
            && frame.generation > last_generation
        {
            intervals.push(elapsed_ms(last_observed));
            last_observed = Instant::now();
            last_generation = frame.generation;
            session.mark_frame_presented(frame.generation);
        }
        thread::sleep(Duration::from_millis(2));
    }
    if intervals.is_empty() {
        return Err("playback produced no additional frames during the sampling window".into());
    }
    let after = session.snapshot().runtime_metrics;
    Ok(json!({
        "observed_intervals_ms": summarize(&intervals),
        "elapsed_ms": elapsed_ms(start),
        "frames_read": after.video_frames_read.saturating_sub(before.video_frames_read),
        "frames_published": after.video_frames_published.saturating_sub(before.video_frames_published),
        "frames_dropped": after.video_frames_dropped.saturating_sub(before.video_frames_dropped),
        "video_process_spawns": after.video_process_spawns.saturating_sub(before.video_process_spawns),
        "render_conversion_total_us": after.render_image_conversion_total_us.saturating_sub(before.render_image_conversion_total_us),
        "render_image_bytes": after.render_image_bytes.saturating_sub(before.render_image_bytes),
        "first_video_frame_read_ms": after.first_video_frame_read_ms,
        "first_video_frame_published_ms": after.first_video_frame_published_ms,
        "first_render_image_ms": after.first_render_image_ms,
        "session_video_process_spawns": after.video_process_spawns,
        "audio_process_spawns": after.audio_process_spawns.saturating_sub(before.audio_process_spawns),
        "audio_output_callbacks": after.audio_output_callbacks.saturating_sub(before.audio_output_callbacks),
        "audio_output_underruns": after.audio_output_underruns.saturating_sub(before.audio_output_underruns),
        "first_audio_pcm_ms": after.first_audio_pcm_ms,
        "first_audio_callback_ms": after.first_audio_callback_ms,
    }))
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn summarize(samples: &[f64]) -> Value {
    let mut sorted = samples.to_owned();
    sorted.sort_by(f64::total_cmp);
    let percentile = |percent: usize| sorted[(sorted.len() * percent).div_ceil(100) - 1];
    json!({
        "count": samples.len(), "min": sorted[0], "p50": percentile(50),
        "p95": percentile(95), "max": sorted[sorted.len() - 1], "samples": samples,
    })
}

fn parse_options() -> BenchmarkResult<Option<Options>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        eprintln!(
            "Usage: preview-benchmark INPUT SOURCE_WIDTH SOURCE_HEIGHT DURATION_SECONDS [--iterations 5] [--warmup 1] [--max-width 1280] [--max-height 720] [--fps 30] [--playback-seconds 1] [--timeout-seconds 30] [--audio-track INDEX] [--output report.json]"
        );
        return Ok(None);
    }
    if args.len() < 4 {
        return Err("expected input path, source width, source height, and duration".into());
    }
    let mut options = Options {
        path: PathBuf::from(&args[0]).canonicalize()?,
        width: args[1].parse()?,
        height: args[2].parse()?,
        duration: args[3].parse()?,
        iterations: 5,
        warmup: 1,
        max_width: 1280,
        max_height: 720,
        fps: 30,
        playback_seconds: 1.0,
        timeout_seconds: 30,
        audio_track: None,
        output: None,
    };
    let mut flags = args[4..].chunks_exact(2);
    for pair in &mut flags {
        match pair[0].as_str() {
            "--iterations" => options.iterations = pair[1].parse()?,
            "--warmup" => options.warmup = pair[1].parse()?,
            "--max-width" => options.max_width = pair[1].parse()?,
            "--max-height" => options.max_height = pair[1].parse()?,
            "--fps" => options.fps = pair[1].parse()?,
            "--playback-seconds" => options.playback_seconds = pair[1].parse()?,
            "--timeout-seconds" => options.timeout_seconds = pair[1].parse()?,
            "--audio-track" => options.audio_track = Some(pair[1].parse()?),
            "--output" => options.output = Some(PathBuf::from(&pair[1])),
            flag => return Err(format!("unknown option {flag}").into()),
        }
    }
    if !flags.remainder().is_empty() {
        return Err("every option requires a value".into());
    }
    if !(1..=100).contains(&options.iterations) || options.warmup > 10 {
        return Err("iterations must be 1..100 and warmup 0..10".into());
    }
    if !options.playback_seconds.is_finite()
        || !(0.1..=30.0).contains(&options.playback_seconds)
        || !options.duration.is_finite()
        || options.duration < options.playback_seconds + 0.5
        || options.timeout_seconds == 0
    {
        return Err("invalid duration: playback must be 0.1..30s, source at least 0.5s longer, and timeout positive".into());
    }
    session_config(&options).validate()?;
    Ok(Some(options))
}

struct Deadline {
    finished: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Deadline {
    fn new(seconds: u64, operation: &'static str) -> Self {
        let (finished, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            if rx.recv_timeout(Duration::from_secs(seconds)).is_err() {
                eprintln!("preview benchmark timed out after {seconds}s: {operation}");
                std::process::exit(124);
            }
        });
        Self {
            finished: Some(finished),
            worker: Some(worker),
        }
    }
}

impl Drop for Deadline {
    fn drop(&mut self) {
        if let Some(finished) = self.finished.take() {
            let _ = finished.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
