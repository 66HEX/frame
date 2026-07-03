mod chrome;
mod components;
mod conversion;
mod file_list_panel;
mod files;
mod fixtures;
mod input;
mod logs_panel;
mod logs_state;
mod metadata;
mod motion;
mod preview_actions;
mod preview_panel;
mod primitives;
mod render;
mod runtime;
mod settings_actions;
mod settings_panel;
mod state;
#[cfg(test)]
mod tests;
mod update_actions;
mod workspace;
pub use runtime::{frame_window_options, init_app, open_frame_window};

use chrome::{
    AppSettingsSheetProps, app_settings_sheet, drag_drop_overlay, titlebar, update_dialog,
};
use input::{FrameTextInputKind, FrameTextInputUiState};
use logs_panel::logs_view;
use motion::{
    SETTINGS_SHEET_MOTION_DURATION, SUBTITLE_POPOVER_MOTION_DURATION, hover_motion, mix_color,
    mix_scalar, motion_is_hidden, motion_target, retarget_hover_motion, selected_motion,
    set_motion_target, settings_sheet_right_inset, subtitle_popover_slide_offset,
};
use preview_panel::{
    FlipAxis, PreviewCanvasRenderState, PreviewCropRenderState, PreviewMediaRenderState,
    PreviewOverlayRenderState, PreviewPanelProps, PreviewTimecodeInputFocuses, crop_aspect_id,
    crop_base_dimensions, crop_rect_from_settings, crop_rect_is_full, crop_settings_from_rect,
    default_crop_rect, full_crop_rect, is_known_crop_aspect, next_rotation,
    preview_crop_controls_enabled, preview_crop_source_dimensions, preview_duration_seconds,
    preview_playback_state, preview_source_media_kind, preview_transform_controls_enabled,
    timeline_slider_percent_from_bounds,
};
use primitives::color;
use runtime::hide_native_macos_titlebar_controls;
use workspace::workspace_view;

use crate::{
    ActiveView, CONTENT_PADDING, FILE_LIST_ROW_SPAN, FILE_ROW_HEIGHT, FrameAppState,
    LEFT_COLUMN_SPAN, LEFT_GRID_ROWS, PANEL_HEADER_HEIGHT, PREVIEW_PANEL_PADDING,
    PREVIEW_PLAYHEAD_HEIGHT, PREVIEW_ROW_SPAN, PREVIEW_TIMELINE_CONTROL_HEIGHT,
    PREVIEW_TIMELINE_HANDLE_WIDTH, PREVIEW_TIMELINE_TOP_MARGIN, PREVIEW_TOOLBAR_BUTTON_SIZE,
    PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET, PREVIEW_TRACK_HEIGHT, RIGHT_COLUMN_SPAN,
    SETTINGS_CONTROL_HEIGHT, SETTINGS_PANEL_PADDING, SETTINGS_TAB_BUTTON_SIZE,
    SETTINGS_TAB_ICON_SIZE, TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_BUTTON_HEIGHT,
    TITLEBAR_DIVIDER_HEIGHT, TITLEBAR_HEIGHT, TITLEBAR_ICON_BUTTON_SIZE, TITLEBAR_ICON_SIZE,
    TITLEBAR_LINUX_WINDOW_BUTTON_SIZE, TITLEBAR_LINUX_WINDOW_CONTROLS_GAP,
    TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X, TITLEBAR_LOGO_SIZE, TITLEBAR_NAV_BUTTON_HEIGHT,
    TITLEBAR_PLATFORM_DIVIDER_HEIGHT, TITLEBAR_SEGMENT_HEIGHT, TITLEBAR_TOP_PADDING,
    TITLEBAR_TRAFFIC_LIGHT_DOT_SIZE, TITLEBAR_TRAFFIC_LIGHT_SIZE,
    TITLEBAR_TRAFFIC_LIGHT_STROKE_WIDTH, TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
    TITLEBAR_WINDOWS_WINDOW_ICON_SIZE, TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE, VisualFixture,
    WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WORKSPACE_COLUMNS, WORKSPACE_GAP,
    active_view_from_env_value,
    app_info::FRAME_APP_ID,
    app_persistence::{AppPersistence, AppSettings},
    assets::{self},
    capabilities::detect_available_encoders,
    conversion_events::{ActiveLogFile, ConversionEventState, LogLine, all_conversions_settled},
    conversion_runner::{
        ConversionProcessController, conversion_task_from_file, run_conversion_batch_with_control,
    },
    file_filters::{
        AUDIO_FILE_EXTENSIONS, IMAGE_FILE_EXTENSIONS, filter_supported_source_paths,
        is_supported_overlay_image_path, is_supported_subtitle_path,
    },
    file_queue::{
        BatchSelectionState, FileItem, FileQueue, FileStateTone, FileStatus, RowActionAvailability,
        format_file_size,
    },
    format_total_size,
    native_dialogs::{
        overlay_image_dialog, pick_overlay_image_file, pick_source_files, pick_subtitle_file,
        source_file_dialog, subtitle_file_dialog,
    },
    notifications::{AppNotifier, conversion_finished_notification_for_task_ids},
    preview::{
        ASPECT_OPTIONS, CropRect, DragHandle, MAX_OVERLAY_WIDTH, MIN_OVERLAY_WIDTH, MediaSnapshot,
        MetadataStatus as PreviewMetadataStatus, OverlayDragHandle, OverlayDragPoint,
        OverlayModeChange, OverlaySizeDirection, PlaybackMediaCommand, Point as PreviewPoint,
        PreviewControlAvailability, PreviewControlInput, PreviewMediaKind, PreviewOverlay,
        PreviewOverlayState, PreviewPlaybackState, PreviewRotation, SourceMediaKind,
        TimelineDragTarget, adjust_rect_to_ratio, aspect_value, clamp_rect, format_time,
        parse_time_to_seconds, preview_control_availability, transform_crop_rect,
    },
    preview_engine::{
        DEFAULT_PREVIEW_FPS, DEFAULT_PREVIEW_MAX_HEIGHT, DEFAULT_PREVIEW_MAX_WIDTH,
        MIN_PREVIEW_DIMENSION, PreviewCommand, PreviewRenderPresentation, PreviewSession,
        PreviewSessionConfig, PreviewSourceKind as EnginePreviewSourceKind, PreviewTransform,
        render_image_from_frame,
    },
    settings::{
        ConversionConfig, CropSettings, DEFAULT_SUBTITLE_FONT_COLOR,
        DEFAULT_SUBTITLE_OUTLINE_COLOR, MetadataField, OverlaySettings, PresetDefinition,
        PresetNotice, PresetNoticeTone, PresetOption, ProcessingMode, SettingsTab,
        SourceInfoSection, SourceKind, SourceMetadata, SourceTags, SubtitleFontOption,
        SubtitleFontSizeOption, apply_audio_bitrate, apply_audio_bitrate_mode,
        apply_audio_channels, apply_audio_codec, apply_audio_normalize, apply_audio_quality,
        apply_audio_volume, apply_crf, apply_custom_height, apply_custom_width, apply_fps,
        apply_gif_colors, apply_gif_dither, apply_gif_loop, apply_hw_decode,
        apply_image_jpeg_huffman, apply_image_jpeg_quality, apply_image_png_compression,
        apply_image_png_prediction, apply_image_tiff_compression, apply_image_webp_compression,
        apply_image_webp_lossless, apply_image_webp_preset, apply_image_webp_quality,
        apply_metadata_field, apply_metadata_mode, apply_nvenc_spatial_aq, apply_nvenc_temporal_aq,
        apply_output_container, apply_pixel_format, apply_preset, apply_processing_mode,
        apply_quality, apply_resolution, apply_scaling_algorithm, apply_subtitle_burn_path,
        apply_subtitle_font_color, apply_subtitle_font_name, apply_subtitle_font_size,
        apply_subtitle_outline_color, apply_subtitle_position, apply_trim_times,
        apply_video_bitrate, apply_video_bitrate_mode, apply_video_codec, apply_video_preset,
        apply_videotoolbox_allow_sw, audio_channel_options, audio_codec_options,
        audio_codec_supports_vbr, audio_quality_range, audio_track_options, create_custom_preset,
        default_presets, fps_options, gif_color_options, gif_dither_options,
        image_jpeg_huffman_options, image_png_prediction_options, image_tiff_compression_options,
        image_webp_preset_options, is_gif_container, is_hardware_video_codec, is_nvenc_video_codec,
        is_videotoolbox_video_codec, metadata_field_options, metadata_field_value,
        metadata_mode_options, normalize_output_config, normalized_hex_color,
        output_container_options, output_processing_mode_options, preset_options,
        resolution_options, resolve_active_settings_tab, sanitize_output_name,
        scaling_algorithm_options, source_info_sections, subtitle_burn_file_label,
        subtitle_color_value, subtitle_font_options, subtitle_font_size_options,
        subtitle_position_options, subtitle_track_options, toggle_audio_track_selection,
        toggle_subtitle_track_selection, video_codec_options, video_pixel_format_options,
        video_preset_options, visible_settings_tabs,
    },
    source_metadata::{
        MetadataStatus, SourceMetadataEntry, SourceMetadataStore, probe_source_metadata,
    },
    theme,
    update_runtime::{
        build_update_client, unix_timestamp, update_check_is_due, updates_disabled_explanation,
    },
    visual_fixture_from_env_value,
};
use frame_core::capabilities::AvailableEncoders;
use frame_core::events::ConversionEvent;
use frame_core::types::DEFAULT_MAX_CONCURRENCY;
use frame_updater::{DownloadProgress, UpdateChannel, UpdateCheck, UpdateInfo, UpdatePackage};
use gpui::{
    App, Bounds, BoxShadow, ClickEvent, ClientSideFrameOptions, ClipboardItem, Context,
    DispatchPhase, DragMoveEvent, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, ExternalPaths, FileDropEvent, FocusHandle, GlobalElementId,
    InteractiveElement, IntoElement, KeyBinding, LayoutId, Lerp, Menu, MenuItem, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, PaintQuad, PinchEvent, Pixels,
    PlatformInput, Point, Position, PromptButton, PromptLevel, Render, RenderImage, Rgba,
    ScrollDelta, ScrollHandle, ScrollStrategy, ScrollWheelEvent, ShapedLine, SharedString,
    StatefulInteractiveElement, Style, Task, TextRenderingMode, TextRun, TitlebarOptions,
    TransformationMatrix, UTF16Selection, UniformListScrollHandle, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
    actions, canvas, deferred, div, ease_out_quint, fill, hsla, img, linear_color_stop,
    linear_gradient, point, prelude::*, px, radians, relative, size, svg, uniform_list,
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSView, NSWindowButton};
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    ops::Range,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, TryRecvError},
    },
    time::{Duration, Instant},
};

actions!(
    frame_app,
    [
        Quit,
        TextInputBackspace,
        TextInputDelete,
        TextInputLeft,
        TextInputRight,
        TextInputSelectLeft,
        TextInputSelectRight,
        TextInputHome,
        TextInputEnd,
        TextInputSelectAll,
        TextInputCopy,
        TextInputCut,
        TextInputPaste,
    ]
);

const FILE_LIST_ACTIONS_WIDTH: f32 = 64.0;
const FILE_LIST_ACTION_BUTTON_SIZE: f32 = 24.0;
const FILE_LIST_ACTION_ICON_SIZE: f32 = 16.0;
const LOG_LINE_NUMBER_WIDTH: f32 = 32.0;
const LOG_LINE_HEIGHT: f32 = 24.0;
const LOG_SCROLL_BUTTON_OFFSET: f32 = 10.0;
const LOG_SCROLL_BUTTON_PADDING: f32 = 4.0;
const LOG_SCROLL_BUTTON_SIZE: f32 = 24.0;
const LOG_SCROLL_ICON_SIZE: f32 = 16.0;
const LOG_COPY_FEEDBACK_DURATION: Duration = Duration::from_millis(1_200);
const TRAFFIC_LIGHT_GROUP: &str = "titlebar-traffic-lights";
const TRAFFIC_CLOSE_FILL: &str = "#ff5f56";
const TRAFFIC_CLOSE_BORDER: &str = "#e0443e";
const TRAFFIC_CLOSE_SYMBOL: &str = "#4a0002";
const TRAFFIC_MINIMIZE_FILL: &str = "#ffbd2e";
const TRAFFIC_MINIMIZE_BORDER: &str = "#dea123";
const TRAFFIC_MINIMIZE_SYMBOL: &str = "#5a3900";
const TRAFFIC_ZOOM_FILL: &str = "#27c93f";
const TRAFFIC_ZOOM_BORDER: &str = "#1aab29";
const TRAFFIC_ZOOM_SYMBOL: &str = "#004200";
const ROOT_DROP_GROUP: &str = "frame-root-drop-target";
const DEFAULT_CROP_X: f64 = 0.1;
const DEFAULT_CROP_Y: f64 = 0.1;
const DEFAULT_CROP_SIZE: f64 = 0.8;
const CROP_HANDLE_SIZE: f32 = 10.0;
const FRAME_TEXT_INPUT_CONTEXT: &str = "FrameTextInput";
const TEXT_INPUT_CARET_WIDTH: f32 = 1.5;
const TEXT_INPUT_CARET_HEIGHT: f32 = theme::TEXT_INPUT_CARET_HEIGHT;
const TEXT_INPUT_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const TEXT_INPUT_BLINK_PAUSE: Duration = Duration::from_millis(300);
const PREVIEW_CANVAS_DEFAULT_ZOOM: f64 = 1.0;
const PREVIEW_CANVAS_INITIAL_CONTAIN_SCALE: f64 = 0.9;
const PREVIEW_CANVAS_MIN_ZOOM: f64 = 0.25;
const PREVIEW_CANVAS_MAX_ZOOM: f64 = 8.0;
const PREVIEW_CANVAS_ZOOM_STEP: f64 = 1.18;
const PREVIEW_CANVAS_WHEEL_ZOOM_STEP: f64 = 1.05;
const PREVIEW_CANVAS_WHEEL_PIXEL_DELTA_PER_STEP: f64 = 48.0;
const PREVIEW_CANVAS_WHEEL_DEADZONE: f64 = 0.08;
const PREVIEW_CANVAS_WHEEL_MAX_STEPS_PER_EVENT: f64 = 8.0;
const PREVIEW_CANVAS_MAX_PAN: f64 = 2.0;
const PREVIEW_CANVAS_LERP_FACTOR: f64 = 0.2;
const PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON: f64 = 0.5;
const PREVIEW_CANVAS_PAN_SNAP_EPSILON: f64 = 0.01;
const PREVIEW_CANVAS_ZOOM_SNAP_EPSILON: f64 = PREVIEW_CANVAS_PAN_SNAP_EPSILON / 10_000.0;
const PREVIEW_ADAPTIVE_SCALE: f64 = 1.15;
const PREVIEW_DIMENSION_QUANTUM: u32 = 64;
const PREVIEW_MIN_ADAPTIVE_WIDTH: u32 = 640;
const PREVIEW_MIN_ADAPTIVE_HEIGHT: u32 = 360;
const PREVIEW_DIMENSION_DEBOUNCE_INTERVAL: Duration = Duration::from_millis(120);
const PREVIEW_FRAME_TICK_INTERVAL: Duration = Duration::from_millis(16);

#[expect(
    clippy::struct_excessive_bools,
    reason = "Root UI state keeps independent boolean flags for GPUI render and input paths."
)]
pub struct FrameRoot {
    active_view: ActiveView,
    file_queue: FileQueue,
    conversion_events: ConversionEventState,
    logs_scroll_handle: UniformListScrollHandle,
    last_log_scroll_target: Option<LogScrollTarget>,
    logs_follow_tail: bool,
    copied_log_file_id: Option<String>,
    log_copy_feedback_epoch: usize,
    is_processing: bool,
    settings_ui: SettingsUiState,
    drag_drop_ui: DragDropUiState,
    max_concurrency: usize,
    text_input_ui: FrameTextInputUiState,
    source_metadata: SourceMetadataStore,
    conversion_processes: ConversionProcessController,
    available_encoders: AvailableEncoders,
    active_conversion_task_ids: Vec<String>,
    notifier: AppNotifier,
    subtitle_font_families: Vec<String>,
    presets: Vec<PresetDefinition>,
    subtitle_ui: SubtitleUiState,
    preview_ui: PreviewUiState,
    native_titlebar_controls_hidden: bool,
    next_file_sequence: u64,
    persistence: Option<AppPersistence>,
    auto_update_check: bool,
    update_channel: UpdateChannel,
    skipped_update_version: Option<String>,
    last_update_check_at: Option<u64>,
    update_ui: UpdateUiState,
}

#[derive(Default)]
struct DragDropUiState {
    is_open: bool,
    is_present: bool,
}

struct SettingsUiState {
    is_open: bool,
    is_present: bool,
    active_tab: SettingsTab,
    max_concurrency_draft: String,
    max_concurrency_error: Option<String>,
    preset_name_draft: String,
    preset_notice: Option<PresetNotice>,
    next_custom_preset_sequence: u64,
}

#[derive(Clone, Debug, Default)]
struct UpdateUiState {
    status: UpdateStatus,
    dialog_open: bool,
    dialog_present: bool,
    dialog_info: Option<Box<UpdateInfo>>,
}

#[derive(Clone, Debug, Default)]
enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Box<UpdateInfo>),
    Downloading {
        version: String,
        progress_percent: Option<u8>,
        received_bytes: u64,
        total_bytes: Option<u64>,
    },
    ReadyToInstall(Box<UpdatePackage>),
    Installing,
    Disabled(String),
    Error(String),
}

impl UpdateStatus {
    const fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading { .. } | Self::Installing
        )
    }
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_present: false,
            active_tab: SettingsTab::Source,
            max_concurrency_draft: DEFAULT_MAX_CONCURRENCY.to_string(),
            max_concurrency_error: None,
            preset_name_draft: String::new(),
            preset_notice: None,
            next_custom_preset_sequence: 0,
        }
    }
}

struct SubtitleUiState {
    popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    font_select_scroll_handle: ScrollHandle,
    font_size_select_scroll_handle: ScrollHandle,
    font_color_draft: String,
    outline_color_draft: String,
    font_color_hsv_draft: SettingsSubtitleHsv,
    outline_color_hsv_draft: SettingsSubtitleHsv,
    color_picker_bounds: SettingsSubtitleColorPickerBounds,
}

impl Default for SubtitleUiState {
    fn default() -> Self {
        Self {
            popover: None,
            rendered_popover: None,
            font_select_scroll_handle: ScrollHandle::new(),
            font_size_select_scroll_handle: ScrollHandle::new(),
            font_color_draft: DEFAULT_SUBTITLE_FONT_COLOR.to_uppercase(),
            outline_color_draft: DEFAULT_SUBTITLE_OUTLINE_COLOR.to_uppercase(),
            font_color_hsv_draft: settings_panel::hex_to_subtitle_hsv(DEFAULT_SUBTITLE_FONT_COLOR),
            outline_color_hsv_draft: settings_panel::hex_to_subtitle_hsv(
                DEFAULT_SUBTITLE_OUTLINE_COLOR,
            ),
            color_picker_bounds: SettingsSubtitleColorPickerBounds::default(),
        }
    }
}

struct PreviewUiState {
    canvas_file_id: Option<String>,
    canvas: PreviewCanvasState,
    canvas_pan_drag: Option<PreviewCanvasPanDragState>,
    canvas_bounds: Option<Bounds<Pixels>>,
    crop_file_id: Option<String>,
    crop_mode: bool,
    draft_crop: Option<CropRect>,
    crop_aspect: String,
    crop_drag: Option<PreviewCropDragState>,
    overlay_file_id: Option<String>,
    overlay: PreviewOverlayState,
    overlay_dimensions_key: Option<String>,
    pending_overlay_dimensions_key: Option<String>,
    overlay_image_dimensions: Option<PreviewOverlayImageDimensions>,
    overlay_opacity_slider_bounds: Option<Bounds<Pixels>>,
    timeline_track_bounds: Option<Bounds<Pixels>>,
    playback_file_id: Option<String>,
    playback: PreviewPlaybackState,
    active_preview_dimensions: Option<PreviewRuntimeDimensions>,
    preview_dimensions_debounce_until: Option<Instant>,
    runtime_key: Option<PreviewRuntimeKey>,
    pending_runtime_key: Option<PreviewRuntimeKey>,
    render_presentation: PreviewRenderPresentation,
    rendered_presentation: PreviewRenderPresentation,
    session: Option<Arc<PreviewSession>>,
    render_generation: u64,
    render_image: Option<Arc<RenderImage>>,
    runtime_error: Option<String>,
    frame_tick_active: bool,
}

impl Default for PreviewUiState {
    fn default() -> Self {
        Self {
            canvas_file_id: None,
            canvas: PreviewCanvasState::default(),
            canvas_pan_drag: None,
            canvas_bounds: None,
            crop_file_id: None,
            crop_mode: false,
            draft_crop: None,
            crop_aspect: "free".to_string(),
            crop_drag: None,
            overlay_file_id: None,
            overlay: PreviewOverlayState::new(),
            overlay_dimensions_key: None,
            pending_overlay_dimensions_key: None,
            overlay_image_dimensions: None,
            overlay_opacity_slider_bounds: None,
            timeline_track_bounds: None,
            playback_file_id: None,
            playback: PreviewPlaybackState::new(false),
            active_preview_dimensions: None,
            preview_dimensions_debounce_until: None,
            runtime_key: None,
            pending_runtime_key: None,
            render_presentation: PreviewRenderPresentation::default(),
            rendered_presentation: PreviewRenderPresentation::default(),
            session: None,
            render_generation: 0,
            render_image: None,
            runtime_error: None,
            frame_tick_active: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewCanvasState {
    current_zoom: f64,
    target_zoom: f64,
    current_pan_x: f64,
    current_pan_y: f64,
    target_pan_x: f64,
    target_pan_y: f64,
    auto_fit_pending: bool,
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
        }
    }
}

#[expect(
    clippy::struct_field_names,
    reason = "Drag state fields intentionally share the start prefix to distinguish initial values from live pan values."
)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewCanvasPanDragState {
    start_position: Point<Pixels>,
    start_pan_x: f64,
    start_pan_y: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum PreviewCanvasZoomDirection {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewOverlayImageDimensions {
    width: u32,
    height: u32,
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
struct PreviewCropDragState {
    handle: DragHandle,
    start_rect: CropRect,
    start_point: PreviewPoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreviewRuntimeKey {
    file_id: String,
    path: String,
    source_kind: EnginePreviewSourceKind,
    source_width: Option<u32>,
    source_height: Option<u32>,
    duration_millis: u64,
    preview_dimensions: PreviewRuntimeDimensions,
    visual_hash: u64,
    audio_hash: u64,
}

impl PreviewRuntimeKey {
    fn can_reconfigure_to(&self, next: &Self) -> bool {
        self.file_id == next.file_id
            && self.path == next.path
            && self.source_kind == next.source_kind
            && self.source_width == next.source_width
            && self.source_height == next.source_height
            && self.duration_millis == next.duration_millis
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewRuntimeDimensions {
    max_width: u32,
    max_height: u32,
}

#[derive(Clone, Debug)]
struct PreviewRuntimeRequest {
    key: PreviewRuntimeKey,
    config: PreviewSessionConfig,
    presentation: PreviewRenderPresentation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitlePopover {
    FontName,
    FontSize,
    FontColor,
    OutlineColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitleColorTarget {
    Font,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct SettingsSubtitleHsv {
    h: f64,
    s: f64,
    v: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum SettingsSubtitleColorDragKind {
    SaturationValue,
    Hue,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct SettingsSubtitleColorDrag {
    target: SettingsSubtitleColorTarget,
    kind: SettingsSubtitleColorDragKind,
    base_hsv: SettingsSubtitleHsv,
}

#[derive(Clone, Copy, Debug, Default)]
struct SettingsSubtitleColorPickerBounds {
    font_sv: Option<Bounds<Pixels>>,
    font_hue: Option<Bounds<Pixels>>,
    outline_sv: Option<Bounds<Pixels>>,
    outline_hue: Option<Bounds<Pixels>>,
}

#[derive(Clone, Copy)]
struct SettingsRenderState<'a> {
    active_tab: SettingsTab,
    config: &'a ConversionConfig,
    metadata: Option<&'a SourceMetadata>,
    metadata_status: MetadataStatus,
    metadata_error: Option<&'a str>,
    settings_disabled: bool,
    output_name: &'a str,
    output_name_focus: Option<&'a FocusHandle>,
    audio_bitrate_focus: Option<&'a FocusHandle>,
    video_width_focus: Option<&'a FocusHandle>,
    video_height_focus: Option<&'a FocusHandle>,
    video_bitrate_focus: Option<&'a FocusHandle>,
    gif_loop_focus: Option<&'a FocusHandle>,
    metadata_focuses: SettingsMetadataInputFocuses<'a>,
    subtitle_color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    subtitle_popover: Option<SettingsSubtitlePopover>,
    subtitle_rendered_popover: Option<SettingsSubtitlePopover>,
    subtitle_font_select_scroll_handle: &'a ScrollHandle,
    subtitle_font_size_select_scroll_handle: &'a ScrollHandle,
    subtitle_font_color_draft: &'a str,
    subtitle_outline_color_draft: &'a str,
    subtitle_font_color_hsv_draft: SettingsSubtitleHsv,
    subtitle_outline_color_hsv_draft: SettingsSubtitleHsv,
    preset_name: &'a str,
    preset_name_focus: Option<&'a FocusHandle>,
    presets: &'a [PresetDefinition],
    preset_notice: Option<&'a PresetNotice>,
    subtitle_fonts: &'a [String],
    available_encoders: &'a AvailableEncoders,
}

#[derive(Clone, Copy)]
struct SettingsVideoInputFocuses<'a> {
    width: Option<&'a FocusHandle>,
    height: Option<&'a FocusHandle>,
    bitrate: Option<&'a FocusHandle>,
    gif_loop: Option<&'a FocusHandle>,
}

#[derive(Clone, Copy)]
struct SettingsMetadataInputFocuses<'a> {
    title: Option<&'a FocusHandle>,
    artist: Option<&'a FocusHandle>,
    album: Option<&'a FocusHandle>,
    genre: Option<&'a FocusHandle>,
    date: Option<&'a FocusHandle>,
    comment: Option<&'a FocusHandle>,
}

#[derive(Clone, Copy)]
struct SettingsSubtitleColorInputFocuses<'a> {
    font: Option<&'a FocusHandle>,
    outline: Option<&'a FocusHandle>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LogScrollTarget {
    file_id: String,
    line_count: usize,
}
