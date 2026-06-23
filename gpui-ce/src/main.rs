use frame_core::events::ConversionEvent;
use frame_gpui_ce::{
    ActiveView, CONTENT_PADDING, FILE_LIST_ROW_SPAN, FILE_ROW_HEIGHT, FrameAppState,
    LEFT_COLUMN_SPAN, LEFT_GRID_ROWS, PANEL_HEADER_HEIGHT, PREVIEW_PANEL_PADDING,
    PREVIEW_PLAYHEAD_HEIGHT, PREVIEW_ROW_SPAN, PREVIEW_TIMELINE_CONTROL_HEIGHT,
    PREVIEW_TIMELINE_HANDLE_WIDTH, PREVIEW_TIMELINE_TOP_MARGIN, PREVIEW_TOOLBAR_BUTTON_SIZE,
    PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET, PREVIEW_TRACK_HEIGHT, RIGHT_COLUMN_SPAN,
    SETTINGS_CONTROL_HEIGHT, SETTINGS_PANEL_PADDING, SETTINGS_TAB_BUTTON_SIZE,
    SETTINGS_TAB_ICON_SIZE, TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_BUTTON_HEIGHT,
    TITLEBAR_DIVIDER_HEIGHT, TITLEBAR_HEIGHT, TITLEBAR_ICON_BUTTON_SIZE, TITLEBAR_ICON_SIZE,
    TITLEBAR_LOGO_SIZE, TITLEBAR_NAV_BUTTON_HEIGHT, TITLEBAR_SEGMENT_HEIGHT, TITLEBAR_TOP_PADDING,
    TITLEBAR_TRAFFIC_LIGHT_SIZE, VisualFixture, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
    WORKSPACE_COLUMNS, WORKSPACE_GAP, active_view_from_env_value,
    assets::{self, FrameAssets},
    conversion_events::{ActiveLogFile, ConversionEventState, LogLine},
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
        ConversionConfig, CropSettings, SettingsTab, SourceInfoSection, SourceKind, SourceMetadata,
        apply_output_container, apply_processing_mode, apply_trim_times, normalize_output_config,
        output_container_options, output_processing_mode_options, resolve_active_settings_tab,
        source_info_sections, visible_settings_tabs,
    },
    source_metadata::{
        MetadataStatus, SourceMetadataEntry, SourceMetadataStore, probe_source_metadata,
    },
    theme, visual_fixture_from_env_value,
};
use gpui::{
    App, Bounds, BoxShadow, ClickEvent, Context, DragMoveEvent, ExternalPaths, FontWeight,
    InteractiveElement, IntoElement, KeyBinding, Menu, MenuItem, PathPromptOptions, Pixels, Render,
    Rgba, SharedString, StatefulInteractiveElement, TitlebarOptions, UniformListScrollHandle,
    Window, WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations,
    WindowOptions, actions, div, hsla, point, prelude::*, px, relative, size, svg, uniform_list,
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSView, NSWindowButton};
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::path::PathBuf;

actions!(frame_gpui_ce, [Quit]);

const FILE_LIST_ACTIONS_WIDTH: f32 = 64.0;
const FILE_LIST_ACTION_BUTTON_SIZE: f32 = 24.0;
const FILE_LIST_CHECKBOX_SIZE: f32 = 12.0;
const LOG_LINE_NUMBER_WIDTH: f32 = 32.0;
const LOG_LINE_HEIGHT: f32 = 24.0;
const TRAFFIC_LIGHT_GROUP: &str = "titlebar-traffic-lights";
const DEFAULT_CROP_X: f64 = 0.1;
const DEFAULT_CROP_Y: f64 = 0.1;
const DEFAULT_CROP_SIZE: f64 = 0.8;
const CROP_HANDLE_SIZE: f32 = 10.0;

struct FrameRoot {
    active_view: ActiveView,
    file_queue: FileQueue,
    conversion_events: ConversionEventState,
    logs_scroll_handle: UniformListScrollHandle,
    last_log_scroll_target: Option<LogScrollTarget>,
    is_processing: bool,
    is_settings_open: bool,
    settings_active_tab: SettingsTab,
    source_metadata: SourceMetadataStore,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LogScrollTarget {
    file_id: String,
    line_count: usize,
}

impl FrameRoot {
    fn new() -> Self {
        let mut root = Self {
            active_view: active_view_from_env_value(
                std::env::var("FRAME_GPUI_INITIAL_VIEW").ok().as_deref(),
            ),
            file_queue: FileQueue::new(),
            conversion_events: ConversionEventState::new(),
            logs_scroll_handle: UniformListScrollHandle::new(),
            last_log_scroll_target: None,
            is_processing: false,
            is_settings_open: false,
            settings_active_tab: SettingsTab::Source,
            source_metadata: SourceMetadataStore::default(),
            preview_crop_file_id: None,
            preview_crop_mode: false,
            preview_draft_crop: None,
            preview_crop_aspect: "free".to_string(),
            preview_crop_drag: None,
            native_titlebar_controls_hidden: false,
            next_file_sequence: 0,
        };

        root.apply_visual_fixture(visual_fixture_from_env_value(
            std::env::var("FRAME_GPUI_VISUAL_FIXTURE").ok().as_deref(),
        ));
        root
    }

    fn apply_visual_fixture(&mut self, fixture: Option<VisualFixture>) {
        match fixture {
            Some(VisualFixture::LogsActive) => self.apply_logs_active_fixture(),
            Some(VisualFixture::PreviewCrop) => self.apply_preview_crop_fixture(),
            Some(VisualFixture::PreviewReady) => self.apply_preview_ready_fixture(),
            None => {}
        }
    }

    fn apply_logs_active_fixture(&mut self) {
        self.active_view = ActiveView::Logs;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-video",
            "/tmp/source_render.mov",
            1_572_864_000,
        ));
        self.file_queue
            .update_status("fixture-video", FileStatus::Converting, 64);

        for line in [
            "ffmpeg version 7.1.1 Copyright (c) 2000-2025 the FFmpeg developers",
            "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'source_render.mov':",
            "Stream #0:0: Video: prores (HQ), yuv422p10le, 3840x2160, 24 fps",
            "Stream mapping:",
            "frame=  148 fps= 27 q=-0.0 size=   65536kB time=00:00:06.16 bitrate=87145.2kbits/s speed=1.12x",
            "frame=  296 fps= 28 q=-0.0 size=  131072kB time=00:00:12.33 bitrate=87042.7kbits/s speed=1.14x",
            "frame=  444 fps= 29 q=-0.0 size=  196608kB time=00:00:18.50 bitrate=87054.9kbits/s speed=1.16x",
        ] {
            self.conversion_events.apply_conversion_event(
                &mut self.file_queue,
                ConversionEvent::log("fixture-video", line),
            );
        }
    }

    fn apply_preview_ready_fixture(&mut self) {
        self.active_view = ActiveView::Workspace;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-preview",
            "/tmp/source_render.mov",
            1_572_864_000,
        ));
        self.source_metadata.mark_ready(
            "fixture-preview".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.400000".to_string()),
                bitrate: Some("12000000".to_string()),
                video_codec: Some("prores".to_string()),
                audio_codec: Some("aac".to_string()),
                resolution: Some("3840x2160".to_string()),
                frame_rate: Some(24.0),
                width: Some(3840),
                height: Some(2160),
                video_bitrate_kbps: Some(12_000.0),
                ..SourceMetadata::default()
            },
        );
    }

    fn apply_preview_crop_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.preview_crop_file_id = Some("fixture-preview".to_string());
        self.preview_crop_mode = true;
        self.preview_draft_crop = Some(CropRect {
            x: 0.18,
            y: 0.16,
            width: 0.64,
            height: 0.64,
        });
        self.preview_crop_aspect = "1:1".to_string();
    }

    fn app_state(&self) -> FrameAppState {
        FrameAppState::from_file_queue(self.active_view, self.is_processing, &self.file_queue)
    }

    fn prompt_add_source(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Add Source".into()),
        });

        cx.spawn(async move |this, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    eprintln!("Failed to open file picker: {error}");
                    return;
                }
            };
            if paths.is_empty() {
                return;
            }

            this.update(cx, |root, cx| root.import_source_paths(paths, cx))
                .ok();
        })
        .detach();
    }

    fn import_source_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let imports = self.allocate_file_imports(paths);
        if imports.is_empty() {
            return;
        }

        cx.spawn(async move |this, cx| {
            let files = cx
                .background_spawn(async move {
                    imports
                        .into_iter()
                        .map(|(id, path)| FileItem::from_os_path(id, &path))
                        .collect::<Vec<_>>()
                })
                .await;
            let probe_targets = files
                .iter()
                .map(|file| (file.id.clone(), file.path.clone()))
                .collect::<Vec<_>>();

            this.update(cx, |root, cx| {
                if root.file_queue.add_files(files) > 0 {
                    for (file_id, file_path) in probe_targets {
                        root.queue_source_metadata_probe(file_id, file_path, cx);
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn allocate_file_imports(&mut self, paths: Vec<PathBuf>) -> Vec<(String, PathBuf)> {
        paths
            .into_iter()
            .map(|path| {
                let id = self.next_file_id();
                (id, path)
            })
            .collect()
    }

    fn next_file_id(&mut self) -> String {
        self.next_file_sequence += 1;
        format!("file-{}", self.next_file_sequence)
    }

    fn selected_source_metadata_entry(&self) -> SourceMetadataEntry {
        self.source_metadata.selected_entry(&self.file_queue)
    }

    fn selected_source_metadata(&self) -> Option<SourceMetadata> {
        self.file_queue
            .selected_file_id()
            .and_then(|id| self.source_metadata.metadata_for(id))
            .cloned()
    }

    fn selected_config(&self) -> Option<&ConversionConfig> {
        self.file_queue.selected_file().map(|file| &file.config)
    }

    fn update_selected_config(
        &mut self,
        update: impl FnOnce(&mut ConversionConfig) -> bool,
    ) -> bool {
        self.file_queue
            .selected_file_mut()
            .is_some_and(|file| update(&mut file.config))
    }

    fn normalize_selected_config(&mut self, metadata: Option<&SourceMetadata>) -> bool {
        self.update_selected_config(|config| normalize_output_config(config, metadata))
    }

    fn sync_preview_crop_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        selected_config: &ConversionConfig,
    ) {
        if self.preview_crop_file_id.as_deref() != selected_file_id {
            self.preview_crop_file_id = selected_file_id.map(str::to_string);
            self.preview_crop_mode = false;
            self.preview_draft_crop = None;
            self.preview_crop_drag = None;
        }

        if !self.preview_crop_mode {
            self.preview_crop_aspect = selected_config
                .crop
                .as_ref()
                .and_then(|crop| crop.aspect_ratio.clone())
                .unwrap_or_else(|| "free".to_string());
            self.preview_draft_crop = None;
            self.preview_crop_drag = None;
        }
    }

    fn preview_crop_render_state(
        &self,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
    ) -> PreviewCropRenderState {
        PreviewCropRenderState {
            crop_mode: self.preview_crop_mode,
            draft_crop: self.preview_draft_crop,
            applied_crop: crop_rect_from_settings(config.crop.as_ref(), config),
            crop_aspect: self.preview_crop_aspect.clone(),
            has_crop_dimensions: preview_crop_source_dimensions(metadata, &config.rotation)
                .is_some(),
            rotation: config.rotation.clone(),
            flip_horizontal: config.flip_horizontal,
            flip_vertical: config.flip_vertical,
        }
    }

    fn toggle_selected_crop_mode(&mut self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if !preview_crop_controls_enabled(
            metadata.as_ref(),
            config,
            self.file_queue.selected_file_locked(),
        ) {
            return false;
        }

        let applied_crop = crop_rect_from_settings(config.crop.as_ref(), config);
        let crop_aspect = config
            .crop
            .as_ref()
            .and_then(|crop| crop.aspect_ratio.clone())
            .unwrap_or_else(|| "free".to_string());

        if self.preview_crop_mode {
            self.preview_crop_mode = false;
            self.preview_draft_crop = None;
            self.preview_crop_drag = None;
            return true;
        }

        self.preview_crop_mode = true;
        self.preview_draft_crop = Some(applied_crop.unwrap_or_else(default_crop_rect));
        self.preview_crop_aspect = crop_aspect;
        true
    }

    fn select_preview_crop_aspect(&mut self, aspect_id: &str) -> bool {
        if !self.preview_crop_mode || !is_known_crop_aspect(aspect_id) {
            return false;
        }

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let Some(dimensions) = preview_crop_source_dimensions(metadata.as_ref(), &config.rotation)
        else {
            return false;
        };
        let is_side_rotation = is_side_rotation(&config.rotation);

        let previous_aspect = self.preview_crop_aspect.clone();
        let previous_rect = self.preview_draft_crop;
        self.preview_crop_aspect = aspect_id.to_string();
        if let Some(rect) = self.preview_draft_crop {
            self.preview_draft_crop = Some(if let Some(ratio) = aspect_value(aspect_id) {
                clamp_rect(adjust_rect_to_ratio(
                    rect,
                    ratio,
                    f64::from(dimensions.width),
                    f64::from(dimensions.height),
                    is_side_rotation,
                ))
            } else {
                clamp_rect(rect)
            });
        }

        previous_aspect != self.preview_crop_aspect || previous_rect != self.preview_draft_crop
    }

    fn reset_preview_crop_selection(&mut self) -> bool {
        if !self.preview_crop_mode {
            return false;
        }

        let previous_rect = self.preview_draft_crop;
        let previous_aspect = self.preview_crop_aspect.clone();
        self.preview_draft_crop = Some(if self.preview_draft_crop.is_some() {
            full_crop_rect()
        } else {
            default_crop_rect()
        });
        self.preview_crop_aspect = "free".to_string();
        previous_rect != self.preview_draft_crop || previous_aspect != self.preview_crop_aspect
    }

    fn apply_selected_crop(&mut self) -> bool {
        if !self.preview_crop_mode {
            return false;
        }
        let Some(draft_crop) = self.preview_draft_crop else {
            return false;
        };

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if preview_crop_source_dimensions(metadata.as_ref(), &config.rotation).is_none() {
            return false;
        }

        let next_crop = if crop_rect_is_full(draft_crop) {
            None
        } else {
            crop_settings_from_rect(
                draft_crop,
                &self.preview_crop_aspect,
                &config.rotation,
                config.flip_horizontal,
                config.flip_vertical,
                metadata.as_ref(),
            )
        };
        let cleared_crop = next_crop.is_none();

        let changed = self.update_selected_config(|config| {
            let changed = config.crop != next_crop;
            config.crop = next_crop;
            changed
        });
        self.preview_crop_mode = false;
        self.preview_draft_crop = None;
        self.preview_crop_drag = None;
        if cleared_crop {
            self.preview_crop_aspect = "free".to_string();
        }
        changed
    }

    fn rotate_selected_preview(&mut self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if !preview_transform_controls_enabled(
            metadata.as_ref(),
            config,
            self.file_queue.selected_file_locked(),
        ) {
            return false;
        }

        let next_rotation = next_rotation(&config.rotation);
        let applied_crop = crop_rect_from_settings(config.crop.as_ref(), config);
        let aspect_id = crop_aspect_id(config.crop.as_ref()).to_string();
        let flip_horizontal = config.flip_horizontal;
        let flip_vertical = config.flip_vertical;
        let next_crop = applied_crop.and_then(|rect| {
            crop_settings_from_rect(
                rect,
                &aspect_id,
                &next_rotation,
                flip_horizontal,
                flip_vertical,
                metadata.as_ref(),
            )
        });

        self.update_selected_config(|config| {
            let changed = config.rotation != next_rotation
                || (applied_crop.is_some() && config.crop != next_crop);
            config.rotation = next_rotation;
            if applied_crop.is_some() {
                config.crop = next_crop;
            }
            changed
        })
    }

    fn toggle_selected_flip(&mut self, axis: FlipAxis) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        if !preview_transform_controls_enabled(
            metadata.as_ref(),
            config,
            self.file_queue.selected_file_locked(),
        ) {
            return false;
        }

        let next_flip_horizontal = if axis == FlipAxis::Horizontal {
            !config.flip_horizontal
        } else {
            config.flip_horizontal
        };
        let next_flip_vertical = if axis == FlipAxis::Vertical {
            !config.flip_vertical
        } else {
            config.flip_vertical
        };
        let applied_crop = crop_rect_from_settings(config.crop.as_ref(), config);
        let aspect_id = crop_aspect_id(config.crop.as_ref()).to_string();
        let rotation = config.rotation.clone();
        let next_crop = applied_crop.and_then(|rect| {
            crop_settings_from_rect(
                rect,
                &aspect_id,
                &rotation,
                next_flip_horizontal,
                next_flip_vertical,
                metadata.as_ref(),
            )
        });

        self.update_selected_config(|config| {
            let changed = config.flip_horizontal != next_flip_horizontal
                || config.flip_vertical != next_flip_vertical
                || (applied_crop.is_some() && config.crop != next_crop);
            config.flip_horizontal = next_flip_horizontal;
            config.flip_vertical = next_flip_vertical;
            if applied_crop.is_some() {
                config.crop = next_crop;
            }
            changed
        })
    }

    fn apply_preview_crop_drag(&mut self, handle: DragHandle, point: PreviewPoint) -> bool {
        if !self.preview_crop_mode {
            return false;
        }
        let Some(current_rect) = self.preview_draft_crop else {
            return false;
        };

        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let Some(dimensions) = preview_crop_source_dimensions(metadata.as_ref(), &config.rotation)
        else {
            return false;
        };
        let is_side_rotation = is_side_rotation(&config.rotation);

        let drag_state = match self.preview_crop_drag {
            Some(state) if state.handle == handle => state,
            _ => {
                let state = PreviewCropDragState {
                    handle,
                    start_rect: current_rect,
                    start_point: point,
                };
                self.preview_crop_drag = Some(state);
                state
            }
        };

        let next_rect = frame_gpui_ce::preview::apply_visual_crop_drag(
            frame_gpui_ce::preview::VisualCropDrag {
                start_rect: drag_state.start_rect,
                handle,
                start_point: drag_state.start_point,
                current_point: point,
                aspect_id: &self.preview_crop_aspect,
                source_width: f64::from(dimensions.width),
                source_height: f64::from(dimensions.height),
                is_side_rotation,
            },
        );
        let changed = self.preview_draft_crop != Some(next_rect);
        self.preview_draft_crop = Some(next_rect);
        changed
    }

    fn end_preview_crop_drag(&mut self) -> bool {
        let had_drag = self.preview_crop_drag.is_some();
        self.preview_crop_drag = None;
        had_drag
    }

    fn apply_selected_trim_drag(&mut self, target: TimelineDragTarget, percent: f64) -> bool {
        if target == TimelineDragTarget::Scrub {
            return false;
        }

        let metadata = self.selected_source_metadata();
        let duration_seconds = preview_duration_seconds(metadata.as_ref());
        if duration_seconds <= 0.0 {
            return false;
        }

        let Some(config) = self.selected_config() else {
            return false;
        };
        let metadata_status = if metadata.is_some() {
            PreviewMetadataStatus::Ready
        } else {
            PreviewMetadataStatus::Idle
        };
        let availability = preview_control_availability(PreviewControlInput {
            metadata_status,
            source_media_kind: preview_source_media_kind(metadata.as_ref()),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        });
        if availability.trim_disabled {
            return false;
        }

        let mut playback = preview_playback_state(
            availability.media_kind,
            duration_seconds,
            config.start_time.as_deref(),
            config.end_time.as_deref(),
        );
        if !playback.begin_handle_drag(target) {
            return false;
        }

        let Some(trim) = playback.drag_to_percent(percent).trim else {
            return false;
        };

        self.update_selected_config(|config| {
            apply_trim_times(config, trim.start_time, trim.end_time)
        })
    }

    fn resolve_selected_settings_tab(&mut self, metadata: Option<&SourceMetadata>) {
        let next_tab = self
            .selected_config()
            .map_or(SettingsTab::Source, |config| {
                resolve_active_settings_tab(self.settings_active_tab, config, metadata)
            });
        self.settings_active_tab = next_tab;
    }

    fn queue_source_metadata_probe(
        &mut self,
        file_id: String,
        file_path: String,
        cx: &mut Context<Self>,
    ) {
        self.source_metadata.mark_loading(file_id.clone());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { probe_source_metadata(&file_path) })
                .await;

            this.update(cx, |root, cx| {
                match result {
                    Ok(metadata) => {
                        root.source_metadata.mark_ready(file_id.clone(), metadata);
                        if root.file_queue.selected_file_id() == Some(file_id.as_str()) {
                            let selected_metadata = root.selected_source_metadata();
                            root.normalize_selected_config(selected_metadata.as_ref());
                            root.resolve_selected_settings_tab(selected_metadata.as_ref());
                        }
                    }
                    Err(error) => {
                        root.source_metadata
                            .mark_error(file_id.clone(), error.to_string());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn update_log_scroll_target(&mut self) {
        if self.active_view != ActiveView::Logs {
            return;
        }

        let Some(file_id) = self.conversion_events.selected_log_file_id() else {
            self.last_log_scroll_target = None;
            return;
        };

        let target = LogScrollTarget {
            file_id: file_id.to_string(),
            line_count: self.conversion_events.logs_for(file_id).len(),
        };
        if target.line_count == 0 {
            self.last_log_scroll_target = Some(target);
            return;
        }

        if self.last_log_scroll_target.as_ref() != Some(&target) {
            self.logs_scroll_handle.scroll_to_bottom();
            self.last_log_scroll_target = Some(target);
        }
    }
}

impl Render for FrameRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.native_titlebar_controls_hidden {
            self.native_titlebar_controls_hidden = hide_native_macos_titlebar_controls(window);
        }

        let state = self.app_state();
        let source_metadata_entry = self.selected_source_metadata_entry();
        let source_metadata = source_metadata_entry.metadata.clone();
        self.normalize_selected_config(source_metadata.as_ref());
        self.resolve_selected_settings_tab(source_metadata.as_ref());
        self.conversion_events
            .ensure_selected_log_file(&self.file_queue);
        self.update_log_scroll_target();
        let selected_file_id = self.file_queue.selected_file_id().map(str::to_string);
        let selected_file = self.file_queue.selected_file();
        let selected_config_snapshot =
            selected_file.map_or_else(ConversionConfig::default, |file| file.config.clone());
        let selected_output_name =
            selected_file.map_or_else(String::new, |file| file.output_name.clone());
        self.sync_preview_crop_for_selection(
            selected_file_id.as_deref(),
            &selected_config_snapshot,
        );
        let preview_crop =
            self.preview_crop_render_state(source_metadata.as_ref(), &selected_config_snapshot);
        let content = div().flex_1().p(px(CONTENT_PADDING));
        let content = match state.active_view {
            ActiveView::Workspace => content.child(workspace_view(
                &self.file_queue,
                SettingsRenderState {
                    active_tab: self.settings_active_tab,
                    config: &selected_config_snapshot,
                    metadata: source_metadata.as_ref(),
                    metadata_status: source_metadata_entry.status,
                    metadata_error: source_metadata_entry.error.as_deref(),
                    settings_disabled: self.file_queue.selected_file_locked(),
                    output_name: &selected_output_name,
                },
                preview_crop,
                cx,
            )),
            ActiveView::Logs => content.child(logs_view(
                &self.file_queue,
                &self.conversion_events,
                &self.logs_scroll_handle,
                cx,
            )),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(color(theme::BACKGROUND))
            .text_color(color(theme::FOREGROUND))
            .font_family(assets::FRAME_FONT_FAMILY)
            .font_weight(FontWeight::SEMIBOLD)
            .on_drop(cx.listener(|root, paths: &ExternalPaths, _window, cx| {
                cx.stop_propagation();
                root.import_source_paths(paths.paths().to_vec(), cx);
            }))
            .child(titlebar(state, cx))
            .child(content)
    }
}

fn titlebar(state: FrameAppState, cx: &mut Context<FrameRoot>) -> impl IntoElement {
    div()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .pt(px(TITLEBAR_TOP_PADDING))
        .window_control_area(WindowControlArea::Drag)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_6()
                .child(macos_window_controls(cx))
                .child(frame_logo())
                .child(titlebar_divider())
                .child(titlebar_navigation(state.active_view, cx))
                .child(titlebar_divider())
                .child(titlebar_stats(state)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_2()
                .child(
                    action_button(assets::ICON_SETTINGS, None, ButtonVariant::Secondary, true)
                        .id("titlebar-settings")
                        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                            root.is_settings_open = !root.is_settings_open;
                            cx.notify();
                        })),
                )
                .child(
                    action_button(
                        assets::ICON_PLUS,
                        Some("ADD SOURCE"),
                        ButtonVariant::Secondary,
                        true,
                    )
                    .id("titlebar-add-source")
                    .on_click(cx.listener(
                        |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            root.prompt_add_source(cx);
                        },
                    )),
                )
                .child(action_button(
                    assets::ICON_PLAY,
                    Some(if state.is_processing {
                        "PROCESSING"
                    } else {
                        "START"
                    }),
                    ButtonVariant::Default,
                    state.can_start_conversion(),
                )),
        )
}

fn macos_window_controls(cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .mr_2()
        .group(TRAFFIC_LIGHT_GROUP)
        .child(
            traffic_light(
                assets::ICON_TRAFFIC_CLOSE_DOT,
                assets::ICON_TRAFFIC_CLOSE_SYMBOL,
            )
            .id("titlebar-close")
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
        .child(
            traffic_light(
                assets::ICON_TRAFFIC_MINIMIZE_DOT,
                assets::ICON_TRAFFIC_MINIMIZE_SYMBOL,
            )
            .id("titlebar-minimize")
            .window_control_area(WindowControlArea::Min)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            })),
        )
        .child(
            traffic_light(
                assets::ICON_TRAFFIC_ZOOM_DOT,
                assets::ICON_TRAFFIC_ZOOM_SYMBOL,
            )
            .id("titlebar-zoom")
            .window_control_area(WindowControlArea::Max)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            })),
        )
}

fn traffic_light(dot_icon: &'static str, symbol_icon: &'static str) -> gpui::Div {
    div()
        .w(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .child(
            svg()
                .path(dot_icon)
                .absolute()
                .inset_0()
                .w(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
                .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
                .text_color(color(theme::FOREGROUND)),
        )
        .child(
            svg()
                .path(symbol_icon)
                .absolute()
                .inset_0()
                .w(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
                .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
                .opacity(0.0)
                .group_hover(TRAFFIC_LIGHT_GROUP, |style| style.opacity(1.0))
                .text_color(color(theme::FOREGROUND)),
        )
}

fn frame_logo() -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(
            svg()
                .path(assets::ICON_FRAME)
                .w(px(TITLEBAR_LOGO_SIZE))
                .h(px(TITLEBAR_LOGO_SIZE))
                .text_color(color(theme::FRAME_GRAY_600)),
        )
}

fn titlebar_divider() -> gpui::Div {
    vertical_separator(TITLEBAR_DIVIDER_HEIGHT)
}

fn titlebar_navigation(active_view: ActiveView, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .h(px(TITLEBAR_SEGMENT_HEIGHT))
        .flex()
        .items_center()
        .gap_1()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::FRAME_GRAY_100))
        .px(px(3.0))
        .py(px(2.0))
        .shadow(input_highlight_shadows())
        .child(titlebar_segment(
            assets::ICON_LAYOUT_LIST,
            "WORKSPACE",
            ActiveView::Workspace,
            active_view == ActiveView::Workspace,
            cx,
        ))
        .child(titlebar_segment(
            assets::ICON_TERMINAL,
            "LOGS",
            ActiveView::Logs,
            active_view == ActiveView::Logs,
            cx,
        ))
}

fn titlebar_stats(state: FrameAppState) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(titlebar_stat(
            assets::ICON_HARD_DRIVE,
            format!("STORAGE {}", format_total_size(state.total_size_bytes)),
        ))
        .child(titlebar_stat(
            assets::ICON_FILE_VIDEO,
            format!("ITEMS {}", state.file_count),
        ))
}

fn titlebar_stat(icon: &'static str, label: String) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(icon_svg(
            icon,
            TITLEBAR_ICON_SIZE,
            color(theme::FRAME_GRAY_600),
        ))
        .child(label)
}

fn titlebar_segment(
    icon: &'static str,
    label: &'static str,
    view: ActiveView,
    selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let colors = button_colors(ButtonVariant::Secondary, selected, true);
    div()
        .h(px(TITLEBAR_NAV_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .id(match view {
            ActiveView::Workspace => "titlebar-workspace",
            ActiveView::Logs => "titlebar-logs",
        })
        .px_2()
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .text_color(if selected {
            color(theme::FOREGROUND)
        } else {
            color(theme::FRAME_GRAY_600)
        })
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .hover(move |style| {
            let style = style.text_color(color(theme::FOREGROUND)).cursor_pointer();
            if selected {
                style.bg(color(colors.hover_background))
            } else {
                style
            }
        })
        .active(move |style| style.bg(color(colors.active_background)))
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root.active_view != view {
                root.active_view = view;
                cx.notify();
            }
            cx.stop_propagation();
        }))
        .child(icon_svg(
            icon,
            TITLEBAR_ICON_SIZE,
            if selected {
                color(theme::FOREGROUND)
            } else {
                color(theme::FRAME_GRAY_600)
            },
        ))
        .child(label)
}

#[derive(Clone, Copy)]
enum ButtonVariant {
    Default,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonColors {
    background: theme::RgbaToken,
    hover_background: theme::RgbaToken,
    active_background: theme::RgbaToken,
    foreground: theme::RgbaToken,
    hover_foreground: theme::RgbaToken,
    opacity: f32,
}

fn button_colors(variant: ButtonVariant, selected: bool, enabled: bool) -> ButtonColors {
    let active_variant = matches!(variant, ButtonVariant::Default) || selected;
    if !enabled {
        let (background, foreground, opacity) = if active_variant {
            (
                theme::FRAME_GRAY_400.with_alpha(0.10),
                theme::FOREGROUND.with_alpha(0.50),
                1.0,
            )
        } else if matches!(variant, ButtonVariant::Ghost) {
            (theme::TRANSPARENT, theme::FRAME_GRAY_600, 0.5)
        } else {
            (
                theme::FRAME_GRAY_100,
                theme::FOREGROUND.with_alpha(0.50),
                0.5,
            )
        };
        return ButtonColors {
            background,
            hover_background: background,
            active_background: background,
            foreground,
            hover_foreground: foreground,
            opacity,
        };
    }

    if active_variant {
        ButtonColors {
            background: theme::FRAME_GRAY_400,
            hover_background: theme::FRAME_GRAY_400.with_alpha(0.18),
            active_background: theme::FRAME_GRAY_400.with_alpha(0.18),
            foreground: theme::FOREGROUND,
            hover_foreground: theme::FOREGROUND,
            opacity: 1.0,
        }
    } else if matches!(variant, ButtonVariant::Ghost) {
        ButtonColors {
            background: theme::TRANSPARENT,
            hover_background: theme::FRAME_GRAY_100,
            active_background: theme::FRAME_GRAY_100,
            foreground: theme::FRAME_GRAY_600,
            hover_foreground: theme::FOREGROUND,
            opacity: 1.0,
        }
    } else {
        ButtonColors {
            background: theme::FRAME_GRAY_100,
            hover_background: theme::FRAME_GRAY_200,
            active_background: theme::FRAME_GRAY_200,
            foreground: theme::FOREGROUND,
            hover_foreground: theme::FOREGROUND,
            opacity: 1.0,
        }
    }
}

fn action_button(
    icon: &'static str,
    label: Option<&'static str>,
    variant: ButtonVariant,
    enabled: bool,
) -> gpui::Div {
    let is_icon_only = label.is_none();
    let colors = button_colors(variant, false, enabled);
    let button_icon_color = color(colors.foreground);

    let button = div()
        .h(px(TITLEBAR_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(colors.background))
        .shadow(button_highlight_shadows())
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
        })
        .when(!enabled, |this| this.cursor_not_allowed());

    if is_icon_only {
        button.w(px(TITLEBAR_ICON_BUTTON_SIZE)).child(icon_svg(
            icon,
            TITLEBAR_ACTION_ICON_SIZE,
            button_icon_color,
        ))
    } else {
        button
            .px(px(10.0))
            .child(icon_svg(icon, TITLEBAR_ICON_SIZE, button_icon_color))
            .child(label.unwrap_or_default())
    }
}

fn icon_svg(path: &'static str, size: f32, icon_color: Rgba) -> impl IntoElement {
    svg()
        .path(path)
        .w(px(size))
        .h(px(size))
        .text_color(icon_color)
}

fn parse_hex(hex: &str) -> Rgba {
    let hex = hex.trim_start_matches('#');
    let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    color(theme::RgbaToken::from_rgb(red, green, blue))
}

fn input_highlight_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.20),
            offset: point(px(0.0), px(0.5)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_400).into(),
            offset: point(px(0.0), px(-0.5)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
    ]
}

fn button_highlight_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: color(theme::FRAME_GRAY_400).into(),
            offset: point(px(0.0), px(0.5)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_200).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.5),
            inset: true,
        },
    ]
}

fn workspace_view(
    file_queue: &FileQueue,
    settings: SettingsRenderState<'_>,
    preview_crop: PreviewCropRenderState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(WORKSPACE_COLUMNS)
        .gap(px(WORKSPACE_GAP))
        .size_full()
        .child(
            div()
                .col_span(LEFT_COLUMN_SPAN)
                .grid()
                .grid_rows(LEFT_GRID_ROWS)
                .gap(px(WORKSPACE_GAP))
                .size_full()
                .child(
                    preview_panel(file_queue, settings, preview_crop, cx)
                        .row_span(PREVIEW_ROW_SPAN),
                )
                .child(file_list_panel(file_queue, cx).row_span(FILE_LIST_ROW_SPAN)),
        )
        .child(settings_panel(settings, cx).col_span(RIGHT_COLUMN_SPAN))
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewCropRenderState {
    crop_mode: bool,
    draft_crop: Option<CropRect>,
    applied_crop: Option<CropRect>,
    crop_aspect: String,
    has_crop_dimensions: bool,
    rotation: String,
    flip_horizontal: bool,
    flip_vertical: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewShellState {
    selected_file_name: Option<String>,
    metadata_status: PreviewMetadataStatus,
    metadata_error: Option<String>,
    controls_disabled: bool,
    availability: PreviewControlAvailability,
    playback: PreviewPlaybackState,
    duration_seconds: f64,
    crop: PreviewCropRenderState,
}

fn preview_panel(
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

fn preview_shell_state(
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

fn preview_metadata_status(status: MetadataStatus) -> PreviewMetadataStatus {
    match status {
        MetadataStatus::Idle => PreviewMetadataStatus::Idle,
        MetadataStatus::Loading => PreviewMetadataStatus::Loading,
        MetadataStatus::Ready => PreviewMetadataStatus::Ready,
        MetadataStatus::Error => PreviewMetadataStatus::Error,
    }
}

fn preview_source_media_kind(metadata: Option<&SourceMetadata>) -> Option<SourceMediaKind> {
    metadata.map(|metadata| match metadata.source_kind() {
        SourceKind::Video => SourceMediaKind::Video,
        SourceKind::Audio => SourceMediaKind::Audio,
        SourceKind::Image => SourceMediaKind::Image,
    })
}

fn preview_duration_seconds(metadata: Option<&SourceMetadata>) -> f64 {
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

fn preview_playback_state(
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CropSourceDimensions {
    width: u32,
    height: u32,
}

fn preview_transform_controls_enabled(
    metadata: Option<&SourceMetadata>,
    config: &ConversionConfig,
    controls_disabled: bool,
) -> bool {
    let metadata_status = if metadata.is_some() {
        PreviewMetadataStatus::Ready
    } else {
        PreviewMetadataStatus::Idle
    };
    let availability = preview_control_availability(PreviewControlInput {
        metadata_status,
        source_media_kind: preview_source_media_kind(metadata),
        controls_disabled,
        processing_mode: config.processing_mode,
        container: Some(config.container.as_str()),
    });

    availability.media_kind != PreviewMediaKind::Unknown
        && !availability.hide_visual_controls
        && !controls_disabled
}

fn preview_crop_controls_enabled(
    metadata: Option<&SourceMetadata>,
    config: &ConversionConfig,
    controls_disabled: bool,
) -> bool {
    preview_transform_controls_enabled(metadata, config, controls_disabled)
        && preview_crop_source_dimensions(metadata, &config.rotation).is_some()
}

fn preview_crop_source_dimensions(
    metadata: Option<&SourceMetadata>,
    _rotation: &str,
) -> Option<CropSourceDimensions> {
    let metadata = metadata?;
    let (Some(width), Some(height)) = (metadata.width, metadata.height) else {
        return None;
    };
    if width == 0 || height == 0 {
        return None;
    }

    Some(CropSourceDimensions { width, height })
}

fn crop_base_dimensions(
    metadata: Option<&SourceMetadata>,
    rotation: &str,
) -> Option<CropSourceDimensions> {
    let dimensions = preview_crop_source_dimensions(metadata, rotation)?;
    if is_side_rotation(rotation) {
        Some(CropSourceDimensions {
            width: dimensions.height,
            height: dimensions.width,
        })
    } else {
        Some(dimensions)
    }
}

fn crop_rect_from_settings(
    crop: Option<&CropSettings>,
    config: &ConversionConfig,
) -> Option<CropRect> {
    let crop = crop.filter(|crop| crop.enabled)?;
    let (Some(source_width), Some(source_height)) = (crop.source_width, crop.source_height) else {
        return None;
    };
    if source_width == 0 || source_height == 0 {
        return None;
    }

    let raw_rect = CropRect {
        x: f64::from(crop.x) / f64::from(source_width),
        y: f64::from(crop.y) / f64::from(source_height),
        width: f64::from(crop.width) / f64::from(source_width),
        height: f64::from(crop.height) / f64::from(source_height),
    };

    Some(clamp_rect(transform_crop_rect(
        raw_rect,
        PreviewRotation::from(config.rotation.as_str()),
        config.flip_horizontal,
        config.flip_vertical,
        true,
    )))
}

fn crop_settings_from_rect(
    rect: CropRect,
    aspect_id: &str,
    rotation: &str,
    flip_horizontal: bool,
    flip_vertical: bool,
    metadata: Option<&SourceMetadata>,
) -> Option<CropSettings> {
    let dimensions = crop_base_dimensions(metadata, rotation)?;
    let output_rect = clamp_rect(transform_crop_rect(
        rect,
        PreviewRotation::from(rotation),
        flip_horizontal,
        flip_vertical,
        false,
    ));

    Some(CropSettings {
        enabled: true,
        x: round_unit_to_u32(output_rect.x, dimensions.width),
        y: round_unit_to_u32(output_rect.y, dimensions.height),
        width: round_unit_to_u32(output_rect.width, dimensions.width),
        height: round_unit_to_u32(output_rect.height, dimensions.height),
        source_width: Some(dimensions.width),
        source_height: Some(dimensions.height),
        aspect_ratio: (aspect_id != "free").then(|| aspect_id.to_string()),
    })
}

fn round_unit_to_u32(value: f64, scale: u32) -> u32 {
    let scaled = (value * f64::from(scale)).round();
    if scaled <= 0.0 || !scaled.is_finite() {
        0
    } else if scaled >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        scaled as u32
    }
}

fn default_crop_rect() -> CropRect {
    CropRect {
        x: DEFAULT_CROP_X,
        y: DEFAULT_CROP_Y,
        width: DEFAULT_CROP_SIZE,
        height: DEFAULT_CROP_SIZE,
    }
}

fn full_crop_rect() -> CropRect {
    CropRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    }
}

fn crop_rect_is_full(rect: CropRect) -> bool {
    rect.x <= 0.001 && rect.y <= 0.001 && rect.width >= 0.999 && rect.height >= 0.999
}

fn crop_aspect_id(crop: Option<&CropSettings>) -> &str {
    crop.and_then(|crop| crop.aspect_ratio.as_deref())
        .unwrap_or("free")
}

fn is_known_crop_aspect(aspect_id: &str) -> bool {
    ASPECT_OPTIONS.iter().any(|option| option.id == aspect_id)
}

fn next_rotation(rotation: &str) -> String {
    match rotation {
        "0" => "90",
        "90" => "180",
        "180" => "270",
        _ => "0",
    }
    .to_string()
}

fn is_side_rotation(rotation: &str) -> bool {
    matches!(rotation, "90" | "270")
}

fn normalized_point_from_bounds(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
) -> PreviewPoint {
    let width = bounds.size.width.as_f32();
    let height = bounds.size.height.as_f32();
    if width <= 0.0 || height <= 0.0 {
        return PreviewPoint { x: 0.0, y: 0.0 };
    }

    let x = ((position.x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0);
    let y = ((position.y - bounds.origin.y).as_f32() / height).clamp(0.0, 1.0);
    PreviewPoint {
        x: f64::from(x),
        y: f64::from(y),
    }
}

fn preview_viewport(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let mut viewport = div()
        .id("preview-viewport")
        .relative()
        .flex_1()
        .min_h_0()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex("#000000"))
        .shadow(input_highlight_shadows())
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewCropDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let point = normalized_point_from_bounds(event.event.position, event.bounds);
                if root.apply_preview_crop_drag(drag.handle, point) {
                    cx.notify();
                }
            },
        ))
        .capture_any_mouse_up(cx.listener(|root, _, _window, cx| {
            if root.end_preview_crop_drag() {
                cx.notify();
            }
        }))
        .child(preview_viewport_content(state));

    if state.crop.crop_mode && state.crop.draft_crop.is_some() {
        viewport = viewport
            .child(preview_crop_overlay(state))
            .child(preview_crop_aspect_bar(state, cx));
    }

    if preview_visual_controls_visible(state) {
        viewport = viewport
            .child(preview_toolbar(state, cx))
            .child(preview_zoom_toolbar(state));
    }

    viewport
}

fn preview_viewport_content(state: &PreviewShellState) -> gpui::Div {
    let content = div()
        .max_w(px(360.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_center()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600));

    let Some(file_name) = state.selected_file_name.as_deref() else {
        return content.child("Drop files or use Add Source");
    };

    match state.metadata_status {
        PreviewMetadataStatus::Idle | PreviewMetadataStatus::Loading => {
            content.child("Analyzing source...")
        }
        PreviewMetadataStatus::Error => {
            let mut error = content
                .text_color(color(theme::FRAME_RED))
                .child("Preview unavailable");
            if let Some(message) = state.metadata_error.as_deref() {
                error = error.child(
                    div()
                        .max_w(px(320.0))
                        .truncate()
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(message.to_string()),
                );
            }
            error
        }
        PreviewMetadataStatus::Ready => {
            if state.availability.media_kind == PreviewMediaKind::Unknown {
                return content.child("Preview unavailable");
            }

            content
                .child(preview_media_placeholder(state.availability.media_kind))
                .child(
                    div()
                        .max_w(px(320.0))
                        .truncate()
                        .whitespace_nowrap()
                        .text_color(color(theme::FOREGROUND))
                        .child(file_name.to_string()),
                )
                .child(preview_media_kind_label(state.availability.media_kind))
        }
    }
}

fn preview_media_placeholder(media_kind: PreviewMediaKind) -> gpui::Div {
    div()
        .w(px(240.0))
        .h(px(136.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_MD))
        .border_1()
        .border_color(color(theme::FRAME_GRAY_200))
        .bg(color(theme::BACKGROUND))
        .shadow(input_highlight_shadows())
        .child(icon_svg(
            preview_media_icon(media_kind),
            32.0,
            color(theme::FRAME_GRAY_600),
        ))
}

fn preview_media_icon(media_kind: PreviewMediaKind) -> &'static str {
    match media_kind {
        PreviewMediaKind::Video | PreviewMediaKind::Unknown => assets::ICON_FILE_VIDEO,
        PreviewMediaKind::Audio => assets::ICON_MUSIC,
        PreviewMediaKind::Image => assets::ICON_FILE_IMAGE,
    }
}

fn preview_media_kind_label(media_kind: PreviewMediaKind) -> &'static str {
    match media_kind {
        PreviewMediaKind::Video => "VIDEO SOURCE",
        PreviewMediaKind::Audio => "AUDIO SOURCE",
        PreviewMediaKind::Image => "IMAGE SOURCE",
        PreviewMediaKind::Unknown => "UNKNOWN SOURCE",
    }
}

fn preview_visual_controls_visible(state: &PreviewShellState) -> bool {
    state.availability.media_kind != PreviewMediaKind::Unknown
        && !state.availability.hide_visual_controls
}

fn preview_visual_controls_enabled(state: &PreviewShellState) -> bool {
    preview_visual_controls_visible(state) && !state.controls_disabled
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FlipAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewCropDrag {
    handle: DragHandle,
}

fn preview_crop_overlay(state: &PreviewShellState) -> gpui::Div {
    let rect = state.crop.draft_crop.unwrap_or_else(default_crop_rect);
    let x = rect.x as f32;
    let y = rect.y as f32;
    let width = rect.width as f32;
    let height = rect.height as f32;
    let right = (x + width).min(1.0);
    let bottom = (y + height).min(1.0);

    div()
        .absolute()
        .inset_0()
        .child(crop_mask_rect(0.0, 0.0, 1.0, y.clamp(0.0, 1.0)))
        .child(crop_mask_rect(0.0, y, x.clamp(0.0, 1.0), height))
        .child(crop_mask_rect(right, y, (1.0 - right).max(0.0), height))
        .child(crop_mask_rect(0.0, bottom, 1.0, (1.0 - bottom).max(0.0)))
        .child(crop_outline_rect(x, y, width, height))
        .child(crop_vertical_guide_line(x + width / 3.0, y, height))
        .child(crop_vertical_guide_line(x + (width * 2.0) / 3.0, y, height))
        .child(crop_horizontal_guide_line(x, y + height / 3.0, width))
        .child(crop_horizontal_guide_line(
            x,
            y + (height * 2.0) / 3.0,
            width,
        ))
        .child(preview_crop_handle(DragHandle::NorthWest, x, y, state))
        .child(preview_crop_handle(
            DragHandle::North,
            x + width / 2.0,
            y,
            state,
        ))
        .child(preview_crop_handle(DragHandle::NorthEast, right, y, state))
        .child(preview_crop_handle(
            DragHandle::East,
            right,
            y + height / 2.0,
            state,
        ))
        .child(preview_crop_handle(
            DragHandle::SouthEast,
            right,
            bottom,
            state,
        ))
        .child(preview_crop_handle(
            DragHandle::South,
            x + width / 2.0,
            bottom,
            state,
        ))
        .child(preview_crop_handle(DragHandle::SouthWest, x, bottom, state))
        .child(preview_crop_handle(
            DragHandle::West,
            x,
            y + height / 2.0,
            state,
        ))
}

fn crop_mask_rect(left: f32, top: f32, width: f32, height: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(relative(height.clamp(0.0, 1.0)))
        .bg(hsla(0.0, 0.0, 0.0, 0.55))
}

fn crop_outline_rect(left: f32, top: f32, width: f32, height: f32) -> gpui::Stateful<gpui::Div> {
    div()
        .id("preview-crop-move-handle")
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(relative(height.clamp(0.0, 1.0)))
        .border_1()
        .border_color(color(theme::FOREGROUND.with_alpha(0.90)))
        .cursor_grab()
        .on_drag(
            PreviewCropDrag {
                handle: DragHandle::Move,
            },
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        )
}

fn crop_vertical_guide_line(left: f32, top: f32, height: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(px(1.0))
        .h(relative(height.clamp(0.0, 1.0)))
        .bg(color(theme::FOREGROUND.with_alpha(0.70)))
}

fn crop_horizontal_guide_line(left: f32, top: f32, width: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(px(1.0))
        .bg(color(theme::FOREGROUND.with_alpha(0.70)))
}

fn preview_crop_handle(
    handle: DragHandle,
    x: f32,
    y: f32,
    state: &PreviewShellState,
) -> gpui::Stateful<gpui::Div> {
    crop_handle_cursor(
        div()
            .id(format!("preview-crop-handle-{}", crop_handle_id(handle)))
            .absolute()
            .left(relative(x.clamp(0.0, 1.0)))
            .top(relative(y.clamp(0.0, 1.0)))
            .ml(px(-(CROP_HANDLE_SIZE / 2.0)))
            .mt(px(-(CROP_HANDLE_SIZE / 2.0)))
            .w(px(CROP_HANDLE_SIZE))
            .h(px(CROP_HANDLE_SIZE))
            .rounded_full()
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.45))
            .bg(color(theme::FOREGROUND))
            .shadow(card_surface_shadows()),
        handle,
        is_side_rotation(&state.crop.rotation),
    )
    .on_drag(
        PreviewCropDrag { handle },
        |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
    )
}

fn crop_handle_cursor(
    handle: gpui::Stateful<gpui::Div>,
    drag_handle: DragHandle,
    is_side_rotation: bool,
) -> gpui::Stateful<gpui::Div> {
    match frame_gpui_ce::preview::handle_cursor(drag_handle, is_side_rotation) {
        "ns-resize" => handle.cursor_ns_resize(),
        "ew-resize" => handle.cursor_ew_resize(),
        "nesw-resize" => handle.cursor_nesw_resize(),
        "nwse-resize" => handle.cursor_nwse_resize(),
        _ => handle.cursor_grab(),
    }
}

fn crop_handle_id(handle: DragHandle) -> &'static str {
    match handle {
        DragHandle::Move => "move",
        DragHandle::North => "n",
        DragHandle::South => "s",
        DragHandle::East => "e",
        DragHandle::West => "w",
        DragHandle::NorthEast => "ne",
        DragHandle::NorthWest => "nw",
        DragHandle::SouthEast => "se",
        DragHandle::SouthWest => "sw",
    }
}

fn preview_crop_aspect_bar(state: &PreviewShellState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    let mut bar = div()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows());

    for option in ASPECT_OPTIONS {
        let id = option.id;
        bar = bar.child(
            compact_text_button(option.display, state.crop.crop_aspect == id, true).on_click(
                cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    if root.select_preview_crop_aspect(id) {
                        cx.notify();
                    }
                }),
            ),
        );
    }

    let bar = bar
        .child(preview_toolbar_separator().h(px(18.0)).w(px(1.0)))
        .child(
            compact_text_button("Reset", false, true).on_click(cx.listener(
                |root, _: &ClickEvent, _window, cx| {
                    if root.reset_preview_crop_selection() {
                        cx.notify();
                    }
                },
            )),
        )
        .child(
            compact_text_button("Apply", false, state.crop.has_crop_dimensions).on_click(
                cx.listener(|root, _: &ClickEvent, _window, cx| {
                    if root.apply_selected_crop() {
                        cx.notify();
                    }
                }),
            ),
        );

    div()
        .absolute()
        .bottom(px(16.0))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(bar)
}

fn compact_text_button(
    label: &'static str,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };
    let colors = button_colors(variant, selected, enabled);

    div()
        .id(format!(
            "preview-crop-action-{}",
            label.to_ascii_lowercase()
        ))
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .child(label)
}

fn preview_toolbar(state: &PreviewShellState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    let transform_enabled = preview_visual_controls_enabled(state);
    let crop_enabled = transform_enabled && state.crop.has_crop_dimensions;
    let overlay_enabled = transform_enabled && state.availability.overlay_available;

    div()
        .absolute()
        .top(px(PREVIEW_TOOLBAR_OFFSET))
        .left(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows())
        .child(
            preview_tool_button(assets::ICON_ROTATE_CW, false, transform_enabled).on_click(
                cx.listener(|root, _: &ClickEvent, _window, cx| {
                    if root.rotate_selected_preview() {
                        cx.notify();
                    }
                }),
            ),
        )
        .child(
            preview_tool_button(
                assets::ICON_FLIP_HORIZONTAL,
                state.crop.flip_horizontal,
                transform_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Horizontal) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                assets::ICON_FLIP_VERTICAL,
                state.crop.flip_vertical,
                transform_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Vertical) {
                    cx.notify();
                }
            })),
        )
        .child(preview_toolbar_separator())
        .child(
            preview_tool_button(
                assets::ICON_CROP,
                state.crop.crop_mode || state.crop.applied_crop.is_some(),
                crop_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_crop_mode() {
                    cx.notify();
                }
            })),
        )
        .child(preview_tool_button(
            assets::ICON_FILE_IMAGE,
            false,
            overlay_enabled,
        ))
}

fn preview_zoom_toolbar(state: &PreviewShellState) -> gpui::Div {
    let enabled = preview_visual_controls_enabled(state);

    div()
        .absolute()
        .right(px(PREVIEW_TOOLBAR_OFFSET))
        .bottom(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows())
        .child(preview_tool_button(assets::ICON_ZOOM_OUT, false, enabled))
        .child(preview_tool_button(assets::ICON_ZOOM_IN, false, enabled))
}

fn preview_toolbar_separator() -> gpui::Div {
    div()
        .h(px(1.0))
        .w_full()
        .bg(color(theme::BACKGROUND))
        .shadow(horizontal_separator_shadows())
}

fn preview_tool_button(
    icon: &'static str,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };
    let colors = button_colors(variant, false, enabled);
    let icon_color = color(colors.foreground);
    let button_id = format!("preview-tool-{}", icon.replace(['/', '.'], "-"));

    div()
        .id(button_id)
        .w(px(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .h(px(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .text_color(icon_color)
        .opacity(colors.opacity)
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .when(!enabled, |this| this.cursor_not_allowed())
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .child(icon_svg(icon, PREVIEW_TOOLBAR_ICON_SIZE, icon_color))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewTimelineDrag {
    target: TimelineDragTarget,
}

struct PreviewTimelineDragPreview;

impl Render for PreviewTimelineDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

fn preview_timeline(state: &PreviewShellState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    let labels = preview_timeline_labels(state);
    let trim_enabled = preview_trim_enabled(state);

    div()
        .mt(px(PREVIEW_TIMELINE_TOP_MARGIN))
        .px_2()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .flex()
                .gap_4()
                .child(preview_timecode_field(
                    "START TIME",
                    labels.start,
                    trim_enabled,
                    128.0,
                ))
                .child(preview_timecode_field(
                    "END TIME",
                    labels.end,
                    trim_enabled,
                    128.0,
                ))
                .child(preview_timecode_field(
                    "DURATION",
                    labels.duration,
                    false,
                    104.0,
                )),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(preview_timeline_label("TRIM"))
                .child(preview_timeline_track(state, cx)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(preview_timeline_label(" "))
                .child(preview_play_button(state)),
        )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreviewTimelineLabels {
    start: String,
    end: String,
    duration: String,
}

fn preview_timeline_labels(state: &PreviewShellState) -> PreviewTimelineLabels {
    if state.availability.media_kind == PreviewMediaKind::Image
        || state.availability.media_kind == PreviewMediaKind::Unknown
        || state.duration_seconds <= 0.0
    {
        return PreviewTimelineLabels {
            start: "--:--:--.---".to_string(),
            end: "--:--:--.---".to_string(),
            duration: "--:--:--.---".to_string(),
        };
    }

    PreviewTimelineLabels {
        start: format_time(state.playback.start_value()),
        end: format_time(state.playback.end_value()),
        duration: format_time(state.playback.end_value() - state.playback.start_value()),
    }
}

fn preview_trim_enabled(state: &PreviewShellState) -> bool {
    !state.availability.trim_disabled && state.duration_seconds > 0.0
}

fn preview_timecode_field(
    label: &'static str,
    value: String,
    enabled: bool,
    width: f32,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(preview_timeline_label(label))
        .child(
            div()
                .w(px(width))
                .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
                .flex()
                .items_center()
                .rounded(px(theme::RADIUS_SM))
                .bg(color(theme::BACKGROUND))
                .px(px(10.0))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(if enabled {
                    color(theme::FOREGROUND)
                } else {
                    color(theme::FRAME_GRAY_600)
                })
                .shadow(input_highlight_shadows())
                .child(value),
        )
}

fn preview_timeline_label(label: &'static str) -> gpui::Div {
    div()
        .h(px(12.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(label)
}

fn preview_timeline_track(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let enabled = preview_trim_enabled(state);
    let track_top = centered_offset(PREVIEW_TIMELINE_CONTROL_HEIGHT, PREVIEW_TRACK_HEIGHT);
    let playhead_top = centered_offset(PREVIEW_TIMELINE_CONTROL_HEIGHT, PREVIEW_PLAYHEAD_HEIGHT);
    let start_fraction = timeline_fraction_from_percent(
        state
            .playback
            .to_timeline_percent(state.playback.start_value()),
    );
    let end_fraction = timeline_fraction_from_percent(
        state
            .playback
            .to_timeline_percent(state.playback.end_value()),
    );
    let playhead_fraction = timeline_fraction_from_percent(
        state
            .playback
            .to_timeline_percent(state.playback.current_time()),
    );

    div()
        .id("preview-timeline-track")
        .relative()
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w_full()
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| this.cursor_pointer())
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewTimelineDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let percent =
                    timeline_slider_percent_from_bounds(event.event.position, event.bounds);
                if root.apply_selected_trim_drag(drag.target, percent) {
                    cx.notify();
                }
            },
        ))
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(track_top))
                .h(px(PREVIEW_TRACK_HEIGHT))
                .rounded(px(1.5))
                .bg(color(theme::FRAME_GRAY_100))
                .shadow(input_highlight_shadows()),
        )
        .child(
            div()
                .absolute()
                .left(relative(start_fraction))
                .right(relative((1.0 - end_fraction).max(0.0)))
                .top(px(track_top))
                .h(px(PREVIEW_TRACK_HEIGHT))
                .rounded(px(1.0))
                .bg(color(theme::FOREGROUND)),
        )
        .child(
            div()
                .absolute()
                .left(relative(playhead_fraction))
                .ml(px(-0.5))
                .top(px(playhead_top))
                .h(px(PREVIEW_PLAYHEAD_HEIGHT))
                .w(px(1.0))
                .bg(color(theme::FOREGROUND)),
        )
        .child(preview_timeline_handle(
            TimelineDragTarget::Start,
            start_fraction,
            enabled,
        ))
        .child(preview_timeline_handle(
            TimelineDragTarget::End,
            end_fraction,
            enabled,
        ))
}

fn preview_timeline_handle(
    target: TimelineDragTarget,
    fraction: f32,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle_id = match target {
        TimelineDragTarget::Start => "preview-timeline-start-handle",
        TimelineDragTarget::End => "preview-timeline-end-handle",
        TimelineDragTarget::Scrub => "preview-timeline-scrub-handle",
    };

    let handle = div()
        .id(handle_id)
        .absolute()
        .top_0()
        .left(relative(fraction))
        .ml(px(-(PREVIEW_TIMELINE_HANDLE_WIDTH / 2.0)))
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w(px(PREVIEW_TIMELINE_HANDLE_WIDTH))
        .when(enabled, |this| this.cursor_ew_resize());

    if enabled {
        handle.on_drag(
            PreviewTimelineDrag { target },
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        )
    } else {
        handle
    }
}

fn preview_play_button(state: &PreviewShellState) -> impl IntoElement {
    let enabled = preview_trim_enabled(state);
    let icon = if state.playback.is_playing() {
        assets::ICON_PAUSE
    } else {
        assets::ICON_PLAY
    };

    preview_tool_button(icon, false, enabled)
}

fn centered_offset(container: f32, child: f32) -> f32 {
    ((container - child) / 2.0).max(0.0)
}

fn timeline_fraction_from_percent(percent: f64) -> f32 {
    (percent / 100.0).clamp(0.0, 1.0) as f32
}

fn timeline_slider_percent_from_bounds(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
) -> f64 {
    let width = bounds.size.width.as_f32();
    if width <= 0.0 {
        return 0.0;
    }

    let x = (position.x - bounds.origin.x).as_f32();
    f64::from((x / width).clamp(0.0, 1.0))
}

fn logs_view(
    queue: &FileQueue,
    conversion_events: &ConversionEventState,
    scroll_handle: &UniformListScrollHandle,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let active_files = conversion_events.active_log_files(queue);
    let selected_id = conversion_events.selected_log_file_id();

    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .child(logs_tab_strip(&active_files, selected_id, cx))
        .child(logs_body(
            conversion_events,
            selected_id,
            !active_files.is_empty(),
            scroll_handle,
            cx,
        ))
}

fn logs_tab_strip(
    active_files: &[ActiveLogFile],
    selected_id: Option<&str>,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut tabs = div()
        .size_full()
        .flex()
        .items_center()
        .gap_6()
        .overflow_hidden()
        .px_4();

    for file in active_files {
        tabs = tabs.child(log_tab_button(
            file,
            selected_id.is_some_and(|id| id == file.id),
            cx,
        ));
    }

    if active_files.is_empty() {
        tabs = tabs.child(
            div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child("No active processes"),
        );
    }

    div()
        .h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .relative()
        .child(tabs)
        .child(panel_bottom_separator())
}

fn log_tab_button(
    file: &ActiveLogFile,
    selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let file_id = file.id.clone();

    div()
        .id(element_id("logs-tab", &file.id))
        .flex_none()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(if selected {
            color(theme::FOREGROUND)
        } else {
            color(theme::FRAME_GRAY_600)
        })
        .hover(|style| style.text_color(color(theme::FOREGROUND)).cursor_pointer())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root
                .conversion_events
                .select_log_file(&root.file_queue, &file_id)
            {
                cx.notify();
            }
            cx.stop_propagation();
        }))
        .child(file.name.clone())
}

fn logs_body(
    conversion_events: &ConversionEventState,
    selected_id: Option<&str>,
    has_active_files: bool,
    scroll_handle: &UniformListScrollHandle,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let body = div()
        .id("logs-body")
        .relative()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden();

    if !has_active_files {
        return body.child(logs_empty_state("Select a task to view console output"));
    }

    let Some(selected_id) = selected_id else {
        return body.child(logs_empty_state("Select a task to view console output"));
    };

    let line_count = conversion_events.logs_for(selected_id).len();
    if line_count == 0 {
        return body.child(logs_empty_state("Process started, waiting for output..."));
    }

    body.child(log_lines_list(selected_id, line_count, scroll_handle, cx))
}

fn log_lines_list(
    selected_id: &str,
    line_count: usize,
    scroll_handle: &UniformListScrollHandle,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let selected_id = selected_id.to_string();
    let list_id = element_id("logs-line-list", &selected_id);

    uniform_list(
        list_id,
        line_count,
        cx.processor(move |root, range, _window, _cx| {
            root.conversion_events
                .log_line_window_for(&selected_id, range)
                .iter()
                .map(log_line_row)
                .collect()
        }),
    )
    .track_scroll(scroll_handle)
    .size_full()
    .p(px(2.0))
    .text_color(color(theme::FOREGROUND))
    .line_height(px(LOG_LINE_HEIGHT))
}

fn log_line_row(line: &LogLine) -> gpui::Div {
    div()
        .min_h(px(LOG_LINE_HEIGHT))
        .w_full()
        .flex()
        .items_start()
        .rounded(px(theme::RADIUS_XS))
        .px_1()
        .py(px(2.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .hover(|style| style.bg(color(theme::FRAME_GRAY_100)))
        .child(
            div()
                .flex_none()
                .w(px(LOG_LINE_NUMBER_WIDTH))
                .mr(px(12.0))
                .pt(px(0.5))
                .text_right()
                .text_color(color(theme::FRAME_GRAY_400))
                .child(line.index.to_string()),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(line.text.clone()),
        )
}

fn logs_empty_state(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(message)
}

fn settings_panel(settings: SettingsRenderState<'_>, cx: &mut Context<FrameRoot>) -> gpui::Div {
    let active_tab =
        resolve_active_settings_tab(settings.active_tab, settings.config, settings.metadata);
    let mut tab_rail = div().flex().items_center().justify_start().gap_1();
    for tab in visible_settings_tabs(settings.config, settings.metadata) {
        tab_rail = tab_rail.child(settings_tab_button(tab, active_tab == tab, cx));
    }

    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .child(
            div()
                .h(px(PANEL_HEADER_HEIGHT))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .relative()
                .px_4()
                .child(tab_rail)
                .child(panel_bottom_separator()),
        )
        .child(
            div()
                .id("settings-panel-body")
                .flex_1()
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .p(px(SETTINGS_PANEL_PADDING))
                .child(settings_tab_content(active_tab, settings, cx)),
        )
}

fn settings_tab_button(
    tab: SettingsTab,
    selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let colors = button_colors(ButtonVariant::Secondary, selected, true);
    let icon_color = if selected {
        color(theme::FOREGROUND)
    } else {
        color(theme::FRAME_GRAY_600)
    };

    div()
        .id(format!("settings-tab-{}", tab.id()))
        .w(px(SETTINGS_TAB_BUTTON_SIZE))
        .h(px(SETTINGS_TAB_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .hover(move |style| {
            style
                .bg(color(if selected {
                    colors.hover_background
                } else {
                    theme::FRAME_GRAY_100
                }))
                .cursor_pointer()
        })
        .active(move |style| style.bg(color(colors.active_background)))
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            root.settings_active_tab = tab;
            cx.stop_propagation();
            cx.notify();
        }))
        .child(icon_svg(
            settings_tab_icon(tab),
            SETTINGS_TAB_ICON_SIZE,
            icon_color,
        ))
}

fn settings_tab_content(
    tab: SettingsTab,
    settings: SettingsRenderState<'_>,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let content = div()
        .flex()
        .flex_col()
        .gap_4()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600));

    match tab {
        SettingsTab::Source => content.child(settings_source_tab(
            settings.metadata,
            settings.metadata_status,
            settings.metadata_error,
        )),
        SettingsTab::Output => content.child(settings_output_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.output_name,
            cx,
        )),
        SettingsTab::Video => {
            content.child(settings_section("VIDEO").child(settings_value_row("STATUS", "Ready")))
        }
        SettingsTab::Images => {
            content.child(settings_section("IMAGES").child(settings_value_row("STATUS", "Ready")))
        }
        SettingsTab::Audio => {
            content.child(settings_section("AUDIO").child(settings_value_row("STATUS", "Ready")))
        }
        SettingsTab::Subtitles => content
            .child(settings_section("SUBTITLES").child(settings_value_row("STATUS", "Ready"))),
        SettingsTab::Metadata => {
            content.child(settings_section("METADATA").child(settings_value_row("STATUS", "Ready")))
        }
        SettingsTab::Presets => {
            content.child(settings_section("PRESETS").child(settings_value_row("STATUS", "Ready")))
        }
    }
}

fn settings_section(label: &'static str) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(settings_section_label(label))
}

fn settings_source_tab(
    metadata: Option<&SourceMetadata>,
    status: MetadataStatus,
    error: Option<&str>,
) -> gpui::Div {
    match status {
        MetadataStatus::Loading => {
            return div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child("Analyzing source...");
        }
        MetadataStatus::Error => {
            let mut error_view = div()
                .flex()
                .flex_col()
                .gap_1()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_RED))
                .child("Failed to read source metadata.");
            if let Some(error) = error {
                error_view = error_view.child(
                    div()
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(error.to_string()),
                );
            }
            return error_view;
        }
        MetadataStatus::Idle | MetadataStatus::Ready => {}
    }

    let Some(metadata) = metadata else {
        return div()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .text_color(color(theme::FRAME_GRAY_600))
            .child("Metadata unavailable.");
    };

    let sections = source_info_sections(metadata);
    if sections.is_empty() {
        return div()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .text_color(color(theme::FRAME_GRAY_600))
            .child("Metadata unavailable.");
    }

    let mut content = div().flex().flex_col().gap_6();
    for section in sections {
        content = match section {
            SourceInfoSection::Rows { title, rows } => {
                content.child(settings_section(title).child(settings_source_rows(rows)))
            }
            SourceInfoSection::Tracks { title, tracks } => {
                content.child(settings_section(title).child(settings_source_tracks(tracks)))
            }
        };
    }
    content
}

fn settings_source_rows(rows: Vec<frame_gpui_ce::settings::SourceInfoRow>) -> gpui::Div {
    let mut grid = div().flex().flex_col().gap_2();
    for row in rows {
        grid = grid.child(settings_value_row(row.label, row.value));
    }
    grid
}

fn settings_source_tracks(tracks: Vec<frame_gpui_ce::settings::SourceTrackSection>) -> gpui::Div {
    let mut list = div().flex().flex_col().gap_4();
    for track in tracks {
        list = list.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_track_header(track.label))
                .child(settings_source_rows(track.rows)),
        );
    }
    list
}

fn settings_track_header(label: String) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(label)
        .child(
            div()
                .h(px(1.0))
                .flex_1()
                .bg(color(theme::BACKGROUND))
                .shadow(input_highlight_shadows()),
        )
}

fn settings_section_label(label: &'static str) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(label)
        .child(
            div()
                .h(px(1.0))
                .w_full()
                .bg(color(theme::BACKGROUND))
                .shadow(input_highlight_shadows()),
        )
}

fn settings_output_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    output_name: &str,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            settings_section("PROCESSING MODE")
                .child(settings_processing_mode_grid(config, metadata, settings_disabled, cx))
                .child(settings_hint_text(config.processing_mode.hint())),
        )
        .child(
            settings_section("OUTPUT NAME")
                .child(settings_output_name_field(output_name, settings_disabled))
                .child(settings_hint_text(
                    "Stored next to the original file. Extension follows the selected container automatically.",
                )),
        )
        .child(
            settings_section("OUTPUT CONTAINER").child(settings_container_grid(
                config,
                metadata,
                settings_disabled,
                cx,
            )),
        )
}

fn settings_processing_mode_grid(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for option in output_processing_mode_options(config, metadata, settings_disabled) {
        let mode = option.mode;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            settings_choice_button(
                format!("output-mode-{}", option.mode.id()),
                option.label,
                option.is_selected,
                is_enabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }

                let metadata = root.selected_source_metadata();
                if root.update_selected_config(|config| {
                    apply_processing_mode(config, metadata.as_ref(), mode)
                }) {
                    root.resolve_selected_settings_tab(metadata.as_ref());
                    cx.notify();
                }
            })),
        );
    }
    grid
}

fn settings_container_grid(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for option in output_container_options(config, metadata, settings_disabled) {
        let container = option.container;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            settings_choice_button(
                format!("output-container-{container}"),
                container.to_uppercase(),
                option.is_selected,
                is_enabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }

                let metadata = root.selected_source_metadata();
                let changed = root.update_selected_config(|config| {
                    apply_output_container(config, &container)
                        | normalize_output_config(config, metadata.as_ref())
                });
                if changed {
                    root.resolve_selected_settings_tab(metadata.as_ref());
                    cx.notify();
                }
            })),
        );
    }
    grid
}

fn settings_choice_button(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Secondary, selected, enabled);
    let label = label.into();

    div()
        .id(id.into())
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .child(label)
}

fn settings_output_name_field(output_name: &str, disabled: bool) -> gpui::Div {
    let value = if output_name.is_empty() {
        "my_render_final"
    } else {
        output_name
    }
    .to_string();

    div()
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::BACKGROUND))
        .px(px(10.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(if output_name.is_empty() || disabled {
            color(theme::FRAME_GRAY_600)
        } else {
            color(theme::FOREGROUND)
        })
        .shadow(input_highlight_shadows())
        .child(value)
}

fn settings_hint_text(text: &'static str) -> gpui::Div {
    div()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(text)
}

fn settings_value_row(label: &'static str, value: impl Into<String>) -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap_4()
        .child(div().text_color(color(theme::FRAME_GRAY_600)).child(label))
        .child(
            div()
                .text_right()
                .text_color(color(theme::FOREGROUND))
                .child(value.into()),
        )
}

fn settings_tab_icon(tab: SettingsTab) -> &'static str {
    match tab {
        SettingsTab::Source => assets::ICON_FILE_UP,
        SettingsTab::Output => assets::ICON_FILE_DOWN,
        SettingsTab::Video => assets::ICON_FILE_VIDEO,
        SettingsTab::Images => assets::ICON_FILE_IMAGE,
        SettingsTab::Audio => assets::ICON_MUSIC,
        SettingsTab::Subtitles => assets::ICON_CAPTIONS,
        SettingsTab::Metadata => assets::ICON_TAGS,
        SettingsTab::Presets => assets::ICON_BOOKMARK,
    }
}

fn file_list_panel(queue: &FileQueue, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface()
        .drag_over::<ExternalPaths>(|style, _paths, _window, _cx| {
            style
                .border_1()
                .border_dashed()
                .border_color(color(theme::FRAME_GRAY_600))
                .shadow(drop_target_shadows())
        })
        .child(file_list_header(queue.batch_selection_state(), cx))
        .child(file_list_body(queue, cx))
}

fn file_list_header(selection: BatchSelectionState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .relative()
        .px_4()
        .child(
            div()
                .flex_1()
                .grid()
                .grid_cols(12)
                .gap(px(WORKSPACE_GAP))
                .items_center()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(
                    div().col_span(1).flex().items_center().child(
                        checkbox_hit_area(
                            selection.is_checked,
                            selection.is_indeterminate,
                            selection.is_enabled,
                        )
                        .id("file-list-header-checkbox")
                        .when(selection.is_enabled, |this| this.cursor_pointer())
                        .on_click(cx.listener(
                            |root, _: &ClickEvent, _window, cx| {
                                if !root.file_queue.files().is_empty() {
                                    root.file_queue.toggle_all_batch_selection();
                                    cx.notify();
                                }
                            },
                        )),
                    ),
                )
                .child(header_label("NAME", 5, false))
                .child(header_label("SIZE", 2, true))
                .child(header_label("TARGET", 2, true))
                .child(header_label("STATE", 2, true)),
        )
        .child(
            div()
                .ml_4()
                .w(px(FILE_LIST_ACTIONS_WIDTH))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .text_right()
                .child("ACTIONS"),
        )
        .child(panel_bottom_separator())
}

fn file_list_body(queue: &FileQueue, cx: &mut Context<FrameRoot>) -> impl IntoElement {
    let body = div()
        .id("file-list-body")
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll();
    if queue.files().is_empty() {
        return body.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(theme::TEXT_ROW_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child("Drop files or use Add Source"),
        );
    }

    let mut body = body;
    for file in queue.files() {
        body = body.child(file_list_row(
            file,
            queue.selected_file_id() == Some(file.id.as_str()),
            cx,
        ));
    }
    body
}

fn file_list_row(
    file: &FileItem,
    is_selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let group_name = format!("file-list-row-{}", file.id);
    let select_id = file.id.clone();

    div()
        .h(px(FILE_ROW_HEIGHT))
        .w_full()
        .id(element_id("file-list-row", &select_id))
        .group(group_name.clone())
        .flex()
        .items_center()
        .relative()
        .px_4()
        .bg(if is_selected {
            color(theme::FRAME_GRAY_100)
        } else {
            color(theme::TRANSPARENT)
        })
        .hover(|style| style.bg(color(theme::FRAME_GRAY_100)).cursor_pointer())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root.file_queue.select_existing_file(&select_id) {
                cx.notify();
            }
        }))
        .child(
            div()
                .flex_1()
                .grid()
                .grid_cols(12)
                .gap(px(WORKSPACE_GAP))
                .items_center()
                .text_size(px(theme::TEXT_ROW_SIZE))
                .child(
                    div()
                        .col_span(1)
                        .flex()
                        .items_center()
                        .child(row_checkbox_control(
                            file.id.clone(),
                            file.is_selected_for_conversion,
                            cx,
                        )),
                )
                .child(row_label(
                    file.name.clone(),
                    5,
                    false,
                    color(theme::FOREGROUND),
                ))
                .child(row_label(
                    format_file_size(file.size_bytes),
                    2,
                    true,
                    color(theme::FRAME_GRAY_600),
                ))
                .child(row_label(
                    file.original_format.clone(),
                    2,
                    true,
                    color(theme::FRAME_GRAY_600),
                ))
                .child(row_label(
                    file.row_state_label(),
                    2,
                    true,
                    state_tone_color(file.row_state_tone()),
                )),
        )
        .child(row_actions_cell(
            file.id.clone(),
            file.row_actions(),
            group_name,
            cx,
        ))
        .child(panel_bottom_separator())
}

fn header_label(label: &'static str, span: u16, align_right: bool) -> gpui::Div {
    let cell = div().col_span(span).truncate();
    let cell = if align_right { cell.text_right() } else { cell };
    cell.child(label)
}

fn row_label(label: String, span: u16, align_right: bool, text_color: Rgba) -> gpui::Div {
    let cell = div()
        .col_span(span)
        .truncate()
        .whitespace_nowrap()
        .text_color(text_color);
    let cell = if align_right { cell.text_right() } else { cell };
    cell.child(label)
}

fn row_actions_cell(
    file_id: String,
    actions: RowActionAvailability,
    group_name: String,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let mut cell = div()
        .id(element_id("file-row-actions", &file_id))
        .ml_4()
        .w(px(FILE_LIST_ACTIONS_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_end()
        .gap_1()
        .opacity(0.0)
        .group_hover(group_name, |style| style.opacity(1.0))
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }));

    if actions.can_pause {
        let id = file_id.clone();
        cell = cell.child(
            row_action_button("||", true)
                .id(element_id("file-row-action-pause", &id))
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if root.file_queue.pause_file(&id) {
                        cx.notify();
                    }
                })),
        );
    }
    if actions.can_resume {
        let id = file_id.clone();
        cell = cell.child(
            row_action_button(">", true)
                .id(element_id("file-row-action-resume", &id))
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if root.file_queue.resume_file(&id) {
                        cx.notify();
                    }
                })),
        );
    }

    if actions.can_delete {
        let id = file_id;
        cell.child(
            row_action_button("X", true)
                .id(element_id("file-row-action-delete", &id))
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if root.file_queue.remove_interactive_file(&id).is_some() {
                        root.source_metadata.remove(&id);
                        cx.notify();
                    }
                })),
        )
    } else {
        cell.child(row_action_button("X", false))
    }
}

fn row_action_button(label: &'static str, enabled: bool) -> gpui::Div {
    div()
        .w(px(FILE_LIST_ACTION_BUTTON_SIZE))
        .h(px(FILE_LIST_ACTION_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_MD))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .bg(if enabled {
            color(theme::TRANSPARENT)
        } else {
            color(theme::FRAME_GRAY_100)
        })
        .text_color(if enabled {
            color(theme::FOREGROUND)
        } else {
            color(theme::FRAME_GRAY_400)
        })
        .when(enabled, |this| {
            this.hover(|style| style.bg(color(theme::FRAME_GRAY_100)).cursor_pointer())
        })
        .child(label)
}

fn row_checkbox_control(
    file_id: String,
    is_checked: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    checkbox_hit_area(is_checked, false, true)
        .id(element_id("file-row-checkbox", &file_id))
        .cursor_pointer()
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if root.file_queue.toggle_batch_selection(&file_id).is_some() {
                cx.notify();
            }
        }))
}

fn checkbox_hit_area(is_checked: bool, is_indeterminate: bool, is_enabled: bool) -> gpui::Div {
    div()
        .w(px(theme::MIN_HIT_AREA))
        .h(px(FILE_ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_start()
        .child(checkbox_indicator(is_checked, is_indeterminate, is_enabled))
}

fn checkbox_indicator(is_checked: bool, is_indeterminate: bool, is_enabled: bool) -> gpui::Div {
    let is_active = is_checked || is_indeterminate;
    let mut indicator = div()
        .w(px(FILE_LIST_CHECKBOX_SIZE))
        .h(px(FILE_LIST_CHECKBOX_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_XS))
        .border_1()
        .border_color(if is_active || is_enabled {
            color(theme::FRAME_GRAY_600)
        } else {
            color(theme::FRAME_GRAY_200)
        })
        .bg(if is_active {
            color(theme::FRAME_GRAY_600)
        } else {
            color(theme::TRANSPARENT)
        });

    if is_indeterminate {
        indicator = indicator.child(
            div()
                .w(px(6.0))
                .h(px(2.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(color(theme::FOREGROUND)),
        );
    }

    indicator
}

fn state_tone_color(tone: FileStateTone) -> Rgba {
    match tone {
        FileStateTone::Foreground => color(theme::FOREGROUND),
        FileStateTone::Muted => color(theme::FRAME_GRAY_600),
        FileStateTone::Amber => color(theme::FRAME_AMBER),
        FileStateTone::Red => color(theme::FRAME_RED),
    }
}

fn vertical_separator(height: f32) -> gpui::Div {
    div()
        .flex()
        .h(px(height))
        .w(px(2.0))
        .child(div().h_full().w(px(1.0)).bg(color(theme::BACKGROUND)))
        .child(div().h_full().w(px(1.0)).bg(color(theme::FRAME_GRAY_100)))
}

fn panel_bottom_separator() -> gpui::Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(px(1.0))
        .bg(color(theme::BACKGROUND))
        .shadow(horizontal_separator_shadows())
}

fn element_id(prefix: &str, id: &str) -> String {
    format!("{prefix}-{id}")
}

trait FrameSurface {
    fn card_surface(self) -> Self;
}

impl FrameSurface for gpui::Div {
    fn card_surface(self) -> Self {
        self.rounded(px(theme::RADIUS_LG))
            .bg(color(theme::FRAME_GRAY_100))
            .shadow(card_surface_shadows())
    }
}

fn card_surface_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.10),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(6.0),
            spread_radius: px(-1.0),
            inset: false,
        },
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.10),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(4.0),
            spread_radius: px(-2.0),
            inset: false,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_200).into(),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_100).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: true,
        },
    ]
}

fn horizontal_separator_shadows() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color(theme::FRAME_GRAY_100).into(),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

fn drop_target_shadows() -> Vec<BoxShadow> {
    let mut shadows = card_surface_shadows();
    shadows.push(BoxShadow {
        color: color(theme::FRAME_GRAY_600.with_alpha(0.55)).into(),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(1.0),
        inset: true,
    });
    shadows
}

fn color(token: theme::RgbaToken) -> Rgba {
    Rgba {
        r: token.red,
        g: token.green,
        b: token.blue,
        a: token.alpha,
    }
}

fn init_app(cx: &mut App, name: impl Into<SharedString>) {
    cx.activate(true);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    cx.set_menus(vec![Menu {
        name: name.into(),
        items: vec![MenuItem::action("Quit", Quit)],
        disabled: false,
    }]);
    cx.on_window_closed(|cx, _| {
        if cx.windows().is_empty() {
            cx.quit();
        }
    })
    .detach();
}

fn frame_window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
fn hide_native_macos_titlebar_controls(window: &Window) -> bool {
    let Ok(window_handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };

    let RawWindowHandle::AppKit(appkit_handle) = window_handle.as_raw() else {
        return true;
    };

    // SAFETY: GPUI exposes a valid AppKit NSView handle for the live window.
    let ns_view = unsafe { &*appkit_handle.ns_view.as_ptr().cast::<NSView>() };
    let Some(ns_window) = ns_view.window() else {
        return false;
    };

    for button_kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(button) = ns_window.standardWindowButton(button_kind) {
            button.setHidden(true);
        }
    }

    true
}

#[cfg(not(target_os = "macos"))]
fn hide_native_macos_titlebar_controls(_window: &Window) -> bool {
    true
}

fn main() {
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(|cx| {
            assets::load_frame_fonts(cx).expect("failed to load Frame fonts");
            let bounds =
                Bounds::centered(None, size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)), cx);
            cx.open_window(frame_window_options(bounds), |_, cx| {
                cx.new(|_| FrameRoot::new())
            })
            .expect("failed to open Frame GPUI window");

            init_app(cx, "Frame");
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    mod frame_root_imports {
        use super::*;

        #[test]
        fn allocate_file_imports_assigns_incrementing_ids() {
            let mut root = FrameRoot::new();

            let imports = root.allocate_file_imports(vec![
                PathBuf::from("/tmp/one.mp4"),
                PathBuf::from("/tmp/two.mp4"),
            ]);

            assert_eq!(imports[0].0, "file-1");
            assert_eq!(imports[1].0, "file-2");
        }

        #[test]
        fn allocate_file_imports_continues_after_previous_batch() {
            let mut root = FrameRoot::new();
            root.allocate_file_imports(vec![PathBuf::from("/tmp/one.mp4")]);

            let imports = root.allocate_file_imports(vec![PathBuf::from("/tmp/two.mp4")]);

            assert_eq!(imports[0].0, "file-2");
        }

        #[test]
        fn allocate_file_imports_returns_empty_for_empty_drop() {
            let mut root = FrameRoot::new();

            let imports = root.allocate_file_imports(Vec::new());

            assert!(imports.is_empty());
        }
    }

    mod frame_root_config {
        use super::*;

        #[test]
        fn update_selected_config_mutates_only_selected_file() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
            root.file_queue
                .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
            root.file_queue.select_existing_file("second");

            root.update_selected_config(|config| {
                config.container = "webm".to_string();
                true
            });

            assert_eq!(
                root.file_queue
                    .file_by_id("first")
                    .map(|file| file.config.container.as_str()),
                Some("mp4")
            );
            assert_eq!(
                root.file_queue
                    .file_by_id("second")
                    .map(|file| file.config.container.as_str()),
                Some("webm")
            );
        }

        #[test]
        fn normalize_selected_config_clears_trim_for_selected_image_only() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.file_queue
                .add_file(FileItem::from_path("image", "/tmp/two.png", 1));

            for id in ["video", "image"] {
                root.file_queue.select_existing_file(id);
                root.update_selected_config(|config| {
                    config.start_time = Some("00:00:05.000".to_string());
                    config.end_time = Some("00:00:30.000".to_string());
                    true
                });
            }
            root.file_queue.select_existing_file("image");

            root.normalize_selected_config(Some(&SourceMetadata {
                media_kind: Some(SourceKind::Image),
                ..SourceMetadata::default()
            }));

            assert_eq!(
                root.file_queue
                    .file_by_id("video")
                    .and_then(|file| file.config.start_time.as_deref()),
                Some("00:00:05.000")
            );
            assert_eq!(
                root.file_queue
                    .file_by_id("image")
                    .and_then(|file| file.config.start_time.as_deref()),
                None
            );
        }

        #[test]
        fn apply_selected_trim_drag_updates_selected_file_start_time() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.source_metadata.mark_ready(
                "video".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    duration: Some("90.0".to_string()),
                    ..SourceMetadata::default()
                },
            );

            let changed = root.apply_selected_trim_drag(TimelineDragTarget::Start, 0.25);

            assert!(changed);
            assert_eq!(
                root.file_queue
                    .file_by_id("video")
                    .and_then(|file| file.config.start_time.as_deref()),
                Some("00:00:22.500")
            );
            assert_eq!(
                root.file_queue
                    .file_by_id("video")
                    .and_then(|file| file.config.end_time.as_deref()),
                None
            );
        }

        #[test]
        fn apply_selected_trim_drag_preserves_gap_when_end_moves_before_start() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.source_metadata.mark_ready(
                "video".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    duration: Some("90.0".to_string()),
                    ..SourceMetadata::default()
                },
            );
            root.update_selected_config(|config| {
                config.start_time = Some("00:00:20.000".to_string());
                true
            });

            let changed = root.apply_selected_trim_drag(TimelineDragTarget::End, 0.10);

            assert!(changed);
            assert_eq!(
                root.file_queue
                    .file_by_id("video")
                    .and_then(|file| file.config.end_time.as_deref()),
                Some("00:00:21.000")
            );
        }

        #[test]
        fn apply_selected_trim_drag_ignores_image_sources() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("image", "/tmp/one.png", 1));
            root.source_metadata.mark_ready(
                "image".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Image),
                    duration: Some("90.0".to_string()),
                    ..SourceMetadata::default()
                },
            );

            let changed = root.apply_selected_trim_drag(TimelineDragTarget::Start, 0.25);

            assert!(!changed);
            assert_eq!(
                root.file_queue
                    .file_by_id("image")
                    .and_then(|file| file.config.start_time.as_deref()),
                None
            );
        }

        #[test]
        fn toggle_selected_crop_mode_initializes_default_video_draft() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.source_metadata.mark_ready(
                "video".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    width: Some(1920),
                    height: Some(1080),
                    ..SourceMetadata::default()
                },
            );

            let changed = root.toggle_selected_crop_mode();

            assert!(changed);
            assert!(root.preview_crop_mode);
            assert_eq!(root.preview_draft_crop, Some(default_crop_rect()));
            assert_eq!(root.preview_crop_aspect, "free");
        }

        #[test]
        fn apply_selected_crop_writes_selected_file_crop_pixels() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
            root.file_queue
                .add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
            root.file_queue.select_existing_file("second");
            root.source_metadata.mark_ready(
                "second".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    width: Some(1920),
                    height: Some(1080),
                    ..SourceMetadata::default()
                },
            );
            root.preview_crop_mode = true;
            root.preview_draft_crop = Some(CropRect {
                x: 0.25,
                y: 0.25,
                width: 0.5,
                height: 0.5,
            });
            root.preview_crop_aspect = "16:9".to_string();

            let changed = root.apply_selected_crop();

            assert!(changed);
            assert_eq!(
                root.file_queue
                    .file_by_id("first")
                    .and_then(|file| file.config.crop.as_ref()),
                None
            );
            assert_eq!(
                root.file_queue
                    .file_by_id("second")
                    .and_then(|file| file.config.crop.as_ref()),
                Some(&CropSettings {
                    enabled: true,
                    x: 480,
                    y: 270,
                    width: 960,
                    height: 540,
                    source_width: Some(1920),
                    source_height: Some(1080),
                    aspect_ratio: Some("16:9".to_string()),
                })
            );
        }

        #[test]
        fn apply_selected_full_crop_clears_existing_crop() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.source_metadata.mark_ready(
                "video".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    width: Some(1920),
                    height: Some(1080),
                    ..SourceMetadata::default()
                },
            );
            root.update_selected_config(|config| {
                config.crop = Some(CropSettings {
                    enabled: true,
                    x: 100,
                    y: 100,
                    width: 1000,
                    height: 600,
                    source_width: Some(1920),
                    source_height: Some(1080),
                    aspect_ratio: None,
                });
                true
            });
            root.preview_crop_mode = true;
            root.preview_draft_crop = Some(full_crop_rect());

            let changed = root.apply_selected_crop();

            assert!(changed);
            assert_eq!(
                root.file_queue
                    .file_by_id("video")
                    .and_then(|file| file.config.crop.as_ref()),
                None
            );
        }

        #[test]
        fn rotate_and_flip_preview_update_selected_config() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.source_metadata.mark_ready(
                "video".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    width: Some(1920),
                    height: Some(1080),
                    ..SourceMetadata::default()
                },
            );

            assert!(root.rotate_selected_preview());
            assert!(root.toggle_selected_flip(FlipAxis::Horizontal));

            let config = &root.file_queue.file_by_id("video").unwrap().config;
            assert_eq!(config.rotation, "90");
            assert!(config.flip_horizontal);
            assert!(!config.flip_vertical);
        }

        #[test]
        fn apply_preview_crop_drag_updates_draft_without_persisting_config() {
            let mut root = FrameRoot::new();
            root.file_queue
                .add_file(FileItem::from_path("video", "/tmp/one.mp4", 1));
            root.source_metadata.mark_ready(
                "video".to_string(),
                SourceMetadata {
                    media_kind: Some(SourceKind::Video),
                    width: Some(1920),
                    height: Some(1080),
                    ..SourceMetadata::default()
                },
            );
            root.preview_crop_mode = true;
            root.preview_draft_crop = Some(CropRect {
                x: 0.10,
                y: 0.10,
                width: 0.50,
                height: 0.50,
            });

            assert!(
                !root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.50, y: 0.50 },)
            );
            assert!(
                root.apply_preview_crop_drag(DragHandle::Move, PreviewPoint { x: 0.60, y: 0.55 },)
            );

            let draft = root.preview_draft_crop.unwrap();
            assert!((draft.x - 0.20).abs() < 0.000_001);
            assert!((draft.y - 0.15).abs() < 0.000_001);
            assert_eq!(draft.width, 0.50);
            assert_eq!(draft.height, 0.50);
            assert_eq!(
                root.file_queue
                    .file_by_id("video")
                    .and_then(|file| file.config.crop.as_ref()),
                None
            );
        }
    }

    mod frame_window_options {
        use super::*;

        #[test]
        fn keeps_transparent_titlebar_without_positioning_native_controls() {
            let options = frame_window_options(Bounds::default());
            let titlebar = options
                .titlebar
                .as_ref()
                .expect("custom Frame controls still need a transparent native titlebar host");

            assert!(titlebar.appears_transparent);
            assert_eq!(titlebar.traffic_light_position, None);
        }

        #[test]
        fn preserves_original_minimum_window_size() {
            let options = frame_window_options(Bounds::default());

            assert_eq!(
                options.window_min_size,
                Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)))
            );
        }
    }

    mod visual_fixtures {
        use super::*;

        #[test]
        fn preview_ready_fixture_seeds_selected_video_metadata() {
            let mut root = FrameRoot::new();

            root.apply_visual_fixture(Some(VisualFixture::PreviewReady));

            assert_eq!(root.active_view, ActiveView::Workspace);
            assert_eq!(
                root.file_queue
                    .selected_file()
                    .map(|file| file.name.as_str()),
                Some("source_render.mov")
            );
            assert_eq!(
                root.selected_source_metadata()
                    .map(|metadata| metadata.source_kind()),
                Some(SourceKind::Video)
            );
        }

        #[test]
        fn preview_crop_fixture_enters_crop_mode() {
            let mut root = FrameRoot::new();

            root.apply_visual_fixture(Some(VisualFixture::PreviewCrop));

            assert!(root.preview_crop_mode);
            assert!(root.preview_draft_crop.is_some());
            assert_eq!(root.preview_crop_aspect, "1:1");
        }
    }

    mod button_state_colors {
        use super::*;

        #[test]
        fn default_button_hover_matches_original_frame_gray_400_90() {
            let colors = button_colors(ButtonVariant::Default, false, true);

            assert_eq!(
                colors.hover_background,
                theme::FRAME_GRAY_400.with_alpha(0.18)
            );
            assert_eq!(colors.active_background, colors.hover_background);
        }

        #[test]
        fn secondary_button_hover_matches_original_frame_gray_200() {
            let colors = button_colors(ButtonVariant::Secondary, false, true);

            assert_eq!(colors.hover_background, theme::FRAME_GRAY_200);
        }

        #[test]
        fn disabled_default_button_uses_original_half_alpha_background() {
            let colors = button_colors(ButtonVariant::Default, false, false);

            assert_eq!(colors.background, theme::FRAME_GRAY_400.with_alpha(0.10));
            assert_eq!(colors.opacity, 1.0);
        }

        #[test]
        fn disabled_secondary_button_keeps_original_whole_button_opacity() {
            let colors = button_colors(ButtonVariant::Secondary, false, false);

            assert_eq!(colors.background, theme::FRAME_GRAY_100);
            assert_eq!(colors.opacity, 0.5);
        }

        #[test]
        fn ghost_button_matches_original_transparent_icon_button_states() {
            let colors = button_colors(ButtonVariant::Ghost, false, true);

            assert_eq!(colors.background, theme::TRANSPARENT);
            assert_eq!(colors.hover_background, theme::FRAME_GRAY_100);
            assert_eq!(colors.foreground, theme::FRAME_GRAY_600);
            assert_eq!(colors.hover_foreground, theme::FOREGROUND);
        }
    }

    mod preview_shell {
        use super::*;

        fn settings_state<'a>(
            config: &'a ConversionConfig,
            metadata: Option<&'a SourceMetadata>,
            status: MetadataStatus,
        ) -> SettingsRenderState<'a> {
            SettingsRenderState {
                active_tab: SettingsTab::Source,
                config,
                metadata,
                metadata_status: status,
                metadata_error: None,
                settings_disabled: false,
                output_name: "",
            }
        }

        fn crop_state() -> PreviewCropRenderState {
            PreviewCropRenderState {
                crop_mode: false,
                draft_crop: None,
                applied_crop: None,
                crop_aspect: "free".to_string(),
                has_crop_dimensions: false,
                rotation: "0".to_string(),
                flip_horizontal: false,
                flip_vertical: false,
            }
        }

        #[test]
        fn ready_video_metadata_populates_timeline_labels() {
            let config = ConversionConfig::default();
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.4".to_string()),
                ..SourceMetadata::default()
            };
            let file = FileItem::from_path("video", "/tmp/render.mov", 1024);

            let state = preview_shell_state(
                Some(&file),
                settings_state(&config, Some(&metadata), MetadataStatus::Ready),
                crop_state(),
            );
            let labels = preview_timeline_labels(&state);

            assert_eq!(state.availability.media_kind, PreviewMediaKind::Video);
            assert!(preview_trim_enabled(&state));
            assert_eq!(labels.start, "00:00:00.000");
            assert_eq!(labels.end, "00:01:30.400");
            assert_eq!(labels.duration, "00:01:30.400");
        }

        #[test]
        fn ready_video_metadata_uses_configured_trim_bounds() {
            let config = ConversionConfig {
                start_time: Some("00:00:05.000".to_string()),
                end_time: Some("00:00:30.250".to_string()),
                ..ConversionConfig::default()
            };
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.4".to_string()),
                ..SourceMetadata::default()
            };
            let file = FileItem::from_path("video", "/tmp/render.mov", 1024);

            let state = preview_shell_state(
                Some(&file),
                settings_state(&config, Some(&metadata), MetadataStatus::Ready),
                crop_state(),
            );
            let labels = preview_timeline_labels(&state);

            assert_eq!(labels.start, "00:00:05.000");
            assert_eq!(labels.end, "00:00:30.250");
            assert_eq!(labels.duration, "00:00:25.250");
        }

        #[test]
        fn image_metadata_uses_placeholder_timeline_labels() {
            let config = ConversionConfig::default();
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Image),
                duration: Some("10.0".to_string()),
                ..SourceMetadata::default()
            };
            let file = FileItem::from_path("image", "/tmp/still.png", 1024);

            let state = preview_shell_state(
                Some(&file),
                settings_state(&config, Some(&metadata), MetadataStatus::Ready),
                crop_state(),
            );
            let labels = preview_timeline_labels(&state);

            assert_eq!(state.availability.media_kind, PreviewMediaKind::Image);
            assert!(state.availability.trim_disabled);
            assert_eq!(labels.start, "--:--:--.---");
            assert_eq!(labels.end, "--:--:--.---");
            assert_eq!(labels.duration, "--:--:--.---");
        }

        #[test]
        fn audio_metadata_hides_visual_controls() {
            let config = ConversionConfig::default();
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                duration: Some("00:00:12.500".to_string()),
                ..SourceMetadata::default()
            };

            let state = preview_shell_state(
                None,
                settings_state(&config, Some(&metadata), MetadataStatus::Ready),
                crop_state(),
            );

            assert_eq!(state.availability.media_kind, PreviewMediaKind::Audio);
            assert!(state.availability.hide_visual_controls);
            assert!(!preview_visual_controls_visible(&state));
            assert_eq!(preview_duration_seconds(Some(&metadata)), 12.5);
        }

        #[test]
        fn loading_metadata_keeps_preview_unknown() {
            let config = ConversionConfig::default();
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.0".to_string()),
                ..SourceMetadata::default()
            };

            let state = preview_shell_state(
                None,
                settings_state(&config, Some(&metadata), MetadataStatus::Loading),
                crop_state(),
            );

            assert_eq!(state.availability.media_kind, PreviewMediaKind::Unknown);
            assert!(state.availability.trim_disabled);
        }

        #[test]
        fn centered_offset_never_returns_negative_values() {
            assert_eq!(centered_offset(30.0, 6.0), 12.0);
            assert_eq!(centered_offset(6.0, 30.0), 0.0);
        }

        #[test]
        fn timeline_fraction_from_percent_clamps_to_track_range() {
            assert_eq!(timeline_fraction_from_percent(-25.0), 0.0);
            assert_eq!(timeline_fraction_from_percent(50.0), 0.5);
            assert_eq!(timeline_fraction_from_percent(125.0), 1.0);
        }

        #[test]
        fn timeline_slider_percent_from_bounds_clamps_pointer_to_track() {
            let bounds = Bounds {
                origin: point(px(10.0), px(0.0)),
                size: size(px(100.0), px(30.0)),
            };

            assert_eq!(
                timeline_slider_percent_from_bounds(point(px(60.0), px(0.0)), bounds),
                0.5
            );
            assert_eq!(
                timeline_slider_percent_from_bounds(point(px(-10.0), px(0.0)), bounds),
                0.0
            );
            assert_eq!(
                timeline_slider_percent_from_bounds(point(px(140.0), px(0.0)), bounds),
                1.0
            );
        }
    }
}
