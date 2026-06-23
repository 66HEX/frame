use frame_core::events::ConversionEvent;
use frame_gpui_ce::{
    ActiveView, CONTENT_PADDING, FILE_LIST_ROW_SPAN, FILE_ROW_HEIGHT, FrameAppState,
    LEFT_COLUMN_SPAN, LEFT_GRID_ROWS, PANEL_HEADER_HEIGHT, PREVIEW_ROW_SPAN, RIGHT_COLUMN_SPAN,
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
    settings::{
        ConversionConfig, SettingsTab, SourceInfoSection, SourceMetadata, apply_output_container,
        apply_processing_mode, normalize_output_config, output_container_options,
        output_processing_mode_options, resolve_active_settings_tab, source_info_sections,
        visible_settings_tabs,
    },
    source_metadata::{
        MetadataStatus, SourceMetadataEntry, SourceMetadataStore, probe_source_metadata,
    },
    theme, visual_fixture_from_env_value,
};
use gpui::{
    App, Bounds, BoxShadow, ClickEvent, Context, ExternalPaths, FontWeight, InteractiveElement,
    IntoElement, KeyBinding, Menu, MenuItem, PathPromptOptions, Pixels, Render, Rgba, SharedString,
    StatefulInteractiveElement, UniformListScrollHandle, Window, WindowBackgroundAppearance,
    WindowBounds, WindowControlArea, WindowDecorations, WindowOptions, actions, div, hsla, point,
    prelude::*, px, size, svg, uniform_list,
};
use std::path::PathBuf;

actions!(frame_gpui_ce, [Quit]);

const FILE_LIST_ACTIONS_WIDTH: f32 = 64.0;
const FILE_LIST_ACTION_BUTTON_SIZE: f32 = 24.0;
const FILE_LIST_CHECKBOX_SIZE: f32 = 12.0;
const LOG_LINE_NUMBER_WIDTH: f32 = 32.0;
const LOG_LINE_HEIGHT: f32 = 24.0;

struct FrameRoot {
    active_view: ActiveView,
    file_queue: FileQueue,
    conversion_events: ConversionEventState,
    logs_scroll_handle: UniformListScrollHandle,
    last_log_scroll_target: Option<LogScrollTarget>,
    is_processing: bool,
    is_settings_open: bool,
    settings_active_tab: SettingsTab,
    conversion_config: ConversionConfig,
    source_metadata: SourceMetadataStore,
    output_name: String,
    next_file_sequence: u64,
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
            conversion_config: ConversionConfig::default(),
            source_metadata: SourceMetadataStore::default(),
            output_name: String::new(),
            next_file_sequence: 0,
        };

        root.apply_visual_fixture(visual_fixture_from_env_value(
            std::env::var("FRAME_GPUI_VISUAL_FIXTURE").ok().as_deref(),
        ));
        root
    }

    fn apply_visual_fixture(&mut self, fixture: Option<VisualFixture>) {
        let Some(VisualFixture::LogsActive) = fixture else {
            return;
        };

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
                            normalize_output_config(
                                &mut root.conversion_config,
                                selected_metadata.as_ref(),
                            );
                            root.settings_active_tab = resolve_active_settings_tab(
                                root.settings_active_tab,
                                &root.conversion_config,
                                selected_metadata.as_ref(),
                            );
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.app_state();
        let source_metadata_entry = self.selected_source_metadata_entry();
        let source_metadata = source_metadata_entry.metadata.clone();
        normalize_output_config(&mut self.conversion_config, source_metadata.as_ref());
        self.settings_active_tab = resolve_active_settings_tab(
            self.settings_active_tab,
            &self.conversion_config,
            source_metadata.as_ref(),
        );
        self.conversion_events
            .ensure_selected_log_file(&self.file_queue);
        self.update_log_scroll_target();
        let content = div().flex_1().p(px(CONTENT_PADDING));
        let content = match state.active_view {
            ActiveView::Workspace => content.child(workspace_view(
                &self.file_queue,
                SettingsRenderState {
                    active_tab: self.settings_active_tab,
                    config: &self.conversion_config,
                    metadata: source_metadata.as_ref(),
                    metadata_status: source_metadata_entry.status,
                    metadata_error: source_metadata_entry.error.as_deref(),
                    settings_disabled: self.file_queue.selected_file_locked(),
                    output_name: &self.output_name,
                },
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
            .font_weight(FontWeight::BLACK)
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
        .child(
            traffic_light("#ff5f56", "#e0443e")
                .id("titlebar-close")
                .window_control_area(WindowControlArea::Close)
                .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    window.remove_window();
                })),
        )
        .child(
            traffic_light("#ffbd2e", "#dea123")
                .id("titlebar-minimize")
                .window_control_area(WindowControlArea::Min)
                .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    window.minimize_window();
                })),
        )
        .child(
            traffic_light("#27c93f", "#1aab29")
                .id("titlebar-zoom")
                .window_control_area(WindowControlArea::Max)
                .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    window.zoom_window();
                })),
        )
}

fn traffic_light(fill: &'static str, border: &'static str) -> gpui::Div {
    div()
        .w(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .child(
            div()
                .w(px(12.0))
                .h(px(12.0))
                .rounded_full()
                .bg(parse_hex(fill))
                .border_1()
                .border_color(parse_hex(border)),
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonColors {
    background: theme::RgbaToken,
    hover_background: theme::RgbaToken,
    active_background: theme::RgbaToken,
    foreground: theme::RgbaToken,
}

fn button_colors(variant: ButtonVariant, selected: bool, enabled: bool) -> ButtonColors {
    let active_variant = matches!(variant, ButtonVariant::Default) || selected;
    if !enabled {
        let background = if active_variant {
            theme::FRAME_GRAY_400.with_alpha(0.10)
        } else {
            theme::FRAME_GRAY_100.with_alpha(0.50)
        };
        return ButtonColors {
            background,
            hover_background: background,
            active_background: background,
            foreground: theme::FOREGROUND.with_alpha(0.50),
        };
    }

    if active_variant {
        ButtonColors {
            background: theme::FRAME_GRAY_400,
            hover_background: theme::FRAME_GRAY_400.with_alpha(0.18),
            active_background: theme::FRAME_GRAY_400.with_alpha(0.16),
            foreground: theme::FOREGROUND,
        }
    } else {
        ButtonColors {
            background: theme::FRAME_GRAY_100,
            hover_background: theme::FRAME_GRAY_200,
            active_background: theme::FRAME_GRAY_400.with_alpha(0.14),
            foreground: theme::FOREGROUND,
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
        .when(enabled, |this| {
            this.hover(move |style| style.bg(color(colors.hover_background)).cursor_pointer())
        });

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
                    panel("PREVIEW")
                        .row_span(PREVIEW_ROW_SPAN)
                        .items_center()
                        .justify_center(),
                )
                .child(file_list_panel(file_queue, cx).row_span(FILE_LIST_ROW_SPAN)),
        )
        .child(settings_panel(settings, cx).col_span(RIGHT_COLUMN_SPAN))
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
            settings_choice_button(option.label, option.is_selected, is_enabled)
                .id(format!("output-mode-{}", option.mode.id()))
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if !is_enabled {
                        return;
                    }

                    let metadata = root.selected_source_metadata();
                    if apply_processing_mode(&mut root.conversion_config, metadata.as_ref(), mode) {
                        root.settings_active_tab = resolve_active_settings_tab(
                            root.settings_active_tab,
                            &root.conversion_config,
                            metadata.as_ref(),
                        );
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
            settings_choice_button(container.to_uppercase(), option.is_selected, is_enabled)
                .id(format!("output-container-{container}"))
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if !is_enabled {
                        return;
                    }

                    let metadata = root.selected_source_metadata();
                    let changed = apply_output_container(&mut root.conversion_config, &container)
                        | normalize_output_config(&mut root.conversion_config, metadata.as_ref());
                    if changed {
                        root.settings_active_tab = resolve_active_settings_tab(
                            root.settings_active_tab,
                            &root.conversion_config,
                            metadata.as_ref(),
                        );
                        cx.notify();
                    }
                })),
        );
    }
    grid
}

fn settings_choice_button(label: impl Into<String>, selected: bool, enabled: bool) -> gpui::Div {
    let colors = button_colors(ButtonVariant::Secondary, selected, enabled);

    div()
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
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(move |style| style.bg(color(colors.hover_background)).cursor_pointer())
        })
        .child(label.into())
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
        .h(px(height))
        .w(px(1.0))
        .bg(color(theme::BACKGROUND))
        .shadow(vertical_separator_shadows())
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

fn panel(label: &'static str) -> gpui::Div {
    div()
        .flex()
        .card_surface()
        .text_xs()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(label)
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

fn vertical_separator_shadows() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color(theme::FRAME_GRAY_100).into(),
        offset: point(px(1.0), px(0.0)),
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
        titlebar: None,
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
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

    mod frame_window_options {
        use super::*;

        #[test]
        fn disables_native_titlebar_when_custom_frame_controls_are_rendered() {
            let options = frame_window_options(Bounds::default());

            assert!(options.titlebar.is_none());
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

    mod button_state_colors {
        use super::*;

        #[test]
        fn default_button_hover_matches_original_frame_gray_400_90() {
            let colors = button_colors(ButtonVariant::Default, false, true);

            assert_eq!(
                colors.hover_background,
                theme::FRAME_GRAY_400.with_alpha(0.18)
            );
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
        }
    }
}
