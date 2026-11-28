use crate::media_rules::is_audio_codec_allowed;

const COMMON: &[u32] = &[
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000, 64_000, 88_200, 96_000,
    176_400, 192_000, 352_800, 384_000,
];
const AAC: &[u32] = &[
    7350, 8000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000, 64_000, 88_200,
    96_000,
];
const MP3: &[u32] = &[
    8000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000,
];

/// Common output rates for the codec and muxer. Custom-rate codecs can also
/// store values between these menu choices.
#[must_use]
pub fn audio_sample_rates(container: &str, codec: &str) -> &'static [u32] {
    if !is_audio_codec_allowed(container, codec) {
        return &[];
    }
    match codec {
        "aac" => AAC,
        "libfdk_aac" => &AAC[1..],
        "mp3" if container.eq_ignore_ascii_case("mp4") => &MP3[3..],
        "mp3" => MP3,
        "mp2" => &[16_000, 22_050, 24_000, 32_000, 44_100, 48_000],
        "ac3" => &[32_000, 44_100, 48_000],
        "pcm_bluray" => &[48_000, 96_000, 192_000],
        // Opus stream headers always describe a 48 kHz playback clock.
        "libopus" => &[48_000],
        "vorbis" => &COMMON[..10],
        "flac" | "alac" | "pcm_s16le" => COMMON,
        _ => &[],
    }
}

#[must_use]
pub fn audio_sample_rate_custom(container: &str, codec: &str) -> bool {
    is_audio_codec_allowed(container, codec)
        && matches!(codec, "flac" | "alac" | "pcm_s16le" | "vorbis")
}

#[must_use]
pub fn audio_sample_rate_error(container: &str, codec: &str, rate: u32) -> Option<String> {
    let valid = if audio_sample_rate_custom(container, codec) {
        match codec {
            // The FLAC STREAMINFO sample rate occupies 20 bits.
            "flac" => (1..=1_048_575).contains(&rate),
            // Frame uses Vorbis's managed-bitrate setup, including the 5.1
            // setup above 50 kHz. Channel/bitrate constraints are checked below.
            "vorbis" => (8_000..=70_000).contains(&rate),
            // swresample/resample.c reduces the conversion ratio with an
            // INT32_MAX/2 bound on its numerator and denominator.
            _ => (1..=i32::MAX.unsigned_abs() / 2).contains(&rate),
        }
    } else {
        audio_sample_rates(container, codec).contains(&rate)
    };
    (!valid).then(|| {
        format!(
            "{codec} in {} cannot export audio at {rate} Hz.",
            container.to_ascii_uppercase()
        )
    })
}

#[must_use]
pub fn audio_sample_rate_encoding_error(
    container: &str,
    codec: &str,
    rate: u32,
    bitrate_kbps: &str,
    channels: Option<u16>,
) -> Option<String> {
    if let Some(reason) = audio_sample_rate_error(container, codec, rate) {
        return Some(reason);
    }
    let count = u32::from(channels.unwrap_or(1).max(1));
    // Both mux fields are written from signed-int products in libav:
    // riffenc.c ff_put_wav_header (PCM bytes/sec), and alacenc.c
    // alac_encode_init (bit rate in the ALAC magic cookie). ALAC can
    // negotiate 24-bit samples, depending on the source/filter format.
    let units_per_sample = match codec {
        "pcm_s16le" if container.eq_ignore_ascii_case("wav") => 2,
        "alac" => 24,
        _ => 0,
    };
    if units_per_sample > 0 && rate > i32::MAX.unsigned_abs() / (count * units_per_sample) {
        return Some(format!(
            "{codec} at {rate} Hz exceeds the {} stream header limit for {count} channels.",
            container.to_ascii_uppercase()
        ));
    }
    let bitrate = bitrate_kbps.parse::<f64>().ok()? * 1000.;
    if codec == "mp2" && rate < 32_000 && bitrate > 160_000. {
        return Some(format!("MP2 at {rate} Hz supports at most 160 kb/s."));
    }
    if codec != "vorbis" {
        return None;
    }
    let channels = channels.filter(|value| *value > 0)?;
    // Xiph libvorbis lib/modes/setup_{8,11,16,22,32,44,44u,44p51}.h.
    // get_setup_template accepts any matching setup, including an uncoupled
    // alternative and either adjacent setup at their shared boundary.
    let supported = VORBIS_SETUPS
        .iter()
        .any(|&(min_hz, max_hz, layout, low, high)| {
            let per_channel = bitrate / f64::from(channels);
            (min_hz..=max_hz).contains(&rate)
                && (layout == 0 || layout == channels)
                && (f64::from(low)..=f64::from(high)).contains(&per_channel)
        });
    (!supported).then(|| format!("Vorbis cannot encode {channels} channels at {rate} Hz and {bitrate_kbps} kb/s. Choose another rate or bitrate."))
}

// Hz interval, channel count (zero = uncoupled), bitrate range per channel.
const VORBIS_SETUPS: &[(u32, u32, u16, u32, u32)] = &[
    (8000, 9000, 2, 6000, 32_000),
    (8000, 9000, 0, 8000, 42_000),
    (9000, 15_000, 2, 8000, 44_000),
    (9000, 15_000, 0, 12_000, 50_000),
    (15_000, 19_000, 2, 12_000, 86_000),
    (15_000, 19_000, 0, 16_000, 100_000),
    (19_000, 26_000, 2, 15_000, 86_000),
    (19_000, 26_000, 0, 16_000, 90_000),
    (26_000, 40_000, 2, 18_000, 190_000),
    (26_000, 40_000, 0, 30_000, 190_000),
    (40_000, 50_000, 2, 22_500, 250_001),
    (40_000, 50_000, 0, 32_000, 240_001),
    (40_000, 70_000, 6, 14_000, 240_001),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus_and_custom_input_follow_encoded_clock_and_header_limits() {
        for (container, codec, allowed, forbidden) in [
            ("mp4", "mp3", 16_000, 8000),
            ("mp3", "mp3", 8000, 7000),
            ("webm", "libopus", 48_000, 24_000),
            ("m2ts", "pcm_bluray", 192_000, 44_100),
            ("m4a", "aac", 7350, 7500),
            ("flac", "flac", 1_048_575, 1_048_576),
            ("wav", "pcm_s16le", 12_345, u32::MAX),
            ("m4a", "alac", 12_345, u32::MAX),
            ("webm", "vorbis", 12_345, 96_000),
        ] {
            assert!(
                audio_sample_rate_error(container, codec, allowed).is_none(),
                "{container}/{codec}/{allowed}"
            );
            assert!(
                audio_sample_rate_error(container, codec, forbidden).is_some(),
                "{container}/{codec}/{forbidden}"
            );
        }
        assert!(audio_sample_rates("wav", "aac").is_empty());
        assert!(
            audio_sample_rate_encoding_error("wav", "pcm_s16le", 600_000_000, "128", Some(2))
                .is_some()
        );
        assert!(
            audio_sample_rate_encoding_error("m4a", "alac", 50_000_000, "128", Some(2)).is_some()
        );
    }

    #[test]
    fn vorbis_and_mp2_sample_rate_requires_compatible_bitrate_and_channels() {
        for (hz, ch, ok) in [
            (8000, 2, false),
            (16_000, 1, false),
            (16_000, 2, true),
            (24_000, 1, false),
            (32_000, 1, true),
            (44_100, 2, true),
            (64_000, 2, false),
            (64_000, 6, true),
        ] {
            assert_eq!(
                audio_sample_rate_encoding_error("webm", "vorbis", hz, "128", Some(ch)).is_none(),
                ok,
                "{hz}Hz/{ch}ch"
            );
        }
        assert!(audio_sample_rate_encoding_error("m2t", "mp2", 24_000, "160", Some(2)).is_none());
        assert!(audio_sample_rate_encoding_error("m2t", "mp2", 24_000, "192", Some(2)).is_some());
    }
}
