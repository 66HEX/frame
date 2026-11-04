use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::clock::{
    PreviewPlaybackClock, clock_generation_matches, lock_playback,
};
use crate::preview_engine::metrics::PreviewRuntimeMetricsStore;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

pub(super) const AUDIO_BUFFER_SECONDS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioOutputSpec {
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
}

pub(super) struct AudioPreviewOutput {
    pub(super) stream: cpal::Stream,
    pub(super) buffer: Arc<Mutex<AudioSampleBuffer>>,
}

pub(super) struct AudioSampleBuffer {
    pub(super) samples: VecDeque<f32>,
    pub(super) capacity: usize,
}

pub(super) fn build_audio_output_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            *config,
            move |data: &mut [f32], _| {
                fill_audio_output_f32(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build f32 output stream: {err}"))
        })
}

pub(super) fn build_audio_output_stream_f64(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            *config,
            move |data: &mut [f64], _| {
                fill_audio_output_f64(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build f64 output stream: {err}"))
        })
}

pub(super) fn build_audio_output_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            *config,
            move |data: &mut [i16], _| {
                fill_audio_output_i16(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build i16 output stream: {err}"))
        })
}

pub(super) fn build_audio_output_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> Result<cpal::Stream, PreviewEngineError> {
    let buffer = Arc::clone(buffer);
    let metrics = metrics.clone();
    let playback = Arc::clone(playback);
    device
        .build_output_stream(
            *config,
            move |data: &mut [u16], _| {
                fill_audio_output_u16(data, &buffer, &metrics, &playback, generation);
            },
            |err| eprintln!("frame preview audio stream error: {err}"),
            None,
        )
        .map_err(|err| {
            PreviewEngineError::Audio(format!("failed to build u16 output stream: {err}"))
        })
}

pub(super) fn fill_audio_output_f32(
    data: &mut [f32],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(0.0);
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = value;
        } else {
            *sample = 0.0;
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

pub(super) fn fill_audio_output_f64(
    data: &mut [f64],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(0.0);
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = f64::from(value);
        } else {
            *sample = 0.0;
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

pub(super) fn fill_audio_output_i16(
    data: &mut [i16],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(0);
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = f32_to_i16(value);
        } else {
            *sample = 0;
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

pub(super) fn fill_audio_output_u16(
    data: &mut [u16],
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) {
    if !audio_output_is_playing(playback, generation) {
        data.fill(f32_to_u16(0.0));
        return;
    }

    let mut buffer = lock_audio_buffer(buffer);
    let mut underrun = false;
    for sample in data {
        if let Some(value) = buffer.samples.pop_front() {
            *sample = f32_to_u16(value);
        } else {
            *sample = f32_to_u16(0.0);
            underrun = true;
        }
    }
    drop(buffer);
    record_audio_callback_if_current(metrics, playback, generation, underrun);
}

pub(super) fn push_audio_sample_bytes(
    buffer: &Arc<Mutex<AudioSampleBuffer>>,
    bytes: &[u8],
) -> usize {
    let mut samples = 0_usize;
    {
        let mut buffer = lock_audio_buffer(buffer);
        for bytes in bytes.chunks_exact(4) {
            let sample = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if buffer.samples.len() >= buffer.capacity {
                let _ = buffer.samples.pop_front();
            }
            buffer.samples.push_back(sample.clamp(-1.0, 1.0));
            samples = samples.saturating_add(1);
        }
        drop(buffer);
    }
    samples
}

pub(super) fn audio_output_is_playing(
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
) -> bool {
    let playback = lock_playback(playback);
    playback.generation_matches(generation) && playback.playing && !playback.ended
}

pub(super) fn record_audio_callback_if_current(
    metrics: &PreviewRuntimeMetricsStore,
    playback: &Arc<Mutex<PreviewPlaybackClock>>,
    generation: u64,
    underrun: bool,
) {
    if clock_generation_matches(playback, generation) {
        metrics.record_audio_output_callback(generation, underrun);
    }
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
        let channels = config.channels;
        let sample_rate = config.sample_rate;
        let capacity = usize::try_from(sample_rate)
            .unwrap_or(48_000)
            .saturating_mul(usize::from(channels))
            .saturating_mul(AUDIO_BUFFER_SECONDS);
        let buffer = Arc::new(Mutex::new(AudioSampleBuffer {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }));

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_audio_output_stream_f32(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            cpal::SampleFormat::F64 => build_audio_output_stream_f64(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            cpal::SampleFormat::I16 => build_audio_output_stream_i16(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            cpal::SampleFormat::U16 => build_audio_output_stream_u16(
                &device, &config, &buffer, metrics, playback, generation,
            )?,
            format => {
                return Err(PreviewEngineError::Audio(format!(
                    "unsupported output sample format: {format:?}"
                )));
            }
        };

        Ok(Self { stream, buffer })
    }

    pub(super) fn play(&self) -> Result<(), PreviewEngineError> {
        self.stream.play().map_err(|err| {
            PreviewEngineError::Audio(format!("failed to start output stream: {err}"))
        })
    }
}
