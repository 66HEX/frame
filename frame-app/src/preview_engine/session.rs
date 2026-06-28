use std::sync::{Mutex, MutexGuard};

use super::{
    LatestFrameSnapshot, LatestFrameStore, PreviewCommand, PreviewDimensions, PreviewEngineError,
    PreviewPlaybackSnapshot, PreviewSessionConfig, PreviewSessionSnapshot, PreviewSessionStatus,
    PreviewSourceKind, RunningPreviewPipeline, load_still_image_frame, start_gstreamer_pipeline,
};

pub struct PreviewSession {
    config: PreviewSessionConfig,
    dimensions: PreviewDimensions,
    duration_seconds: Mutex<f64>,
    frame_store: LatestFrameStore,
    pipeline: Mutex<Option<RunningPreviewPipeline>>,
    playing: Mutex<bool>,
    status: Mutex<PreviewSessionStatus>,
}

impl PreviewSession {
    pub fn start(config: PreviewSessionConfig) -> Result<Self, PreviewEngineError> {
        config.validate()?;
        let frame_store = LatestFrameStore::new();

        match config.source_kind {
            PreviewSourceKind::Image => {
                let frame = load_still_image_frame(&config.path, config.transform)?;
                let dimensions = frame.dimensions();
                frame_store.publish(frame);
                Ok(Self {
                    config,
                    dimensions,
                    duration_seconds: Mutex::new(0.0),
                    frame_store,
                    pipeline: Mutex::new(None),
                    playing: Mutex::new(false),
                    status: Mutex::new(PreviewSessionStatus::Ready),
                })
            }
            PreviewSourceKind::Video | PreviewSourceKind::Audio => {
                let (pipeline, dimensions, duration_seconds) =
                    start_gstreamer_pipeline(&config, frame_store.clone())?;
                let playing = config.source_kind == PreviewSourceKind::Video;
                Ok(Self {
                    config,
                    dimensions,
                    duration_seconds: Mutex::new(duration_seconds),
                    frame_store,
                    pipeline: Mutex::new(Some(pipeline)),
                    playing: Mutex::new(playing),
                    status: Mutex::new(PreviewSessionStatus::Ready),
                })
            }
        }
    }

    #[cfg(test)]
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

    pub fn command(&self, command: PreviewCommand) -> Result<(), PreviewEngineError> {
        let pipeline = lock(&self.pipeline);
        let Some(pipeline) = pipeline.as_ref() else {
            return Ok(());
        };

        match command {
            PreviewCommand::Play => {
                pipeline.resume()?;
                *lock(&self.playing) = true;
            }
            PreviewCommand::Pause => {
                pipeline.pause()?;
                *lock(&self.playing) = false;
            }
            PreviewCommand::SeekFast(seconds) => {
                let was_playing = *lock(&self.playing);
                pipeline.seek(seconds, !was_playing, false)?;
            }
            PreviewCommand::SeekPrecise(seconds) => {
                let was_playing = *lock(&self.playing);
                pipeline.seek(seconds, !was_playing, true)?;
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> PreviewSessionSnapshot {
        let pipeline = lock(&self.pipeline);
        let duration = pipeline.as_ref().map_or_else(
            || *lock(&self.duration_seconds),
            |pipeline| self.update_duration(pipeline.duration()),
        );
        let position = pipeline
            .as_ref()
            .map_or(0.0, RunningPreviewPipeline::position);

        PreviewSessionSnapshot {
            file_id: self.config.file_id.clone(),
            source_kind: self.config.source_kind,
            dimensions: self.dimensions,
            status: lock(&self.status).clone(),
            playback: PreviewPlaybackSnapshot {
                position_seconds: position,
                duration_seconds: duration,
                playing: *lock(&self.playing),
            },
            frame_generation: self.frame_store.generation(),
        }
    }

    pub fn stop(&self) {
        if let Some(mut pipeline) = lock(&self.pipeline).take() {
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
        if let Some(mut pipeline) = lock(&self.pipeline).take() {
            pipeline.stop();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
