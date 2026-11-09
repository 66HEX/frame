//! Audio stream constraints shared by the inspector and conversion preflight.

use crate::media_rules::is_audio_stream_codec_allowed;

#[must_use]
pub fn audio_track_limit(container: &str) -> Option<usize> {
    matches!(
        container.to_ascii_lowercase().as_str(),
        "mp3" | "wav" | "flac"
    )
    .then_some(1)
}

#[must_use]
pub fn audio_track_count_error(container: &str, count: usize) -> Option<String> {
    audio_track_limit(container).filter(|limit| count > *limit).map(|_| {
        format!("{} supports one audio track. Select one source track or choose another output format.", container.to_ascii_uppercase())
    })
}

#[must_use]
pub fn audio_stream_copy_error(
    container: &str,
    codec: &str,
    sample_rate: Option<&str>,
) -> Option<&'static str> {
    if !is_audio_stream_codec_allowed(container, codec) {
        Some("This audio codec cannot be copied to the selected format.")
    } else if mp3_requires_mp4_resampling(container, codec, sample_rate) {
        Some("MP4 needs MP3 audio at 16 kHz or higher. Choose Re-encode or another output format.")
    } else {
        None
    }
}

/// Actual demuxer and selected-track facts from the existing source probe.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioStreamCopySource<'a> {
    pub format: Option<&'a str>,
    pub channels: Option<u16>,
    pub codec_tag: Option<u32>,
}

/// Checks codec/rate support plus the known source-specific ADPCM copy failures.
///
/// `FFmpeg` does not expose audio block alignment in its probe JSON. These narrow
/// rules identify tested demuxer paths that lose it; unknown facts stay allowed.
/// A MOV `wfex` sample entry has a WAVE tag and must not be treated as `msSF`.
#[must_use]
pub fn audio_stream_copy_error_with_source(
    container: &str,
    codec: &str,
    sample_rate: Option<&str>,
    source: AudioStreamCopySource<'_>,
) -> Option<&'static str> {
    if let Some(reason) = audio_stream_copy_error(container, codec, sample_rate) {
        return Some(reason);
    }
    if !container.eq_ignore_ascii_case("mkv") {
        return None;
    }
    let has_format = |expected: &str| {
        source.format.is_some_and(|format| {
            format
                .split(',')
                .any(|name| name.trim().eq_ignore_ascii_case(expected))
        })
    };
    let missing_ms_alignment = codec.eq_ignore_ascii_case("adpcm_ms") && has_format("nut");
    let missing_swf_alignment = codec.eq_ignore_ascii_case("adpcm_swf")
        && source.channels == Some(1)
        && (has_format("nut")
            || has_format("flv")
            || (source.codec_tag == Some(0x4653_736d)
                && ["mov", "mp4", "m4a", "3gp", "3g2", "mj2"]
                    .into_iter()
                    .any(has_format)));
    (missing_ms_alignment || missing_swf_alignment)
        .then_some("This audio track cannot be copied to MKV. Choose Re-encode.")
}

#[must_use]
pub fn mp3_requires_mp4_resampling(
    container: &str,
    codec: &str,
    sample_rate: Option<&str>,
) -> bool {
    container.eq_ignore_ascii_case("mp4")
        && matches!(codec.to_ascii_lowercase().as_str(), "mp3" | "libmp3lame")
        && sample_rate
            .and_then(|rate| rate.parse::<u32>().ok())
            .is_some_and(|rate| rate < 16_000)
}

#[must_use]
pub fn effective_audio_channels(mode: &str, source_channels: Option<u16>) -> Option<u16> {
    match mode {
        "mono" => Some(1),
        "stereo" => Some(2),
        _ => source_channels,
    }
}

/// Checks the default libopus mapping family used by Frame.
///
/// With no layout label, the CLI guesses a layout before the encoder sees it.
/// Its defaults for three and four channels are 2.1 and 4.0, which libopus
/// rejects; the supported alternatives are 3.0 and quad.
#[must_use]
pub fn opus_channels_error(channels: Option<u16>, layout: Option<&str>) -> Option<&'static str> {
    let Some(channels) = channels.filter(|count| *count > 0) else {
        return Some("The source channel count is unknown. Choose Mono or Stereo for Opus.");
    };
    let expected = match channels {
        1 => "mono",
        2 => "stereo",
        3 => "3.0",
        4 => "quad",
        5 => "5.0",
        6 => "5.1",
        7 => "6.1",
        8 => "7.1",
        _ => return Some("Opus supports up to eight channels. Choose Mono or Stereo."),
    };
    let layout = layout.map(str::trim).filter(|layout| !layout.is_empty());
    if layout.is_some_and(|layout| !layout.eq_ignore_ascii_case(expected))
        || (layout.is_none() && matches!(channels, 3 | 4))
    {
        return Some(
            "Opus cannot preserve this channel layout. Choose Mono or Stereo, or another audio codec.",
        );
    }
    None
}

#[must_use]
pub fn opus_encoding_error(
    bitrate_kbps: &str,
    channels: Option<u16>,
    layout: Option<&str>,
) -> Option<String> {
    if let Some(reason) = opus_channels_error(channels, layout) {
        return Some(reason.to_string());
    }
    let maximum = 256 * u32::from(channels.unwrap_or(1));
    let valid = bitrate_kbps
        .parse::<f64>()
        .ok()
        .is_some_and(|bitrate| (0.5..=f64::from(maximum)).contains(&bitrate));
    (!valid).then(|| {
        format!("Opus bitrate must be between 0.5 and {maximum} kbps for this channel count.")
    })
}

#[must_use]
pub fn stereo_width_supports_channels(mode: &str, source_channels: Option<u16>) -> bool {
    effective_audio_channels(mode, source_channels) == Some(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp3_sample_rate_guard_applies_only_below_the_mp4_boundary() {
        for rate in ["8000", "11025", "12000"] {
            assert!(audio_stream_copy_error("mp4", "mp3", Some(rate)).is_some());
            assert_eq!(audio_stream_copy_error("mov", "mp3", Some(rate)), None);
            assert_eq!(audio_stream_copy_error("mkv", "mp3", Some(rate)), None);
        }
        for rate in ["16000", "22050", "44100", "48000"] {
            assert_eq!(audio_stream_copy_error("mp4", "mp3", Some(rate)), None);
        }
    }

    #[test]
    fn adpcm_copy_guard_uses_exact_source_facts_without_rejecting_working_paths() {
        let check = |container, codec, format, channels, codec_tag| {
            audio_stream_copy_error_with_source(
                container,
                codec,
                None,
                AudioStreamCopySource {
                    format,
                    channels,
                    codec_tag,
                },
            )
        };
        for format in ["nut", "NUT", "other,nut"] {
            for channels in [None, Some(1), Some(2)] {
                assert!(check("mkv", "adpcm_ms", Some(format), channels, None).is_some());
            }
        }
        for format in ["nut", "flv", "mov,mp4,m4a,3gp,3g2,mj2", " MOV "] {
            assert!(check("MKV", "ADPCM_SWF", Some(format), Some(1), Some(0x4653_736d)).is_some());
            assert!(check("mkv", "adpcm_swf", Some(format), Some(2), Some(0x4653_736d)).is_none());
            assert!(check("mkv", "adpcm_swf", Some(format), None, Some(0x4653_736d)).is_none());
        }
        for format in [
            None,
            Some("wav"),
            Some("avi"),
            Some("matroska,webm"),
            Some("notnut"),
            Some("movish"),
        ] {
            for codec in ["adpcm_ms", "adpcm_swf"] {
                assert!(check("mkv", codec, format, Some(1), Some(0x4653_736d)).is_none());
            }
        }
        for tag in [None, Some(0x5346), Some(0), Some(u32::MAX)] {
            assert!(check("mkv", "adpcm_swf", Some("mov"), Some(1), tag).is_none());
        }
        assert!(check("mov", "adpcm_swf", Some("flv"), Some(1), None).is_none());
        assert!(check("mkv", "adpcm_yamaha", Some("nut"), Some(1), None).is_none());
    }

    #[test]
    fn opus_limits_depend_on_every_effective_channel_count() {
        for channels in [1, 2, 5, 6, 7, 8] {
            let maximum = u32::from(channels) * 256;
            for bitrate in ["0.5".to_string(), maximum.to_string()] {
                assert_eq!(opus_encoding_error(&bitrate, Some(channels), None), None);
            }
            for bitrate in [
                "0.499".to_string(),
                (maximum + 1).to_string(),
                "NaN".to_string(),
                "inf".to_string(),
            ] {
                assert!(opus_encoding_error(&bitrate, Some(channels), None).is_some());
            }
        }
    }

    #[test]
    fn opus_rejects_incompatible_layouts_without_remapping_them() {
        for (channels, layout) in [
            (3, Some("2.1")),
            (4, Some("4.0")),
            (6, Some("5.1(side)")),
            (6, Some("hexagonal")),
            (3, None),
            (4, None),
            (9, None),
        ] {
            assert!(opus_channels_error(Some(channels), layout).is_some());
        }
        assert_eq!(opus_channels_error(Some(3), Some("3.0")), None);
        assert_eq!(opus_channels_error(Some(4), Some("quad")), None);
        assert!(opus_channels_error(None, None).is_some());
    }
}
