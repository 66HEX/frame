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
mod workspace;
pub use runtime::{frame_window_options, init_app};

use chrome::{app_settings_sheet, drag_drop_overlay, titlebar};
use input::{FrameTextInputKind, FrameTextInputUiState};
use logs_panel::logs_view;
use preview_panel::{
    FlipAxis, PreviewCropRenderState, crop_aspect_id, crop_rect_from_settings, crop_rect_is_full,
    crop_settings_from_rect, default_crop_rect, full_crop_rect, is_known_crop_aspect,
    is_side_rotation, next_rotation, preview_crop_controls_enabled, preview_crop_source_dimensions,
    preview_duration_seconds, preview_playback_state, preview_source_media_kind,
    preview_transform_controls_enabled,
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
    TITLEBAR_LOGO_SIZE, TITLEBAR_NAV_BUTTON_HEIGHT, TITLEBAR_SEGMENT_HEIGHT, TITLEBAR_TOP_PADDING,
    TITLEBAR_TRAFFIC_LIGHT_DOT_SIZE, TITLEBAR_TRAFFIC_LIGHT_SIZE,
    TITLEBAR_TRAFFIC_LIGHT_STROKE_WIDTH, VisualFixture, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
    WORKSPACE_COLUMNS, WORKSPACE_GAP, active_view_from_env_value,
    app_info::FRAME_APP_ID,
    app_persistence::{AppPersistence, AppSettings},
    assets::{self},
    conversion_events::{ActiveLogFile, ConversionEventState, LogLine, all_conversions_settled},
    conversion_runner::{
        ConversionProcessController, conversion_task_from_file, run_conversion_batch_with_control,
    },
    file_filters::{filter_supported_source_paths, is_supported_subtitle_path},
    file_queue::{
        BatchSelectionState, FileItem, FileQueue, FileStateTone, FileStatus, RowActionAvailability,
        format_file_size,
    },
    format_total_size,
    native_dialogs::{pick_source_files, pick_subtitle_file},
    preview::{
        ASPECT_OPTIONS, CropRect, DragHandle, MediaSnapshot,
        MetadataStatus as PreviewMetadataStatus, Point as PreviewPoint, PreviewControlAvailability,
        PreviewControlInput, PreviewMediaKind, PreviewPlaybackState, PreviewRotation,
        SourceMediaKind, TimelineDragTarget, adjust_rect_to_ratio, aspect_value, clamp_rect,
        format_time, parse_time_to_seconds, preview_control_availability, transform_crop_rect,
    },
    settings::{
        ConversionConfig, CropSettings, DEFAULT_SUBTITLE_FONT_COLOR,
        DEFAULT_SUBTITLE_OUTLINE_COLOR, MetadataField, PresetDefinition, PresetNotice,
        PresetNoticeTone, PresetOption, ProcessingMode, SettingsTab, SourceInfoSection, SourceKind,
        SourceMetadata, SourceTags, SubtitleFontOption, SubtitleFontSizeOption,
        apply_audio_bitrate, apply_audio_bitrate_mode, apply_audio_channels, apply_audio_codec,
        apply_audio_normalize, apply_audio_quality, apply_audio_volume, apply_crf,
        apply_custom_height, apply_custom_width, apply_fps, apply_gif_colors, apply_gif_dither,
        apply_gif_loop, apply_hw_decode, apply_metadata_field, apply_metadata_mode,
        apply_nvenc_spatial_aq, apply_nvenc_temporal_aq, apply_output_container,
        apply_pixel_format, apply_preset, apply_processing_mode, apply_quality, apply_resolution,
        apply_scaling_algorithm, apply_subtitle_burn_path, apply_subtitle_font_color,
        apply_subtitle_font_name, apply_subtitle_font_size, apply_subtitle_outline_color,
        apply_subtitle_position, apply_trim_times, apply_video_bitrate, apply_video_bitrate_mode,
        apply_video_codec, apply_video_preset, apply_videotoolbox_allow_sw, audio_channel_options,
        audio_codec_options, audio_codec_supports_vbr, audio_quality_range, audio_track_options,
        create_custom_preset, default_presets, fps_options, gif_color_options, gif_dither_options,
        is_gif_container, is_hardware_video_codec, is_nvenc_video_codec,
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
    theme, visual_fixture_from_env_value,
};
use frame_core::capabilities::AvailableEncoders;
use frame_core::events::ConversionEvent;
use frame_core::types::DEFAULT_MAX_CONCURRENCY;
use gpui::{
    App, Bounds, BoxShadow, ClickEvent, ClipboardItem, Context, DragMoveEvent, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, ExternalPaths, FocusHandle, GlobalElementId,
    InteractiveElement, IntoElement, KeyBinding, LayoutId, Menu, MenuItem, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, Position, PromptButton,
    PromptLevel, Render, Rgba, ShapedLine, SharedString, StatefulInteractiveElement, Style, Task,
    TextRun, TitlebarOptions, UTF16Selection, UniformListScrollHandle, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
    actions, deferred, div, fill, hsla, linear_color_stop, linear_gradient, point, prelude::*, px,
    relative, size, svg, uniform_list,
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSView, NSWindowButton};
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    ops::Range,
    path::PathBuf,
    sync::mpsc::{self, TryRecvError},
    time::Duration,
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

pub struct FrameRoot {
    active_view: ActiveView,
    file_queue: FileQueue,
    conversion_events: ConversionEventState,
    logs_scroll_handle: UniformListScrollHandle,
    last_log_scroll_target: Option<LogScrollTarget>,
    is_processing: bool,
    settings_ui: SettingsUiState,
    max_concurrency: usize,
    text_input_ui: FrameTextInputUiState,
    source_metadata: SourceMetadataStore,
    conversion_processes: ConversionProcessController,
    available_encoders: AvailableEncoders,
    subtitle_font_families: Vec<String>,
    presets: Vec<PresetDefinition>,
    subtitle_ui: SubtitleUiState,
    preview_ui: PreviewUiState,
    native_titlebar_controls_hidden: bool,
    next_file_sequence: u64,
    persistence: Option<AppPersistence>,
}

struct SettingsUiState {
    is_open: bool,
    active_tab: SettingsTab,
    max_concurrency_draft: String,
    max_concurrency_error: Option<String>,
    preset_name_draft: String,
    preset_notice: Option<PresetNotice>,
    next_custom_preset_sequence: u64,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            is_open: false,
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
    crop_file_id: Option<String>,
    crop_mode: bool,
    draft_crop: Option<CropRect>,
    crop_aspect: String,
    crop_drag: Option<PreviewCropDragState>,
}

impl Default for PreviewUiState {
    fn default() -> Self {
        Self {
            crop_file_id: None,
            crop_mode: false,
            draft_crop: None,
            crop_aspect: "free".to_string(),
            crop_drag: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewCropDragState {
    handle: DragHandle,
    start_rect: CropRect,
    start_point: PreviewPoint,
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
