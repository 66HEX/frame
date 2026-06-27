//! Media compatibility rules shared by the conversion core and GPUI frontend state.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

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
    container_video_codec_compatibility: HashMap<String, HashSet<String>>,
    container_encoder_pixel_format_compatibility: HashMap<String, HashMap<String, HashSet<String>>>,
    container_video_stream_codec_compatibility: HashMap<String, HashSet<String>>,
    container_audio_codec_compatibility: HashMap<String, HashSet<String>>,
    container_audio_stream_codec_compatibility: HashMap<String, HashSet<String>>,
    container_subtitle_codec_compatibility: HashMap<String, HashSet<String>>,
    default_audio_codec: HashMap<String, String>,
    default_audio_codec_fallback: String,
    video_codec_fallback_order: Vec<String>,
}

impl From<MediaRulesRaw> for MediaRules {
    fn from(raw: MediaRulesRaw) -> Self {
        Self {
            all_containers: raw.all_containers,
            audio_only_containers: normalized_set(raw.audio_only_containers),
            video_only_containers: normalized_set(raw.video_only_containers),
            image_containers: normalized_set(raw.image_containers),
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
pub fn is_gif_container(container: &str) -> bool {
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
    let container = normalize(container);
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

#[must_use]
pub fn is_subtitle_codec_allowed(container: &str, codec: &str) -> bool {
    codec_allowed(
        container,
        codec,
        &MEDIA_RULES.container_subtitle_codec_compatibility,
        true,
    )
}

#[must_use]
pub fn default_audio_codec_for_container(container: &str) -> &str {
    MEDIA_RULES
        .default_audio_codec
        .get(&normalize(container))
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
    let container = normalize(container);
    let codec = normalize(codec);
    rules.get(&container).is_none_or(|allowed| {
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
            ]
        );
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
    fn mp4_rejects_flac_reencode_audio_like_shared_rules() {
        assert!(!is_audio_codec_allowed("mp4", "flac"));
    }

    #[test]
    fn mov_accepts_any_audio_codec_like_shared_rules() {
        assert!(is_audio_codec_allowed("mov", "flac"));
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
