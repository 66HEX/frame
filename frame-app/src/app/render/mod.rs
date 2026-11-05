use crate::app::accessibility::{APP_ROOT_FOCUS_ID, FrameFocusKey, handle_tab_navigation};
use crate::app::chrome::titlebar;
use crate::app::input::FrameTextInputKind;
use crate::app::inspector::state::{SettingsMetadataInputFocuses, SettingsRenderState};
use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorInputFocuses, SettingsSubtitleColorPopoverFocuses,
    SettingsSubtitleFocuses, SettingsSubtitleSelectFocuses,
};
use crate::app::layout::CONTENT_PADDING;
use crate::app::logs::view::logs_view;
use crate::app::model::ActiveView;
use crate::app::preferences::state::AppearancePopover;
use crate::app::preview;
use crate::app::preview::{
    PreviewEditToolbarFocus, PreviewEditToolbarFocuses, PreviewPanelProps,
    PreviewTimecodeInputFocuses, PreviewToolFocuses, PreviewViewportFocuses,
    preview_source_media_kind,
};
use crate::app::primitives::color;
use crate::app::queue::import::FileDropLifecycleProbe;
use crate::app::queue::state::ROOT_DROP_GROUP;
use crate::app::root::FrameRoot;
use crate::app::workspace::{welcome_view, workspace_view};
use crate::preview::{
    MetadataStatus as PreviewMetadataStatus, PreviewControlInput, preview_control_availability,
};
use crate::settings::ConversionConfig;
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{
    Context, DragMoveEvent, ExternalPaths, IntoElement, MouseButton, MouseDownEvent, Render,
    Window, div, px,
};
use preview_focus::{PreviewFocusContext, PreviewFocusHandles};
use subtitle_focus::{SubtitleFocusContext, SubtitleFocusHandles};
use window_frame::linux_window_frame;

mod overlays;
mod preview_focus;
mod subtitle_focus;
mod window_frame;

impl Render for FrameRoot {
    #[expect(
        clippy::too_many_lines,
        reason = "The root GPUI render function assembles the full application shell from a single state snapshot."
    )]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(
            crate::appearance::BASE_REM_PX * self.appearance.ui_scale.factor()
        ));
        let palette = theme::palette(self.appearance.color_theme);
        self.begin_accessibility_frame();
        self.register_shell_focus(window, cx);
        let app_root_focus = self.ensure_focus(
            FrameFocusKey::Control(APP_ROOT_FOCUS_ID.to_string()),
            false,
            cx,
        );
        if window.focused(cx).is_none() {
            app_root_focus.focus(window, cx);
        }

        self.reconcile_text_input_focus(window, cx);

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
        if cx.reduce_motion() {
            self.settle_preview_canvas_animation();
        }
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
        let PreviewFocusHandles {
            preview_viewport_focus,
            preview_crop_tool_focus,
            preview_overlay_tool_focus,
            crop_width_focus,
            crop_height_focus,
            crop_option_focuses,
            crop_unit_focus,
            crop_unit_option_focuses,
            crop_custom_values,
            crop_toolbar_panel_focus,
            crop_toolbar_first_focus,
            crop_toolbar_last_focus,
            overlay_toolbar_panel_focus,
            overlay_toolbar_first_focus,
            overlay_toolbar_last_focus,
        } = self.prepare_preview_focus(
            PreviewFocusContext {
                preview_availability: &preview_availability,
                preview_crop: &preview_crop,
                preview_overlay: &preview_overlay,
                preview_render_image: &preview_render_image,
                app_root_focus: &app_root_focus,
            },
            window,
            cx,
        );
        // Let the workspace shrink to the window even when settings have long content.
        let content = div().flex_1().min_h_0().p(theme::ui_rem(CONTENT_PADDING));
        let active_content_view = if state.file_count == 0 {
            None
        } else {
            Some(state.active_view)
        };
        let content = match active_content_view {
            None => content.child(welcome_view(palette, window, cx)),
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
                let preview_start_time_value =
                    self.text_input_value(FrameTextInputKind::PreviewStartTime);
                let preview_end_time_value =
                    self.text_input_value(FrameTextInputKind::PreviewEndTime);
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
                let metadata_service_name_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataServiceName, cx);
                let metadata_service_provider_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::MetadataServiceProvider, cx);
                let preset_name_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::PresetName, cx);
                let external_subtitle_language_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::ExternalSubtitleLanguage, cx);
                let external_subtitle_title_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::ExternalSubtitleTitle, cx);
                let subtitle_font_color_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::SubtitleFontColorHex, cx);
                let subtitle_outline_color_focus =
                    self.ensure_text_input_focus(FrameTextInputKind::SubtitleOutlineColorHex, cx);
                let SubtitleFocusHandles {
                    subtitle_burn_file_focus,
                    subtitle_add_external_focus,
                    subtitle_font_trigger_focus,
                    subtitle_font_panel_focus,
                    subtitle_font_first_focus,
                    subtitle_font_last_focus,
                    subtitle_size_trigger_focus,
                    subtitle_size_panel_focus,
                    subtitle_size_first_focus,
                    subtitle_size_last_focus,
                    subtitle_font_color_trigger_focus,
                    subtitle_font_color_panel_focus,
                    subtitle_font_color_sv_focus,
                    subtitle_font_color_hue_focus,
                    subtitle_outline_color_trigger_focus,
                    subtitle_outline_color_panel_focus,
                    subtitle_outline_color_sv_focus,
                    subtitle_outline_color_hue_focus,
                } = self.prepare_subtitle_focus(
                    SubtitleFocusContext {
                        selected_config_snapshot: &selected_config_snapshot,
                        app_root_focus: &app_root_focus,
                        subtitle_font_color_focus: &subtitle_font_color_focus,
                        subtitle_outline_color_focus: &subtitle_outline_color_focus,
                    },
                    window,
                    cx,
                );
                let crop_settings_preview = self.crop_settings_preview();
                let settings = SettingsRenderState {
                    palette,
                    active_tab: self.settings_ui.active_tab,
                    tooltip_visible_id: self.tooltip_ui.visible_id.as_deref(),
                    config: crop_settings_preview
                        .as_ref()
                        .unwrap_or(&selected_config_snapshot),
                    crop_editing: self.preview_ui.crop_mode,
                    metadata: source_metadata.as_ref(),
                    metadata_status: source_metadata_entry.status,
                    metadata_error: source_metadata_entry.error.as_deref(),
                    settings_disabled: self.file_queue.selected_file_locked(),
                    output_name: &selected_output_name,
                    output_name_focus: Some(&output_name_focus),
                    audio_bitrate_focus: Some(&audio_bitrate_focus),
                    // Draft crop dimensions are display-only until Apply commits them.
                    video_width_focus: (!self.preview_ui.crop_mode).then_some(&video_width_focus),
                    video_height_focus: (!self.preview_ui.crop_mode).then_some(&video_height_focus),
                    video_bitrate_focus: Some(&video_bitrate_focus),
                    gif_loop_focus: Some(&gif_loop_focus),
                    metadata_focuses: SettingsMetadataInputFocuses {
                        title: Some(&metadata_title_focus),
                        artist: Some(&metadata_artist_focus),
                        album: Some(&metadata_album_focus),
                        genre: Some(&metadata_genre_focus),
                        date: Some(&metadata_date_focus),
                        comment: Some(&metadata_comment_focus),
                        service_name: Some(&metadata_service_name_focus),
                        service_provider: Some(&metadata_service_provider_focus),
                    },
                    subtitle_focuses: SettingsSubtitleFocuses {
                        add_external_files: Some(&subtitle_add_external_focus),
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
                    external_subtitle_language_focus: Some(&external_subtitle_language_focus),
                    external_subtitle_title_focus: Some(&external_subtitle_title_focus),
                    external_subtitle_track_index: self.subtitle_ui.external_track_index,
                    subtitle_mode: self.subtitle_ui.mode,
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
                    &self.file_list_scroll_handle,
                    &settings,
                    PreviewPanelProps {
                        canvas: preview_canvas,
                        crop: preview_crop,
                        overlay: preview_overlay,
                        viewport_focuses: PreviewViewportFocuses {
                            viewport: &preview_viewport_focus,
                            crop_options: preview::PreviewCropOptionsProps {
                                popover: self.preview_ui.crop_options.popover,
                                custom: self.preview_ui.crop_options.custom,
                                ratio: self.preview_ui.crop_options.is_ratio,
                                width: &self.preview_ui.crop_options.width,
                                height: &self.preview_ui.crop_options.height,
                                custom_values: crop_custom_values,
                                scroll: &self.preview_ui.crop_options.scroll,
                                width_focus: &crop_width_focus,
                                height_focus: &crop_height_focus,
                                option_focuses: &crop_option_focuses,
                                unit_focus: &crop_unit_focus,
                                unit_option_focuses: &crop_unit_option_focuses,
                            },
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
                            start_value: &preview_start_time_value,
                            end_value: &preview_end_time_value,
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
                palette,
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
            .bg(color(palette.background))
            .text_color(color(palette.text_primary))
            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
            .font_family(assets::FRAME_FONT_FAMILY)
            .line_height(gpui::relative(theme::metrics::TEXT_LINE_HEIGHT))
            .font_weight(theme::TEXT_WEIGHT_REGULAR)
            .font_features(assets::frame_font_features())
            .on_action(cx.listener(Self::increase_ui_scale))
            .on_action(cx.listener(Self::decrease_ui_scale))
            .on_action(cx.listener(Self::reset_ui_scale))
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
                    if root.settings_ui.theme_popover.is_open() {
                        root.close_app_settings_appearance_popover(AppearancePopover::Theme);
                        cx.notify();
                    }
                    if root.settings_ui.ui_scale_popover.is_open() {
                        root.close_app_settings_appearance_popover(AppearancePopover::UiScale);
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
                    if !root.update_installation_in_progress() && root.open_drag_drop_overlay() {
                        cx.notify();
                    }
                },
            ))
            .child(titlebar(state, palette, window, cx))
            .child(content)
            .child(FileDropLifecycleProbe { owner: cx.entity() });

        root = self.render_overlays(root, palette, window, cx);

        self.finish_accessibility_frame(window, cx, Some(&app_root_focus));

        linux_window_frame(root, window, palette)
    }
}
