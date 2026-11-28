//! Negotiate the selected file and filters against the installed worker before starting a batch.

use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits};
use crate::capabilities::HardwareEncoder;
use crate::runtime_binaries::media_worker_executable;
use frame_core::args::{VideoRoute, build_ffmpeg_args, build_output_path, effective_video_encoder};
use frame_core::error::ConversionError;
use frame_core::types::{ConversionTask, ProbeMetadata};
use std::path::Path;
use std::time::Duration;

pub fn needs_video_route(task: &ConversionTask) -> bool {
    !task.config.copies_video()
        && HardwareEncoder::from_name(&task.config.video_codec).is_some()
        && !frame_core::utils::is_audio_only_container(&task.config.container)
        && !frame_core::media_rules::is_image_container(&task.config.container)
        && task.config.container != "gif"
}

pub fn plan_task_video(
    task: &mut ConversionTask,
    cancelled: impl Fn() -> bool,
) -> Result<Option<VideoRoute>, ConversionError> {
    if !needs_video_route(task) {
        return Ok(None);
    }
    let worker = media_worker_executable();
    let probe = crate::source_probe::probe(&worker, Path::new(&task.file_path), &cancelled)?;
    let Some(encoder) = effective_video_encoder(&task.config, &probe) else {
        return Ok(None);
    };
    let requested = encoder.to_owned();
    let output = build_output_path(
        &task.output_directory,
        &task.config.container,
        task.output_name.as_deref(),
    );
    let args = build_ffmpeg_args(&task.file_path, &output, &task.config, &probe)?;
    match probe_route(&worker, &args, &requested, &cancelled) {
        Ok(route) => Ok(Some(route)),
        Err(error) => {
            if cancelled() {
                return Err(error);
            }
            let Some(alternative) = software_alternative(task, &probe) else {
                return Err(ConversionError::InvalidInput(format!(
                    "Cannot use {requested}. Choose another encoder in Video settings. {error}"
                )));
            };
            let args = build_ffmpeg_args(&task.file_path, &output, &alternative, &probe)?;
            let mut route = probe_route(&worker, &args, &alternative.video_codec, &cancelled)
                .map_err(|_| error)?;
            route.fallback = Some(format!(
                "{requested} could not run this export. Use {} on the CPU at {} kbps.",
                alternative.video_codec, alternative.video_bitrate
            ));
            task.config = alternative;
            Ok(Some(route))
        }
    }
}

// Only offer settings that have the same meaning on the software encoder.
fn software_alternative(
    task: &ConversionTask,
    probe: &ProbeMetadata,
) -> Option<frame_core::types::ConversionConfig> {
    if task.config.video_bitrate_mode != "bitrate" {
        return None;
    }
    let mut config = task.config.clone();
    let codec = match config.video_codec.split_once('_')?.0 {
        "h264" => "libx264",
        "hevc" => "libx265",
        "av1" => "libsvtav1",
        _ => return None,
    };
    if !frame_core::media_rules::is_video_codec_allowed(&config.container, codec) {
        return None;
    }
    if config.pixel_format == "auto" {
        // Match the 4:2:0 output family of the selected hardware encoder.
        config.pixel_format = if codec != "libx264" && probe.component_depth? > 8 {
            "yuv420p10le"
        } else {
            "yuv420p"
        }
        .to_string();
    }
    codec.clone_into(&mut config.video_codec);
    if codec == "libsvtav1" { "8" } else { "medium" }.clone_into(&mut config.preset);
    config.hw_decode = false;
    config.nvenc_spatial_aq = false;
    config.nvenc_temporal_aq = false;
    config.videotoolbox_allow_sw = false;
    Some(config)
}

fn probe_route(
    worker: &str,
    args: &[String],
    encoder: &str,
    cancelled: impl Fn() -> bool,
) -> Result<VideoRoute, ConversionError> {
    static ROUTE_PROBES: ExecutionDomain = ExecutionDomain::new();
    let mut command = CommandSpec::new(worker);
    command.args.push("preflight".into());
    command.args.extend(args.iter().map(Into::into));
    let result = ROUTE_PROBES
        .run(
            &command,
            &[],
            Limits {
                timeout: Duration::from_secs(30),
                ..Limits::default()
            },
            cancelled,
        )
        .map_err(|error| ConversionError::Probe(format!("Video preflight failed: {error}")))?;
    if !result.success {
        return Err(ConversionError::Probe(
            String::from_utf8_lossy(&result.stderr.bytes)
                .trim()
                .to_owned(),
        ));
    }
    let route: VideoRoute = serde_json::from_slice(&result.stdout.bytes).map_err(|error| {
        ConversionError::Probe(format!("Invalid video preflight report: {error}"))
    })?;
    if result.stdout.truncated
        || route.encoder != encoder
        || route.device.is_empty()
        || route.format.is_empty()
    {
        return Err(ConversionError::Probe(
            "Incomplete video preflight report".into(),
        ));
    }
    Ok(route)
}
