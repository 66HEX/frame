//! Rejects trimmed Opus stream copy when selected packet timing is incomplete.
//! This is a scoped preflight, not a decoded-audio or sample-accurate trim proof.

use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits, StopReason};
use crate::capabilities::runtime::{Runtime, RuntimeIdentity};
use frame_core::error::ConversionError;
use frame_core::trim::{OutputTrim, RawFfmpegTime};
use frame_core::types::{ConversionConfig, ConversionTask, ProbeMetadata};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const CHECK_TIME: Duration = Duration::from_secs(15);
const CAPTURE_BYTES: usize = 4 * 1024 * 1024;
// RFC 7845 sections 3 and 4.6: maximum packet duration and seek pre-roll.
const PACKET_MICROS: i64 = 120_000;
const PREROLL_MICROS: i64 = 80_000;

pub(super) struct OpusProof {
    paths: Vec<PathBuf>,
    source: RuntimeIdentity,
    runtime: Runtime,
}

impl OpusProof {
    pub fn check_unchanged(&self) -> Result<(), ConversionError> {
        if !self.source.matches_paths(&self.paths) || !self.runtime.identity_unchanged() {
            return Err(refused(
                "the source or probe changed during the timing check",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Window {
    start: i64,
    end: Option<i64>,
    origin: i64,
}

impl Window {
    fn from_config(
        config: &ConversionConfig,
        probe: &ProbeMetadata,
    ) -> Result<Option<Self>, ConversionError> {
        let trim = OutputTrim::from_config(config)?;
        let mut input = Vec::new();
        let mut output = Vec::new();
        trim.append_input_args(&mut input);
        trim.append_output_args(&mut output);
        let start = input
            .get(1)
            .map_or(Some(0), |value| {
                RawFfmpegTime::parse(value).map(|value| value.microseconds)
            })
            .ok_or_else(|| refused("the trim start cannot be represented"))?;
        let limit = output
            .get(1)
            .map(|value| {
                RawFfmpegTime::parse(value)
                    .map(|value| value.microseconds)
                    .ok_or_else(|| refused("the trim end cannot be represented"))
            })
            .transpose()?;
        if start <= 0 && limit.is_none() {
            return Ok(None);
        }
        let timing = probe
            .source_timing
            .as_ref()
            .filter(|timing| !timing.format_start_invalid)
            .ok_or_else(|| refused("the source clock is unavailable"))?;
        let origin = timing.format_start_microseconds.unwrap_or(0);
        let end = if output.first().is_some_and(|option| option == "-t") {
            limit
                .map(|duration| {
                    start
                        .checked_add(duration)
                        .ok_or_else(|| refused("the trim duration overflows"))
                })
                .transpose()?
        } else {
            limit
        };
        let start = origin
            .checked_add(start)
            .ok_or_else(|| refused("the source clock overflows"))?;
        let end = end
            .map(|end| {
                origin
                    .checked_add(end)
                    .ok_or_else(|| refused("the source clock overflows"))
            })
            .transpose()?;
        Ok(Some(Self { start, end, origin }))
    }

    fn seek(&self) -> Option<i64> {
        self.start
            .checked_sub(PREROLL_MICROS + PACKET_MICROS)
            .filter(|start| *start > self.origin)
    }

    fn scan_end(&self) -> Result<Option<i64>, ConversionError> {
        self.end
            .map(|end| {
                end.checked_add(PACKET_MICROS)
                    .ok_or_else(|| refused("the scan interval overflows"))
            })
            .transpose()
    }

    fn interval(&self) -> Result<String, ConversionError> {
        let start = self.seek().map_or_else(String::new, seconds);
        let end = self.scan_end()?.map_or_else(String::new, seconds);
        Ok(format!("{start}%{end}"))
    }
}

pub(super) fn check(
    task: &ConversionTask,
    probe: &ProbeMetadata,
    executable: &str,
    cancelled: impl Fn() -> bool,
) -> Result<Option<OpusProof>, ConversionError> {
    if task.config.processing_mode != "copy" || task.config.container != "opus" {
        return Ok(None);
    }
    let selected: BTreeSet<_> = task
        .config
        .selected_audio_tracks
        .iter()
        .copied()
        .filter(|index| {
            probe
                .audio_tracks
                .iter()
                .any(|track| track.index == *index && track.codec == "opus")
        })
        .collect();
    if selected.is_empty() {
        return Ok(None);
    }
    let Some(window) = Window::from_config(&task.config, probe)? else {
        return Ok(None);
    };
    let paths = vec![PathBuf::from(&task.file_path)];
    let source = RuntimeIdentity::capture_paths(&paths)
        .ok_or_else(|| refused("the source identity could not be observed"))?;
    let runtime = Runtime::resolve(executable)
        .map_err(|error| refused(&format!("the probe could not run: {error}")))?;
    let proof = OpusProof {
        paths,
        source,
        runtime,
    };
    let deadline = Instant::now() + CHECK_TIME;
    let mut remaining_bytes = CAPTURE_BYTES;
    for index in selected {
        scan_track(
            &proof.runtime,
            Path::new(&task.file_path),
            index,
            &window,
            deadline,
            &mut remaining_bytes,
            &cancelled,
        )?;
    }
    proof.check_unchanged()?;
    Ok(Some(proof))
}

fn scan_track(
    runtime: &Runtime,
    path: &Path,
    index: u32,
    window: &Window,
    deadline: Instant,
    remaining_bytes: &mut usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), ConversionError> {
    let timeout = deadline.saturating_duration_since(Instant::now());
    if timeout.is_zero() || *remaining_bytes == 0 {
        return Err(refused("the timing check reached its time or output limit"));
    }
    let mut command = runtime.command.clone();
    command.args = if let Some(probe) =
        crate::runtime_binaries::worker_subcommand(&command.program, "probe")
    {
        vec![
            probe.into(),
            "opus-timing".into(),
            index.to_string().into(),
            window
                .seek()
                .map_or_else(String::new, |us| us.to_string())
                .into(),
            window
                .scan_end()?
                .map_or_else(String::new, |us| us.to_string())
                .into(),
            path.as_os_str().to_owned(),
        ]
    } else {
        vec![
            "-v".into(),
            "error".into(),
            "-select_streams".into(),
            index.to_string().into(),
            "-read_intervals".into(),
            window.interval()?.into(),
            "-show_packets".into(),
            "-show_streams".into(),
            "-show_format".into(),
            "-show_entries".into(),
            "packet=stream_index,pts,duration:stream=index,codec_name,time_base,start_pts:format=start_time".into(),
            "-of".into(),
            "json=compact=1".into(),
            path.as_os_str().to_owned(),
        ]
    };
    let bytes = read_scan(
        &command,
        Limits {
            timeout,
            cleanup_timeout: Duration::from_secs(1),
            stdout_bytes: *remaining_bytes,
            stderr_bytes: 64 * 1024,
        },
        cancelled,
    )?;
    *remaining_bytes = remaining_bytes.saturating_sub(bytes.len());
    let scan: Scan = serde_json::from_slice(&bytes)
        .map_err(|_| refused("the timing check returned invalid metadata"))?;
    validate_scan(&scan, index, window)
}

fn read_scan(
    command: &CommandSpec,
    limits: Limits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, ConversionError> {
    static OPUS_CHECKS: ExecutionDomain = ExecutionDomain::new();
    let result = OPUS_CHECKS
        .run(command, &[], limits, cancelled)
        .map_err(|error| {
            refused(match error.reason {
                StopReason::TimedOut => "the timing check reached its time limit",
                StopReason::Cancelled => "the timing check was cancelled",
                StopReason::CleanupInProgress => {
                    "the previous timing check is still stopping; retry shortly"
                }
                StopReason::Io(_) => "the timing check could not run",
            })
        })?;
    if cancelled() {
        return Err(refused("the timing check was cancelled"));
    }
    if !result.success || result.stdout.truncated || result.stderr.truncated {
        return Err(refused(
            "the timing check failed or reached its output limit",
        ));
    }
    Ok(result.stdout.bytes)
}

#[derive(Deserialize)]
struct Scan {
    packets: Vec<Packet>,
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Deserialize)]
struct Packet {
    stream_index: u32,
    pts: Option<i64>,
    duration: Option<i64>,
}

#[derive(Deserialize)]
struct Stream {
    index: u32,
    codec_name: String,
    time_base: String,
    start_pts: Option<i64>,
}

#[derive(Deserialize)]
struct Format {
    start_time: Option<String>,
}

fn validate_scan(scan: &Scan, index: u32, window: &Window) -> Result<(), ConversionError> {
    let [stream] = scan.streams.as_slice() else {
        return Err(refused("the selected stream could not be identified"));
    };
    if stream.index != index || stream.codec_name != "opus" {
        return Err(refused(
            "the selected stream changed during the timing check",
        ));
    }
    let origin = scan.format.start_time.as_deref().map_or(Some(0), |value| {
        RawFfmpegTime::parse(value).map(|value| value.microseconds)
    });
    if origin != Some(window.origin) {
        return Err(refused("the source clock changed during the timing check"));
    }
    let (numerator, denominator) = stream
        .time_base
        .split_once('/')
        .and_then(|(n, d)| Some((n.parse::<u32>().ok()?, d.parse::<u32>().ok()?)))
        .filter(|(n, d)| *n > 0 && *d > 0)
        .ok_or_else(|| incomplete(index))?;
    let first = scan
        .packets
        .first()
        .and_then(|packet| packet.pts)
        .ok_or_else(|| refused("the selected interval could not be observed"))?;
    for packet in &scan.packets {
        if packet.stream_index != index
            || packet.pts.is_none_or(|pts| pts == i64::MIN)
            || packet.duration.is_none_or(|duration| duration <= 0)
        {
            return Err(incomplete(index));
        }
    }
    // ffprobe seeks to a nearby point. Check the observed first packet rather
    // than treating the requested interval as evidence that pre-roll was read.
    if window.seek().is_some() {
        let first_scaled = i128::from(first) * i128::from(numerator) * 1_000_000;
        let boundary = i128::from(window.start) - i128::from(PREROLL_MICROS);
        if first_scaled > boundary * i128::from(denominator) && stream.start_pts != Some(first) {
            return Err(refused(
                "the timing check did not include the required pre-roll",
            ));
        }
    }
    Ok(())
}

fn incomplete(index: u32) -> ConversionError {
    refused(&format!(
        "audio track #{index} has incomplete packet timing"
    ))
}

fn refused(detail: &str) -> ConversionError {
    ConversionError::InvalidInput(format!(
        "Cannot safely copy this Opus trim: {detail}. Adjust the range or choose Re-encode."
    ))
}

fn seconds(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let magnitude = value.unsigned_abs();
    format!(
        "{sign}{}.{:06}",
        magnitude / 1_000_000,
        magnitude % 1_000_000
    )
}

#[cfg(test)]
mod tests;
