//! Media compatibility rules shared by the conversion core and GPUI frontend state.
//!
//! The MOV and Matroska stream-copy lists in `media-rules.json` follow `FFmpeg`
//! n8.1.2: `libavformat/isom_tags.c`, `riff.c`, and `matroska.c`. MOV accepts its
//! native tags plus BMP/WAVE fallback tags; Matroska accepts native codec IDs
//! plus VFW, `QuickTime`, and ACM fallback tags. The lists exclude the codec-level
//! restrictions in `movenc.c` (VP8/VP9/AV1 and FLAC/Opus/TrueHD in MOV).
//! Blu-ray PCM is excluded: MOV may preserve its foreign tag and exit
//! successfully, but the resulting stream cannot be decoded.
//! ADPCM MS/IMA WAV are excluded from MOV copy: demuxed tracks lack the
//! `frame_size` required by `mov_write_packet`, even when block alignment is set.
//! WMA1/2 lose required block alignment in MOV, and 64-bit integer PCM is
//! misidentified as 16-bit there; their apparently successful muxes are unusable.
//! QTRLE loses its pixel depth and SMC its palette in Matroska `CodecPrivate`.
//! `TrueHD` copied into 188-byte MPEG-TS is misidentified on demux; the M2TS path
//! remains supported.
//! Codec eligibility does not replace validation of track-specific parameters.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::container::media_rules_key;

const ANY_CODEC_TOKEN: &str = "*";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaRulesRaw {
    all_containers: Vec<String>,
    audio_only_containers: Vec<String>,
    #[serde(default)]
    video_only_containers: Vec<String>,
    #[serde(default)]
    image_containers: Vec<String>,
    container_video_codec_compatibility: HashMap<String, Vec<String>>,
    #[serde(default)]
    container_encoder_pixel_format_compatibility: HashMap<String, HashMap<String, Vec<String>>>,
    #[serde(default)]
    container_video_stream_codec_compatibility: HashMap<String, Vec<String>>,
    container_audio_codec_compatibility: HashMap<String, Vec<String>>,
    #[serde(default)]
    container_audio_stream_codec_compatibility: HashMap<String, Vec<String>>,
    #[serde(default)]
    container_audio_stream_copy_tags: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    container_subtitle_codec_compatibility: HashMap<String, Vec<String>>,
    #[serde(default)]
    default_audio_codec: HashMap<String, String>,
    default_audio_codec_fallback: String,
    #[serde(default)]
    video_codec_fallback_order: Vec<String>,
}

#[derive(Debug)]
struct MediaRules {
    all_containers: Vec<String>,
    audio_only_containers: HashSet<String>,
    video_only_containers: HashSet<String>,
    image_containers: HashSet<String>,
    container_video_codec_order: HashMap<String, Vec<String>>,
    container_video_codec_compatibility: HashMap<String, HashSet<String>>,
    container_encoder_pixel_format_compatibility: HashMap<String, HashMap<String, HashSet<String>>>,
    container_video_stream_codec_compatibility: HashMap<String, HashSet<String>>,
    container_audio_codec_compatibility: HashMap<String, HashSet<String>>,
    container_audio_stream_codec_compatibility: HashMap<String, HashSet<String>>,
    container_audio_stream_copy_tags: HashMap<String, HashMap<String, String>>,
    container_subtitle_codec_compatibility: HashMap<String, HashSet<String>>,
    default_audio_codec: HashMap<String, String>,
    default_audio_codec_fallback: String,
    video_codec_fallback_order: Vec<String>,
}

impl From<MediaRulesRaw> for MediaRules {
    fn from(raw: MediaRulesRaw) -> Self {
        let container_video_codec_order =
            normalized_codec_vec_map(raw.container_video_codec_compatibility.clone());

        Self {
            all_containers: raw.all_containers,
            audio_only_containers: normalized_set(raw.audio_only_containers),
            video_only_containers: normalized_set(raw.video_only_containers),
            image_containers: normalized_set(raw.image_containers),
            container_video_codec_order,
            container_video_codec_compatibility: normalized_codec_map(
                raw.container_video_codec_compatibility,
            ),
            container_encoder_pixel_format_compatibility: normalized_nested_codec_map(
                raw.container_encoder_pixel_format_compatibility,
            ),
            container_video_stream_codec_compatibility: normalized_codec_map(
                raw.container_video_stream_codec_compatibility,
            ),
            container_audio_codec_compatibility: normalized_codec_map(
                raw.container_audio_codec_compatibility,
            ),
            container_audio_stream_codec_compatibility: normalized_codec_map(
                raw.container_audio_stream_codec_compatibility,
            ),
            container_audio_stream_copy_tags: raw.container_audio_stream_copy_tags,
            container_subtitle_codec_compatibility: normalized_codec_map(
                raw.container_subtitle_codec_compatibility,
            ),
            default_audio_codec: raw
                .default_audio_codec
                .into_iter()
                .map(|(container, codec)| (normalize(container), codec))
                .collect(),
            default_audio_codec_fallback: raw.default_audio_codec_fallback,
            video_codec_fallback_order: raw.video_codec_fallback_order,
        }
    }
}

static MEDIA_RULES: LazyLock<MediaRules> = LazyLock::new(|| {
    let raw: MediaRulesRaw = serde_json::from_str(include_str!("../media-rules.json"))
        .expect("Media rules JSON is invalid");

    raw.into()
});

#[must_use]
pub fn all_containers() -> &'static [String] {
    &MEDIA_RULES.all_containers
}

#[must_use]
pub fn audio_only_containers() -> &'static HashSet<String> {
    &MEDIA_RULES.audio_only_containers
}

#[must_use]
pub fn image_containers() -> &'static HashSet<String> {
    &MEDIA_RULES.image_containers
}

#[must_use]
pub fn video_codec_fallback_order() -> &'static [String] {
    &MEDIA_RULES.video_codec_fallback_order
}

#[must_use]
pub fn video_codecs_for_container(container: &str) -> Option<&'static [String]> {
    MEDIA_RULES
        .container_video_codec_order
        .get(&media_rules_key(container))
        .map(Vec::as_slice)
}

#[must_use]
pub fn is_audio_only_container(container: &str) -> bool {
    MEDIA_RULES
        .audio_only_containers
        .contains(&normalize(container))
}

#[must_use]
pub fn is_video_only_container(container: &str) -> bool {
    MEDIA_RULES
        .video_only_containers
        .contains(&normalize(container))
}

#[must_use]
pub fn is_image_container(container: &str) -> bool {
    MEDIA_RULES.image_containers.contains(&normalize(container))
}

#[must_use]
pub const fn is_gif_container(container: &str) -> bool {
    container.eq_ignore_ascii_case("gif")
}

#[must_use]
pub fn container_supports_audio(container: &str) -> bool {
    !is_video_only_container(container) && !is_image_container(container)
}

#[must_use]
pub fn container_supports_subtitles(container: &str) -> bool {
    !is_audio_only_container(container)
        && !is_video_only_container(container)
        && !is_image_container(container)
}

#[must_use]
pub fn is_video_codec_allowed(container: &str, codec: &str) -> bool {
    codec_allowed(
        container,
        codec,
        &MEDIA_RULES.container_video_codec_compatibility,
        false,
    )
}

#[must_use]
pub fn is_video_stream_codec_allowed(container: &str, codec: &str) -> bool {
    codec_allowed(
        container,
        codec,
        &MEDIA_RULES.container_video_stream_codec_compatibility,
        true,
    )
}

#[must_use]
pub fn is_video_pixel_format_allowed(container: &str, encoder: &str, pixel_format: &str) -> bool {
    let container = media_rules_key(container);
    let encoder = normalize(encoder);
    let pixel_format = normalize(pixel_format);
    if pixel_format == "auto" {
        return true;
    }

    let Some(container_rules) = MEDIA_RULES
        .container_encoder_pixel_format_compatibility
        .get(&container)
    else {
        return true;
    };

    let Some(allowed) = container_rules
        .get(&encoder)
        .or_else(|| container_rules.get(ANY_CODEC_TOKEN))
    else {
        return false;
    };

    allowed.contains(ANY_CODEC_TOKEN) || allowed.contains(&pixel_format)
}

#[must_use]
pub fn is_audio_codec_allowed(container: &str, codec: &str) -> bool {
    codec_allowed(
        container,
        codec,
        &MEDIA_RULES.container_audio_codec_compatibility,
        true,
    )
}

#[must_use]
pub fn is_audio_stream_codec_allowed(container: &str, codec: &str) -> bool {
    codec_allowed(
        container,
        codec,
        &MEDIA_RULES.container_audio_stream_codec_compatibility,
        true,
    )
}

/// Returns a canonical tag to replace an incompatible input tag during copy.
///
/// These overrides are limited to verified, decodable copy combinations;
/// PCM continues to use the muxer's bit-depth and byte-order-aware selection.
#[must_use]
pub fn audio_stream_copy_tag(container: &str, codec: &str) -> Option<&'static str> {
    MEDIA_RULES
        .container_audio_stream_copy_tags
        .get(&media_rules_key(container))
        .and_then(|tags| tags.get(&normalize(codec)))
        .map(String::as_str)
}

#[must_use]
pub fn is_subtitle_codec_allowed(container: &str, codec: &str) -> bool {
    codec_allowed(
        container,
        codec,
        &MEDIA_RULES.container_subtitle_codec_compatibility,
        true,
    )
}

/// How a selected subtitle stream can be represented in the output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubtitleStreamAction {
    Copy,
    Encode(&'static str),
}

/// Returns the same subtitle decision used by the UI and argument builder.
///
/// Text conversion is limited to codecs with decoders in the supported `FFmpeg`
/// runtime. TTML and HDMV text may be copied where supported, but not decoded.
#[must_use]
pub fn subtitle_stream_action(container: &str, codec: &str) -> Option<SubtitleStreamAction> {
    if is_subtitle_codec_allowed(container, codec) {
        return Some(SubtitleStreamAction::Copy);
    }
    let codec = normalize(codec);
    let container = media_rules_key(container);
    if is_decodable_text_subtitle_codec(&codec) {
        let encoder = match container.as_str() {
            "mp4" | "mov" => "mov_text",
            "mkv" => "srt",
            "webm" => "webvtt",
            _ => return None,
        };
        return Some(SubtitleStreamAction::Encode(encoder));
    }
    if container == "mpegts" && matches!(codec.as_str(), "hdmv_pgs_subtitle" | "dvd_subtitle") {
        return Some(SubtitleStreamAction::Encode("dvbsub"));
    }
    None
}

/// Returns whether a subtitle stream can be copied or standards-compliantly
/// transcoded for the selected output profile.
#[must_use]
pub fn is_subtitle_stream_supported(container: &str, codec: &str) -> bool {
    subtitle_stream_action(container, codec).is_some()
}

fn is_decodable_text_subtitle_codec(codec: &str) -> bool {
    matches!(
        codec,
        "text"
            | "ssa"
            | "mov_text"
            | "srt"
            | "microdvd"
            | "eia_608"
            | "jacosub"
            | "sami"
            | "realtext"
            | "stl"
            | "subviewer1"
            | "subviewer"
            | "subrip"
            | "webvtt"
            | "mpl2"
            | "vplayer"
            | "pjs"
            | "ass"
    )
}

#[must_use]
pub fn default_audio_codec_for_container(container: &str) -> &str {
    MEDIA_RULES
        .default_audio_codec
        .get(&media_rules_key(container))
        .map_or(
            MEDIA_RULES.default_audio_codec_fallback.as_str(),
            String::as_str,
        )
}

fn codec_allowed(
    container: &str,
    codec: &str,
    rules: &HashMap<String, HashSet<String>>,
    wildcard_allowed: bool,
) -> bool {
    let container = media_rules_key(container);
    let codec = normalize(codec);
    rules.get(&container).is_some_and(|allowed| {
        (wildcard_allowed && allowed.contains(ANY_CODEC_TOKEN)) || allowed.contains(&codec)
    })
}

fn normalized_set(values: Vec<String>) -> HashSet<String> {
    values.into_iter().map(normalize).collect()
}

fn normalized_codec_map(source: HashMap<String, Vec<String>>) -> HashMap<String, HashSet<String>> {
    source
        .into_iter()
        .map(|(container, codecs)| (normalize(container), normalized_set(codecs)))
        .collect()
}

fn normalized_codec_vec_map(source: HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    source
        .into_iter()
        .map(|(container, codecs)| {
            (
                normalize(container),
                codecs.into_iter().map(normalize).collect(),
            )
        })
        .collect()
}

fn normalized_nested_codec_map(
    source: HashMap<String, HashMap<String, Vec<String>>>,
) -> HashMap<String, HashMap<String, HashSet<String>>> {
    source
        .into_iter()
        .map(|(container, codec_map)| {
            (
                normalize(container),
                codec_map
                    .into_iter()
                    .map(|(codec, values)| (normalize(codec), normalized_set(values)))
                    .collect(),
            )
        })
        .collect()
}

fn normalize(value: impl AsRef<str>) -> String {
    value.as_ref().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_containers_preserve_shared_json_order() {
        assert_eq!(
            all_containers(),
            &[
                "mp4".to_string(),
                "mkv".to_string(),
                "webm".to_string(),
                "mov".to_string(),
                "m2t".to_string(),
                "mts".to_string(),
                "m2ts".to_string(),
                "gif".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "webp".to_string(),
                "bmp".to_string(),
                "tiff".to_string(),
                "mp3".to_string(),
                "m4a".to_string(),
                "wav".to_string(),
                "flac".to_string(),
                "opus".to_string(),
            ]
        );
    }

    #[test]
    fn transport_stream_profiles_have_strict_complete_rules() {
        assert!(is_video_codec_allowed("m2t", "mpeg2video"));
        assert!(is_audio_codec_allowed("m2t", "mp2"));
        assert!(is_subtitle_codec_allowed("m2t", "dvb_subtitle"));
        assert!(is_subtitle_stream_supported("m2t", "hdmv_pgs_subtitle"));
        assert!(is_video_stream_codec_allowed("m2ts", "h264"));
        assert!(is_audio_stream_codec_allowed("mts", "ac3"));
        assert!(is_subtitle_codec_allowed("m2ts", "hdmv_pgs_subtitle"));
        assert!(!is_subtitle_codec_allowed("m2ts", "subrip"));
        assert!(!is_video_codec_allowed("unknown", "libx264"));
    }

    #[test]
    fn mp4_accepts_text_subtitle_transcode_but_rejects_pgs() {
        assert!(is_subtitle_stream_supported("mp4", "subrip"));
        assert!(!is_subtitle_stream_supported("mp4", "hdmv_pgs_subtitle"));
    }

    #[test]
    fn mp4_supports_audio_and_subtitles_like_shared_rules() {
        assert!(container_supports_audio("mp4"));
        assert!(container_supports_subtitles("mp4"));
    }

    #[test]
    fn image_containers_do_not_support_audio_or_subtitles() {
        assert!(!container_supports_audio("png"));
        assert!(!container_supports_subtitles("png"));
    }

    #[test]
    fn video_codecs_for_container_preserves_shared_json_order() {
        assert_eq!(
            video_codecs_for_container("png"),
            Some(&["png".to_string()][..])
        );
    }

    #[test]
    fn mp4_rejects_flac_reencode_audio_like_shared_rules() {
        assert!(!is_audio_codec_allowed("mp4", "flac"));
    }

    #[test]
    fn mov_and_mkv_distinguish_audio_encoding_from_copy() {
        for codec in ["libopus", "flac", "pcm_bluray"] {
            assert!(!is_audio_codec_allowed("mov", codec));
        }
        assert!(is_audio_codec_allowed("mkv", "flac"));
        assert!(!is_audio_codec_allowed("mkv", "pcm_bluray"));
        assert!(!is_audio_stream_codec_allowed("mov", "pcm_bluray"));
        assert!(!is_audio_stream_codec_allowed("mkv", "pcm_bluray"));
        assert!(!is_audio_stream_codec_allowed("mov", "truehd"));
        assert!(!is_audio_stream_codec_allowed("mkv", "pcm_s8"));
        for codec in ["wmav1", "wmav2", "pcm_s64le", "dfpwm"] {
            assert!(!is_audio_stream_codec_allowed("mov", codec));
            assert!(is_audio_stream_codec_allowed("mkv", codec));
        }
    }

    #[test]
    fn mov_copy_canonical_tags_do_not_override_pcm_selection() {
        for (codec, tag) in [
            ("aac_latm", "0x0216736d"),
            ("adpcm_g726", "0x4500736d"),
            ("g723_1", "0x4200736d"),
            ("vorbis", "0x6f56736d"),
        ] {
            assert_eq!(audio_stream_copy_tag("mov", codec), Some(tag));
            assert_eq!(audio_stream_copy_tag("mkv", codec), None);
        }
        assert_eq!(audio_stream_copy_tag("mov", "pcm_s24le"), None);
    }

    #[test]
    fn stream_copy_lists_reject_unsupported_muxer_pairs() {
        for codec in ["vp8", "vp9", "av1", "unknown"] {
            assert!(!is_video_stream_codec_allowed("mov", codec));
        }
        assert!(is_video_stream_codec_allowed("mkv", "rawvideo"));
        for codec in ["mp3", "eac3"] {
            assert!(!is_audio_stream_codec_allowed("m4a", codec));
        }
        assert!(!is_audio_stream_codec_allowed("wav", "pcm_s8"));
        assert!(is_audio_stream_codec_allowed("wav", "pcm_u8"));
        for codec in ["qtrle", "smc"] {
            assert!(!is_video_stream_codec_allowed("mkv", codec));
            assert!(is_video_stream_codec_allowed("mov", codec));
        }
        assert!(!is_audio_stream_codec_allowed("m2t", "truehd"));
        assert!(is_audio_stream_codec_allowed("m2ts", "truehd"));
    }

    #[test]
    fn stream_copy_lists_preserve_muxer_fallback_codecs() {
        for container in ["mov", "mkv"] {
            for codec in ["h261", "h263", "flv1", "msmpeg4v2", "msmpeg4v3", "zlib"] {
                assert!(
                    is_video_stream_codec_allowed(container, codec),
                    "{container}/{codec}"
                );
            }
        }
        assert!(is_audio_stream_codec_allowed("mov", "adpcm_ima_qt"));
        assert!(!is_audio_stream_codec_allowed("mkv", "adpcm_ima_qt"));
        for codec in ["adpcm_ima_wav", "adpcm_ms"] {
            assert!(is_audio_stream_codec_allowed("mkv", codec));
            assert!(!is_audio_stream_codec_allowed("mov", codec));
        }
    }

    #[test]
    fn subtitle_actions_distinguish_copy_transcode_and_missing_decoder() {
        for (container, codec, expected) in [
            ("mkv", "mov_text", Some(SubtitleStreamAction::Encode("srt"))),
            (
                "mov",
                "subrip",
                Some(SubtitleStreamAction::Encode("mov_text")),
            ),
            ("mov", "ass", Some(SubtitleStreamAction::Encode("mov_text"))),
            ("mkv", "ass", Some(SubtitleStreamAction::Copy)),
            ("mp4", "ttml", Some(SubtitleStreamAction::Copy)),
            ("mov", "ttml", Some(SubtitleStreamAction::Copy)),
            ("webm", "ttml", None),
            ("mkv", "ttml", None),
            ("mp4", "hdmv_text_subtitle", None),
            (
                "mkv",
                "hdmv_text_subtitle",
                Some(SubtitleStreamAction::Copy),
            ),
            ("mkv", "arib_caption", Some(SubtitleStreamAction::Copy)),
            (
                "m2ts",
                "hdmv_text_subtitle",
                Some(SubtitleStreamAction::Copy),
            ),
        ] {
            assert_eq!(
                subtitle_stream_action(container, codec),
                expected,
                "{container}/{codec}"
            );
            assert_eq!(
                is_subtitle_stream_supported(container, codec),
                expected.is_some()
            );
        }
    }

    #[test]
    fn opus_container_only_accepts_opus_audio() {
        assert!(is_audio_only_container("OPUS"));
        assert!(container_supports_audio("opus"));
        assert!(!container_supports_subtitles("opus"));
        assert_eq!(default_audio_codec_for_container("opus"), "libopus");
        assert!(is_audio_codec_allowed("opus", "libopus"));
        assert!(is_audio_stream_codec_allowed("opus", "opus"));
        for codec in ["flac", "aac", "vorbis"] {
            assert!(!is_audio_codec_allowed("opus", codec));
            assert!(!is_audio_stream_codec_allowed("opus", codec));
        }
        assert!(!all_containers().iter().any(|container| container == "ogg"));
    }

    #[test]
    fn webm_default_audio_codec_matches_shared_rules() {
        assert_eq!(default_audio_codec_for_container("webm"), "libopus");
    }

    #[test]
    fn av1_nvenc_pixel_format_rules_are_loaded_from_shared_json() {
        assert!(is_video_pixel_format_allowed(
            "mp4",
            "av1_nvenc",
            "yuv420p10le"
        ));
        assert!(!is_video_pixel_format_allowed(
            "mp4",
            "av1_nvenc",
            "yuv444p"
        ));
    }
}
