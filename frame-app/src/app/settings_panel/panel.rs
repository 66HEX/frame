use super::super::accessibility::focus_visible_ring;
use super::{
    ButtonVariant, ClickEvent, Context, FluentBuilder, FrameRoot, FrameSurface, InteractiveElement,
    IntoElement, PANEL_HEADER_HEIGHT, ParentElement, SETTINGS_PANEL_PADDING,
    SETTINGS_TAB_BUTTON_SIZE, SETTINGS_TAB_ICON_SIZE, SettingsPresetsTabState, SettingsRenderState,
    SettingsSubtitlesTabState, SettingsTab, SettingsVideoInputFocuses, SourceKind,
    StatefulInteractiveElement, Styled, Window, apply_button_motion, button_colors,
    button_highlight_shadows, button_motion, color, div, frame_tooltip, icon_svg, mix_color,
    panel_bottom_separator, resolve_active_settings_tab, settings_audio_filters_tab,
    settings_audio_tab, settings_images_tab, settings_metadata_tab, settings_output_tab,
    settings_presets_tab, settings_section_label, settings_source_tab, settings_subtitles_tab,
    settings_tab_icon, settings_video_filters_tab, settings_video_tab, theme,
    visible_settings_tabs,
};
use crate::settings::source_kind_for;

pub(in crate::app) fn settings_panel(
    settings: &SettingsRenderState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = settings.palette;
    let active_tab =
        resolve_active_settings_tab(settings.active_tab, settings.config, settings.metadata);
    let visible_tabs = visible_settings_tabs(settings.config, settings.metadata);
    let mut tab_rail = div()
        .id("settings-tab-list")
        .role(gpui::Role::TabList)
        .aria_label("Settings sections")
        .flex()
        .items_center()
        .justify_start()
        .gap_1();
    for tab in &visible_tabs {
        tab_rail = tab_rail.child(settings_tab_button(
            *tab,
            active_tab == *tab,
            &visible_tabs,
            settings.tooltip_visible_id,
            palette,
            window,
            cx,
        ));
    }

    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .card_surface(palette)
        .child(
            div()
                .min_h(theme::ui_rem(PANEL_HEADER_HEIGHT))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .relative()
                .px_4()
                .child(tab_rail)
                .child(panel_bottom_separator(palette)),
        )
        .child(
            div()
                .id("settings-panel-body")
                .flex_1()
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .p(theme::ui_rem(SETTINGS_PANEL_PADDING))
                .child(settings_tab_content(active_tab, settings, window, cx)),
        )
}

pub(in crate::app) fn settings_tab_button(
    tab: SettingsTab,
    selected: bool,
    visible_tabs: &[SettingsTab],
    tooltip_visible_id: Option<&str>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let colors = button_colors(ButtonVariant::Secondary, selected, true, palette);
    let tab_id = format!("settings-tab-{}", tab.id());
    let motion = button_motion(format!("{tab_id}-hover"), window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    let background = if selected {
        mix_color(colors.background, colors.hover_background, hover_progress)
    } else {
        mix_color(palette.transparent, palette.fill_subtle, hover_progress)
    };
    let foreground = mix_color(
        if selected {
            palette.text_primary
        } else {
            palette.text_muted
        },
        palette.text_primary,
        hover_progress,
    );
    let keyboard_tabs = visible_tabs.to_vec();

    let button = div()
        .id(tab_id.clone())
        .role(gpui::Role::Tab)
        .aria_label(tab.label())
        .aria_selected(selected)
        .focusable()
        .tab_stop(true)
        .focus_visible(move |style| focus_visible_ring(style, palette))
        .group(tab_id)
        .w(theme::ui_rem(SETTINGS_TAB_BUTTON_SIZE))
        .h(theme::ui_rem(SETTINGS_TAB_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .when(selected, |this| {
            this.shadow(button_highlight_shadows(palette))
        })
        .hover(gpui::Styled::cursor_pointer)
        .active(move |style| style.bg(color(colors.active_background)))
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            root.settings_ui.active_tab = tab;
            cx.stop_propagation();
            cx.notify();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_tab) =
                    settings_tab_for_key(tab, &keyboard_tabs, event.keystroke.key.as_str())
                else {
                    return;
                };
                root.settings_ui.active_tab = next_tab;
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .child(icon_svg(
            settings_tab_icon(tab),
            SETTINGS_TAB_ICON_SIZE,
            foreground,
        ));
    let button = apply_button_motion(button, motion, true);

    frame_tooltip(
        tab.id(),
        tab.label(),
        tooltip_visible_id == Some(tab.id()),
        button,
        palette,
        window,
        cx,
    )
}

fn settings_tab_for_key(
    current: SettingsTab,
    visible_tabs: &[SettingsTab],
    key: &str,
) -> Option<SettingsTab> {
    let current_index = visible_tabs.iter().position(|tab| *tab == current)?;
    match key {
        "left" => Some(if current_index == 0 {
            *visible_tabs.last()?
        } else {
            visible_tabs[current_index - 1]
        }),
        "right" => Some(visible_tabs[(current_index + 1) % visible_tabs.len()]),
        "home" => visible_tabs.first().copied(),
        "end" => visible_tabs.last().copied(),
        _ => None,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Settings tab dispatch is intentionally explicit for routing clarity."
)]
pub(in crate::app) fn settings_tab_content(
    tab: SettingsTab,
    settings: &SettingsRenderState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = settings.palette;
    let content = div()
        .flex()
        .flex_col()
        .gap_4()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(palette.text_muted));

    match tab {
        SettingsTab::Source => content.child(settings_source_tab(
            settings.metadata,
            settings.metadata_status,
            settings.metadata_error,
            palette,
        )),
        SettingsTab::Output => content.child(settings_output_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.output_name,
            settings.output_name_focus,
            palette,
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
            palette,
            window,
            cx,
        )),
        SettingsTab::VideoFilters => content.child(settings_video_filters_tab(
            settings.config,
            settings.settings_disabled,
            source_kind_for(settings.metadata) == SourceKind::Image,
            settings.available_filters,
            palette,
            window,
            cx,
        )),
        SettingsTab::Images => content.child(settings_images_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.video_width_focus,
            settings.video_height_focus,
            palette,
            window,
            cx,
        )),
        SettingsTab::Audio => content.child(settings_audio_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.available_encoders,
            settings.audio_bitrate_focus,
            palette,
            window,
            cx,
        )),
        SettingsTab::AudioFilters => content.child(settings_audio_filters_tab(
            settings.config,
            settings.settings_disabled,
            settings.available_filters,
            palette,
            window,
            cx,
        )),
        SettingsTab::Subtitles => content.child(settings_subtitles_tab(
            SettingsSubtitlesTabState {
                config: settings.config,
                metadata: settings.metadata,
                settings_disabled: settings.settings_disabled,
                available_encoders: settings.available_encoders,
                subtitle_fonts: settings.subtitle_fonts,
                focuses: settings.subtitle_focuses,
                external_language_focus: settings.external_subtitle_language_focus,
                external_title_focus: settings.external_subtitle_title_focus,
                external_track_index: settings.external_subtitle_track_index,
                mode: settings.subtitle_mode,
                color_focuses: settings.subtitle_color_focuses,
                active_popover: settings.subtitle_popover,
                rendered_popover: settings.subtitle_rendered_popover,
                font_select_scroll_handle: settings.subtitle_font_select_scroll_handle,
                font_size_select_scroll_handle: settings.subtitle_font_size_select_scroll_handle,
                font_color_draft: settings.subtitle_font_color_draft,
                outline_color_draft: settings.subtitle_outline_color_draft,
                font_color_hsv_draft: settings.subtitle_font_color_hsv_draft,
                outline_color_hsv_draft: settings.subtitle_outline_color_hsv_draft,
                palette,
            },
            window,
            cx,
        )),
        SettingsTab::Metadata => content.child(settings_metadata_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.metadata_focuses,
            palette,
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
                palette,
            },
            window,
            cx,
        )),
    }
}

pub(in crate::app) fn settings_section(
    label: &'static str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(settings_section_label(label, palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABS: &[SettingsTab] = &[SettingsTab::Source, SettingsTab::Output, SettingsTab::Video];

    #[test]
    fn settings_tab_for_key_handles_the_navigation_matrix() {
        let cases = [
            (SettingsTab::Source, "left", Some(SettingsTab::Video)),
            (SettingsTab::Video, "right", Some(SettingsTab::Source)),
            (SettingsTab::Output, "home", Some(SettingsTab::Source)),
            (SettingsTab::Output, "end", Some(SettingsTab::Video)),
            (SettingsTab::Output, "space", None),
        ];

        for (current, key, expected) in cases {
            assert_eq!(
                settings_tab_for_key(current, TABS, key),
                expected,
                "navigation failed for current={current:?}, key={key:?}"
            );
        }
    }
}
