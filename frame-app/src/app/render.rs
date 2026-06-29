use super::files::FileDropLifecycleProbe;
use super::*;

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
        let content = div().flex_1().p(px(CONTENT_PADDING));
        let content = match state.active_view {
            ActiveView::Workspace => {
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
                content.child(workspace_view(
                    &self.file_queue,
                    SettingsRenderState {
                        active_tab: self.settings_ui.active_tab,
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
                        subtitle_color_focuses: SettingsSubtitleColorInputFocuses {
                            font: Some(&subtitle_font_color_focus),
                            outline: Some(&subtitle_outline_color_focus),
                        },
                        subtitle_popover: self.subtitle_ui.popover,
                        subtitle_rendered_popover: self.subtitle_ui.rendered_popover,
                        subtitle_font_select_scroll_handle: &self
                            .subtitle_ui
                            .font_select_scroll_handle,
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
                    },
                    PreviewPanelProps {
                        canvas: preview_canvas,
                        crop: preview_crop,
                        overlay: preview_overlay,
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
            ActiveView::Logs => content.child(logs_view(
                &self.file_queue,
                &self.conversion_events,
                &self.logs_scroll_handle,
                self.logs_follow_tail,
                window,
                cx,
            )),
        };

        let mut root = div()
            .size_full()
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
                root.import_source_paths(paths.paths().to_vec(), cx);
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
            root = root.child(app_settings_sheet(
                AppSettingsSheetProps {
                    is_open: self.settings_ui.is_open,
                    current_max_concurrency: self.max_concurrency,
                    draft_max_concurrency: &self.settings_ui.max_concurrency_draft,
                    error: self.settings_ui.max_concurrency_error.as_deref(),
                    auto_update_check: self.auto_update_check,
                    update_status: &self.update_ui.status,
                    value_focus: &value_focus,
                },
                window,
                cx,
            ));
        }

        if self.drag_drop_ui.is_present {
            root = root.child(drag_drop_overlay(self.drag_drop_ui.is_open, window, cx));
        }

        if self.update_ui.dialog_present {
            root = root.child(update_dialog(
                self.update_ui.dialog_open,
                &self.update_ui.status,
                self.update_ui.dialog_info.as_deref(),
                window,
                cx,
            ));
        }

        root
    }
}
