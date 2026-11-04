use super::ffmpeg_backend::PreviewPlaybackReader;
use super::metrics::PreviewRuntimeMetricsStore;
use super::{
    LatestFrameSnapshot, LatestFrameStore, PreviewCommand, PreviewDimensions, PreviewEngineError,
    PreviewPlaybackSnapshot, PreviewSessionConfig, PreviewSessionSnapshot, PreviewSessionStatus,
    PreviewSourceKind, RunningPreviewProcess, start_ffmpeg_preview_process,
};
use std::sync::{Mutex, MutexGuard};

pub struct PreviewSession {
    config: Mutex<PreviewSessionConfig>,
    dimensions: Mutex<PreviewDimensions>,
    duration_seconds: Mutex<f64>,
    frame_store: LatestFrameStore,
    metrics: PreviewRuntimeMetricsStore,
    pipeline: Mutex<Option<RunningPreviewProcess>>,
    playback_reader: Mutex<Option<PreviewPlaybackReader>>,
    playing: Mutex<bool>,
    status: Mutex<PreviewSessionStatus>,
}

impl PreviewSession {
    /// Starts a preview session for an image, video, or audio source.
    ///
    /// # Errors
    ///
    /// Returns an error when the config is invalid or the `FFmpeg` preview
    /// process cannot render the initial frame.
    pub fn start(config: PreviewSessionConfig) -> Result<Self, PreviewEngineError> {
        config.validate()?;
        let frame_store = LatestFrameStore::new();
        let metrics = PreviewRuntimeMetricsStore::new();

        match config.source_kind {
            PreviewSourceKind::Image | PreviewSourceKind::Video | PreviewSourceKind::Audio => {
                let (pipeline, dimensions, duration_seconds) =
                    start_ffmpeg_preview_process(&config, frame_store.clone(), metrics.clone())?;
                Ok(Self {
                    config: Mutex::new(config),
                    dimensions: Mutex::new(dimensions),
                    duration_seconds: Mutex::new(duration_seconds),
                    frame_store,
                    metrics,
                    playback_reader: Mutex::new(Some(pipeline.playback_reader())),
                    pipeline: Mutex::new(Some(pipeline)),
                    playing: Mutex::new(false),
                    status: Mutex::new(PreviewSessionStatus::Ready),
                })
            }
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn new_for_test(config: PreviewSessionConfig) -> Self {
        let frame_store = LatestFrameStore::new();
        let metrics = PreviewRuntimeMetricsStore::new();
        Self {
            dimensions: Mutex::new(config.target_dimensions()),
            duration_seconds: Mutex::new(config.duration_seconds),
            config: Mutex::new(config),
            frame_store,
            metrics,
            pipeline: Mutex::new(None),
            playback_reader: Mutex::new(None),
            playing: Mutex::new(false),
            status: Mutex::new(PreviewSessionStatus::Ready),
        }
    }

    /// Rebuilds the preview pipeline in-place while keeping the existing frame
    /// store alive until the replacement process has produced its first frame.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement config is invalid or the
    /// replacement `FFmpeg` process cannot be started.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "The pipeline mutex guard must live across the in-place FFmpeg reconfiguration."
    )]
    pub fn reconfigure(&self, config: PreviewSessionConfig) -> Result<(), PreviewEngineError> {
        config.validate()?;
        let mut pipeline_guard = lock(&self.pipeline);
        let playing = *lock(&self.playing);
        let Some(pipeline) = pipeline_guard.as_mut() else {
            // A queued reconfiguration must not reopen a session already stopped.
            return Ok(());
        };
        let seconds = pipeline.position();
        let dimensions = pipeline.reconfigure(config.clone(), seconds, playing, true)?;
        self.store_reconfigured_state(config, dimensions);
        Ok(())
    }

    #[must_use]
    pub fn latest_frame(&self) -> Option<LatestFrameSnapshot> {
        self.frame_store.latest()
    }

    #[must_use]
    pub fn frame_store(&self) -> LatestFrameStore {
        self.frame_store.clone()
    }

    pub fn mark_frame_presented(&self, generation: u64) {
        self.frame_store.mark_presented(generation);
        self.metrics.record_frame_presented();
    }

    /// Sends a playback command to the running preview pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying pipeline command fails.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "The pipeline mutex guard must live across the selected FFmpeg process command."
    )]
    pub fn command(&self, command: PreviewCommand) -> Result<(), PreviewEngineError> {
        let pipeline_guard = lock(&self.pipeline);
        let Some(pipeline) = pipeline_guard.as_ref() else {
            return Ok(());
        };
        let next_playing = match command {
            PreviewCommand::Play => {
                pipeline.resume()?;
                Some(true)
            }
            PreviewCommand::Pause => {
                pipeline.pause()?;
                Some(false)
            }
            PreviewCommand::SeekFast(seconds) => {
                let was_playing = *lock(&self.playing);
                pipeline.seek(seconds, !was_playing, false)?;
                None
            }
            PreviewCommand::SeekPrecise(seconds) => {
                let was_playing = *lock(&self.playing);
                pipeline.seek(seconds, !was_playing, true)?;
                None
            }
        };

        if let Some(playing) = next_playing {
            *lock(&self.playing) = playing;
        }

        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> PreviewSessionSnapshot {
        // The command mutex may be held for the entire decode/reconfiguration.
        // Read only the independently published clock; never join workers here.
        let duration_seconds = *lock(&self.duration_seconds);
        let reader = lock(&self.playback_reader).clone();
        let playback = reader.map_or(
            PreviewPlaybackSnapshot {
                position_seconds: 0.0,
                duration_seconds,
                playing: false,
            },
            |reader| reader.snapshot(duration_seconds, *lock(&self.playing)),
        );

        let config = lock(&self.config);
        PreviewSessionSnapshot {
            file_id: config.file_id.clone(),
            source_kind: config.source_kind,
            dimensions: *lock(&self.dimensions),
            status: lock(&self.status).clone(),
            playback,
            frame_generation: self.frame_store.generation(),
            frame_stats: self.frame_store.stats(),
            runtime_metrics: self.metrics.snapshot(),
        }
    }

    /// Stops the preview pipeline and releases its `FFmpeg` processes.
    ///
    /// # Errors
    ///
    /// Returns an error when the preview pipeline cannot stop a child process.
    pub fn stop(&self) -> Result<(), PreviewEngineError> {
        let pipeline = { lock(&self.pipeline).take() };
        *lock(&self.playback_reader) = None;
        let result = pipeline.map_or(Ok(()), |mut pipeline| pipeline.stop());
        *lock(&self.playing) = false;
        result
    }

    fn store_reconfigured_state(
        &self,
        config: PreviewSessionConfig,
        dimensions: PreviewDimensions,
    ) {
        let duration_seconds = config.duration_seconds;
        *lock(&self.config) = config;
        *lock(&self.dimensions) = dimensions;
        *lock(&self.duration_seconds) = duration_seconds;
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        let pipeline = { lock(&self.pipeline).take() };
        if let Some(mut pipeline) = pipeline {
            let _ = pipeline.stop();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, mpsc};
    use std::thread;
    use std::time::Duration;

    fn test_config() -> PreviewSessionConfig {
        PreviewSessionConfig {
            file_id: "nonblocking-preview".to_string(),
            path: PathBuf::from("/missing/preview.mp4"),
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
        }
    }

    #[test]
    fn snapshots_do_not_wait_for_pipeline_reconfiguration_or_shutdown() {
        let session = Arc::new(PreviewSession::new_for_test(test_config()));
        let frame =
            super::super::rendered_frame_from_bgra_payload(1, 1, 4, 0, vec![0; 4]).expect("frame");
        let _ = session.frame_store.publish(frame);
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker_session = Arc::clone(&session);
        let worker = thread::spawn(move || {
            let guard = lock(&worker_session.pipeline);
            locked_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            drop(guard);
        });
        locked_rx.recv().unwrap();

        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            snapshot_tx.send(session.snapshot()).unwrap();
            assert!(session.latest_frame().is_some());
        });
        let snapshot = snapshot_rx.recv_timeout(Duration::from_secs(1));
        // Release before asserting so a regression reports a failure, not a hang.
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        reader.join().unwrap();

        let snapshot = snapshot.expect("UI snapshot must not wait for the FFmpeg pipeline lock");
        assert_eq!(snapshot.file_id, "nonblocking-preview");
        assert_eq!(snapshot.frame_generation, 1);
    }

    #[test]
    fn queued_reconfiguration_does_not_restart_a_stopped_session() {
        let config = test_config();
        let session = PreviewSession::new_for_test(config.clone());
        session.stop().expect("stop");
        let mut replacement = config;
        replacement.conversion_config.flip_horizontal = true;

        // The missing file would fail if this tried to start a new FFmpeg process.
        session
            .reconfigure(replacement)
            .expect("stopped session stays stopped");

        assert!(lock(&session.pipeline).is_none());
        assert!(!lock(&session.config).conversion_config.flip_horizontal);
        assert!(!session.snapshot().playback.playing);
    }
}
