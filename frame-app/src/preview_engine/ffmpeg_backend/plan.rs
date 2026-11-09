use crate::preview_engine::error::PreviewEngineError;
use crate::preview_engine::ffmpeg_backend::audio_output::AudioOutputSpec;
use crate::preview_engine::ffmpeg_backend::clock::parsed_time;
use crate::preview_engine::types::{PreviewSessionConfig, PreviewSourceKind};
use frame_core::preview::{
    PreviewAudioFfmpegOptions, PreviewFfmpegOptions, PreviewFfmpegPlan, build_ffmpeg_preview_args,
    build_ffmpeg_preview_audio_args, build_ffmpeg_snapshot_preview_args,
};

pub(super) fn preview_plan(
    config: &PreviewSessionConfig,
    seconds: f64,
    realtime: bool,
    precise: bool,
) -> Result<PreviewFfmpegPlan, PreviewEngineError> {
    build_preview_plan(config, seconds, realtime, precise, false)
}

pub(super) fn snapshot_preview_plan(
    config: &PreviewSessionConfig,
    seconds: f64,
    precise: bool,
) -> Result<PreviewFfmpegPlan, PreviewEngineError> {
    build_preview_plan(config, seconds, false, precise, true)
}

fn build_preview_plan(
    config: &PreviewSessionConfig,
    seconds: f64,
    realtime: bool,
    precise: bool,
    snapshot: bool,
) -> Result<PreviewFfmpegPlan, PreviewEngineError> {
    let end_seconds = parsed_time(config.conversion_config.end_time.as_deref())
        .filter(|end_seconds| *end_seconds > seconds);
    let builder = if snapshot {
        build_ffmpeg_snapshot_preview_args
    } else {
        build_ffmpeg_preview_args
    };
    builder(
        config.path.to_string_lossy().as_ref(),
        &config.conversion_config,
        &PreviewFfmpegOptions {
            start_seconds: seconds,
            end_seconds,
            source_width: config.source_width,
            source_height: config.source_height,
            source_is_palette: config.source_is_palette,
            source_sample_aspect_ratio: config.source_sample_aspect_ratio,
            source_clock: config.source_clock,
            preview_burn_clock: config.preview_burn_clock,
            max_width: config.max_width,
            max_height: config.max_height,
            fps: config.fps,
            realtime,
            precise_seek: precise,
            source_is_image: config.source_kind == PreviewSourceKind::Image,
        },
    )
    .map_err(|err| PreviewEngineError::Ffmpeg(err.to_string()))
}

pub(super) fn preview_audio_plan(
    config: &PreviewSessionConfig,
    seconds: f64,
    realtime: bool,
    precise: bool,
    output: AudioOutputSpec,
) -> Result<frame_core::preview::PreviewAudioFfmpegPlan, PreviewEngineError> {
    let selected_track = config.selected_audio_track.ok_or_else(|| {
        PreviewEngineError::InvalidInput(
            "Audio preview requires an explicitly selected source track".to_string(),
        )
    })?;
    let end_seconds = parsed_time(config.conversion_config.end_time.as_deref())
        .filter(|end_seconds| *end_seconds > seconds);
    build_ffmpeg_preview_audio_args(
        config.path.to_string_lossy().as_ref(),
        &config.conversion_config,
        &PreviewAudioFfmpegOptions {
            start_seconds: seconds,
            end_seconds,
            sample_rate: output.sample_rate,
            channels: output.channels,
            realtime,
            precise_seek: precise,
            selected_track,
        },
    )
    .map_err(|err| PreviewEngineError::Ffmpeg(err.to_string()))
}

pub(super) fn insert_frame_limit(args: &mut Vec<String>, frames: u32) {
    let insert_at = args.len().saturating_sub(1);
    args.insert(insert_at, "-frames:v".to_string());
    args.insert(insert_at + 1, frames.to_string());
}
