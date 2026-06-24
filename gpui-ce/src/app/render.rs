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
