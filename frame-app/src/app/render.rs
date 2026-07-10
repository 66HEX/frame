use super::accessibility::{APP_ROOT_FOCUS_ID, handle_tab_navigation};
use super::files::FileDropLifecycleProbe;
use super::preview_panel::{
    PreviewEditToolbarFocus, PreviewEditToolbarFocuses, PreviewToolFocuses, PreviewViewportFocuses,
};
use super::*;
use crate::app::chrome::UpdateDialogView;

impl Render for FrameRoot {
    #[expect(
        clippy::too_many_lines,
        reason = "The root GPUI render function assembles the full application shell from a single state snapshot."
    )]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.begin_accessibility_frame();
        let app_root_focus = self.ensure_focus(
            FrameFocusKey::Control(APP_ROOT_FOCUS_ID.to_string()),
            false,
            cx,
        );
        if window.focused(cx).is_none() {
            app_root_focus.focus(window, cx);
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
        let preview_runtime_request = self.selected_preview_runtime_request(&source_metadata_entry);
        if self.text_input_ui.active.is_some() && self.focused_text_input_kind(window).is_none() {
            self.stop_text_input_cursor();
        }
        self.sync_preview_crop_for_selection(
            selected_file_id.as_deref(),
            &selected_config_snapshot,
        );
        self.sync_preview_overlay_for_selection(
            selected_file_id.as_deref(),
            &selected_config_snapshot,
            cx,
        );
        self.sync_preview_canvas_for_selection(selected_file_id.as_deref());
        self.sync_preview_runtime_for_selection(preview_runtime_request, cx);
        self.sync_preview_playback_for_selection(
            selected_file_id.as_deref(),
            source_metadata.as_ref(),
            &selected_config_snapshot,
            cx,
        );
        self.sync_preview_canvas_auto_fit();
        let preview_crop =
            self.preview_crop_render_state(source_metadata.as_ref(), &selected_config_snapshot);
        let preview_overlay = self.preview_overlay_render_state();
        let preview_canvas = self.preview_canvas_render_state();
        let preview_playback = self.preview_playback_state();
        let preview_presentation = self.preview_ui.render_presentation;
        let preview_render_image = self.preview_render_image();
        let preview_runtime_error = self.preview_runtime_error();
        let preview_availability = preview_control_availability(PreviewControlInput {
            metadata_status: if source_metadata.is_some() {
                PreviewMetadataStatus::Ready
            } else {
                PreviewMetadataStatus::Idle
            },
            source_media_kind: source_metadata.as_ref().map(preview_source_media_kind),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: selected_config_snapshot.processing_mode,
            container: Some(selected_config_snapshot.container.as_str()),
        });
        let preview_visual_controls_enabled = preview_availability.media_kind
            != PreviewMediaKind::Unknown
            && !preview_availability.hide_visual_controls
            && !self.file_queue.selected_file_locked();
        let crop_tool_enabled = preview_visual_controls_enabled && preview_crop.has_crop_dimensions;
        let overlay_tool_enabled =
            preview_visual_controls_enabled && preview_availability.overlay_available;
        let preview_viewport_pan_enabled = preview_render_image.is_some()
            && preview_visual_controls_enabled
            && !preview_crop.crop_mode
            && !preview_overlay.overlay_mode;
        let preview_viewport_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-viewport".to_string()),
            preview_viewport_pan_enabled,
            cx,
        );
        let preview_crop_tool_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-tool-crop".to_string()),
            crop_tool_enabled,
            cx,
        );
        let preview_overlay_tool_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-tool-overlay".to_string()),
            overlay_tool_enabled,
            cx,
        );
        let crop_toolbar_active = preview_crop.crop_mode && preview_crop.draft_crop.is_some();
        let crop_toolbar_panel_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-crop-toolbar".to_string()),
            false,
            cx,
        );
        let crop_toolbar_first_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-crop-action-free".to_string()),
            crop_toolbar_active,
            cx,
        );
        let crop_toolbar_last_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-crop-action-apply".to_string()),
            crop_toolbar_active,
            cx,
        );
        let overlay_toolbar_active =
            preview_overlay.overlay_mode && preview_overlay.overlay.is_some();
        let overlay_toolbar_panel_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-overlay-toolbar".to_string()),
            false,
            cx,
        );
        let overlay_toolbar_first_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-overlay-replace".to_string()),
            overlay_toolbar_active,
            cx,
        );
        let overlay_toolbar_last_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-overlay-done".to_string()),
            overlay_toolbar_active,
            cx,
        );
        if !crop_toolbar_active
            && !overlay_toolbar_active
            && (crop_toolbar_first_focus.is_focused(window)
                || crop_toolbar_last_focus.is_focused(window))
        {
            if crop_tool_enabled {
                preview_crop_tool_focus.focus(window, cx);
            } else {
                app_root_focus.focus(window, cx);
            }
        }
        if !overlay_toolbar_active
            && !crop_toolbar_active
            && (overlay_toolbar_first_focus.is_focused(window)
                || overlay_toolbar_last_focus.is_focused(window))
        {
            if overlay_tool_enabled {
                preview_overlay_tool_focus.focus(window, cx);
            } else {
                app_root_focus.focus(window, cx);
            }
        }
        if !preview_viewport_pan_enabled && preview_viewport_focus.is_focused(window) {
            app_root_focus.focus(window, cx);
        }
        if crop_toolbar_active && !crop_toolbar_panel_focus.contains_focused(window, cx) {
            crop_toolbar_first_focus.focus(window, cx);
        }
        if overlay_toolbar_active && !overlay_toolbar_panel_focus.contains_focused(window, cx) {
            overlay_toolbar_first_focus.focus(window, cx);
        }
        let content = div().flex_1().p(px(CONTENT_PADDING));
        let active_content_view = if state.file_count == 0 {
            None
        } else {
            Some(state.active_view)
        };
        let content = match active_content_view {
            None => content.child(welcome_view(window, cx)),
            Some(ActiveView::Workspace) => {
                let output_name_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::OutputName, cx);
                let audio_bitrate_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::AudioBitrate, cx);
                let video_width_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::VideoCustomWidth, cx);
                let video_height_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::VideoCustomHeight, cx);
                let video_bitrate_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::VideoBitrate, cx);
                let gif_loop_focus = self.ensure_text_input_focus(FrameTextInputKind::GifLoop, cx);
                let preview_start_time_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::PreviewStartTime, cx);
                let preview_end_time_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::PreviewEndTime, cx);
                let metadata_title_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataTitle, cx);
                let metadata_artist_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataArtist, cx);
                let metadata_album_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataAlbum, cx);
                let metadata_genre_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataGenre, cx);
                let metadata_date_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataDate, cx);
                let metadata_comment_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataComment, cx);
                let preset_name_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::PresetName, cx);
                let subtitle_font_color_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::SubtitleFontColorHex, cx);
                let subtitle_outline_color_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::SubtitleOutlineColorHex, cx);
                let subtitles_copy_mode =
                    selected_config_snapshot.processing_mode == ProcessingMode::Copy;
                let subtitles_tab_active = self.settings_ui.active_tab == SettingsTab::Subtitles;
                let subtitles_enabled = subtitles_tab_active
                    && !self.file_queue.selected_file_locked()
                    && !subtitles_copy_mode;
                let subtitle_font_option_count = subtitle_font_options(
                    &selected_config_snapshot,
                    &self.subtitle_font_families,
                    !subtitles_enabled,
                )
                .len();
                let subtitle_font_select_enabled =
                    subtitles_enabled && subtitle_font_option_count > 0;
                let subtitle_font_size_option_count =
                    subtitle_font_size_options(&selected_config_snapshot, !subtitles_enabled).len();
                let subtitle_burn_file_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-burn-file".to_string()),
                    subtitles_enabled,
                    cx,
                );
                let subtitle_font_trigger_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-select".to_string()),
                    subtitle_font_select_enabled,
                    cx,
                );
                let subtitle_font_panel_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-options".to_string()),
                    false,
                    cx,
                );
                let subtitle_font_popover_active = subtitles_enabled
                    && self.subtitle_ui.popover == Some(SettingsSubtitlePopover::FontName);
                let subtitle_font_first_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-first-option".to_string()),
                    subtitle_font_popover_active && subtitle_font_option_count > 0,
                    cx,
                );
                let subtitle_font_last_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-last-option".to_string()),
                    subtitle_font_popover_active && subtitle_font_option_count > 1,
                    cx,
                );
                let subtitle_size_trigger_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-size-select".to_string()),
                    subtitles_enabled,
                    cx,
                );
                let subtitle_size_panel_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-size-options".to_string()),
                    false,
                    cx,
                );
                let subtitle_size_popover_active = subtitles_enabled
                    && self.subtitle_ui.popover == Some(SettingsSubtitlePopover::FontSize);
                let subtitle_size_first_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-size-first-option".to_string()),
                    subtitle_size_popover_active && subtitle_font_size_option_count > 0,
                    cx,
                );
                let subtitle_size_last_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-size-last-option".to_string()),
                    subtitle_size_popover_active && subtitle_font_size_option_count > 1,
                    cx,
                );
                let subtitle_font_color_trigger_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-color".to_string()),
                    subtitles_enabled,
                    cx,
                );
                let subtitle_font_color_panel_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-color-picker".to_string()),
                    false,
                    cx,
                );
                let subtitle_font_color_popover_active = subtitles_enabled
                    && self.subtitle_ui.popover == Some(SettingsSubtitlePopover::FontColor);
                let subtitle_font_color_sv_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-color-sv".to_string()),
                    subtitle_font_color_popover_active,
                    cx,
                );
                let subtitle_font_color_hue_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-font-color-hue".to_string()),
                    subtitle_font_color_popover_active,
                    cx,
                );
                let subtitle_outline_color_trigger_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-outline-color".to_string()),
                    subtitles_enabled,
                    cx,
                );
                let subtitle_outline_color_panel_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-outline-color-picker".to_string()),
                    false,
                    cx,
                );
                let subtitle_outline_color_popover_active = subtitles_enabled
                    && self.subtitle_ui.popover == Some(SettingsSubtitlePopover::OutlineColor);
                let subtitle_outline_color_sv_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-outline-color-sv".to_string()),
                    subtitle_outline_color_popover_active,
                    cx,
                );
                let subtitle_outline_color_hue_focus = self.ensure_focus(
                    FrameFocusKey::Control("settings-subtitle-outline-color-hue".to_string()),
                    subtitle_outline_color_popover_active,
                    cx,
                );
                if !subtitle_font_popover_active
                    && (subtitle_font_first_focus.is_focused(window)
                        || subtitle_font_last_focus.is_focused(window))
                {
                    if subtitle_font_select_enabled {
                        subtitle_font_trigger_focus.focus(window, cx);
                    } else {
                        app_root_focus.focus(window, cx);
                    }
                }
                if !subtitle_size_popover_active
                    && (subtitle_size_first_focus.is_focused(window)
                        || subtitle_size_last_focus.is_focused(window))
                {
                    if subtitles_enabled {
                        subtitle_size_trigger_focus.focus(window, cx);
                    } else {
                        app_root_focus.focus(window, cx);
                    }
                }
                if !subtitle_font_color_popover_active
                    && (subtitle_font_color_sv_focus.is_focused(window)
                        || subtitle_font_color_hue_focus.is_focused(window)
                        || subtitle_font_color_focus.is_focused(window))
                {
                    if subtitles_enabled {
                        subtitle_font_color_trigger_focus.focus(window, cx);
                    } else {
                        app_root_focus.focus(window, cx);
                    }
                }
                if !subtitle_outline_color_popover_active
                    && (subtitle_outline_color_sv_focus.is_focused(window)
                        || subtitle_outline_color_hue_focus.is_focused(window)
                        || subtitle_outline_color_focus.is_focused(window))
                {
                    if subtitles_enabled {
                        subtitle_outline_color_trigger_focus.focus(window, cx);
                    } else {
                        app_root_focus.focus(window, cx);
                    }
                }
                if subtitle_font_popover_active
                    && !subtitle_font_panel_focus.contains_focused(window, cx)
                {
                    subtitle_font_first_focus.focus(window, cx);
                }
                if subtitle_size_popover_active
                    && !subtitle_size_panel_focus.contains_focused(window, cx)
                {
                    subtitle_size_first_focus.focus(window, cx);
                }
                if subtitle_font_color_popover_active
                    && !subtitle_font_color_panel_focus.contains_focused(window, cx)
                {
                    subtitle_font_color_sv_focus.focus(window, cx);
                }
                if subtitle_outline_color_popover_active
                    && !subtitle_outline_color_panel_focus.contains_focused(window, cx)
                {
                    subtitle_outline_color_sv_focus.focus(window, cx);
                }
                let settings = SettingsRenderState {
                    active_tab: self.settings_ui.active_tab,
                    tooltip_visible_id: self.tooltip_ui.visible_id.as_deref(),
                    config: &selected_config_snapshot,
                    metadata: source_metadata.as_ref(),
                    metadata_status: source_metadata_entry.status,
                    metadata_error: source_metadata_entry.error.as_deref(),
                    settings_disabled: self.file_queue.selected_file_locked(),
                    output_name: &selected_output_name,
                    output_name_focus: Some(&output_name_focus),
                    audio_bitrate_focus: Some(&audio_bitrate_focus),
                    video_width_focus: Some(&video_width_focus),
                    video_height_focus: Some(&video_height_focus),
                    video_bitrate_focus: Some(&video_bitrate_focus),
                    gif_loop_focus: Some(&gif_loop_focus),
                    metadata_focuses: SettingsMetadataInputFocuses {
                        title: Some(&metadata_title_focus),
                        artist: Some(&metadata_artist_focus),
                        album: Some(&metadata_album_focus),
                        genre: Some(&metadata_genre_focus),
                        date: Some(&metadata_date_focus),
                        comment: Some(&metadata_comment_focus),
                    },
                    subtitle_focuses: SettingsSubtitleFocuses {
                        burn_file: Some(&subtitle_burn_file_focus),
                        font_select: SettingsSubtitleSelectFocuses {
                            trigger: Some(&subtitle_font_trigger_focus),
                            panel: Some(&subtitle_font_panel_focus),
                            first_option: Some(&subtitle_font_first_focus),
                            last_option: Some(&subtitle_font_last_focus),
                        },
                        font_size_select: SettingsSubtitleSelectFocuses {
                            trigger: Some(&subtitle_size_trigger_focus),
                            panel: Some(&subtitle_size_panel_focus),
                            first_option: Some(&subtitle_size_first_focus),
                            last_option: Some(&subtitle_size_last_focus),
                        },
                        font_color: SettingsSubtitleColorPopoverFocuses {
                            trigger: Some(&subtitle_font_color_trigger_focus),
                            panel: Some(&subtitle_font_color_panel_focus),
                            sv: Some(&subtitle_font_color_sv_focus),
                            hue: Some(&subtitle_font_color_hue_focus),
                        },
                        outline_color: SettingsSubtitleColorPopoverFocuses {
                            trigger: Some(&subtitle_outline_color_trigger_focus),
                            panel: Some(&subtitle_outline_color_panel_focus),
                            sv: Some(&subtitle_outline_color_sv_focus),
                            hue: Some(&subtitle_outline_color_hue_focus),
                        },
                    },
                    subtitle_color_focuses: SettingsSubtitleColorInputFocuses {
                        font: Some(&subtitle_font_color_focus),
                        outline: Some(&subtitle_outline_color_focus),
                    },
                    subtitle_popover: self.subtitle_ui.popover,
                    subtitle_rendered_popover: self.subtitle_ui.rendered_popover,
                    subtitle_font_select_scroll_handle: &self.subtitle_ui.font_select_scroll_handle,
                    subtitle_font_size_select_scroll_handle: &self
                        .subtitle_ui
                        .font_size_select_scroll_handle,
                    subtitle_font_color_draft: &self.subtitle_ui.font_color_draft,
                    subtitle_outline_color_draft: &self.subtitle_ui.outline_color_draft,
                    subtitle_font_color_hsv_draft: self.subtitle_ui.font_color_hsv_draft,
                    subtitle_outline_color_hsv_draft: self.subtitle_ui.outline_color_hsv_draft,
                    preset_name: &self.settings_ui.preset_name_draft,
                    preset_name_focus: Some(&preset_name_focus),
                    presets: &self.presets,
                    preset_notice: self.settings_ui.preset_notice.as_ref(),
                    subtitle_fonts: &self.subtitle_font_families,
                    available_encoders: &self.available_encoders,
                    available_filters: &self.available_filters,
                };
                content.child(workspace_view(
                    &self.file_queue,
                    &settings,
                    PreviewPanelProps {
                        canvas: preview_canvas,
                        crop: preview_crop,
                        overlay: preview_overlay,
                        viewport_focuses: PreviewViewportFocuses {
                            viewport: &preview_viewport_focus,
                            tools: PreviewToolFocuses {
                                crop: &preview_crop_tool_focus,
                                overlay: &preview_overlay_tool_focus,
                            },
                            edit_toolbars: PreviewEditToolbarFocuses {
                                crop: PreviewEditToolbarFocus {
                                    panel: &crop_toolbar_panel_focus,
                                    first: &crop_toolbar_first_focus,
                                    last: &crop_toolbar_last_focus,
                                },
                                overlay: PreviewEditToolbarFocus {
                                    panel: &overlay_toolbar_panel_focus,
                                    first: &overlay_toolbar_first_focus,
                                    last: &overlay_toolbar_last_focus,
                                },
                            },
                        },
                        timecode_focuses: PreviewTimecodeInputFocuses {
                            start: Some(&preview_start_time_focus),
                            end: Some(&preview_end_time_focus),
                        },
                        playback: preview_playback,
                        presentation: preview_presentation,
                        render_image: preview_render_image,
                        runtime_error: preview_runtime_error,
                    },
                    window,
                    cx,
                ))
            }
            Some(ActiveView::Logs) => content.child(logs_view(
                &self.file_queue,
                &self.conversion_events,
                &self.logs_scroll_handle,
                self.logs_follow_tail,
                self.copied_log_file_id.as_deref(),
                window,
                cx,
            )),
        };

        let mut root = div()
            .id(APP_ROOT_FOCUS_ID)
            .size_full()
            .track_focus(&app_root_focus)
            .tab_stop(false)
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .group(ROOT_DROP_GROUP)
            .bg(color(theme::BACKGROUND))
            .text_color(color(theme::FOREGROUND))
            .text_size(px(theme::TEXT_UI_SIZE))
            .font_family(assets::FRAME_FONT_FAMILY)
            .font_weight(theme::TEXT_WEIGHT_REGULAR)
            .font_features(assets::frame_font_features())
            .on_key_down(
                cx.listener(|_root, event: &gpui::KeyDownEvent, window, cx| {
                    handle_tab_navigation(event, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|root, _event: &MouseDownEvent, _window, cx| {
                    if root.subtitle_ui.popover.is_some() {
                        root.close_subtitle_popover();
                        cx.notify();
                    }
                }),
            )
            .on_drop(cx.listener(|root, paths: &ExternalPaths, _window, cx| {
                cx.stop_propagation();
                root.close_drag_drop_overlay();
                Self::import_source_paths(paths.paths().to_vec(), cx);
                cx.notify();
            }))
            .on_drag_move(cx.listener(
                |root, _event: &DragMoveEvent<ExternalPaths>, _window, cx| {
                    if root.open_drag_drop_overlay() {
                        cx.notify();
                    }
                },
            ))
            .child(titlebar(state, window, cx))
            .child(content)
            .child(FileDropLifecycleProbe { owner: cx.entity() });

        if self.settings_ui.is_present {
            let value_focus = self.ensure_text_input_focus(FrameTextInputKind::MaxConcurrency, cx);
            let output_directory_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-output-directory".to_string()),
                true,
                cx,
            );
            let auto_update_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-auto-update-check".to_string()),
                true,
                cx,
            );
            let check_now_enabled = !self.update_ui.status.is_busy();
            let check_now_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-check-now".to_string()),
                check_now_enabled,
                cx,
            );
            let download_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-download".to_string()),
                matches!(&self.update_ui.status, UpdateStatus::Available(_)),
                cx,
            );
            let skip_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-skip".to_string()),
                matches!(&self.update_ui.status, UpdateStatus::Available(_)),
                cx,
            );
            let install_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-install".to_string()),
                matches!(&self.update_ui.status, UpdateStatus::ReadyToInstall(_)),
                cx,
            );
            let dismiss_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-dismiss".to_string()),
                matches!(
                    &self.update_ui.status,
                    UpdateStatus::UpToDate | UpdateStatus::Disabled(_) | UpdateStatus::Error(_)
                ),
                cx,
            );
            let last_focus = match &self.update_ui.status {
                UpdateStatus::Available(_) => &skip_focus,
                UpdateStatus::ReadyToInstall(_) => &install_focus,
                UpdateStatus::UpToDate | UpdateStatus::Disabled(_) | UpdateStatus::Error(_) => {
                    &dismiss_focus
                }
                UpdateStatus::Idle => &check_now_focus,
                UpdateStatus::Checking
                | UpdateStatus::Downloading { .. }
                | UpdateStatus::Installing => &auto_update_focus,
            };
            let panel_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-panel".to_string()),
                false,
                cx,
            );
            let close_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-close".to_string()),
                true,
                cx,
            );
            if self.settings_ui.is_open && !panel_focus.contains_focused(window, cx) {
                close_focus.focus(window, cx);
            }
            root = root.child(app_settings_sheet(
                AppSettingsSheetProps {
                    is_open: self.settings_ui.is_open,
                    current_max_concurrency: self.max_concurrency,
                    draft_max_concurrency: &self.settings_ui.max_concurrency_draft,
                    error: self.settings_ui.max_concurrency_error.as_deref(),
                    default_output_directory: self
                        .default_output_directory
                        .as_deref()
                        .and_then(std::path::Path::to_str),
                    output_directory_error: self.settings_ui.output_directory_error.as_deref(),
                    auto_update_check: self.auto_update_check,
                    update_status: &self.update_ui.status,
                    value_focus: &value_focus,
                    output_directory_focus: &output_directory_focus,
                    auto_update_focus: &auto_update_focus,
                    check_now_focus: &check_now_focus,
                    download_focus: &download_focus,
                    skip_focus: &skip_focus,
                    install_focus: &install_focus,
                    dismiss_focus: &dismiss_focus,
                    panel_focus: &panel_focus,
                    close_focus: &close_focus,
                    last_focus,
                },
                window,
                cx,
            ));
        }

        if self.drag_drop_ui.is_present {
            root = root.child(drag_drop_overlay(self.drag_drop_ui.is_open, window, cx));
        }

        if self.update_ui.dialog_present {
            let panel_focus = self.ensure_focus(
                FrameFocusKey::Control("update-dialog-panel".to_string()),
                false,
                cx,
            );
            let close_focus = self.ensure_focus(
                FrameFocusKey::Control("update-dialog-close".to_string()),
                true,
                cx,
            );
            if self.update_ui.dialog_open && !panel_focus.contains_focused(window, cx) {
                close_focus.focus(window, cx);
            }
            root = root.child(update_dialog(
                self.update_ui.dialog_open,
                UpdateDialogView {
                    status: &self.update_ui.status,
                    info: self.update_ui.dialog_info.as_deref(),
                    release_notes_scroll_handle: &self.update_ui.release_notes_scroll_handle,
                    panel_focus: &panel_focus,
                    close_focus: &close_focus,
                },
                window,
                cx,
            ));
        }

        self.finish_accessibility_frame(window, cx, Some(&app_root_focus));

        linux_window_frame(root, window)
    }
}

#[cfg(target_os = "linux")]
fn linux_window_frame(root: gpui::Stateful<gpui::Div>, window: &Window) -> impl IntoElement {
    let should_draw_frame = matches!(
        window.window_decorations(),
        gpui::Decorations::Client { tiling }
            if !(tiling.top || tiling.right || tiling.bottom || tiling.left)
    );

    if !should_draw_frame {
        return div().size_full().child(root);
    }

    div().size_full().p(px(LINUX_WINDOW_FRAME_INSET)).child(
        root.rounded(px(theme::RADIUS_LG))
            .border_1()
            .border_color(color(theme::FRAME_GRAY_200))
            .shadow(linux_window_frame_shadows()),
    )
}

#[cfg(not(target_os = "linux"))]
fn linux_window_frame(root: gpui::Stateful<gpui::Div>, _window: &Window) -> impl IntoElement {
    root
}

#[cfg(target_os = "linux")]
fn linux_window_frame_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.28),
            offset: point(px(0.0), px(8.0)),
            blur_radius: px(8.0),
            spread_radius: px(-7.0),
            inset: false,
        },
        BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.18),
            offset: point(px(0.0), px(3.0)),
            blur_radius: px(5.0),
            spread_radius: px(-3.0),
            inset: false,
        },
        BoxShadow {
            color: color(theme::FRAME_GRAY_200).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: true,
        },
    ]
}
