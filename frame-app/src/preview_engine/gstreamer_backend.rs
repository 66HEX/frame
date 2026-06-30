use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use gst::MessageView;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_pbutils as gst_pbutils;
use gstreamer_video as gst_video;

use crate::numeric::{f64_to_u64, u64_to_f64};

use super::{
    LatestFrameStore, PreviewDimensions, PreviewEngineError, PreviewFrame, PreviewSessionConfig,
    PreviewSourceKind, PreviewTransform,
};

const MAX_FRAME_PACING_DELAY: Duration = Duration::from_millis(100);
const DISCOVERER_DURATION_TIMEOUT: gst::ClockTime = gst::ClockTime::from_seconds(2);
const FRAME_WORKER_PULL_TIMEOUT: gst::ClockTime = gst::ClockTime::from_mseconds(50);
const SEEK_END_EPSILON: f64 = 0.001;
const SEEK_TRANSIENT_GUARD_TIMEOUT: Duration = Duration::from_millis(800);
const SEEK_TRANSIENT_POSITION_TOLERANCE_US: u64 = 100_000;
const SEEK_TRANSIENT_FRAME_TOLERANCE_US: u64 = 500_000;

pub struct RunningPreviewPipeline {
    pipeline: gst::Pipeline,
    audio_volume: Option<gst::Element>,
    fps: u32,
    stop_requested: Arc<AtomicBool>,
    pause_after_next_frame: Arc<AtomicBool>,
    eos_reached: Arc<AtomicBool>,
    clock_generation: Arc<AtomicU64>,
    seek_guard: SeekTransientGuard,
    worker: Option<JoinHandle<()>>,
    bus_worker: Option<JoinHandle<()>>,
}

impl RunningPreviewPipeline {
    /// Pauses the `GStreamer` playback pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error when `GStreamer` rejects the paused state transition.
    pub fn pause(&self) -> Result<(), PreviewEngineError> {
        self.reset_clock();
        self.set_audio_muted(true);
        self.pipeline
            .set_state(gst::State::Paused)
            .map(|_| ())
            .map_err(|err| PreviewEngineError::Gstreamer(format!("failed to pause: {err:?}")))
    }

    /// Resumes the `GStreamer` playback pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error when seeking back from end-of-stream or switching to
    /// the playing state fails.
    pub fn resume(&self) -> Result<(), PreviewEngineError> {
        if self.ended() {
            self.seek(0.0, false, true)?;
        }
        self.reset_clock();
        self.eos_reached.store(false, Ordering::SeqCst);
        self.set_audio_muted(false);
        self.pipeline
            .set_state(gst::State::Playing)
            .map(|_| ())
            .map_err(|err| PreviewEngineError::Gstreamer(format!("failed to resume: {err:?}")))
    }

    /// Seeks the pipeline to a preview timestamp in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error when the target timestamp cannot be clamped for the
    /// current media or `GStreamer` rejects the seek/state transition.
    pub fn seek(
        &self,
        seconds: f64,
        resume_after_seek_frame: bool,
        precise: bool,
    ) -> Result<(), PreviewEngineError> {
        let seconds = clamp_seek_seconds(seconds, self.duration(), self.fps)?;
        let resume_for_seek_frame = resume_after_seek_frame && self.worker.is_some();

        self.reset_clock();
        self.eos_reached.store(false, Ordering::SeqCst);
        if resume_for_seek_frame {
            self.set_audio_muted(true);
            self.pause_after_next_frame.store(true, Ordering::SeqCst);
        }
        if precise {
            self.seek_guard.set_target(seconds);
        } else {
            self.seek_guard.clear();
        }

        let position = gst::ClockTime::from_nseconds(f64_to_u64(seconds * 1_000_000_000.0));
        let result = self
            .pipeline
            .seek_simple(preview_seek_flags(precise), position)
            .map_err(|err| PreviewEngineError::Gstreamer(format!("failed to seek: {err}")));

        if result.is_err() {
            self.pause_after_next_frame.store(false, Ordering::SeqCst);
            self.seek_guard.clear();
            return result;
        }

        if resume_for_seek_frame {
            self.pipeline
                .set_state(gst::State::Playing)
                .map_err(|err| {
                    self.pause_after_next_frame.store(false, Ordering::SeqCst);
                    PreviewEngineError::Gstreamer(format!(
                        "failed to resume for seek frame: {err:?}"
                    ))
                })?;
        }

        Ok(())
    }

    #[must_use]
    pub fn position(&self) -> f64 {
        let position = self
            .pipeline
            .query_position::<gst::ClockTime>()
            .map_or(0.0, |position| {
                u64_to_f64(position.nseconds()) / 1_000_000_000.0
            });
        self.seek_guard.position_or_target(position)
    }

    #[must_use]
    pub fn duration(&self) -> f64 {
        self.pipeline
            .query_duration::<gst::ClockTime>()
            .map_or(0.0, |duration| {
                u64_to_f64(duration.nseconds()) / 1_000_000_000.0
            })
    }

    #[must_use]
    pub fn ended(&self) -> bool {
        self.eos_reached.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        self.stop_requested.store(true, Ordering::SeqCst);
        self.set_audio_muted(true);
        let _ = self.pipeline.set_state(gst::State::Null);

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Some(bus_worker) = self.bus_worker.take() {
            let _ = bus_worker.join();
        }
    }

    fn set_audio_muted(&self, muted: bool) {
        if let Some(volume) = &self.audio_volume {
            volume.set_property("mute", muted);
        }
    }

    fn reset_clock(&self) {
        self.clock_generation.fetch_add(1, Ordering::SeqCst);
    }
}

impl Drop for RunningPreviewPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone, Debug, Default)]
struct SeekTransientGuard {
    state: Arc<Mutex<Option<SeekTransientState>>>,
}

#[derive(Clone, Copy, Debug)]
struct SeekTransientState {
    target_us: u64,
    started_at: Instant,
}

impl SeekTransientGuard {
    fn new() -> Self {
        Self::default()
    }

    fn set_target(&self, seconds: f64) {
        *lock_seek_guard(&self.state) = Some(SeekTransientState {
            target_us: seconds_to_us(seconds),
            started_at: Instant::now(),
        });
    }

    fn clear(&self) {
        *lock_seek_guard(&self.state) = None;
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "The seek guard mutex must cover timeout checks and possible guard clearing."
    )]
    fn position_or_target(&self, position_seconds: f64) -> f64 {
        let target_us = {
            let mut state = lock_seek_guard(&self.state);
            let Some(guard) = state.as_ref() else {
                return position_seconds;
            };
            if guard.started_at.elapsed() >= SEEK_TRANSIENT_GUARD_TIMEOUT {
                *state = None;
                return position_seconds;
            }
            if timestamps_are_close(
                seconds_to_us(position_seconds),
                guard.target_us,
                SEEK_TRANSIENT_POSITION_TOLERANCE_US,
            ) {
                return position_seconds;
            }
            guard.target_us
        };
        u64_to_f64(target_us) / 1_000_000.0
    }

    fn should_hold_frame(&self, timestamp_us: u64) -> bool {
        self.target_for_timestamp(timestamp_us, SEEK_TRANSIENT_FRAME_TOLERANCE_US)
            .is_some()
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "The seek guard mutex must cover timestamp checks and possible guard clearing."
    )]
    fn target_for_timestamp(&self, timestamp_us: u64, tolerance_us: u64) -> Option<u64> {
        let mut state = lock_seek_guard(&self.state);
        let target_us = {
            let guard = state.as_ref()?;
            if guard.started_at.elapsed() >= SEEK_TRANSIENT_GUARD_TIMEOUT {
                *state = None;
                return None;
            }
            if timestamps_are_close(timestamp_us, guard.target_us, tolerance_us) {
                *state = None;
                return None;
            }
            guard.target_us
        };
        Some(target_us)
    }
}

fn lock_seek_guard(
    state: &Mutex<Option<SeekTransientState>>,
) -> MutexGuard<'_, Option<SeekTransientState>> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Starts a `GStreamer` pipeline and frame worker for preview playback.
///
/// # Errors
///
/// Returns an error when `GStreamer` cannot initialize, the pipeline cannot be
/// built, media discovery fails, or required pipeline elements are missing.
pub fn start_gstreamer_pipeline(
    config: &PreviewSessionConfig,
    frame_store: LatestFrameStore,
) -> Result<(RunningPreviewPipeline, PreviewDimensions, f64), PreviewEngineError> {
    gst::init().map_err(|err| PreviewEngineError::Gstreamer(err.to_string()))?;

    let dimensions = config.target_dimensions();
    let pipeline = build_pipeline(config, dimensions)?;
    let audio_volume = pipeline.by_name("preview_audio_volume");
    let appsink = if config.source_kind == PreviewSourceKind::Audio {
        None
    } else {
        Some(
            pipeline
                .by_name("preview_sink")
                .ok_or_else(|| PreviewEngineError::Gstreamer("preview_sink not found".to_string()))?
                .downcast::<gst_app::AppSink>()
                .map_err(|_| {
                    PreviewEngineError::Gstreamer("preview_sink is not an appsink".to_string())
                })?,
        )
    };

    let stop_requested = Arc::new(AtomicBool::new(false));
    let bus_stop = Arc::clone(&stop_requested);
    let pause_after_next_frame = Arc::new(AtomicBool::new(false));
    if appsink.is_some() {
        pause_after_next_frame.store(true, Ordering::SeqCst);
    }
    let clock_generation = Arc::new(AtomicU64::new(0));
    let eos_reached = Arc::new(AtomicBool::new(false));
    let seek_guard = SeekTransientGuard::new();
    let bus = pipeline
        .bus()
        .ok_or_else(|| PreviewEngineError::Gstreamer("preview pipeline has no bus".to_string()))?;

    if let Err(err) = pipeline.set_state(if appsink.is_some() {
        gst::State::Playing
    } else {
        gst::State::Paused
    }) {
        let bus_detail = drain_bus_error_details(&bus);
        let _ = pipeline.set_state(gst::State::Null);
        return Err(PreviewEngineError::Gstreamer(format!(
            "failed to start preview: {err:?}{bus_detail}"
        )));
    }

    let worker = appsink
        .map(|appsink| {
            spawn_frame_worker(
                appsink,
                frame_store,
                Arc::clone(&stop_requested),
                Arc::clone(&pause_after_next_frame),
                Arc::clone(&clock_generation),
                seek_guard.clone(),
                pipeline.clone(),
            )
        })
        .transpose()?;
    let bus_worker = spawn_bus_worker(bus, bus_stop, Arc::clone(&eos_reached))?;
    let duration = normalize_duration(
        pipeline
            .query_duration::<gst::ClockTime>()
            .map_or(0.0, |duration| {
                u64_to_f64(duration.nseconds()) / 1_000_000_000.0
            }),
    )
    .or_else(|| discover_file_duration(&config.path.to_string_lossy()))
    .unwrap_or(config.duration_seconds);

    Ok((
        RunningPreviewPipeline {
            pipeline,
            audio_volume,
            fps: config.fps,
            stop_requested,
            pause_after_next_frame,
            eos_reached,
            clock_generation,
            seek_guard,
            worker,
            bus_worker: Some(bus_worker),
        },
        dimensions,
        duration,
    ))
}

#[must_use]
pub fn discover_file_duration(file_path: &str) -> Option<f64> {
    gst::init().ok()?;
    let uri = gst::glib::filename_to_uri(file_path, None).ok()?;
    let discoverer = gst_pbutils::Discoverer::new(DISCOVERER_DURATION_TIMEOUT).ok()?;
    discoverer
        .discover_uri(uri.as_str())
        .ok()
        .and_then(|info| info.duration())
        .map(|duration| u64_to_f64(duration.nseconds()) / 1_000_000_000.0)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
}

fn spawn_frame_worker(
    appsink: gst_app::AppSink,
    frame_store: LatestFrameStore,
    stop_requested: Arc<AtomicBool>,
    pause_after_next_frame: Arc<AtomicBool>,
    clock_generation: Arc<AtomicU64>,
    seek_guard: SeekTransientGuard,
    pipeline: gst::Pipeline,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-gstreamer".to_string())
        .spawn(move || {
            let mut playback_clock = PlaybackClock::new();
            while !stop_requested.load(Ordering::SeqCst) {
                let sample_generation = clock_generation.load(Ordering::SeqCst);
                let Some(sample) = appsink.try_pull_sample(FRAME_WORKER_PULL_TIMEOUT) else {
                    continue;
                };

                let Some(frame) = frame_from_sample(&sample) else {
                    continue;
                };
                if seek_guard.should_hold_frame(frame.timestamp_us) {
                    continue;
                }
                playback_clock.pace_frame(sample_generation, frame.timestamp_us);
                let _ = frame_store.publish(frame);

                if pause_after_next_frame.swap(false, Ordering::SeqCst) {
                    let _ = pipeline.set_state(gst::State::Paused);
                }
            }
        })
        .map_err(|err| PreviewEngineError::Gstreamer(err.to_string()))
}

fn spawn_bus_worker(
    bus: gst::Bus,
    stop_requested: Arc<AtomicBool>,
    eos_reached: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, PreviewEngineError> {
    thread::Builder::new()
        .name("frame-preview-gstreamer-bus".to_string())
        .spawn(move || {
            while !stop_requested.load(Ordering::SeqCst) {
                let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(250)) else {
                    continue;
                };

                match message.view() {
                    MessageView::Error(_) => break,
                    MessageView::Eos(_) => {
                        eos_reached.store(true, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        })
        .map_err(|err| PreviewEngineError::Gstreamer(err.to_string()))
}

fn build_pipeline(
    config: &PreviewSessionConfig,
    dimensions: PreviewDimensions,
) -> Result<gst::Pipeline, PreviewEngineError> {
    let description = build_pipeline_description(
        dimensions,
        config.fps,
        config.source_kind,
        config.transform,
        pipeline_crop(config),
    );
    let element = gst::parse::launch(&description)
        .map_err(|err| PreviewEngineError::Gstreamer(format!("invalid preview pipeline: {err}")))?;
    let pipeline = element.downcast::<gst::Pipeline>().map_err(|_| {
        PreviewEngineError::Gstreamer("preview pipeline did not produce a pipeline".to_string())
    })?;

    let filesrc = pipeline
        .by_name("preview_src")
        .ok_or_else(|| PreviewEngineError::Gstreamer("preview_src not found".to_string()))?;
    filesrc.set_property("location", config.path.to_string_lossy().as_ref());

    Ok(pipeline)
}

fn build_pipeline_description(
    dimensions: PreviewDimensions,
    fps: u32,
    source_kind: PreviewSourceKind,
    transform: PreviewTransform,
    crop: Option<PreviewPipelineCrop>,
) -> String {
    let audio_branch = "preview_decode. ! queue max-size-buffers=8 max-size-bytes=0 max-size-time=0 ! audioconvert ! audioresample ! volume name=preview_audio_volume mute=true ! autoaudiosink name=preview_audio_sink sync=true";

    if source_kind == PreviewSourceKind::Audio {
        return format!(
            "filesrc name=preview_src ! decodebin name=preview_decode force-sw-decoders=true {audio_branch}"
        );
    }

    let transform_branch = gstreamer_transform_branch(transform);
    let crop_branch = gstreamer_crop_branch(crop);
    format!(
        "filesrc name=preview_src ! decodebin name=preview_decode force-sw-decoders=true preview_decode. ! queue max-size-buffers=8 max-size-bytes=0 max-size-time=0 ! videoconvert ! {transform_branch}{crop_branch}videoscale ! videorate drop-only=true ! video/x-raw,format=BGRA,width={},height={},framerate=[1/1,{}/1] ! appsink name=preview_sink emit-signals=false sync=false max-buffers=2 drop=false wait-on-eos=false {audio_branch}",
        dimensions.width, dimensions.height, fps
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewPipelineCrop {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

fn pipeline_crop(config: &PreviewSessionConfig) -> Option<PreviewPipelineCrop> {
    let crop = config.crop?;
    let source = config.transformed_source_dimensions()?;
    let right_edge = crop.x.checked_add(crop.width)?;
    let bottom_edge = crop.y.checked_add(crop.height)?;
    Some(PreviewPipelineCrop {
        left: crop.x,
        top: crop.y,
        right: source.width.checked_sub(right_edge)?,
        bottom: source.height.checked_sub(bottom_edge)?,
    })
}

fn gstreamer_crop_branch(crop: Option<PreviewPipelineCrop>) -> String {
    crop.map_or_else(String::new, |crop| {
        format!(
            "videocrop left={} top={} right={} bottom={} ! ",
            crop.left, crop.top, crop.right, crop.bottom
        )
    })
}

fn gstreamer_transform_branch(transform: PreviewTransform) -> String {
    let mut branch = String::new();
    if transform.flip_horizontal {
        branch.push_str("videoflip method=horizontal-flip ! ");
    }
    if transform.flip_vertical {
        branch.push_str("videoflip method=vertical-flip ! ");
    }
    match transform.rotation_degrees {
        90 => branch.push_str("videoflip method=clockwise ! "),
        180 => branch.push_str("videoflip method=rotate-180 ! "),
        270 => branch.push_str("videoflip method=counterclockwise ! "),
        _ => {}
    }
    branch
}

fn frame_from_sample(sample: &gst::Sample) -> Option<PreviewFrame> {
    let caps = sample.caps()?;
    let info = gst_video::VideoInfo::from_caps(caps).ok()?;
    let buffer = sample.buffer()?;
    let readable = buffer.map_readable().ok()?;
    let width = info.width();
    let height = info.height();
    let stride = u32::try_from(info.stride()[0]).ok()?;
    let timestamp_us = buffer.pts().map_or(0, |pts| pts.nseconds() / 1_000);
    let payload = tight_bgra_payload(readable.as_slice(), width, height, stride)?;
    PreviewFrame::bgra(
        width,
        height,
        width.saturating_mul(4),
        timestamp_us,
        payload,
    )
    .ok()
}

struct PlaybackClock {
    generation: u64,
    base_pts_us: Option<u64>,
    base_instant: Instant,
}

impl PlaybackClock {
    fn new() -> Self {
        Self {
            generation: 0,
            base_pts_us: None,
            base_instant: Instant::now(),
        }
    }

    fn pace_frame(&mut self, generation: u64, pts_us: u64) {
        if self.generation != generation || self.base_pts_us.is_none() {
            self.reset(generation, pts_us);
            return;
        }

        let Some(base_pts_us) = self.base_pts_us else {
            return;
        };
        let Some(pts_delta_us) = pts_us.checked_sub(base_pts_us) else {
            self.reset(generation, pts_us);
            return;
        };

        let target_elapsed = Duration::from_micros(pts_delta_us);
        let actual_elapsed = self.base_instant.elapsed();
        if target_elapsed > actual_elapsed {
            thread::sleep(
                target_elapsed
                    .checked_sub(actual_elapsed)
                    .unwrap()
                    .min(MAX_FRAME_PACING_DELAY),
            );
        }
    }

    fn reset(&mut self, generation: u64, pts_us: u64) {
        self.generation = generation;
        self.base_pts_us = Some(pts_us);
        self.base_instant = Instant::now();
    }
}

fn tight_bgra_payload(data: &[u8], width: u32, height: u32, stride: u32) -> Option<Vec<u8>> {
    let row_len = usize::try_from(width.checked_mul(4)?).ok()?;
    let height = usize::try_from(height).ok()?;
    let stride = usize::try_from(stride).ok()?;
    if stride < row_len {
        return None;
    }

    if stride == row_len {
        let len = row_len.checked_mul(height)?;
        return data.get(0..len).map(<[u8]>::to_vec);
    }

    let mut payload = Vec::with_capacity(row_len.checked_mul(height)?);
    for row in 0..height {
        let start = row.checked_mul(stride)?;
        let end = start.checked_add(row_len)?;
        payload.extend_from_slice(data.get(start..end)?);
    }
    Some(payload)
}

fn preview_seek_flags(precise: bool) -> gst::SeekFlags {
    if precise {
        gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE
    } else {
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT
    }
}

fn seconds_to_us(seconds: f64) -> u64 {
    f64_to_u64(seconds.max(0.0) * 1_000_000.0)
}

const fn timestamps_are_close(timestamp_us: u64, target_us: u64, tolerance_us: u64) -> bool {
    timestamp_us.abs_diff(target_us) <= tolerance_us
}

fn clamp_seek_seconds(seconds: f64, duration: f64, _fps: u32) -> Result<f64, PreviewEngineError> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(PreviewEngineError::InvalidInput(
            "seek position must be a positive finite number".to_string(),
        ));
    }

    if !duration.is_finite() || duration <= 0.0 {
        return Ok(seconds);
    }

    Ok(seconds.min((duration - SEEK_END_EPSILON).max(0.0)))
}

fn drain_bus_error_details(bus: &gst::Bus) -> String {
    let mut details = Vec::new();

    while let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
        use gst::MessageView;
        match message.view() {
            MessageView::Error(err) => {
                details.push(format!(
                    " error from {:?}: {} ({:?})",
                    err.src().map(gstreamer::prelude::GstObjectExt::path_string),
                    err.error(),
                    err.debug()
                ));
            }
            MessageView::Warning(warning) => {
                details.push(format!(
                    " warning from {:?}: {} ({:?})",
                    warning
                        .src()
                        .map(gstreamer::prelude::GstObjectExt::path_string),
                    warning.error(),
                    warning.debug()
                ));
            }
            _ => {}
        }
    }

    if details.is_empty() {
        String::new()
    } else {
        format!("; bus diagnostics:{}", details.join(";"))
    }
}

fn normalize_duration(duration: f64) -> Option<f64> {
    if duration.is_finite() && duration > 0.0 {
        Some(duration)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::float_cmp,
        reason = "GStreamer backend tests compare exact deterministic timestamp conversions."
    )]

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_pipeline_description_uses_bgra_low_latency_video_sink() {
        let description = build_pipeline_description(
            PreviewDimensions {
                width: 1280,
                height: 720,
            },
            30,
            PreviewSourceKind::Video,
            PreviewTransform::default(),
            None,
        );

        assert!(description.contains("video/x-raw,format=BGRA"));
        assert!(!description.contains("leaky-type=downstream"));
        assert!(description.contains("sync=false max-buffers=2 drop=false"));
    }

    #[test]
    fn build_pipeline_description_caps_preview_fps_without_forcing_upsampling() {
        let description = build_pipeline_description(
            PreviewDimensions {
                width: 1280,
                height: 720,
            },
            30,
            PreviewSourceKind::Video,
            PreviewTransform::default(),
            None,
        );

        assert!(description.contains("framerate=[1/1,30/1]"));
        assert!(!description.contains("framerate=30/1"));
    }

    #[test]
    fn build_audio_pipeline_description_omits_appsink() {
        let description = build_pipeline_description(
            PreviewDimensions {
                width: 1280,
                height: 720,
            },
            30,
            PreviewSourceKind::Audio,
            PreviewTransform::default(),
            None,
        );

        assert!(description.contains("audioconvert"));
        assert!(!description.contains("appsink"));
    }

    #[test]
    fn tight_bgra_payload_removes_row_padding() {
        let data = vec![1, 2, 3, 4, 9, 9, 5, 6, 7, 8, 9, 9];

        let payload = tight_bgra_payload(&data, 1, 2, 6).expect("payload");

        assert_eq!(payload, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn clamp_seek_seconds_keeps_seek_inside_duration() {
        let seconds = clamp_seek_seconds(10.0, 10.0, 30).expect("seek");

        assert!(seconds < 10.0);
    }

    #[test]
    fn seek_transient_guard_holds_stale_position_and_frames_until_target_arrives() {
        let guard = SeekTransientGuard::new();
        guard.set_target(42.0);

        assert_eq!(guard.position_or_target(0.0), 42.0);
        assert!(guard.should_hold_frame(0));
        assert!(!guard.should_hold_frame(42_000_000));
        assert_eq!(guard.position_or_target(0.0), 0.0);
    }

    #[test]
    fn seek_transient_guard_keeps_frame_guard_after_position_matches_target() {
        let guard = SeekTransientGuard::new();
        guard.set_target(42.0);

        assert_eq!(guard.position_or_target(42.0), 42.0);
        assert!(guard.should_hold_frame(0));
    }

    #[test]
    fn seek_transient_guard_expires_instead_of_holding_forever() {
        let guard = SeekTransientGuard::new();
        guard.set_target(42.0);
        {
            let mut state = lock_seek_guard(&guard.state);
            let started_at = Instant::now()
                .checked_sub(SEEK_TRANSIENT_GUARD_TIMEOUT + Duration::from_millis(1))
                .expect("expired instant");
            state.as_mut().expect("guard state").started_at = started_at;
        }

        assert!(!guard.should_hold_frame(0));
        assert_eq!(guard.position_or_target(0.0), 0.0);
    }

    #[test]
    fn build_pipeline_sets_filesrc_location_without_quoting_path() {
        gst::init().expect("gstreamer init");
        let config = PreviewSessionConfig {
            file_id: "file-1".to_string(),
            path: PathBuf::from("/tmp/a clip's test.mp4"),
            source_kind: PreviewSourceKind::Video,
            source_width: Some(1920),
            source_height: Some(1080),
            duration_seconds: 10.0,
            max_width: 1280,
            max_height: 720,
            fps: 30,
            transform: PreviewTransform::default(),
            crop: None,
        };

        let pipeline = build_pipeline(&config, config.target_dimensions()).expect("pipeline");
        let filesrc = pipeline.by_name("preview_src").expect("preview_src");

        assert_eq!(
            filesrc.property::<String>("location"),
            config.path.to_string_lossy()
        );
        let _ = pipeline.set_state(gst::State::Null);
    }

    #[test]
    fn build_pipeline_description_matches_conversion_transform_order() {
        let description = build_pipeline_description(
            PreviewDimensions {
                width: 720,
                height: 1280,
            },
            30,
            PreviewSourceKind::Video,
            PreviewTransform {
                rotation_degrees: 90,
                flip_horizontal: true,
                flip_vertical: true,
            },
            None,
        );

        let hflip = description.find("horizontal-flip").expect("hflip");
        let vflip = description.find("vertical-flip").expect("vflip");
        let rotate = description.find("clockwise").expect("rotate");
        assert!(hflip < vflip && vflip < rotate);
    }

    #[test]
    fn build_pipeline_description_crops_after_transform_before_scaling() {
        let description = build_pipeline_description(
            PreviewDimensions {
                width: 640,
                height: 360,
            },
            30,
            PreviewSourceKind::Video,
            PreviewTransform {
                rotation_degrees: 90,
                flip_horizontal: false,
                flip_vertical: false,
            },
            Some(PreviewPipelineCrop {
                left: 10,
                top: 20,
                right: 30,
                bottom: 40,
            }),
        );

        let rotate = description.find("clockwise").expect("rotate");
        let crop = description.find("videocrop").expect("crop");
        let scale = description.find("videoscale").expect("scale");

        assert!(rotate < crop && crop < scale);
        assert!(description.contains("left=10 top=20 right=30 bottom=40"));
    }
}
