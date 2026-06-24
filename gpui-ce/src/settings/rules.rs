use frame_core::media_rules;

use super::model::{SourceKind, SourceMetadata};

#[must_use]
pub fn source_kind_for(metadata: Option<&SourceMetadata>) -> SourceKind {
    metadata.map_or(SourceKind::Video, SourceMetadata::source_kind)
}

#[must_use]
pub fn is_audio_only_container(container: &str) -> bool {
    media_rules::is_audio_only_container(container)
}

#[must_use]
pub fn is_video_only_container(container: &str) -> bool {
    media_rules::is_video_only_container(container)
}

#[must_use]
pub fn is_image_container(container: &str) -> bool {
    media_rules::is_image_container(container)
}

#[must_use]
pub fn is_gif_container(container: &str) -> bool {
    media_rules::is_gif_container(container)
}

#[must_use]
pub fn container_supports_audio(container: &str) -> bool {
    media_rules::container_supports_audio(container)
}

#[must_use]
pub fn container_supports_subtitles(container: &str) -> bool {
    media_rules::container_supports_subtitles(container)
}

#[must_use]
pub fn is_audio_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_audio_codec_allowed(container, codec)
}

#[must_use]
pub fn is_audio_stream_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_audio_stream_codec_allowed(container, codec)
}

#[must_use]
pub fn is_video_stream_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_video_stream_codec_allowed(container, codec)
}

#[must_use]
pub fn is_subtitle_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_subtitle_codec_allowed(container, codec)
}

#[must_use]
pub fn default_audio_codec_for_container(container: &str) -> &str {
    media_rules::default_audio_codec_for_container(container)
}
