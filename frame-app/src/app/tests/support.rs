//! Shared imports for internal application tests; absent from production builds.

pub(in crate::app) use crate::app::components::popover_state::PopoverState;
pub(in crate::app) use crate::app::input::FrameTextInputKind;
pub(in crate::app) use crate::app::inspector::state::{
    SettingsMetadataInputFocuses, SettingsRenderState,
};
pub(in crate::app) use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorDrag, SettingsSubtitleColorDragKind, SettingsSubtitleColorInputFocuses,
    SettingsSubtitleColorTarget, SettingsSubtitleFocuses, SettingsSubtitleHsv,
    SettingsSubtitleMode, SettingsSubtitlePopover,
};
#[cfg(target_os = "linux")]
pub(in crate::app) use crate::app::layout::LINUX_WINDOW_FRAME_INSET;
pub(in crate::app) use crate::app::layout::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
pub(in crate::app) use crate::app::model::{ActiveView, VisualFixture};
pub(in crate::app) use crate::app::motion::settings_drawer_right_inset;
pub(in crate::app) use crate::app::preferences::state::AppearancePopover;
pub(in crate::app) use crate::app::preview::state::{
    PREVIEW_CANVAS_VISUAL_SETTLE_EPSILON, PREVIEW_CANVAS_WHEEL_ZOOM_STEP,
    PREVIEW_DIMENSION_DEBOUNCE_INTERVAL, PREVIEW_FILTER_DEBOUNCE_INTERVAL, PreviewCanvasState,
    PreviewRuntimeDimensions, PreviewRuntimeKey, TRIM_PREVIEW_SEEK_EPSILON_SECONDS,
    TrimPreviewSeekRequest, TrimPreviewSeekState,
};
pub(in crate::app) use crate::app::preview::{FlipAxis, actions as preview_actions};
pub(in crate::app) use crate::app::updates::state::UpdateStatus;
pub(in crate::app) use crate::app_info::FRAME_APP_ID;
pub(in crate::app) use crate::appearance::{AppearanceSettings, ScalePreset};
pub(in crate::app) use crate::file_queue::{FileItem, FileStatus};
pub(in crate::app) use crate::preview::{
    CropRect, DragHandle, MediaSnapshot, MetadataStatus as PreviewMetadataStatus,
    OverlayDragHandle, OverlayDragPoint, Point as PreviewPoint, PreviewControlAvailability,
    PreviewMediaKind, PreviewOverlay, PreviewPlaybackState, TimelineDragTarget, format_time,
};
pub(in crate::app) use crate::preview_engine::{
    DEFAULT_PREVIEW_FPS, DEFAULT_PREVIEW_MAX_HEIGHT, DEFAULT_PREVIEW_MAX_WIDTH,
    PreviewRenderPresentation, PreviewSession, PreviewSessionConfig,
    PreviewSourceKind as EnginePreviewSourceKind, PreviewTransform,
};
pub(in crate::app) use crate::settings::{
    ConversionConfig, CropSettings, DEFAULT_SUBTITLE_FONT_COLOR, DEFAULT_SUBTITLE_OUTLINE_COLOR,
    OverlaySettings, PresetDefinition, ProcessingMode, SettingsTab, SourceInfoSection, SourceKind,
    SourceMetadata, SubtitleFontOption, SubtitleFontSizeOption, source_info_sections,
};
pub(in crate::app) use crate::source_metadata::MetadataStatus;
pub(in crate::app) use crate::theme;
pub(in crate::app) use frame_core::capabilities::{AvailableEncoders, AvailableFilters};
pub(in crate::app) use frame_core::events::ConversionEvent;
pub(in crate::app) use frame_core::types::DEFAULT_MAX_CONCURRENCY;
pub(in crate::app) use frame_updater::{UpdateChannel, UpdatePackage};
pub(in crate::app) use gpui::prelude::*;
pub(in crate::app) use gpui::{
    Bounds, FocusHandle, MouseButton, Pixels, Point, RenderImage, ScrollDelta, ScrollHandle, point,
    px, size,
};
pub(in crate::app) use std::sync::Arc;
