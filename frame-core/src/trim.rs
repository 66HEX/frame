//! Shared output timing and clipping for imported selectable text subtitles.

use crate::error::ConversionError;
use crate::media_rules::container_supports_subtitles;
use crate::types::ConversionConfig;
use crate::utils::{format_ffmpeg_seconds, parse_time};
use std::path::Path;

/// A request-local raw `FFmpeg` duration literal and its exact parsed offset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawFfmpegTime {
    pub literal: String,
    pub microseconds: i64,
}

impl RawFfmpegTime {
    /// Validates a raw duration, adding only a missing plain-decimal leading zero.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let (sign, body) = value.strip_prefix('-').map_or_else(
            || {
                value
                    .strip_prefix('+')
                    .map_or(("", value), |body| ("+", body))
            },
            |body| ("-", body),
        );
        let literal = body
            .strip_prefix('.')
            .filter(|fraction| {
                !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
            })
            .map_or_else(
                || value.to_string(),
                |fraction| format!("{sign}0.{fraction}"),
            );
        let microseconds = raw_time_microseconds(&literal)?;
        Some(Self {
            literal,
            microseconds,
        })
    }

    /// Converts the already truncated offset for numeric preview APIs.
    #[must_use]
    pub const fn seconds(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "Numeric playback clocks use f64; raw literals and burn clocks retain the exact integer separately."
        )]
        let seconds = self.microseconds as f64 / 1_000_000.0;
        seconds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputTrim {
    input_start: Option<String>,
    output_limit: Option<(&'static str, String)>,
    packet_limit: Option<String>,
    starts_after_zero: bool,
}

impl OutputTrim {
    /// Computes the same validated window for the output and subtitle packets.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid/nonfinite times or an end before the start.
    pub fn from_config(config: &ConversionConfig) -> Result<Self, ConversionError> {
        let start = non_empty(config.start_time.as_deref());
        let end = non_empty(config.end_time.as_deref());
        let raw_start = start
            .map(|value| {
                RawFfmpegTime::parse(value).ok_or_else(|| {
                    ConversionError::InvalidInput(format!("Invalid start time: {value}"))
                })
            })
            .transpose()?;
        let start_seconds = parse_optional_time(start, "start")?;
        let end_seconds = parse_optional_time(end, "end")?;
        if let (Some(start), Some(end)) = (start_seconds, end_seconds)
            && end <= start
        {
            return Err(ConversionError::InvalidInput(
                "End time must be greater than start time".into(),
            ));
        }
        let (output_limit, packet_limit) = match (end, end_seconds, start_seconds) {
            (_, Some(end), Some(start)) => {
                let duration = end - start;
                if !duration.is_finite() {
                    return Err(ConversionError::InvalidInput(
                        "Invalid trim duration".into(),
                    ));
                }
                let formatted = format_ffmpeg_seconds(duration);
                (Some(("-t", formatted.clone())), Some(formatted))
            }
            (Some(raw), Some(_), None) => {
                // Raw -to uses FFmpeg's lexical microsecond truncation. Keep the
                // original argument; rounding it through f64 can add a frame.
                let parsed = RawFfmpegTime::parse(raw).ok_or_else(|| {
                    ConversionError::InvalidInput(format!("Invalid end time: {raw}"))
                })?;
                (
                    Some(("-to", parsed.literal)),
                    Some(format_microseconds(parsed.microseconds)),
                )
            }
            _ => (None, None),
        };
        Ok(Self {
            starts_after_zero: raw_start
                .as_ref()
                .is_some_and(|start| start.microseconds > 0),
            input_start: raw_start.map(|start| start.literal),
            output_limit,
            packet_limit,
        })
    }

    pub fn append_input_args(&self, args: &mut Vec<String>) {
        if let Some(start) = &self.input_start {
            args.extend(["-ss".into(), start.clone()]);
        }
    }

    pub fn append_output_args(&self, args: &mut Vec<String>) {
        if let Some((option, value)) = &self.output_limit {
            args.extend([(*option).into(), value.clone()]);
        }
    }

    const fn has_trim(&self) -> bool {
        self.starts_after_zero || self.output_limit.is_some()
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn parse_optional_time(value: Option<&str>, label: &str) -> Result<Option<f64>, ConversionError> {
    value
        .map(|value| {
            parse_time(value)
                .filter(|seconds| seconds.is_finite())
                .ok_or_else(|| {
                    ConversionError::InvalidInput(format!("Invalid {label} time: {value}"))
                })
        })
        .transpose()
}

// Narrow intersection of Frame's time syntax and FFmpeg av_parse_time's raw
// duration literals. No global parse_time changes or floating-point scaling.
fn raw_time_microseconds(value: &str) -> Option<i64> {
    let negative = value.starts_with('-');
    let body = value.strip_prefix(['-', '+']).unwrap_or(value);
    let parts: Vec<_> = body.split(':').collect();
    if !(1..=3).contains(&parts.len()) || (parts.len() > 1 && value.starts_with('+')) {
        return None;
    }
    let seconds_part = parts.last()?;
    let (seconds, fraction) = seconds_part.split_once('.').unwrap_or((seconds_part, ""));
    let mut whole = ascii_integer(seconds)?;
    if parts.len() > 1 {
        if seconds.len() > 2 || whole >= 60 {
            return None;
        }
        let minutes = parts[parts.len() - 2];
        let minutes_value = ascii_integer(minutes)?;
        if minutes.len() > 2 || minutes_value >= 60 {
            return None;
        }
        whole += minutes_value * 60;
    }
    if parts.len() == 3 {
        let hours = parts[0];
        if hours.len() > 4 {
            return None;
        }
        whole += ascii_integer(hours)? * 3600;
    }
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut subsecond = fraction
        .bytes()
        .take(6)
        .fold(0_u64, |total, digit| total * 10 + u64::from(digit - b'0'));
    for _ in fraction.len().min(6)..6 {
        subsecond *= 10;
    }
    let magnitude = whole.checked_mul(1_000_000)?.checked_add(subsecond)?;
    let micros = i64::try_from(magnitude).ok()?;
    Some(if negative { -micros } else { micros })
}

fn ascii_integer(value: &str) -> Option<u64> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

pub(crate) fn format_microseconds(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let magnitude = value.unsigned_abs();
    format!(
        "{sign}{}.{:06}",
        magnitude / 1_000_000,
        magnitude % 1_000_000
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtitleTrimPlan {
    pub output_ordinals: Vec<usize>,
    pub bitstream_filter: String,
}

impl SubtitleTrimPlan {
    pub(crate) fn for_window(config: &ConversionConfig, window: &OutputTrim) -> Option<Self> {
        if !window.has_trim() || !container_supports_subtitles(&config.container) {
            return None;
        }
        let output_ordinals: Vec<_> = config
            .external_subtitle_tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                Path::new(&track.path)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        ["srt", "ass", "vtt"]
                            .iter()
                            .any(|text| extension.eq_ignore_ascii_case(text))
                    })
            })
            .map(|(offset, _)| config.selected_subtitle_tracks.len() + offset)
            .collect();
        if output_ordinals.is_empty() {
            return None;
        }
        Some(Self {
            output_ordinals,
            bitstream_filter: subtitle_window_filter(window.packet_limit.as_deref()),
        })
    }

    #[must_use]
    pub const fn required_bitstream_filters(&self) -> [&'static str; 2] {
        ["noise", "setts"]
    }

    pub fn append_args(&self, args: &mut Vec<String>) {
        for ordinal in &self.output_ordinals {
            args.extend([format!("-bsf:s:{ordinal}"), self.bitstream_filter.clone()]);
        }
    }
}

/// Plans mandatory packet clipping for mapped external SRT/ASS/VTT tracks.
///
/// The caller first validates stream mapping using the normal argument builder;
/// embedded streams retain their selected order and precede all external tracks.
///
/// # Errors
///
/// Returns an error for invalid trim times, as does the normal output builder.
pub fn subtitle_trim_plan(
    config: &ConversionConfig,
) -> Result<Option<SubtitleTrimPlan>, ConversionError> {
    Ok(SubtitleTrimPlan::for_window(
        config,
        &OutputTrim::from_config(config)?,
    ))
}

fn subtitle_window_filter(length: Option<&str>) -> String {
    // Keep unknown timestamps/durations unchanged. Zero-duration stateful cues
    // survive, with a known negative start clamped so they cannot shift the mux.
    let guard = "gte(duration,0)*not(eq(pts,nopts))*gt(tb,0)";
    let upper_guard = "gte(DURATION,0)*not(eq(PTS,NOPTS))*gt(TB,0)";
    let mut drop = "gt(duration,0)*lte(pts+duration,0)".to_string();
    let duration = length.map_or_else(
        || "max(0,DURATION+min(PTS,0))".to_string(),
        |length| {
            drop.push_str("+gte(pts,");
            drop.push_str(length);
            drop.push_str("/tb)");
            format!("max(0,min(PTS+DURATION,{length}/TB)-max(PTS,0))")
        },
    );
    format!(
        "noise=amount=0:drop='if({guard},{drop},0)',setts=pts='if({upper_guard},max(PTS,0),PTS)':dts='if({upper_guard}*not(eq(DTS,NOPTS)),max(DTS,0),DTS)':duration='if({upper_guard}*gt(DURATION,0),{duration},DURATION)'"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_time_preserves_valid_literals_and_normalizes_only_plain_missing_zero() {
        for (input, literal, microseconds) in [
            (" .123456789 ", "0.123456789", 123_456),
            ("-.123456789", "-0.123456789", -123_456),
            ("+.5", "+0.5", 500_000),
            ("0.999999999", "0.999999999", 999_999),
            ("-00:00:00.5", "-00:00:00.5", -500_000),
            ("-1:30", "-1:30", -90_000_000),
            ("0.0000009", "0.0000009", 0),
            ("9223372036854.775807", "9223372036854.775807", i64::MAX),
        ] {
            let parsed = RawFfmpegTime::parse(input).unwrap();
            assert_eq!(parsed.literal, literal, "{input}");
            assert_eq!(parsed.microseconds, microseconds, "{input}");
        }
        for input in [
            ".",
            "-.",
            ".5e1",
            ".5s",
            "00:00:.5",
            "+1:30",
            "1:60",
            "1e-3",
            "9223372036854.775808",
        ] {
            assert_eq!(RawFfmpegTime::parse(input), None, "{input}");
        }
    }

    #[test]
    fn raw_time_uses_checked_integer_microseconds_and_truncates_fraction_digits() {
        for (input, expected) in [
            ("0.9999999", 999_999),
            ("1.0000009", 1_000_000),
            ("0.0000009", 0),
            ("-0.123456789", -123_456),
            ("00:00:00.123456789", 123_456),
            ("+1.2", 1_200_000),
            ("1.", 1_000_000),
            ("-01:02.0000019", -62_000_001),
            ("1234:05:06.000001", 4_442_706_000_001),
            ("9223372036854.775807", i64::MAX),
            ("-9223372036854.775807", -i64::MAX),
        ] {
            assert_eq!(raw_time_microseconds(input), Some(expected), "{input}");
        }
        for input in [
            "",
            ".5",
            "1e-3",
            "NaN",
            "inf",
            "1..2",
            "1.-2",
            "+01:02",
            "1:60",
            "60:01",
            "12345:01:02",
            "1:002",
            "1:2:3:4",
            "--1",
            "1:-2",
            "１",
            "9223372036854.775808",
            "-9223372036854.775808",
            "18446744073709551616",
            "18446744073709551615",
        ] {
            assert_eq!(raw_time_microseconds(input), None, "{input}");
        }
        assert_eq!(format_microseconds(0), "0.000000");
        assert_eq!(format_microseconds(999_999), "0.999999");
        assert_eq!(format_microseconds(-123_456), "-0.123456");
        assert_eq!(format_microseconds(i64::MAX), "9223372036854.775807");
    }
}
