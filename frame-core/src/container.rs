//! Typed output-container resolution shared by core and the frontend.

/// Packet layout selected for an MPEG transport-stream output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportStreamProfile {
    /// ISO/IEC 13818-1 transport stream with 188-byte packets.
    MpegTs188,
    /// Blu-ray/AVCHD transport stream with a four-byte timestamp prefix.
    M2ts192,
}

impl TransportStreamProfile {
    #[must_use]
    pub const fn packet_size(self) -> u16 {
        match self {
            Self::MpegTs188 => 188,
            Self::M2ts192 => 192,
        }
    }

    #[must_use]
    pub const fn ffmpeg_m2ts_mode(self) -> &'static str {
        match self {
            Self::MpegTs188 => "0",
            Self::M2ts192 => "1",
        }
    }

    #[must_use]
    pub const fn rule_key(self) -> &'static str {
        match self {
            Self::MpegTs188 => "mpegts",
            Self::M2ts192 => "m2ts",
        }
    }
}

/// Resolves a public output id without changing the value persisted in presets.
#[must_use]
pub const fn transport_stream_profile(container: &str) -> Option<TransportStreamProfile> {
    if container.eq_ignore_ascii_case("m2t") {
        Some(TransportStreamProfile::MpegTs188)
    } else if container.eq_ignore_ascii_case("mts") || container.eq_ignore_ascii_case("m2ts") {
        Some(TransportStreamProfile::M2ts192)
    } else {
        None
    }
}

#[must_use]
pub const fn is_transport_stream_container(container: &str) -> bool {
    transport_stream_profile(container).is_some()
}

/// Returns the canonical media-rules key for a public container id.
#[must_use]
pub fn media_rules_key(container: &str) -> String {
    transport_stream_profile(container).map_or_else(
        || container.trim().to_ascii_lowercase(),
        |profile| profile.rule_key().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_all_public_transport_stream_ids_case_insensitively() {
        for (container, profile, packet_size) in [
            ("m2t", TransportStreamProfile::MpegTs188, 188),
            ("MTS", TransportStreamProfile::M2ts192, 192),
            ("m2Ts", TransportStreamProfile::M2ts192, 192),
        ] {
            let resolved = transport_stream_profile(container).unwrap();
            assert_eq!(resolved, profile);
            assert_eq!(resolved.packet_size(), packet_size);
        }
    }

    #[test]
    fn leaves_existing_container_rule_keys_unchanged() {
        assert_eq!(media_rules_key("MP4"), "mp4");
        assert_eq!(media_rules_key("mts"), "m2ts");
        assert_eq!(media_rules_key("m2t"), "mpegts");
    }
}
