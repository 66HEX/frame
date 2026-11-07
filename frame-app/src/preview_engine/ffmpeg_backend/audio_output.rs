use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_drain::{AudioDrainOutcome, AudioDrainState};
use crate::preview_engine::ffmpeg_backend::clock::{
    AudioPlaybackStart, PreviewPlaybackClock, clock_generation_matches, lock_playback,
};
use crate::preview_engine::ffmpeg_backend::process::push_stderr_line;
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub(super) const AUDIO_BUFFER_SECONDS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioOutputSpec {
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
}

pub(super) struct AudioPreviewOutput {
    stream: cpal::Stream,
    pub(super) context: AudioCallbackContext,
    pub(super) frame_bytes: usize,
}

pub(super) struct AudioSampleBuffer {
    pub(super) samples: VecDeque<f32>,
    pub(super) capacity: usize,
    pub(super) spec: AudioOutputSpec,
    pub(super) drain: AudioDrainState,
}

#[derive(Clone)]
pub(super) struct AudioCallbackContext {
    pub(super) buffer: Arc<Mutex<AudioSampleBuffer>>,
    pub(super) metrics: PreviewRuntimeMetricsStore,
    pub(super) playback: Arc<Mutex<PreviewPlaybackClock>>,
    pub(super) start: Arc<AudioPlaybackStart>,
    pub(super) generation: u64,
    pub(super) stderr: Arc<Mutex<VecDeque<String>>>,
    pub(super) failure_ends_playback: bool,
}

#[derive(Clone, Copy)]
pub(super) struct AudioCallbackTiming {
    pub(super) entered_at: Instant,
    pub(super) playback_delay: Duration,
}

impl AudioCallbackTiming {
    pub(super) fn from_timestamp(
        entered_at: Instant,
        timestamp: cpal::OutputStreamTimestamp,
    ) -> Self {
        Self {
            entered_at,
            playback_delay: timestamp
                .playback
                .saturating_duration_since(timestamp.callback),
        }
    }
}

fn build_audio_output_stream<T: cpal::SizedSample + 'static>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    context: &AudioCallbackContext,
    convert: impl Fn(f32) -> T + Copy + Send + 'static,
) -> Result<cpal::Stream, PreviewEngineError> {
    let callback_context = context.clone();
    let error_context = context.clone();
    device
        .build_output_stream(
            *config,
            move |data: &mut [T], info| {
                let entered_at = Instant::now();
                let timestamp = info.timestamp();
                fill_audio_output(
                    data,
                    &callback_context,
                    AudioCallbackTiming::from_timestamp(entered_at, timestamp),
                    convert,
                );
            },
            move |error| error_context.fail(format!("audio preview output failed: {error}")),
            None,
        )
        .map_err(|error| {
            PreviewEngineError::Audio(format!("failed to build audio output stream: {error}"))
        })
}

pub(super) fn fill_audio_output<T: Copy>(
    data: &mut [T],
    context: &AudioCallbackContext,
    timing: AudioCallbackTiming,
    convert: impl Fn(f32) -> T,
) {
    if !audio_output_is_playing(&context.playback, context.generation) {
        data.fill(convert(0.0));
        return;
    }
    let mut buffer = lock_audio_buffer(&context.buffer);
    if buffer.drain.failed() {
        data.fill(convert(0.0));
        return;
    }
    let channels = usize::from(buffer.spec.channels.max(1));
    let real_frames = (data.len() / channels).min(buffer.samples.len() / channels);
    let real_samples = real_frames * channels;
    for sample in &mut data[..real_samples] {
        *sample = convert(buffer.samples.pop_front().expect("complete PCM frame"));
    }
    data[real_samples..].fill(convert(0.0));
    let remaining_frames = buffer.samples.len() / channels;
    let sample_rate = buffer.spec.sample_rate;
    buffer.drain.consumed(
        timing.entered_at,
        timing.playback_delay,
        real_frames,
        remaining_frames,
        sample_rate,
    );
    let underrun = real_samples < data.len() && !buffer.drain.pipe_exhausted;
    let notify = buffer.drain.pipe_exhausted && real_frames > 0;
    let failed = buffer.drain.failed();
    drop(buffer);
    if clock_generation_matches(&context.playback, context.generation) {
        context
            .metrics
            .record_audio_output_callback(context.generation, underrun, real_samples);
    }
    if failed {
        context.report_failure();
    } else if notify {
        context.start.notify_output_changed();
    }
}

pub(super) fn push_audio_sample_bytes(
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    bytes: &[u8],
) -> usize {
    let mut buffer = lock_audio_buffer(buffer);
    let frame_bytes = usize::from(buffer.spec.channels.max(1)) * 4;
    let complete_len = bytes.len() - bytes.len() % frame_bytes;
    let was_empty = buffer.samples.is_empty();
    for bytes in bytes[..complete_len].chunks_exact(4) {
        let sample = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if buffer.samples.len() >= buffer.capacity {
            let _ = buffer.samples.pop_front();
        }
        buffer.samples.push_back(sample.clamp(-1.0, 1.0));
    }
    if was_empty && !buffer.samples.is_empty() {
        buffer.drain.queue_became_nonempty(Instant::now());
    }
    drop(buffer);
    complete_len / 4
}

pub(super) fn audio_output_is_playing(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> bool {
    let playback = lock_playback(playback);
    playback.generation_matches(generation) && playback.playing && !playback.ended
}

pub(super) fn f32_to_i16(sample: f32) -> i16 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio samples are clamped to the i16 range before conversion"
    )]
    let converted = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
    converted
}

pub(super) fn f32_to_u16(sample: f32) -> u16 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "audio samples are clamped to the u16 range before conversion"
    )]
    let converted =
        (sample.clamp(-1.0, 1.0).mul_add(0.5, 0.5) * f32::from(u16::MAX)).round() as u16;
    converted
}

pub(super) fn lock_audio_buffer(
    buffer: &Mutex<AudioSampleBuffer>,
) -> MutexGuard<'_, AudioSampleBuffer> {
    buffer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl AudioCallbackContext {
    pub(super) fn fail(&self, message: String) {
        lock_audio_buffer(&self.buffer).drain.fail(message);
        self.report_failure();
    }

    pub(super) fn report_failure(&self) {
        let report = lock_audio_buffer(&self.buffer).drain.take_failure_report();
        if let Some(message) = report {
            push_stderr_line(&self.stderr, message);
            self.start
                .mark_output_failed(self.failure_ends_playback.then_some(self.generation));
        }
    }

    pub(super) fn record_play_result(
        &self,
        result: Result<(), String>,
        now: Instant,
    ) -> Result<(), PreviewEngineError> {
        if let Err(message) = result {
            self.fail(message.clone());
            return Err(PreviewEngineError::Audio(message));
        }
        let mut buffer = lock_audio_buffer(&self.buffer);
        if buffer.drain.failed() {
            return Err(PreviewEngineError::Audio(
                "audio output failed during startup".into(),
            ));
        }
        let queued_frames = buffer.samples.len() / usize::from(buffer.spec.channels.max(1));
        buffer.drain.started(now, queued_frames);
        drop(buffer);
        Ok(())
    }

    pub(super) fn drain_after_pipe_exhausted(&self, stop: &AtomicBool) -> AudioDrainOutcome {
        lock_audio_buffer(&self.buffer).drain.pipe_exhausted = true;
        let outcome = self
            .start
            .wait_for_audio_drain(stop, self.generation, |now| {
                let mut buffer = lock_audio_buffer(&self.buffer);
                let queued_frames = buffer.samples.len() / usize::from(buffer.spec.channels.max(1));
                buffer.drain.poll(now, queued_frames)
            });
        if outcome == AudioDrainOutcome::Failed {
            self.report_failure();
        }
        outcome
    }
}

impl AudioOutputSpec {
    pub(super) fn default_output() -> Result<Self, PreviewEngineError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            PreviewEngineError::Audio("no default audio output device available".to_string())
        })?;
        let supported_config = device.default_output_config().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to read default output config: {err}"))
        })?;
        let config = supported_config.config();
        Ok(Self {
            sample_rate: config.sample_rate,
            channels: config.channels,
        })
    }
}

impl AudioPreviewOutput {
    pub(super) fn new(
        metrics: &PreviewRuntimeMetricsStore,
        playback: &Arc<Mutex<PreviewPlaybackClock>>,
        start: &Arc<AudioPlaybackStart>,
        stderr: &Arc<Mutex<VecDeque<String>>>,
        failure_ends_playback: bool,
        generation: u64,
    ) -> Result<Self, PreviewEngineError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            PreviewEngineError::Audio("no default audio output device available".to_string())
        })?;
        let supported_config = device.default_output_config().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to read default output config: {err}"))
        })?;
        let sample_format = supported_config.sample_format();
        let config = supported_config.config();
        let capacity = usize::try_from(config.sample_rate)
            .unwrap_or(48_000)
            .saturating_mul(usize::from(config.channels))
            .saturating_mul(AUDIO_BUFFER_SECONDS);
        let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
            samples: VecDeque::with_capacity(capacity),
            capacity,
            spec: AudioOutputSpec {
                sample_rate: config.sample_rate,
                channels: config.channels,
            },
            drain: AudioDrainState::default(),
        }));
        let context = AudioCallbackContext {
            buffer,
            metrics: metrics.clone(),
            playback: Arc::clone(playback),
            start: Arc::clone(start),
            generation,
            stderr: Arc::clone(stderr),
            failure_ends_playback,
        };
        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_audio_output_stream::<f32>(
                &device,
                &config,
                &context,
                std::convert::identity,
            )?,
            cpal::SampleFormat::F64 => {
                build_audio_output_stream::<f64>(&device, &config, &context, f64::from)?
            }
            cpal::SampleFormat::I16 => {
                build_audio_output_stream::<i16>(&device, &config, &context, f32_to_i16)?
            }
            cpal::SampleFormat::U16 => {
                build_audio_output_stream::<u16>(&device, &config, &context, f32_to_u16)?
            }
            format => {
                return Err(PreviewEngineError::Audio(format!(
                    "unsupported output sample format: {format:?}"
                )));
            }
        };
        Ok(Self {
            stream,
            context,
            frame_bytes: usize::from(config.channels.max(1)) * 4,
        })
    }

    pub(super) fn play(&self) -> Result<(), PreviewEngineError> {
        let result = self
            .stream
            .play()
            .map_err(|error| format!("failed to start output stream: {error}"));
        self.context.record_play_result(result, Instant::now())
    }
}
