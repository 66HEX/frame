mod chrome;
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

use chrome::{app_settings_sheet, titlebar};
use input::{FrameTextInputKind, FrameTextInputRuntime};
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
    assets::{self},
    conversion_events::{ActiveLogFile, ConversionEventState, LogLine, all_conversions_settled},
    conversion_runner::{
        ConversionProcessController, conversion_task_from_file, run_conversion_batch_with_control,
    },
    file_queue::{
        BatchSelectionState, FileItem, FileQueue, FileStateTone, FileStatus, RowActionAvailability,
        format_file_size,
    },
    format_total_size,
    preview::{
        ASPECT_OPTIONS, CropRect, DragHandle, MediaSnapshot,
        MetadataStatus as PreviewMetadataStatus, Point as PreviewPoint, PreviewControlAvailability,
        PreviewControlInput, PreviewMediaKind, PreviewPlaybackState, PreviewRotation,
        SourceMediaKind, TimelineDragTarget, adjust_rect_to_ratio, aspect_value, clamp_rect,
        format_time, parse_time_to_seconds, preview_control_availability, transform_crop_rect,
    },
    settings::{
        ConversionConfig, CropSettings, ProcessingMode, SettingsTab, SourceInfoSection, SourceKind,
        SourceMetadata, apply_audio_bitrate, apply_audio_bitrate_mode, apply_audio_channels,
        apply_audio_codec, apply_audio_normalize, apply_audio_quality, apply_audio_volume,
        apply_output_container, apply_processing_mode, apply_trim_times, audio_channel_options,
        audio_codec_options, audio_codec_supports_vbr, audio_quality_range, audio_track_options,
        normalize_output_config, output_container_options, output_processing_mode_options,
        resolve_active_settings_tab, sanitize_output_name, source_info_sections,
        toggle_audio_track_selection, visible_settings_tabs,
    },
    source_metadata::{
        MetadataStatus, SourceMetadataEntry, SourceMetadataStore, probe_source_metadata,
    },
    theme, visual_fixture_from_env_value,
};
use frame_core::events::ConversionEvent;
use frame_core::types::DEFAULT_MAX_CONCURRENCY;
use gpui::{
    App, Bounds, BoxShadow, ClickEvent, ClipboardItem, Context, DragMoveEvent, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, ExternalPaths, FocusHandle, FontWeight,
    GlobalElementId, InteractiveElement, IntoElement, KeyBinding, LayoutId, Menu, MenuItem,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, PathPromptOptions,
    Pixels, Point, Render, Rgba, ShapedLine, SharedString, StatefulInteractiveElement, Style, Task,
    TextRun, TitlebarOptions, UTF16Selection, UniformListScrollHandle, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
    actions, div, fill, hsla, point, prelude::*, px, relative, size, svg, uniform_list,
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
    frame_gpui_ce,
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
const FILE_LIST_CHECKBOX_SIZE: f32 = 14.0;
const FILE_LIST_CHECK_ICON_SIZE: f32 = 12.0;
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
const DEFAULT_CROP_X: f64 = 0.1;
const DEFAULT_CROP_Y: f64 = 0.1;
const DEFAULT_CROP_SIZE: f64 = 0.8;
const CROP_HANDLE_SIZE: f32 = 10.0;
const FRAME_TEXT_INPUT_CONTEXT: &str = "FrameTextInput";
const TEXT_INPUT_CARET_WIDTH: f32 = 1.5;
const TEXT_INPUT_CARET_HEIGHT: f32 = 14.0;
const TEXT_INPUT_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const TEXT_INPUT_BLINK_PAUSE: Duration = Duration::from_millis(300);

pub struct FrameRoot {
    active_view: ActiveView,
    file_queue: FileQueue,
    conversion_events: ConversionEventState,
    logs_scroll_handle: UniformListScrollHandle,
    last_log_scroll_target: Option<LogScrollTarget>,
    is_processing: bool,
    is_settings_open: bool,
    settings_active_tab: SettingsTab,
    max_concurrency: usize,
    max_concurrency_draft: String,
    max_concurrency_error: Option<String>,
    app_settings_value_focus: Option<FocusHandle>,
    settings_output_name_focus: Option<FocusHandle>,
    settings_audio_bitrate_focus: Option<FocusHandle>,
    active_text_input: Option<FrameTextInputKind>,
    max_concurrency_input: FrameTextInputRuntime,
    output_name_input: FrameTextInputRuntime,
    audio_bitrate_input: FrameTextInputRuntime,
    text_input_cursor_visible: bool,
    text_input_cursor_paused: bool,
    text_input_cursor_epoch: usize,
    text_input_cursor_task: Task<()>,
    source_metadata: SourceMetadataStore,
    conversion_processes: ConversionProcessController,
    preview_crop_file_id: Option<String>,
    preview_crop_mode: bool,
    preview_draft_crop: Option<CropRect>,
    preview_crop_aspect: String,
    preview_crop_drag: Option<PreviewCropDragState>,
    native_titlebar_controls_hidden: bool,
    next_file_sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewCropDragState {
    handle: DragHandle,
    start_rect: CropRect,
    start_point: PreviewPoint,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LogScrollTarget {
    file_id: String,
    line_count: usize,
}
