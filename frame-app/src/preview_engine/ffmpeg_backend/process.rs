use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_stream::{
    AudioStartStatus, PendingAudioStart, lock_pending_audio_start,
};
use crate::preview_engine::ffmpeg_backend::clock::{
    AudioPlaybackStart, PreviewPlaybackClock, clamp_seek_seconds, initial_start_seconds,
    lock_playback, parsed_time,
};
use crate::preview_engine::ffmpeg_backend::decode::{
    RenderImageIdentity, decode_single_preview_frame,
};
use crate::preview_engine::frame_store::LatestFrameStore;
use crate::preview_engine::metrics::{PreviewRuntimeMetricsStore, log_preview_runtime_metrics};
use crate::preview_engine::types::{PreviewDimensions, PreviewSessionConfig, PreviewSourceKind};
use crate::runtime_binaries::ffmpeg_executable;
use gpui::{ImageId, RenderImage};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

pub(super) const STDERR_RING_LINES: usize = 24;

pub struct RunningPreviewProcess {
    pub(super) config: PreviewSessionConfig,
    pub(super) frame_store: LatestFrameStore,
    pub(super) metrics: PreviewRuntimeMetricsStore,
    pub(super) executable: String,
    pub(super) process: Mutex<RunningProcessState>,
    pub(super) audio_process: Mutex<RunningProcessState>,
    pub(super) pending_audio_start: Mutex<Option<PendingAudioStart>>,
    pub(super) playback: Arc<Mutex<PreviewPlaybackClock>>,
    pub(super) playback_changed: Arc<Condvar>,
    pub(super) render_image_id: ImageId,
    pub(super) render_image_version: Arc<AtomicU64>,
    pub(super) stderr: Arc<Mutex<VecDeque<String>>>,
}

#[derive(Debug, Default)]
pub(super) struct RunningProcessState {
    pub(super) child: Option<Child>,
    pub(super) stdout_worker: Option<JoinHandle<()>>,
    pub(super) stderr_worker: Option<JoinHandle<()>>,
    pub(super) stop_requested: Option<Arc<AtomicBool>>,
    pub(super) audio_start: Option<Arc<AudioPlaybackStart>>,
}

pub(super) struct ProcessHandles {
    pub(super) child: Option<Child>,
    pub(super) stdout_worker: Option<JoinHandle<()>>,
    pub(super) stderr_worker: Option<JoinHandle<()>>,
    pub(super) stop_requested: Option<Arc<AtomicBool>>,
    pub(super) audio_start: Option<Arc<AudioPlaybackStart>>,
}

impl RunningPreviewProcess {
    #[cfg(test)]
    pub(in crate::preview_engine) fn fail_audio_output_for_test(&self) {
        let generation = lock_playback(&self.playback).generation;
        AudioPlaybackStart::new(
            Arc::clone(&self.playback),
            Arc::clone(&self.playback_changed),
        )
        .mark_output_failed(Some(generation));
    }

    /// Pauses preview playback by terminating the active `FFmpeg` process.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot be terminated or reaped.
    pub fn pause(&self) -> Result<(), PreviewEngineError> {
        self.stop_running_processes()?;
        let (generation, seconds) = {
            let mut playback = lock_playback(&self.playback);
            playback.playing = false;
            (playback.generation, playback.last_frame_seconds)
        };
        self.prewarm_audio_for_generation(&self.config, seconds, true, generation);
        Ok(())
    }

    /// Resumes preview playback by spawning a new `FFmpeg` process at the last
    /// published frame timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when `FFmpeg` cannot be spawned.
    pub fn resume(&self) -> Result<(), PreviewEngineError> {
        let seconds = {
            let playback = lock_playback(&self.playback);
            if playback.playing && !playback.ended && !playback.failed {
                return Ok(());
            }
            if playback.failed {
                let duration = self.duration();
                let end = parsed_time(self.config.conversion_config.end_time.as_deref())
                    .filter(|end| *end > 0.0)
                    .map_or(duration, |end| {
                        if duration > 0.0 {
                            end.min(duration)
                        } else {
                            end
                        }
                    });
                clamp_seek_seconds(playback.last_frame_seconds, end)?
            } else if playback.ended {
                initial_start_seconds(&self.config)
            } else {
                playback.last_frame_seconds
            }
        };
        self.start_streaming(seconds, true)
    }

    /// Seeks preview playback to a source timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the timestamp is invalid, frame rendering fails,
    /// or the replacement `FFmpeg` process cannot be spawned.
    pub fn seek(
        &self,
        seconds: f64,
        render_seek_frame_when_paused: bool,
        precise: bool,
    ) -> Result<(), PreviewEngineError> {
        let seconds = clamp_seek_seconds(seconds, self.duration())?;
        if render_seek_frame_when_paused {
            self.stop_running_processes()?;
            if self.has_visual_stream() {
                self.render_single_frame(seconds, precise)?;
            } else {
                let generation = self.prepare_playback_generation(seconds);
                self.park_playback_generation(generation, seconds);
            }
            lock_playback(&self.playback).playing = false;
            return Ok(());
        }

        self.start_streaming(seconds, precise)
    }

    /// Stops every `FFmpeg` process owned by the preview pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error when a preview video or audio process cannot be stopped.
    pub fn stop(&mut self) -> Result<(), PreviewEngineError> {
        let result = self.stop_running_processes();
        log_preview_runtime_metrics("stop", self.metrics.snapshot());
        result
    }

    /// Rebuilds the `FFmpeg` preview command for a new conversion config while
    /// preserving the current source timestamp whenever possible.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement preview process cannot render its
    /// first frame or cannot be started.
    pub fn reconfigure(
        &mut self,
        config: PreviewSessionConfig,
        seconds: f64,
        playing: bool,
        precise: bool,
    ) -> Result<PreviewDimensions, PreviewEngineError> {
        let seconds = clamp_seek_seconds(seconds, config.duration_seconds)?;
        let dimensions = if matches!(
            config.source_kind,
            PreviewSourceKind::Video | PreviewSourceKind::Image
        ) {
            if playing && config.source_kind == PreviewSourceKind::Video {
                self.start_video_streaming_with_config(&config, seconds, precise)?
            } else {
                let started_at = Instant::now();
                let frame = decode_single_preview_frame(
                    &self.executable,
                    &config,
                    seconds,
                    precise,
                    self.next_render_image_identity(),
                    &self.metrics,
                )?;
                let dimensions = frame.frame.dimensions();
                self.stop_running_processes()?;
                let generation = self.prepare_playback_generation_at(seconds, started_at);
                self.metrics.record_video_frame_read(
                    generation,
                    frame.frame.byte_len,
                    frame.read_elapsed,
                );
                self.publish_frame_for_generation(frame.frame, frame.render_elapsed, generation);
                self.park_playback_generation(generation, seconds);
                self.prewarm_audio_for_generation(&config, seconds, precise, generation);
                dimensions
            }
        } else {
            self.stop_running_processes()?;
            self.config = config.clone();
            let generation = self.prepare_playback_generation(seconds);
            if playing {
                let status = self.start_audio_streaming(seconds, precise, true, generation)?;
                if status == AudioStartStatus::TimedOut {
                    return Err(PreviewEngineError::Audio(
                        "audio preview start readiness timed out".to_string(),
                    ));
                }
                self.start_playback_generation(generation, seconds);
            } else {
                self.park_playback_generation(generation, seconds);
            }
            config.target_dimensions()
        };

        self.config = config;
        Ok(dimensions)
    }

    pub(super) fn new(
        config: PreviewSessionConfig,
        frame_store: LatestFrameStore,
        metrics: PreviewRuntimeMetricsStore,
        executable: String,
    ) -> Self {
        let start_seconds = initial_start_seconds(&config);
        Self {
            config,
            frame_store,
            metrics,
            executable,
            process: Mutex::new(RunningProcessState::default()),
            audio_process: Mutex::new(RunningProcessState::default()),
            pending_audio_start: Mutex::new(None),
            playback: Arc::new(Mutex::new(PreviewPlaybackClock {
                generation: 0,
                base_seconds: start_seconds,
                last_frame_seconds: start_seconds,
                started_at: Instant::now(),
                playing: false,
                ended: false,
                failed: false,
            })),
            playback_changed: Arc::new(Condvar::new()),
            render_image_id: RenderImage::new_image_id(),
            render_image_version: Arc::new(AtomicU64::new(1)),
            stderr: Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_RING_LINES))),
        }
    }

    pub(super) fn next_render_image_identity(&self) -> RenderImageIdentity {
        RenderImageIdentity {
            image_id: self.render_image_id,
            content_version: self.render_image_version.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub(super) const fn has_visual_stream(&self) -> bool {
        matches!(
            self.config.source_kind,
            PreviewSourceKind::Video | PreviewSourceKind::Image
        )
    }

    pub(super) fn stop_running_processes(&self) -> Result<(), PreviewEngineError> {
        let video = self.stop_video_process();
        let audio = self.stop_audio_process();
        video.and(audio)
    }

    pub(super) fn stop_video_process(&self) -> Result<(), PreviewEngineError> {
        let handles = take_process_handles(&self.process);
        stop_process_handles(handles)
    }

    pub(super) fn stop_audio_process(&self) -> Result<(), PreviewEngineError> {
        *lock_pending_audio_start(&self.pending_audio_start) = None;
        let handles = take_process_handles(&self.audio_process);
        stop_process_handles(handles)
    }

    pub(super) fn reap_finished_processes(&self) {
        let video_finished = self.reap_finished_process(&self.process, true);
        if video_finished {
            let _ = self.stop_audio_process();
        }
        self.reap_finished_process(
            &self.audio_process,
            self.config.source_kind == PreviewSourceKind::Audio,
        );
    }

    pub(super) fn reap_finished_process(
        &self,
        process: &Mutex<RunningProcessState>,
        mark_ended: bool,
    ) -> bool {
        let mut state = lock_process(process);
        let Some(child) = state.child.as_mut() else {
            return false;
        };
        let Ok(Some(_status)) = child.try_wait() else {
            return false;
        };

        // A finished audio child can leave a live worker with prewarmed or
        // buffered PCM. Keep its handles and Ready message until the consumer
        // exits; joining now could block forever before Play starts the clock.
        if state.audio_start.is_some()
            && state
                .stdout_worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
        {
            return false;
        }

        state.child = None;
        let stdout_worker = state.stdout_worker.take();
        let stderr_worker = state.stderr_worker.take();
        state.audio_start = None;
        state.stop_requested = None;
        drop(state);

        join_worker(stdout_worker);
        join_worker(stderr_worker);
        if mark_ended {
            let mut playback = lock_playback(&self.playback);
            if !playback.failed {
                playback.playing = false;
                playback.ended = true;
            }
            drop(playback);
            self.playback_changed.notify_all();
        }
        true
    }
}

pub(super) fn stop_process_handles(handles: ProcessHandles) -> Result<(), PreviewEngineError> {
    if let Some(stop_requested) = &handles.stop_requested {
        if let Some(start) = &handles.audio_start {
            start.request_stop(stop_requested);
        } else {
            stop_requested.store(true, Ordering::SeqCst);
        }
    }
    if let Some(mut child) = handles.child {
        if child.try_wait().map_err(PreviewEngineError::Io)?.is_none() {
            child.kill().map_err(PreviewEngineError::Io)?;
        }
        let _ = child.wait();
    }
    join_worker(handles.stdout_worker);
    join_worker(handles.stderr_worker);
    Ok(())
}

pub(super) fn take_process_handles(process: &Mutex<RunningProcessState>) -> ProcessHandles {
    let mut state = lock_process(process);
    ProcessHandles {
        child: state.child.take(),
        stdout_worker: state.stdout_worker.take(),
        stderr_worker: state.stderr_worker.take(),
        stop_requested: state.stop_requested.take(),
        audio_start: state.audio_start.take(),
    }
}

impl Drop for RunningPreviewProcess {
    fn drop(&mut self) {
        let _ = self.stop_running_processes();
    }
}

/// Starts an `FFmpeg`-backed preview runtime.
///
/// # Errors
///
/// Returns an error when the initial preview frame cannot be rendered or the
/// `FFmpeg` executable cannot be launched.
pub(in crate::preview_engine) fn start_ffmpeg_preview_process(
    config: &PreviewSessionConfig,
    frame_store: LatestFrameStore,
    metrics: PreviewRuntimeMetricsStore,
) -> Result<(RunningPreviewProcess, PreviewDimensions, f64), PreviewEngineError> {
    let executable = ffmpeg_executable();
    let process = RunningPreviewProcess::new(config.clone(), frame_store, metrics, executable);
    let dimensions = if matches!(
        config.source_kind,
        PreviewSourceKind::Video | PreviewSourceKind::Image
    ) {
        process.render_single_frame(initial_start_seconds(config), true)?
    } else {
        config.target_dimensions()
    };

    Ok((process, dimensions, config.duration_seconds))
}

pub(super) fn spawn_stderr_worker(
    stderr: impl Read + Send + 'static,
    lines: Arc<Mutex<VecDeque<String>>>,
    stop_requested: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-ffmpeg-stderr".to_string())
        .spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(line) = line else {
                    break;
                };
                push_stderr_line(&lines, line);
            }
        })
        .map_err(|err| PreviewEngineError::Ffmpeg(format!("failed to spawn stderr worker: {err}")))
}

pub(super) fn push_stderr_line(lines: &Arc<Mutex<VecDeque<String>>>, line: String) {
    let mut lines = lock_stderr(lines);
    if lines.len() >= STDERR_RING_LINES {
        let _ = lines.pop_front();
    }
    lines.push_back(line);
}

pub(super) fn stderr_detail(stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!(": {trimmed}")
    }
}

pub(super) fn join_worker(worker: Option<JoinHandle<()>>) {
    if let Some(worker) = worker {
        let _ = worker.join();
    }
}

pub(super) fn lock_process(
    state: &Mutex<RunningProcessState>,
) -> MutexGuard<'_, RunningProcessState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn lock_stderr(lines: &Mutex<VecDeque<String>>) -> MutexGuard<'_, VecDeque<String>> {
    lines
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_keeps_a_playing_generation_without_starting_another_process() {
        let config = PreviewSessionConfig {
            file_id: "already-playing".into(),
            path: "/missing/preview.mp4".into(),
            source_kind: PreviewSourceKind::Video,
            source_width: Some(160),
            source_height: Some(90),
            selected_audio_track: None,
            duration_seconds: 2.0,
            max_width: 320,
            max_height: 180,
            fps: 30,
            conversion_config: crate::conversion_runner::core_config_from_gpui(
                &crate::settings::ConversionConfig::default(),
            ),
        };
        let process = RunningPreviewProcess::new(
            config,
            LatestFrameStore::new(),
            PreviewRuntimeMetricsStore::new(),
            "/missing/ffmpeg".into(),
        );
        {
            let mut playback = lock_playback(&process.playback);
            playback.generation = 7;
            playback.playing = true;
        }

        process.resume().expect("Play must not spawn FFmpeg again");

        assert_eq!(lock_playback(&process.playback).generation, 7);
        assert_eq!(process.metrics.snapshot().video_process_spawns, 0);
    }

    #[test]
    fn failed_audio_retry_clamps_to_source_or_trim_end_and_uses_a_new_generation() {
        for (trim_end, expected_seconds) in [(None, 12.499), (Some("00:00:04.000"), 3.999)] {
            let mut config = super::super::tests::preview_config();
            config.source_kind = PreviewSourceKind::Audio;
            config.source_width = None;
            config.source_height = None;
            config.conversion_config.end_time = trim_end.map(str::to_owned);
            let process = RunningPreviewProcess::new(
                config,
                LatestFrameStore::new(),
                PreviewRuntimeMetricsStore::new(),
                "/missing/ffmpeg".into(),
            );
            {
                let mut clock = lock_playback(&process.playback);
                clock.last_frame_seconds = 50.0;
                clock.fail_generation(0);
            }
            assert!(process.parked_generation_at(50.0).is_none());
            assert!(
                (process
                    .playback_reader()
                    .snapshot(12.5, true)
                    .position_seconds
                    - 50.0)
                    .abs()
                    < f64::EPSILON
            );
            process
                .resume()
                .expect("silent audio retry uses the real generation lifecycle");
            let clock = lock_playback(&process.playback);
            assert_eq!(clock.generation, 1);
            assert!(!clock.failed);
            assert!(clock.playing);
            assert!((clock.base_seconds - expected_seconds).abs() < f64::EPSILON);
            drop(clock);
        }
    }

    #[cfg(unix)]
    #[test]
    fn reaping_an_audio_failure_does_not_turn_it_into_natural_eof() {
        let mut config = super::super::tests::preview_config();
        config.source_kind = PreviewSourceKind::Audio;
        let process = RunningPreviewProcess::new(
            config,
            LatestFrameStore::new(),
            PreviewRuntimeMetricsStore::new(),
            "/missing/ffmpeg".into(),
        );
        process.fail_audio_output_for_test();
        let mut child = std::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap();
        child.wait().unwrap();
        lock_process(&process.audio_process).child = Some(child);
        assert!(process.reap_finished_process(&process.audio_process, true));
        let playback = process.playback_reader().snapshot(12.5, true);
        assert!(!playback.playing);
        assert!(playback.position_seconds.abs() < f64::EPSILON);
        assert!(process.failed());
        assert!(!process.ended());
    }
}
