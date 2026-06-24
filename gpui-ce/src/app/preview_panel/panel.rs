use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct PreviewCropRenderState {
    pub(in crate::app) crop_mode: bool,
    pub(in crate::app) draft_crop: Option<CropRect>,
    pub(in crate::app) applied_crop: Option<CropRect>,
    pub(in crate::app) crop_aspect: String,
    pub(in crate::app) has_crop_dimensions: bool,
    pub(in crate::app) rotation: String,
    pub(in crate::app) flip_horizontal: bool,
    pub(in crate::app) flip_vertical: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct PreviewShellState {
    pub(in crate::app) selected_file_name: Option<String>,
    pub(in crate::app) metadata_status: PreviewMetadataStatus,
    pub(in crate::app) metadata_error: Option<String>,
    pub(in crate::app) controls_disabled: bool,
    pub(in crate::app) availability: PreviewControlAvailability,
    pub(in crate::app) playback: PreviewPlaybackState,
    pub(in crate::app) duration_seconds: f64,
    pub(in crate::app) crop: PreviewCropRenderState,
}

pub(in crate::app) fn preview_panel(
    file_queue: &FileQueue,
    settings: SettingsRenderState<'_>,
    preview_crop: PreviewCropRenderState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let state = preview_shell_state(file_queue.selected_file(), settings, preview_crop);

    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .p(px(PREVIEW_PANEL_PADDING))
        .child(preview_viewport(&state, cx))
        .child(preview_timeline(&state, cx))
}

pub(in crate::app) fn preview_shell_state(
    selected_file: Option<&FileItem>,
    settings: SettingsRenderState<'_>,
    crop: PreviewCropRenderState,
) -> PreviewShellState {
    let metadata_status = preview_metadata_status(settings.metadata_status);
    let source_media_kind = preview_source_media_kind(settings.metadata);
    let availability = preview_control_availability(PreviewControlInput {
        metadata_status,
        source_media_kind,
        controls_disabled: settings.settings_disabled,
        processing_mode: settings.config.processing_mode,
        container: Some(settings.config.container.as_str()),
    });
    let duration_seconds = preview_duration_seconds(settings.metadata);
    let playback = preview_playback_state(
        availability.media_kind,
        duration_seconds,
        settings.config.start_time.as_deref(),
        settings.config.end_time.as_deref(),
    );

    PreviewShellState {
        selected_file_name: selected_file.map(|file| file.name.clone()),
        metadata_status,
        metadata_error: settings.metadata_error.map(str::to_string),
        controls_disabled: settings.settings_disabled,
        availability,
        playback,
        duration_seconds,
        crop,
    }
}

pub(in crate::app) fn preview_metadata_status(status: MetadataStatus) -> PreviewMetadataStatus {
    match status {
        MetadataStatus::Idle => PreviewMetadataStatus::Idle,
        MetadataStatus::Loading => PreviewMetadataStatus::Loading,
        MetadataStatus::Ready => PreviewMetadataStatus::Ready,
        MetadataStatus::Error => PreviewMetadataStatus::Error,
    }
}

pub(in crate::app) fn preview_source_media_kind(
    metadata: Option<&SourceMetadata>,
) -> Option<SourceMediaKind> {
    metadata.map(|metadata| match metadata.source_kind() {
        SourceKind::Video => SourceMediaKind::Video,
        SourceKind::Audio => SourceMediaKind::Audio,
        SourceKind::Image => SourceMediaKind::Image,
    })
}

pub(in crate::app) fn preview_duration_seconds(metadata: Option<&SourceMetadata>) -> f64 {
    let Some(raw) = metadata.and_then(|metadata| metadata.duration.as_deref()) else {
        return 0.0;
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return 0.0;
    }

    let duration = if raw.contains(':') {
        parse_time_to_seconds(raw)
    } else {
        raw.parse::<f64>().unwrap_or(0.0)
    };

    if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        0.0
    }
}

pub(in crate::app) fn preview_playback_state(
    media_kind: PreviewMediaKind,
    duration_seconds: f64,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> PreviewPlaybackState {
    let is_image = media_kind == PreviewMediaKind::Image;
    let mut playback = PreviewPlaybackState::new(is_image);
    if media_kind != PreviewMediaKind::Unknown && !is_image {
        playback.sync_media(MediaSnapshot {
            current_time: 0.0,
            duration: duration_seconds,
            paused: true,
        });
        playback.sync_initial_values(start_time, end_time);
    }
    playback
}
