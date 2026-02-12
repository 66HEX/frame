use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

const ANY_CODEC_TOKEN: &str = "*";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaRulesRaw {
    audio_only_containers: Vec<String>,
    #[serde(default)]
    video_only_containers: Vec<String>,
    container_video_codec_compatibility: HashMap<String, Vec<String>>,
    container_audio_codec_compatibility: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
struct MediaRules {
    audio_only_containers: HashSet<String>,
    video_only_containers: HashSet<String>,
    container_video_codec_compatibility: HashMap<String, HashSet<String>>,
    container_audio_codec_compatibility: HashMap<String, HashSet<String>>,
}

impl From<MediaRulesRaw> for MediaRules {
    fn from(raw: MediaRulesRaw) -> Self {
        Self {
            audio_only_containers: raw
                .audio_only_containers
                .into_iter()
                .map(|container| container.to_ascii_lowercase())
                .collect(),
            video_only_containers: raw
                .video_only_containers
                .into_iter()
                .map(|container| container.to_ascii_lowercase())
                .collect(),
            container_video_codec_compatibility: raw
                .container_video_codec_compatibility
                .into_iter()
                .map(|(container, codecs)| {
                    (
                        container.to_ascii_lowercase(),
                        codecs.into_iter().collect::<HashSet<_>>(),
                    )
                })
                .collect(),
            container_audio_codec_compatibility: raw
                .container_audio_codec_compatibility
                .into_iter()
                .map(|(container, codecs)| {
                    (
                        container.to_ascii_lowercase(),
                        codecs.into_iter().collect::<HashSet<_>>(),
                    )
                })
                .collect(),
        }
    }
}

static MEDIA_RULES: Lazy<MediaRules> = Lazy::new(|| {
    let raw: MediaRulesRaw =
        serde_json::from_str(include_str!("../../../src/lib/shared/media-rules.json"))
            .expect("Shared media rules JSON is invalid");

    raw.into()
});

pub fn is_audio_only_container(container: &str) -> bool {
    MEDIA_RULES
        .audio_only_containers
        .contains(&container.to_ascii_lowercase())
}

pub fn is_video_codec_allowed(container: &str, codec: &str) -> bool {
    let container = container.to_ascii_lowercase();
    match MEDIA_RULES
        .container_video_codec_compatibility
        .get(&container)
    {
        Some(allowed) => allowed.contains(codec),
        None => true,
    }
}

pub fn is_video_only_container(container: &str) -> bool {
    MEDIA_RULES
        .video_only_containers
        .contains(&container.to_ascii_lowercase())
}

pub fn container_supports_audio(container: &str) -> bool {
    !is_video_only_container(container)
}

pub fn container_supports_subtitles(container: &str) -> bool {
    !is_audio_only_container(container) && !is_video_only_container(container)
}

pub fn is_audio_codec_allowed(container: &str, codec: &str) -> bool {
    let container = container.to_ascii_lowercase();
    match MEDIA_RULES
        .container_audio_codec_compatibility
        .get(&container)
    {
        Some(allowed) => allowed.contains(ANY_CODEC_TOKEN) || allowed.contains(codec),
        None => true,
    }
}
