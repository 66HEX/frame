//! View selection, render snapshots and deterministic fixture contracts.

use crate::file_queue::FileQueue;
use crate::numeric::u64_to_f64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveView {
    Workspace,
    Logs,
}

#[must_use]
pub fn active_view_from_env_value(value: Option<&str>) -> ActiveView {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("logs") => ActiveView::Logs,
        _ => ActiveView::Workspace,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualFixture {
    AppSettings,
    AppSettingsThemeOpen,
    AppSettingsUiOpen,
    LogsActive,
    PreviewCrop,
    PreviewAnamorphicCrop,
    PreviewAnamorphicOverlay,
    PreviewCropOptions,
    PreviewCropCustom,
    SettingsVideoAv1,
    SettingsVideoProres,
    SettingsVideoVp9,
    SettingsVideoHardware,
    PreviewReady,
    PreviewTransport,
    PreviewTrim,
    SettingsAudio,
    SettingsAudioFilters,
    SettingsImages,
    SettingsMetadata,
    SettingsOutput,
    SettingsPresets,
    SettingsSource,
    SettingsSubtitles,
    SettingsSubtitlesPopover,
    SettingsVideo,
    SettingsVideoFilters,
    UpdateAvailable,
    WorkspaceAudio,
    WorkspaceEmpty,
    WorkspaceImage,
    WorkspaceLargeQueue,
}

#[must_use]
pub fn visual_fixture_from_env_value(value: Option<&str>) -> Option<VisualFixture> {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("app-settings") => Some(VisualFixture::AppSettings),
        Some("app-settings-theme-open") => Some(VisualFixture::AppSettingsThemeOpen),
        Some("app-settings-ui-open") => Some(VisualFixture::AppSettingsUiOpen),
        Some("logs-active") => Some(VisualFixture::LogsActive),
        Some("preview-crop-options") => Some(VisualFixture::PreviewCropOptions),
        Some("preview-crop-custom") => Some(VisualFixture::PreviewCropCustom),
        Some("settings-video-av1") => Some(VisualFixture::SettingsVideoAv1),
        Some("settings-video-prores") => Some(VisualFixture::SettingsVideoProres),
        Some("settings-video-vp9") => Some(VisualFixture::SettingsVideoVp9),
        Some("settings-video-hardware") => Some(VisualFixture::SettingsVideoHardware),
        Some("preview-anamorphic-crop") => Some(VisualFixture::PreviewAnamorphicCrop),
        Some("preview-anamorphic-overlay") => Some(VisualFixture::PreviewAnamorphicOverlay),
        Some("preview-crop") => Some(VisualFixture::PreviewCrop),
        Some("preview-ready") => Some(VisualFixture::PreviewReady),
        Some("preview-transport") => Some(VisualFixture::PreviewTransport),
        Some("preview-trim") => Some(VisualFixture::PreviewTrim),
        Some("settings-audio") => Some(VisualFixture::SettingsAudio),
        Some("settings-audio-filters") => Some(VisualFixture::SettingsAudioFilters),
        Some("settings-images") => Some(VisualFixture::SettingsImages),
        Some("settings-metadata") => Some(VisualFixture::SettingsMetadata),
        Some("settings-output") => Some(VisualFixture::SettingsOutput),
        Some("settings-presets") => Some(VisualFixture::SettingsPresets),
        Some("settings-source") => Some(VisualFixture::SettingsSource),
        Some("settings-subtitles") => Some(VisualFixture::SettingsSubtitles),
        Some("settings-subtitles-popover") => Some(VisualFixture::SettingsSubtitlesPopover),
        Some("settings-video") => Some(VisualFixture::SettingsVideo),
        Some("settings-video-filters") => Some(VisualFixture::SettingsVideoFilters),
        Some("update-available") => Some(VisualFixture::UpdateAvailable),
        Some("workspace-audio") => Some(VisualFixture::WorkspaceAudio),
        Some("workspace-empty") => Some(VisualFixture::WorkspaceEmpty),
        Some("workspace-image") => Some(VisualFixture::WorkspaceImage),
        Some("workspace-large-queue") => Some(VisualFixture::WorkspaceLargeQueue),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameAppState {
    pub active_view: ActiveView,
    pub is_processing: bool,
    pub file_count: usize,
    pub selected_count: usize,
    pub has_actionable_files: bool,
    pub has_default_output_directory: bool,
    pub total_size_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartAvailability {
    Ready,
    Processing,
    NoFiles,
    NoSelectedFiles,
    NoActionableFiles,
    MissingOutputDirectory,
}

impl StartAvailability {
    #[must_use]
    pub const fn button_label(self) -> &'static str {
        match self {
            Self::Ready => "Start",
            Self::Processing => "Processing",
            Self::NoFiles => "Add a source",
            Self::NoSelectedFiles => "Select files",
            Self::NoActionableFiles => "Nothing pending",
            Self::MissingOutputDirectory => "Choose output",
        }
    }

    #[must_use]
    pub const fn accessibility_label(self) -> &'static str {
        match self {
            Self::Ready => "Start conversion",
            Self::Processing => "Conversion in progress",
            Self::NoFiles => "Add a source to start a conversion",
            Self::NoSelectedFiles => "Select at least one file to start a conversion",
            Self::NoActionableFiles => "No selected files are ready to convert",
            Self::MissingOutputDirectory => "Choose an output folder before starting",
        }
    }

    #[must_use]
    pub const fn button_enabled(self) -> bool {
        matches!(self, Self::Ready | Self::MissingOutputDirectory)
    }
}

impl Default for FrameAppState {
    fn default() -> Self {
        Self {
            active_view: ActiveView::Workspace,
            is_processing: false,
            file_count: 0,
            selected_count: 0,
            has_actionable_files: false,
            has_default_output_directory: false,
            total_size_bytes: 0,
        }
    }
}

impl FrameAppState {
    #[must_use]
    pub const fn can_start_conversion(self) -> bool {
        matches!(self.start_availability(), StartAvailability::Ready)
    }

    #[must_use]
    pub const fn start_availability(self) -> StartAvailability {
        if self.is_processing {
            StartAvailability::Processing
        } else if self.file_count == 0 {
            StartAvailability::NoFiles
        } else if self.selected_count == 0 {
            StartAvailability::NoSelectedFiles
        } else if !self.has_actionable_files {
            StartAvailability::NoActionableFiles
        } else if !self.has_default_output_directory {
            StartAvailability::MissingOutputDirectory
        } else {
            StartAvailability::Ready
        }
    }

    #[must_use]
    pub fn from_file_queue(
        active_view: ActiveView,
        is_processing: bool,
        has_default_output_directory: bool,
        file_queue: &FileQueue,
    ) -> Self {
        Self {
            active_view,
            is_processing,
            file_count: file_queue.files().len(),
            selected_count: file_queue.selected_count(),
            has_actionable_files: file_queue.has_actionable_files(),
            has_default_output_directory,
            total_size_bytes: file_queue.total_size_bytes(),
        }
    }
}

#[must_use]
pub fn format_total_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 KB".to_string();
    }

    let mb = u64_to_f64(bytes) / (1024.0 * 1024.0);
    if mb > 1000.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{mb:.1} MB")
    }
}
