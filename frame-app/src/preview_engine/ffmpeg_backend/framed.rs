//! Selected-runtime capability and decoding for packet-bound presentation.

use super::decode::FrameStreamSpec;
use crate::bounded_process::{self, CleanupOutcome, Limits};
use crate::capabilities::runtime::{Runtime, RuntimeIdentity};
use crate::preview_engine::{
    PreviewEngineError, PreviewRenderPresentation, PreviewSessionConfig, PreviewSourceKind,
};
use frame_core::preview_transport::{FrameMetadata, FramedBgraSpec};
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const STATIC_INPUT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tooling/ffmpeg/frame-preview-v1/probes/static.nut"
));
const STATIC_OUTPUT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tooling/ffmpeg/frame-preview-v1/probes/static.fpv"
));
const DYNAMIC_INPUT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tooling/ffmpeg/frame-preview-v1/probes/dynamic.m2ts"
));
const DYNAMIC_OUTPUT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tooling/ffmpeg/frame-preview-v1/probes/dynamic.fpv"
));
static SUPPORT: OnceLock<Mutex<Vec<(RuntimeIdentity, bool)>>> = OnceLock::new();

pub(super) struct VerifiedTransport {
    runtime: Runtime,
}

impl VerifiedTransport {
    pub(super) fn program(&self) -> &std::path::Path {
        &self.runtime.command.program
    }
    pub(super) fn recheck(&self, cancelled: &AtomicBool) -> Result<(), PreviewEngineError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(failure("preview request cancelled"));
        }
        if !self.runtime.identity_unchanged() {
            return Err(failure("FFmpeg changed after the preview protocol check"));
        }
        Ok(())
    }

    pub(super) fn snapshot(
        &self,
        args: &[String],
        spec: FramedBgraSpec,
        cancelled: &AtomicBool,
    ) -> Result<(Vec<u8>, PreviewRenderPresentation), PreviewEngineError> {
        self.recheck(cancelled)?;
        let mut command = self.runtime.command.clone();
        command.args = args.iter().map(Into::into).collect();
        let output = bounded_process::run(
            &command,
            &[],
            Limits {
                timeout: Duration::from_secs(15),
                stdout_bytes: spec.payload_bytes() + frame_core::preview_transport::HEADER_BYTES,
                stderr_bytes: 64 * 1024,
                ..Limits::default()
            },
            || cancelled.load(Ordering::Acquire),
        )
        .map_err(|error| failure(format!("preview frame did not finish: {error}")))?;
        self.recheck(cancelled)?;
        if !output.success || output.stdout.truncated || output.stderr.truncated {
            return Err(failure(format!(
                "preview frame failed (exit {:?}): {}",
                output.code,
                String::from_utf8_lossy(&output.stderr.bytes)
            )));
        }
        let mut reader = Cursor::new(output.stdout.bytes);
        let frame = spec
            .read_frame(&mut reader, 0)
            .map_err(|error| failure(format!("invalid preview frame: {error}")))?
            .ok_or_else(|| failure("preview returned no frame"))?;
        if reader.position() != u64::try_from(reader.get_ref().len()).unwrap_or(u64::MAX) {
            return Err(failure("preview returned more than one frame"));
        }
        Ok((frame.pixels, presentation(spec, frame.metadata)))
    }
}

/// Only native, identity-bound runtimes enter the framed streaming owner.
/// Wrappers and unresolved PATH names retain the existing raw preview path.
pub(super) fn prepare(
    executable: &str,
    config: &PreviewSessionConfig,
    cancelled: &AtomicBool,
) -> Result<Option<VerifiedTransport>, PreviewEngineError> {
    if !config.source_is_mpeg_ts
        || config.source_kind != PreviewSourceKind::Video
        || config.conversion_config.processing_mode != "copy"
    {
        return Ok(None);
    }
    let runtime = Runtime::resolve(executable).map_err(PreviewEngineError::Io)?;
    let Some(identity) = runtime.identity.clone() else {
        return Ok(None);
    };
    let verified = VerifiedTransport { runtime };
    verified.recheck(cancelled)?;
    let cache = SUPPORT.get_or_init(|| Mutex::new(Vec::new()));
    let cached = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find(|(key, _)| key == &identity)
        .map(|(_, supported)| *supported);
    if let Some(supported) = cached {
        return Ok(supported.then_some(verified));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut supported = true;
    for (input, expected, format, fps, frames) in [
        (STATIC_INPUT, STATIC_OUTPUT, "nut", "2", "2"),
        (DYNAMIC_INPUT, DYNAMIC_OUTPUT, "mpegts", "12", "36"),
    ] {
        verified.recheck(cancelled)?;
        let timeout = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| failure("preview protocol check timed out"))?;
        let mut command = verified.runtime.command.clone();
        command.args = probe_args(format, fps, frames)
            .into_iter()
            .map(Into::into)
            .collect();
        let output = bounded_process::run(
            &command,
            input,
            Limits {
                timeout,
                stdout_bytes: 8192,
                stderr_bytes: 16384,
                ..Limits::default()
            },
            || cancelled.load(Ordering::Acquire),
        )
        .map_err(|error| {
            // In particular, a pending cleanup must never launch a fallback.
            let pending = error.cleanup == CleanupOutcome::Pending;
            failure(format!(
                "preview protocol check did not finish (cleanup pending: {pending}): {error}"
            ))
        })?;
        verified.recheck(cancelled)?;
        if output.stdout.truncated || output.stderr.truncated {
            return Err(failure("preview protocol check exceeded its output limit"));
        }
        if output.success && (output.stdin_error.is_some() || output.stdin_written != input.len()) {
            return Err(failure(
                "preview protocol check did not consume its complete input",
            ));
        }
        if !output.success || output.stdout.bytes != expected {
            supported = false;
            break;
        }
    }
    verified.recheck(cancelled)?;
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if cache.len() == 16 {
        cache.remove(0);
    }
    cache.push((identity, supported));
    drop(cache);
    Ok(supported.then_some(verified))
}

fn probe_args(format: &str, fps: &str, frames: &str) -> Vec<String> {
    let filter = format!("fps={fps},scale=4:2:frame_preview_capture=1,setsar=1,format=bgra");
    [
        "-hide_banner",
        "-nostdin",
        "-v",
        "error",
        "-native_frame_sar:v:0",
        "-f",
        format,
        "-i",
        "pipe:0",
        "-vf",
        &filter,
        "-map",
        "0:v:0",
        "-an",
        "-sn",
        "-dn",
        "-frames:v",
        frames,
        "-c:v",
        "rawvideo",
        "-pix_fmt",
        "bgra",
        "-frame_preview_framed",
        "-f",
        "rawvideo",
        "pipe:1",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub(super) fn presentation(
    spec: FramedBgraSpec,
    metadata: FrameMetadata,
) -> PreviewRenderPresentation {
    PreviewRenderPresentation {
        pixel_aspect_ratio: spec.pixel_aspect_ratio(metadata),
        output_raster_aspect_ratio: metadata.output_raster_aspect_ratio(),
        ..Default::default()
    }
}

pub(super) fn read_payload(
    reader: &mut (impl Read + ?Sized),
    spec: &FrameStreamSpec,
    sequence: u64,
) -> io::Result<Option<(Vec<u8>, PreviewRenderPresentation)>> {
    if spec.framed {
        let framed = FramedBgraSpec::new(spec.width, spec.height)?;
        return framed
            .read_frame(reader, sequence)
            .map(|frame| frame.map(|frame| (frame.pixels, presentation(framed, frame.metadata))));
    }
    let mut payload = vec![0; spec.frame_bytes];
    reader.read_exact(&mut payload)?;
    Ok(Some((payload, spec.presentation)))
}

fn failure(message: impl Into<String>) -> PreviewEngineError {
    PreviewEngineError::Ffmpeg(message.into())
}
