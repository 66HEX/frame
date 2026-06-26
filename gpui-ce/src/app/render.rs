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
        if self.active_text_input.is_some() && self.focused_text_input_kind(window).is_none() {
            self.stop_text_input_cursor();
        }
        self.sync_preview_crop_for_selection(
            selected_file_id.as_deref(),
            &selected_config_snapshot,
        );
        let preview_crop =
            self.preview_crop_render_state(source_metadata.as_ref(), &selected_config_snapshot);
        let content = div().flex_1().p(px(CONTENT_PADDING));
        let content = match state.active_view {
            ActiveView::Workspace => {
                let output_name_focus = self
                    .settings_output_name_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let audio_bitrate_focus = self
                    .settings_audio_bitrate_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let video_width_focus = self
                    .settings_video_width_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let video_height_focus = self
                    .settings_video_height_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let video_bitrate_focus = self
                    .settings_video_bitrate_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let gif_loop_focus = self
                    .settings_gif_loop_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let metadata_title_focus = self
                    .settings_metadata_title_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let metadata_artist_focus = self
                    .settings_metadata_artist_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let metadata_album_focus = self
                    .settings_metadata_album_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let metadata_genre_focus = self
                    .settings_metadata_genre_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let metadata_date_focus = self
                    .settings_metadata_date_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let metadata_comment_focus = self
                    .settings_metadata_comment_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let preset_name_focus = self
                    .settings_preset_name_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let subtitle_font_color_focus = self
                    .settings_subtitle_font_color_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let subtitle_outline_color_focus = self
                    .settings_subtitle_outline_color_focus
                    .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                content.child(workspace_view(
                    &self.file_queue,
                    SettingsRenderState {
                        active_tab: self.settings_active_tab,
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
                        subtitle_popover: self.settings_subtitle_popover,
                        subtitle_font_color_draft: &self.subtitle_font_color_draft,
                        subtitle_outline_color_draft: &self.subtitle_outline_color_draft,
                        preset_name: &self.preset_name_draft,
                        preset_name_focus: Some(&preset_name_focus),
                        presets: &self.presets,
                        preset_notice: self.preset_notice.as_ref(),
                        subtitle_fonts: &self.subtitle_font_families,
                        available_encoders: &self.available_encoders,
                    },
                    preview_crop,
                    window,
                    cx,
                ))
            }
            ActiveView::Logs => content.child(logs_view(
                &self.file_queue,
                &self.conversion_events,
                &self.logs_scroll_handle,
                cx,
            )),
        };

        let mut root = div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(color(theme::BACKGROUND))
            .text_color(color(theme::FOREGROUND))
            .font_family(assets::FRAME_FONT_FAMILY)
            .font_weight(FontWeight::SEMIBOLD)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|root, _event: &MouseDownEvent, _window, cx| {
                    if root.settings_subtitle_popover.is_some() {
                        root.close_subtitle_popover();
                        cx.notify();
                    }
                }),
            )
            .on_drop(cx.listener(|root, paths: &ExternalPaths, _window, cx| {
                cx.stop_propagation();
                root.import_source_paths(paths.paths().to_vec(), cx);
            }))
            .child(titlebar(state, cx))
            .child(content);

        if self.is_settings_open {
            let value_focus = self
                .app_settings_value_focus
                .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            root = root.child(app_settings_sheet(
                self.max_concurrency,
                &self.max_concurrency_draft,
                self.max_concurrency_error.as_deref(),
                &value_focus,
                window,
                cx,
            ));
        }

        root
    }
}
