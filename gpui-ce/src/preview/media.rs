use crate::settings::ProcessingMode;
use frame_core::media_rules;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MetadataStatus {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceMediaKind {
    Video,
    Audio,
    Image,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PreviewMediaKind {
    #[default]
    Unknown,
    Video,
    Audio,
    Image,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewControlInput<'a> {
    pub metadata_status: MetadataStatus,
    pub source_media_kind: Option<SourceMediaKind>,
    pub controls_disabled: bool,
    pub processing_mode: ProcessingMode,
    pub container: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewControlAvailability {
    pub media_kind: PreviewMediaKind,
    pub hide_visual_controls: bool,
    pub trim_disabled: bool,
    pub overlay_available: bool,
}

#[must_use]
pub fn preview_media_kind(
    metadata_status: MetadataStatus,
    source_media_kind: Option<SourceMediaKind>,
) -> PreviewMediaKind {
    if metadata_status != MetadataStatus::Ready {
        return PreviewMediaKind::Unknown;
    }

    match source_media_kind {
        Some(SourceMediaKind::Video) => PreviewMediaKind::Video,
        Some(SourceMediaKind::Audio) => PreviewMediaKind::Audio,
        Some(SourceMediaKind::Image) => PreviewMediaKind::Image,
        None => PreviewMediaKind::Unknown,
    }
}

#[must_use]
pub fn preview_control_availability(input: PreviewControlInput<'_>) -> PreviewControlAvailability {
    let media_kind = preview_media_kind(input.metadata_status, input.source_media_kind);
    let container = input.container.unwrap_or_default();
    let is_audio_only_output = media_rules::is_audio_only_container(container);
    let is_image = media_kind == PreviewMediaKind::Image;

    PreviewControlAvailability {
        media_kind,
        hide_visual_controls: media_kind == PreviewMediaKind::Audio
            || (media_kind == PreviewMediaKind::Video && is_audio_only_output),
        trim_disabled: input.controls_disabled
            || is_image
            || media_kind == PreviewMediaKind::Unknown,
        overlay_available: media_kind == PreviewMediaKind::Video
            && !is_audio_only_output
            && input.processing_mode != ProcessingMode::Copy
            && !media_rules::is_gif_container(container),
    }
}
