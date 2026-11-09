//! Input timestamp facts and the subtitle clock for the streams actually mapped.

use crate::error::ConversionError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TimeBaseFields")]
pub struct TimeBase {
    numerator: u32,
    denominator: u32,
}

#[derive(Deserialize)]
struct TimeBaseFields {
    numerator: u32,
    denominator: u32,
}

impl TryFrom<TimeBaseFields> for TimeBase {
    type Error = &'static str;

    fn try_from(value: TimeBaseFields) -> Result<Self, Self::Error> {
        Self::new(value.numerator, value.denominator).ok_or("Invalid source time base")
    }
}

impl TimeBase {
    #[must_use]
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0
            || denominator == 0
            || numerator > i32::MAX.unsigned_abs()
            || denominator > i32::MAX.unsigned_abs()
        {
            return None;
        }
        let divisor = gcd(u64::from(numerator), u64::from(denominator));
        Some(Self {
            numerator: u32::try_from(u64::from(numerator) / divisor).ok()?,
            denominator: u32::try_from(u64::from(denominator) / divisor).ok()?,
        })
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let (numerator, denominator) = value.split_once('/')?;
        if !numerator.bytes().all(|byte| byte.is_ascii_digit())
            || !denominator.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        Self::new(numerator.parse().ok()?, denominator.parse().ok()?)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStreamTiming {
    pub index: u32,
    #[serde(default, deserialize_with = "deserialize_time_base")]
    pub time_base: Option<TimeBase>,
    pub start_pts: Option<i64>,
}

/// A present payload distinguishes an observed absent start time from an old
/// caller that did not supply timestamp metadata at all.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTiming {
    pub format_start_microseconds: Option<i64>,
    #[serde(default)]
    pub format_start_invalid: bool,
    pub first_video_stream_index: Option<u32>,
    pub streams: Vec<SourceStreamTiming>,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ResolvedSourceClock {
    #[default]
    Unknown,
    Known {
        video_time_base: TimeBase,
        origin_microseconds: i64,
        minimum_start_microseconds: Option<i64>,
    },
    /// Keep an arithmetic failure dormant until a nonzero burn clock uses it.
    Invalid,
}

/// Resolves only streams enabled on the main input, excluding sidecar inputs.
#[must_use]
pub fn resolve_source_clock(
    timing: Option<&SourceTiming>,
    format_name: Option<&str>,
    video_index: Option<u32>,
    enabled_indices: &[u32],
) -> ResolvedSourceClock {
    let Some(timing) = timing else {
        return ResolvedSourceClock::Unknown;
    };
    if timing.format_start_invalid {
        return ResolvedSourceClock::Unknown;
    }
    let Some(format_name) = format_name.filter(|name| !name.trim().is_empty()) else {
        return ResolvedSourceClock::Unknown;
    };
    let Some(video_time_base) = video_index
        .and_then(|index| timing.streams.iter().find(|stream| stream.index == index))
        .and_then(|stream| stream.time_base)
    else {
        return ResolvedSourceClock::Unknown;
    };
    let origin = timing
        .format_start_microseconds
        .filter(|value| *value != i64::MIN)
        .unwrap_or(0);
    let mut minimum_start_microseconds = None;
    if timing
        .format_start_microseconds
        .is_some_and(|value| value != i64::MIN)
        && timestamp_discontinuities(format_name)
    {
        let mut earliest = None;
        for index in enabled_indices {
            let Some(stream) = timing.streams.iter().find(|stream| stream.index == *index) else {
                return ResolvedSourceClock::Unknown;
            };
            let Some(start_pts) = stream.start_pts.filter(|value| *value != i64::MIN) else {
                continue;
            };
            let Some(time_base) = stream.time_base else {
                return ResolvedSourceClock::Unknown;
            };
            let Some(start) = rescale_near_inf(
                start_pts,
                i128::from(time_base.numerator) * 1_000_000,
                i128::from(time_base.denominator),
            ) else {
                return ResolvedSourceClock::Invalid;
            };
            earliest = Some(earliest.map_or(start, |previous: i64| previous.min(start)));
        }
        let Some(earliest) = earliest else {
            return ResolvedSourceClock::Unknown;
        };
        if earliest != origin {
            minimum_start_microseconds = Some(earliest);
        }
    }
    ResolvedSourceClock::Known {
        video_time_base,
        origin_microseconds: origin,
        minimum_start_microseconds,
    }
}

fn deserialize_time_base<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<TimeBase>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}

fn timestamp_discontinuities(format_name: &str) -> bool {
    // FFmpeg n8.1.2 libavformat demuxer flags, including the m4v macro-generated
    // declaration. These are actual demuxer names, never filename extensions.
    format_name.split(',').map(str::trim).any(|name| {
        matches!(
            name,
            "dhav"
                | "dvdvideo"
                | "live_flv"
                | "hls"
                | "m4v"
                | "mpeg"
                | "mpegts"
                | "mpegtsraw"
                | "ogg"
                | "rcwt"
                | "ty"
        )
    })
}

impl ResolvedSourceClock {
    /// Returns a whole-tick expression used identically before/after subtitles.
    /// Unknown metadata uses a symmetric best effort in the current filter TB.
    ///
    /// # Errors
    /// Returns an error when known timestamps overflow or the exact shift is
    /// outside the integer precision of the `FFmpeg` expression engine. This
    /// does not guarantee arbitrary input PTS outside its exact integer range.
    pub fn shift_expression(
        self,
        seek_microseconds: i64,
    ) -> Result<Option<String>, ConversionError> {
        self.shift_expression_for_filter(seek_microseconds, false)
    }

    pub(crate) fn shift_expression_for_filter(
        self,
        seek_microseconds: i64,
        doubles_time_base: bool,
    ) -> Result<Option<String>, ConversionError> {
        if seek_microseconds == 0 {
            return Ok(None);
        }
        let (video_time_base, origin, minimum) = match self {
            Self::Unknown => return Ok(Some(fallback_expression(seek_microseconds))),
            Self::Invalid => return Err(clock_overflow()),
            Self::Known {
                video_time_base,
                origin_microseconds,
                minimum_start_microseconds,
            } => (
                video_time_base,
                origin_microseconds,
                minimum_start_microseconds,
            ),
        };
        let sought = origin
            .checked_add(seek_microseconds)
            .filter(|value| *value != i64::MIN)
            .ok_or_else(clock_overflow)?;
        let adjusted = |time: i64| minimum.map_or(time, |floor| time.max(floor));
        let rescale = |time| {
            rescale_near_inf(
                time,
                i128::from(video_time_base.denominator),
                i128::from(video_time_base.numerator) * 1_000_000,
            )
            .ok_or_else(clock_overflow)
        };
        let ticks = rescale(adjusted(sought))?
            .checked_sub(rescale(adjusted(origin))?)
            .ok_or_else(clock_overflow)?;
        if ticks == 0 {
            return Ok(None);
        }
        // Leave one bit of headroom for setpts arithmetic: shifts near 2^53
        // can erase even small incoming PTS. bwdif is the only current
        // pre-subtitle filter that changes TB, so reserve another bit for it.
        // This conservative bound does not replace the expression's actual TB.
        let max_ticks = 1_u64 << if doubles_time_base { 51 } else { 52 };
        if ticks.unsigned_abs() > max_ticks {
            return Err(clock_overflow());
        }
        let numerator = i128::from(ticks) * i128::from(video_time_base.numerator);
        let denominator = i128::from(video_time_base.denominator);
        let divisor = gcd128(numerator.unsigned_abs(), denominator.unsigned_abs());
        let divisor = i128::try_from(divisor).map_err(|_| clock_overflow())?;
        let numerator = numerator / divisor;
        let denominator = denominator / divisor;
        if numerator.unsigned_abs() > (1_u128 << 53) {
            return Err(clock_overflow());
        }
        Ok(Some(format!("round({numerator}/({denominator}*TB))")))
    }
}

fn fallback_expression(microseconds: i64) -> String {
    let mut seconds = crate::trim::format_microseconds(microseconds);
    if let Some(decimal) = seconds.find('.') {
        while seconds.len() > decimal + 4 && seconds.ends_with('0') {
            seconds.pop();
        }
    }
    format!("round({seconds}/TB)")
}

fn clock_overflow() -> ConversionError {
    ConversionError::InvalidInput("Source timestamps are too large to align subtitles".into())
}

fn rescale_near_inf(value: i64, numerator: i128, denominator: i128) -> Option<i64> {
    if value == i64::MIN {
        return None;
    }
    let magnitude = i128::from(value.unsigned_abs()).checked_mul(numerator)?;
    let rounded = magnitude.checked_add(denominator / 2)? / denominator;
    i64::try_from(if value < 0 { -rounded } else { rounded })
        .ok()
        .filter(|value| *value != i64::MIN)
}

const fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left
}

const fn gcd128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left
}

#[cfg(test)]
#[path = "source_timing/tests.rs"]
mod tests;
