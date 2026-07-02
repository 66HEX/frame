use std::sync::{Mutex, MutexGuard};

use super::{
    LatestFrameSnapshot, LatestFrameStore, PreviewCommand, PreviewDimensions, PreviewEngineError,
    PreviewPlaybackSnapshot, PreviewSessionConfig, PreviewSessionSnapshot, PreviewSessionStatus,
    PreviewSourceKind, RunningPreviewProcess, start_ffmpeg_preview_process,
};

pub struct PreviewSession {
    config: PreviewSessionConfig,
    dimensions: PreviewDimensions,
    duration_seconds: Mutex<f64>,
    frame_store: LatestFrameStore,
    pipeline: Mutex<Option<RunningPreviewProcess>>,
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

        match config.source_kind {
            PreviewSourceKind::Image | PreviewSourceKind::Video | PreviewSourceKind::Audio => {
                let (pipeline, dimensions, duration_seconds) =
                    start_ffmpeg_preview_process(&config, frame_store.clone())?;
                Ok(Self {
                    config,
                    dimensions,
                    duration_seconds: Mutex::new(duration_seconds),
                    frame_store,
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
        Self {
            dimensions: config.target_dimensions(),
            duration_seconds: Mutex::new(config.duration_seconds),
            config,
            frame_store,
            pipeline: Mutex::new(None),
            playing: Mutex::new(false),
            status: Mutex::new(PreviewSessionStatus::Ready),
        }
    }

    #[must_use]
    pub fn latest_frame(&self) -> Option<LatestFrameSnapshot> {
        self.frame_store.latest()
    }

    #[must_use]
    pub fn frame_store(&self) -> LatestFrameStore {
        self.frame_store.clone()
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
        let next_playing = {
            let pipeline = lock(&self.pipeline);
            let Some(pipeline) = pipeline.as_ref() else {
                return Ok(());
            };

            match command {
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
            }
        };

        if let Some(playing) = next_playing {
            *lock(&self.playing) = playing;
        }

        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> PreviewSessionSnapshot {
        let pipeline_snapshot = {
            let pipeline = lock(&self.pipeline);
            pipeline
                .as_ref()
                .map(|pipeline| (pipeline.duration(), pipeline.ended(), pipeline.position()))
        };
        let duration = pipeline_snapshot.map_or_else(
            || *lock(&self.duration_seconds),
            |(duration, _, _)| self.update_duration(duration),
        );
        let ended = pipeline_snapshot.is_some_and(|(_, ended, _)| ended);
        let position = pipeline_snapshot.map_or(
            0.0,
            |(_, ended, position)| {
                if ended { duration } else { position }
            },
        );
        let playing = pipeline_snapshot.is_some_and(|_| !ended && *lock(&self.playing));

        PreviewSessionSnapshot {
            file_id: self.config.file_id.clone(),
            source_kind: self.config.source_kind,
            dimensions: self.dimensions,
            status: lock(&self.status).clone(),
            playback: PreviewPlaybackSnapshot {
                position_seconds: position,
                duration_seconds: duration,
                playing,
            },
            frame_generation: self.frame_store.generation(),
        }
    }

    pub fn stop(&self) {
        let pipeline = { lock(&self.pipeline).take() };
        if let Some(mut pipeline) = pipeline {
            pipeline.stop();
        }
        *lock(&self.playing) = false;
    }

    fn update_duration(&self, duration: f64) -> f64 {
        if duration.is_finite() && duration > 0.0 {
            *lock(&self.duration_seconds) = duration;
            duration
        } else {
            *lock(&self.duration_seconds)
        }
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        let pipeline = { lock(&self.pipeline).take() };
        if let Some(mut pipeline) = pipeline {
            pipeline.stop();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
