use super::*;

pub(in crate::app) fn settings_panel(
    settings: SettingsRenderState<'_>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
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
                .child(settings_tab_content(active_tab, settings, window, cx)),
        )
}

pub(in crate::app) fn settings_tab_button(
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
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
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

pub(in crate::app) fn settings_tab_content(
    tab: SettingsTab,
    settings: SettingsRenderState<'_>,
    window: &Window,
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
            settings.output_name_focus,
            window,
            cx,
        )),
        SettingsTab::Video => content.child(settings_video_tab(
            settings.config,
            settings.settings_disabled,
            settings.available_encoders,
            SettingsVideoInputFocuses {
                width: settings.video_width_focus,
                height: settings.video_height_focus,
                bitrate: settings.video_bitrate_focus,
                gif_loop: settings.gif_loop_focus,
            },
            window,
            cx,
        )),
        SettingsTab::Images => content.child(settings_images_tab(
            settings.config,
            settings.settings_disabled,
            settings.available_encoders,
            settings.video_width_focus,
            settings.video_height_focus,
            window,
            cx,
        )),
        SettingsTab::Audio => content.child(settings_audio_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.available_encoders,
            settings.audio_bitrate_focus,
            window,
            cx,
        )),
        SettingsTab::Subtitles => content.child(settings_subtitles_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.subtitle_fonts,
            settings.subtitle_color_focuses,
            settings.subtitle_popover,
            settings.subtitle_font_color_draft,
            settings.subtitle_outline_color_draft,
            window,
            cx,
        )),
        SettingsTab::Metadata => content.child(settings_metadata_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.metadata_focuses,
            window,
            cx,
        )),
        SettingsTab::Presets => content.child(settings_presets_tab(
            SettingsPresetsTabState {
                config: settings.config,
                metadata: settings.metadata,
                settings_disabled: settings.settings_disabled,
                preset_name: settings.preset_name,
                preset_name_focus: settings.preset_name_focus,
                presets: settings.presets,
                notice: settings.preset_notice,
            },
            window,
            cx,
        )),
    }
}

pub(in crate::app) fn settings_section(label: &'static str) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(settings_section_label(label))
}
