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
    pub(in crate::app) overlay: PreviewOverlayRenderState,
    pub(in crate::app) media: Option<PreviewMediaRenderState>,
    pub(in crate::app) render_image: Option<Arc<RenderImage>>,
    pub(in crate::app) runtime_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewMediaRenderState {
    pub(in crate::app) width: u32,
    pub(in crate::app) height: u32,
}

impl PreviewMediaRenderState {
    pub(in crate::app) fn aspect_ratio(self) -> f32 {
        if self.height == 0 {
            return 1.0;
        }

        self.width as f32 / self.height as f32
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct PreviewOverlayRenderState {
    pub(in crate::app) overlay_mode: bool,
    pub(in crate::app) overlay: Option<PreviewOverlay>,
    pub(in crate::app) image_dimensions: Option<PreviewOverlayImageDimensions>,
}

#[cfg(test)]
impl PreviewOverlayRenderState {
    #[must_use]
    pub(in crate::app) const fn empty() -> Self {
        Self {
            overlay_mode: false,
            overlay: None,
            image_dimensions: None,
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::app) struct PreviewTimecodeInputFocuses<'a> {
    pub(in crate::app) start: Option<&'a FocusHandle>,
    pub(in crate::app) end: Option<&'a FocusHandle>,
}

pub(in crate::app) struct PreviewPanelProps<'a> {
    pub(in crate::app) crop: PreviewCropRenderState,
    pub(in crate::app) overlay: PreviewOverlayRenderState,
    pub(in crate::app) timecode_focuses: PreviewTimecodeInputFocuses<'a>,
    pub(in crate::app) playback: PreviewPlaybackState,
    pub(in crate::app) render_image: Option<Arc<RenderImage>>,
    pub(in crate::app) runtime_error: Option<String>,
}

pub(in crate::app) fn preview_panel(
    file_queue: &FileQueue,
    settings: SettingsRenderState<'_>,
    props: PreviewPanelProps<'_>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let state = preview_shell_state(
        file_queue.selected_file(),
        settings,
        props.crop,
        props.overlay,
        props.playback,
        props.render_image,
        props.runtime_error,
    );

    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .p(px(PREVIEW_PANEL_PADDING))
        .child(preview_viewport(&state, cx))
        .child(preview_timeline(&state, props.timecode_focuses, window, cx))
}

pub(in crate::app) fn preview_shell_state(
    selected_file: Option<&FileItem>,
    settings: SettingsRenderState<'_>,
    crop: PreviewCropRenderState,
    overlay: PreviewOverlayRenderState,
    playback: PreviewPlaybackState,
    render_image: Option<Arc<RenderImage>>,
    runtime_error: Option<String>,
) -> PreviewShellState {
    let metadata_status = preview_metadata_status(settings.metadata_status);
    let source_media_kind = preview_source_media_kind(settings.metadata);
    let media = preview_media_render_state(render_image.as_ref());
    let availability = preview_control_availability(PreviewControlInput {
        metadata_status,
        source_media_kind,
        controls_disabled: settings.settings_disabled,
        processing_mode: settings.config.processing_mode,
        container: Some(settings.config.container.as_str()),
    });
    let duration_seconds = preview_duration_seconds(settings.metadata);
    PreviewShellState {
        selected_file_name: selected_file.map(|file| file.name.clone()),
        metadata_status,
        metadata_error: settings.metadata_error.map(str::to_string),
        controls_disabled: settings.settings_disabled,
        availability,
        playback,
        duration_seconds,
        crop,
        overlay,
        media,
        render_image,
        runtime_error,
    }
}

pub(in crate::app) fn preview_media_render_state(
    render_image: Option<&Arc<RenderImage>>,
) -> Option<PreviewMediaRenderState> {
    let size = render_image?.size(0);
    let width = u32::try_from(size.width.0).ok()?;
    let height = u32::try_from(size.height.0).ok()?;
    if width == 0 || height == 0 {
        return None;
    }

    Some(PreviewMediaRenderState { width, height })
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
