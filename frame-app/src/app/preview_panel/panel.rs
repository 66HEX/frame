use super::*;
use crate::numeric::{f64_to_u32, u32_to_f32};

#[expect(
    clippy::struct_excessive_bools,
    reason = "Preview crop render state mirrors independent toggle controls from the toolbar."
)]
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
    pub(in crate::app) canvas: PreviewCanvasRenderState,
    pub(in crate::app) crop: PreviewCropRenderState,
    pub(in crate::app) overlay: PreviewOverlayRenderState,
    pub(in crate::app) presentation: PreviewRenderPresentation,
    pub(in crate::app) media: Option<PreviewMediaRenderState>,
    pub(in crate::app) render_image: Option<Arc<RenderImage>>,
    pub(in crate::app) runtime_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewCanvasRenderState {
    pub(in crate::app) zoom: f64,
    pub(in crate::app) pan_x: f64,
    pub(in crate::app) pan_y: f64,
    pub(in crate::app) viewport_width: f64,
    pub(in crate::app) viewport_height: f64,
}

impl Default for PreviewCanvasRenderState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            viewport_width: 0.0,
            viewport_height: 0.0,
        }
    }
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

        u32_to_f32(self.width) / u32_to_f32(self.height)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewPresentedFrame {
    pub(in crate::app) full_width: u32,
    pub(in crate::app) full_height: u32,
    pub(in crate::app) visible_x: u32,
    pub(in crate::app) visible_y: u32,
    pub(in crate::app) visible_width: u32,
    pub(in crate::app) visible_height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct PreviewOverlayRenderState {
    pub(in crate::app) overlay_mode: bool,
    pub(in crate::app) has_overlay: bool,
    pub(in crate::app) overlay: Option<PreviewOverlay>,
    pub(in crate::app) image_dimensions: Option<PreviewOverlayImageDimensions>,
}

#[cfg(test)]
impl PreviewOverlayRenderState {
    #[must_use]
    pub(in crate::app) const fn empty() -> Self {
        Self {
            overlay_mode: false,
            has_overlay: false,
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
    pub(in crate::app) canvas: PreviewCanvasRenderState,
    pub(in crate::app) crop: PreviewCropRenderState,
    pub(in crate::app) overlay: PreviewOverlayRenderState,
    pub(in crate::app) viewport_focuses: PreviewViewportFocuses<'a>,
    pub(in crate::app) timecode_focuses: PreviewTimecodeInputFocuses<'a>,
    pub(in crate::app) playback: PreviewPlaybackState,
    pub(in crate::app) presentation: PreviewRenderPresentation,
    pub(in crate::app) render_image: Option<Arc<RenderImage>>,
    pub(in crate::app) runtime_error: Option<String>,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct PreviewViewportFocuses<'a> {
    pub(in crate::app) viewport: &'a FocusHandle,
    pub(in crate::app) tools: PreviewToolFocuses<'a>,
    pub(in crate::app) edit_toolbars: PreviewEditToolbarFocuses<'a>,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct PreviewToolFocuses<'a> {
    pub(in crate::app) crop: &'a FocusHandle,
    pub(in crate::app) overlay: &'a FocusHandle,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct PreviewEditToolbarFocus<'a> {
    pub(in crate::app) panel: &'a FocusHandle,
    pub(in crate::app) first: &'a FocusHandle,
    pub(in crate::app) last: &'a FocusHandle,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct PreviewEditToolbarFocuses<'a> {
    pub(in crate::app) crop: PreviewEditToolbarFocus<'a>,
    pub(in crate::app) overlay: PreviewEditToolbarFocus<'a>,
}

pub(in crate::app) struct PreviewShellStateInput<'a> {
    pub(in crate::app) selected_file: Option<&'a FileItem>,
    pub(in crate::app) settings: &'a SettingsRenderState<'a>,
    pub(in crate::app) crop: PreviewCropRenderState,
    pub(in crate::app) overlay: PreviewOverlayRenderState,
    pub(in crate::app) canvas: PreviewCanvasRenderState,
    pub(in crate::app) playback: PreviewPlaybackState,
    pub(in crate::app) presentation: PreviewRenderPresentation,
    pub(in crate::app) render_image: Option<Arc<RenderImage>>,
    pub(in crate::app) runtime_error: Option<String>,
}

pub(in crate::app) fn preview_panel(
    file_queue: &FileQueue,
    settings: &SettingsRenderState<'_>,
    props: PreviewPanelProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: file_queue.selected_file(),
        settings,
        crop: props.crop,
        overlay: props.overlay,
        canvas: props.canvas,
        playback: props.playback,
        presentation: props.presentation,
        render_image: props.render_image,
        runtime_error: props.runtime_error,
    });

    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .p(px(PREVIEW_PANEL_PADDING))
        .child(preview_viewport(&state, props.viewport_focuses, window, cx))
        .child(preview_timeline(&state, props.timecode_focuses, window, cx))
}

pub(in crate::app) fn preview_shell_state(input: PreviewShellStateInput<'_>) -> PreviewShellState {
    let PreviewShellStateInput {
        selected_file,
        settings,
        crop,
        overlay,
        canvas,
        playback,
        presentation,
        render_image,
        runtime_error,
    } = input;
    let metadata_status = preview_metadata_status(settings.metadata_status);
    let source_media_kind = settings.metadata.map(preview_source_media_kind);
    let media = preview_media_render_state(render_image.as_ref(), presentation);
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
        canvas,
        crop,
        overlay,
        presentation,
        media,
        render_image,
        runtime_error,
    }
}

pub(in crate::app) fn preview_media_render_state(
    render_image: Option<&Arc<RenderImage>>,
    presentation: PreviewRenderPresentation,
) -> Option<PreviewMediaRenderState> {
    let frame = preview_presented_frame(render_image?, presentation)?;
    Some(PreviewMediaRenderState {
        width: frame.visible_width,
        height: frame.visible_height,
    })
}

pub(in crate::app) fn preview_presented_frame(
    render_image: &Arc<RenderImage>,
    presentation: PreviewRenderPresentation,
) -> Option<PreviewPresentedFrame> {
    let size = render_image.size(0);
    let raw_width = u32::try_from(size.width.0).ok()?;
    let raw_height = u32::try_from(size.height.0).ok()?;
    if raw_width == 0 || raw_height == 0 {
        return None;
    }

    let (full_width, full_height) = if presentation.transform.has_side_rotation() {
        (raw_height, raw_width)
    } else {
        (raw_width, raw_height)
    };

    let Some(crop) = presentation.crop else {
        return Some(PreviewPresentedFrame {
            full_width,
            full_height,
            visible_x: 0,
            visible_y: 0,
            visible_width: full_width,
            visible_height: full_height,
        });
    };

    let source_width = presentation.crop_source_width?;
    let source_height = presentation.crop_source_height?;
    let crop_right = crop.x.checked_add(crop.width)?;
    let crop_bottom = crop.y.checked_add(crop.height)?;
    if crop.width == 0
        || crop.height == 0
        || crop_right > source_width
        || crop_bottom > source_height
    {
        return None;
    }

    let left = scale_preview_crop_start(crop.x, source_width, full_width);
    let top = scale_preview_crop_start(crop.y, source_height, full_height);
    let right = scale_preview_crop_end(crop_right, source_width, full_width)
        .max(left.saturating_add(1))
        .min(full_width);
    let bottom = scale_preview_crop_end(crop_bottom, source_height, full_height)
        .max(top.saturating_add(1))
        .min(full_height);

    Some(PreviewPresentedFrame {
        full_width,
        full_height,
        visible_x: left,
        visible_y: top,
        visible_width: right.saturating_sub(left),
        visible_height: bottom.saturating_sub(top),
    })
}

fn scale_preview_crop_start(value: u32, source_extent: u32, image_extent: u32) -> u32 {
    f64_to_u32(
        ((f64::from(value) / f64::from(source_extent)) * f64::from(image_extent))
            .floor()
            .clamp(0.0, f64::from(image_extent.saturating_sub(1))),
    )
}

fn scale_preview_crop_end(value: u32, source_extent: u32, image_extent: u32) -> u32 {
    f64_to_u32(
        ((f64::from(value) / f64::from(source_extent)) * f64::from(image_extent))
            .ceil()
            .clamp(1.0, f64::from(image_extent)),
    )
}

pub(in crate::app) const fn preview_metadata_status(
    status: MetadataStatus,
) -> PreviewMetadataStatus {
    match status {
        MetadataStatus::Idle => PreviewMetadataStatus::Idle,
        MetadataStatus::Loading => PreviewMetadataStatus::Loading,
        MetadataStatus::Ready => PreviewMetadataStatus::Ready,
        MetadataStatus::Error => PreviewMetadataStatus::Error,
    }
}

pub(in crate::app) fn preview_source_media_kind(metadata: &SourceMetadata) -> SourceMediaKind {
    match metadata.source_kind() {
        SourceKind::Video => SourceMediaKind::Video,
        SourceKind::Audio => SourceMediaKind::Audio,
        SourceKind::Image => SourceMediaKind::Image,
    }
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
