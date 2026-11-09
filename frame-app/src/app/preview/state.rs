use super::actions::PreviewMediaCommandQueue;
use crate::preview::{
    CropRect, DragHandle, Point as PreviewPoint, PreviewOverlayState, PreviewPlaybackState,
};
use crate::preview_engine::{
    PreviewRenderPresentation, PreviewSession, PreviewSessionConfig,
    PreviewSourceKind as EnginePreviewSourceKind,
};
use gpui::{Bounds, Pixels, Point, RenderImage, ScrollHandle};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(in crate::app) const DEFAULT_CROP_X: f64 = 0.1;

pub(in crate::app) const DEFAULT_CROP_Y: f64 = 0.1;

pub(in crate::app) const DEFAULT_CROP_SIZE: f64 = 0.8;

pub(in crate::app) const CROP_HANDLE_SIZE: f32 = 10.0;

pub(in crate::app) const PREVIEW_CANVAS_DEFAULT_ZOOM: f64 = 1.0;

pub(in crate::app) const PREVIEW_CANVAS_INITIAL_CONTAIN_SCALE: f64 = 0.9;

pub(in crate::app) const PREVIEW_CANVAS_MIN_ZOOM: f64 = 0.25;

pub(in crate::app) const PREVIEW_CANVAS_MAX_ZOOM: f64 = 8.0;

pub(in crate::app) const PREVIEW_CANVAS_ZOOM_STEP: f64 = 1.18;

pub(in crate::app) const PREVIEW_CANVAS_WHEEL_ZOOM_STEP: f64 = 1.05;

pub(in crate::app) const PREVIEW_CANVAS_WHEEL_PIXEL_DELTA_PER_STEP: f64 = 48.0;

pub(in crate::app) const PREVIEW_CANVAS_WHEEL_DEADZONE: f64 = 0.08;

pub(in crate::app) const PREVIEW_CANVAS_WHEEL_MAX_STEPS_PER_EVENT: f64 = 8.0;

pub(in crate::app) const PREVIEW_CANVAS_MAX_PAN: f64 = 2.0;

pub(in crate::app) const PREVIEW_CANVAS_LERP_FACTOR: f64 = 0.2;

pub(in crate::app) const PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON: f64 = 0.5;

pub(in crate::app) const PREVIEW_CANVAS_PAN_SNAP_EPSILON: f64 = 0.01;

pub(in crate::app) const PREVIEW_CANVAS_ZOOM_SNAP_EPSILON: f64 =
    PREVIEW_CANVAS_PAN_SNAP_EPSILON / 10_000.0;

pub(in crate::app) const PREVIEW_ADAPTIVE_SCALE: f64 = 1.15;

pub(in crate::app) const PREVIEW_DIMENSION_QUANTUM: u32 = 64;

pub(in crate::app) const PREVIEW_MIN_ADAPTIVE_WIDTH: u32 = 640;

pub(in crate::app) const PREVIEW_MIN_ADAPTIVE_HEIGHT: u32 = 360;

pub(in crate::app) const PREVIEW_DIMENSION_DEBOUNCE_INTERVAL: Duration = Duration::from_millis(120);

pub(in crate::app) const PREVIEW_FILTER_DEBOUNCE_INTERVAL: Duration = Duration::from_millis(120);

pub(in crate::app) const PREVIEW_FRAME_TICK_INTERVAL: Duration = Duration::from_millis(16);

pub(in crate::app) const TRIM_PREVIEW_SEEK_INTERVAL: Duration = Duration::from_millis(50);

pub(in crate::app) const TRIM_PREVIEW_SEEK_EPSILON_SECONDS: f64 = 1.0 / 240.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum CropSelect {
    Aspect,
    Units,
}

pub(in crate::app) struct CropOptionsUiState {
    pub(in crate::app) popover: Option<CropSelect>,
    pub(in crate::app) custom: bool,
    pub(in crate::app) custom_dirty: bool,
    pub(in crate::app) custom_edit_base: Option<CropRect>,
    pub(in crate::app) is_ratio: bool,
    pub(in crate::app) width: String,
    pub(in crate::app) height: String,
    pub(in crate::app) scroll: ScrollHandle,
}

impl Default for CropOptionsUiState {
    fn default() -> Self {
        Self {
            popover: None,
            custom: false,
            custom_dirty: false,
            custom_edit_base: None,
            is_ratio: true,
            width: String::new(),
            height: String::new(),
            scroll: ScrollHandle::new(),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(in crate::app) struct PreviewTransportState {
    pub(in crate::app) hovered: bool,
    pub(in crate::app) open: bool,
}

pub(in crate::app) struct PreviewUiState {
    // Only deterministic visual fixtures seed this; real sessions always own frames.
    pub(in crate::app) static_fixture_file_id: Option<String>,
    pub(in crate::app) canvas_file_id: Option<String>,
    pub(in crate::app) canvas: PreviewCanvasState,
    pub(in crate::app) canvas_pan_drag: Option<PreviewCanvasPanDragState>,
    pub(in crate::app) canvas_bounds: Option<Bounds<Pixels>>,
    pub(in crate::app) crop_file_id: Option<String>,
    pub(in crate::app) crop_mode: bool,
    pub(in crate::app) draft_crop: Option<CropRect>,
    pub(in crate::app) crop_aspect: String,
    pub(in crate::app) crop_options: CropOptionsUiState,
    pub(in crate::app) crop_drag: Option<PreviewCropDragState>,
    pub(in crate::app) auto_crop: super::auto_crop::AutoCropUiState,
    pub(in crate::app) overlay_file_id: Option<String>,
    pub(in crate::app) overlay: PreviewOverlayState,
    pub(in crate::app) overlay_dimensions_key: Option<String>,
    pub(in crate::app) pending_overlay_dimensions_key: Option<String>,
    pub(in crate::app) overlay_image_dimensions: Option<PreviewOverlayImageDimensions>,
    pub(in crate::app) overlay_opacity_slider_bounds: Option<Bounds<Pixels>>,
    pub(in crate::app) timeline_track_bounds: Option<Bounds<Pixels>>,
    pub(in crate::app) transport: PreviewTransportState,
    pub(in crate::app) playback_file_id: Option<String>,
    pub(in crate::app) playback: PreviewPlaybackState,
    pub(in crate::app) active_preview_dimensions: Option<PreviewRuntimeDimensions>,
    pub(in crate::app) preview_dimensions_debounce_until: Option<Instant>,
    pub(in crate::app) filter_preview_debounce_until: Option<Instant>,
    pub(in crate::app) runtime_key: Option<PreviewRuntimeKey>,
    pub(in crate::app) pending_runtime_key: Option<PreviewRuntimeKey>,
    pub(in crate::app) failed_runtime_key: Option<PreviewRuntimeKey>,
    pub(in crate::app) runtime_request_generation: u64,
    pub(in crate::app) accepted_request_token: u64,
    pub(in crate::app) desired_runtime_key: Option<PreviewRuntimeKey>,
    pub(in crate::app) rendered_runtime_key: Option<PreviewRuntimeKey>,
    pub(in crate::app) rendered_presentation: PreviewRenderPresentation,
    pub(in crate::app) session: Option<Arc<PreviewSession>>,
    pub(in crate::app) trim_preview_seek: TrimPreviewSeekState,
    pub(in crate::app) media_command_generation: u64,
    pub(in crate::app) media_commands: PreviewMediaCommandQueue,
    pub(in crate::app) render_generation: u64,
    pub(in crate::app) render_image: Option<Arc<RenderImage>>,
    pub(in crate::app) runtime_error: Option<String>,
    pub(in crate::app) frame_tick_active: bool,
}

impl Default for PreviewUiState {
    fn default() -> Self {
        Self {
            static_fixture_file_id: None,
            canvas_file_id: None,
            canvas: PreviewCanvasState::default(),
            canvas_pan_drag: None,
            canvas_bounds: None,
            crop_file_id: None,
            crop_mode: false,
            draft_crop: None,
            crop_aspect: "free".to_string(),
            crop_options: CropOptionsUiState::default(),
            crop_drag: None,
            auto_crop: super::auto_crop::AutoCropUiState::default(),
            overlay_file_id: None,
            overlay: PreviewOverlayState::new(),
            overlay_dimensions_key: None,
            pending_overlay_dimensions_key: None,
            overlay_image_dimensions: None,
            overlay_opacity_slider_bounds: None,
            timeline_track_bounds: None,
            transport: PreviewTransportState::default(),
            playback_file_id: None,
            playback: PreviewPlaybackState::new(false),
            active_preview_dimensions: None,
            preview_dimensions_debounce_until: None,
            filter_preview_debounce_until: None,
            runtime_key: None,
            pending_runtime_key: None,
            failed_runtime_key: None,
            runtime_request_generation: 0,
            accepted_request_token: 0,
            desired_runtime_key: None,
            rendered_runtime_key: None,
            rendered_presentation: PreviewRenderPresentation::default(),
            session: None,
            trim_preview_seek: TrimPreviewSeekState::default(),
            media_command_generation: 0,
            media_commands: PreviewMediaCommandQueue::default(),
            render_generation: 0,
            render_image: None,
            runtime_error: None,
            frame_tick_active: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::app) struct TrimPreviewSeekState {
    pub(in crate::app) restore_seconds: Option<f64>,
    pub(in crate::app) pending_seconds: Option<f64>,
    pub(in crate::app) last_sent_seconds: Option<f64>,
    pub(in crate::app) worker_active: bool,
    pub(in crate::app) pause_before_next_seek: bool,
    pub(in crate::app) finish_after_restore: bool,
    pub(in crate::app) generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct TrimPreviewSeekRequest {
    pub(in crate::app) seconds: f64,
    pub(in crate::app) pause_first: bool,
}

impl TrimPreviewSeekState {
    pub(in crate::app) const fn is_active(&self) -> bool {
        self.restore_seconds.is_some()
            || self.pending_seconds.is_some()
            || self.worker_active
            || self.finish_after_restore
    }

    pub(in crate::app) fn reset(&mut self) {
        let next_generation = self.generation.saturating_add(1);
        *self = Self {
            generation: next_generation,
            ..Self::default()
        };
    }

    pub(in crate::app) fn begin_drag(&mut self, restore_seconds: f64) {
        if self.finish_after_restore {
            self.reset();
        }
        self.restore_seconds.get_or_insert(restore_seconds);
        self.finish_after_restore = false;
    }

    pub(in crate::app) const fn pause_before_next_seek(&mut self) {
        self.pause_before_next_seek = true;
    }

    pub(in crate::app) const fn queue(&mut self, seconds: f64) {
        self.pending_seconds = Some(seconds);
    }

    pub(in crate::app) fn take_next(&mut self) -> Option<TrimPreviewSeekRequest> {
        loop {
            let seconds = self.pending_seconds.take()?;
            let is_restore = self.finish_after_restore
                && self
                    .restore_seconds
                    .is_some_and(|restore| Self::seconds_match(restore, seconds));
            if !is_restore
                && self
                    .last_sent_seconds
                    .is_some_and(|last| Self::seconds_match(last, seconds))
            {
                continue;
            }

            self.last_sent_seconds = Some(seconds);
            let request = TrimPreviewSeekRequest {
                seconds,
                pause_first: self.pause_before_next_seek,
            };
            self.pause_before_next_seek = false;
            return Some(request);
        }
    }

    pub(in crate::app) fn finish(&mut self) -> bool {
        let Some(restore_seconds) = self.restore_seconds else {
            self.reset();
            return false;
        };
        self.pending_seconds = Some(restore_seconds);
        self.finish_after_restore = true;
        true
    }

    pub(in crate::app) fn complete_restore(&mut self, seconds: f64) -> bool {
        if self.finish_after_restore
            && self
                .restore_seconds
                .is_some_and(|restore| Self::seconds_match(restore, seconds))
        {
            self.reset();
            return true;
        }
        false
    }

    pub(in crate::app) fn seconds_match(first: f64, second: f64) -> bool {
        (first - second).abs() < TRIM_PREVIEW_SEEK_EPSILON_SECONDS
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewCanvasState {
    pub(in crate::app) current_zoom: f64,
    pub(in crate::app) target_zoom: f64,
    pub(in crate::app) current_pan_x: f64,
    pub(in crate::app) current_pan_y: f64,
    pub(in crate::app) target_pan_x: f64,
    pub(in crate::app) target_pan_y: f64,
    pub(in crate::app) auto_fit_pending: bool,
    pub(in crate::app) last_animation_tick: Option<Instant>,
}

impl Default for PreviewCanvasState {
    fn default() -> Self {
        Self {
            current_zoom: PREVIEW_CANVAS_DEFAULT_ZOOM,
            target_zoom: PREVIEW_CANVAS_DEFAULT_ZOOM,
            current_pan_x: 0.0,
            current_pan_y: 0.0,
            target_pan_x: 0.0,
            target_pan_y: 0.0,
            auto_fit_pending: true,
            last_animation_tick: None,
        }
    }
}

#[expect(
    clippy::struct_field_names,
    reason = "Drag state fields intentionally share the start prefix to distinguish initial values from live pan values."
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewCanvasPanDragState {
    pub(in crate::app) start_position: Point<Pixels>,
    pub(in crate::app) start_pan_x: f64,
    pub(in crate::app) start_pan_y: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum PreviewCanvasZoomDirection {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewOverlayImageDimensions {
    pub(in crate::app) width: u32,
    pub(in crate::app) height: u32,
}

impl PreviewOverlayImageDimensions {
    pub(in crate::app) fn height_over_width(self) -> f64 {
        if self.width == 0 {
            return 1.0;
        }

        f64::from(self.height) / f64::from(self.width)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct PreviewCropDragState {
    pub(in crate::app) handle: DragHandle,
    pub(in crate::app) start_rect: CropRect,
    pub(in crate::app) start_point: PreviewPoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewRuntimeKey {
    pub(in crate::app) file_id: String,
    pub(in crate::app) path: String,
    pub(in crate::app) source_kind: EnginePreviewSourceKind,
    pub(in crate::app) source_width: Option<u32>,
    pub(in crate::app) source_height: Option<u32>,
    pub(in crate::app) duration_millis: u64,
    pub(in crate::app) preview_dimensions: PreviewRuntimeDimensions,
    pub(in crate::app) visual_hash: u64,
    pub(in crate::app) audio_hash: u64,
    pub(in crate::app) edit_crop: bool,
    pub(in crate::app) edit_overlay: bool,
}

impl PreviewRuntimeKey {
    pub(in crate::app) fn can_reconfigure_to(&self, next: &Self) -> bool {
        self.file_id == next.file_id
            && self.path == next.path
            && self.source_kind == next.source_kind
            && self.source_width == next.source_width
            && self.source_height == next.source_height
            && self.duration_millis == next.duration_millis
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewRuntimeDimensions {
    pub(in crate::app) max_width: u32,
    pub(in crate::app) max_height: u32,
}

#[derive(Clone, Debug)]
pub(in crate::app) struct PreviewRuntimeRequest {
    pub(in crate::app) key: PreviewRuntimeKey,
    pub(in crate::app) config: PreviewSessionConfig,
}
