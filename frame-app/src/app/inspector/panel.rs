use crate::app::accessibility::{TabFocusGroup, tab_focus_base, tab_focus_ring};
use crate::app::components::frame_tooltip;
use crate::app::components::{FrameStyle, frame_section};
use crate::app::inspector::audio::settings_audio_tab;
use crate::app::inspector::images::settings_images_tab;
use crate::app::inspector::metadata::settings_metadata_tab;
use crate::app::inspector::output::settings_output_tab;
use crate::app::inspector::presets::{SettingsPresetsTabState, settings_presets_tab};
use crate::app::inspector::shared::settings_tab_icon;
use crate::app::inspector::source::settings_source_tab;
use crate::app::inspector::state::{SettingsRenderState, SettingsVideoInputFocuses};
use crate::app::inspector::{
    SettingsSubtitlesTabState, settings_audio_filters_tab, settings_subtitles_tab,
    settings_video_filters_tab, settings_video_tab,
};
use crate::app::layout::{
    PANEL_HEADER_HEIGHT, SETTINGS_PANEL_PADDING, SETTINGS_TAB_BUTTON_SIZE, SETTINGS_TAB_ICON_SIZE,
};
use crate::app::primitives::{FrameSurface, button_mouse_down, color, panel_bottom_separator};
use crate::app::root::FrameRoot;
use crate::settings::{
    SettingsTab, SourceKind, resolve_active_settings_tab, source_kind_for, visible_settings_tabs,
};
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    ClickEvent, Context, InteractiveElement, IntoElement, StatefulInteractiveElement, Window, div,
};

pub(in crate::app) fn settings_panel(
    settings: &SettingsRenderState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = settings.palette;
    let active_tab =
        resolve_active_settings_tab(settings.active_tab, settings.config, settings.metadata);
    let visible_tabs = visible_settings_tabs(settings.config, settings.metadata);
    let focuses = TabFocusGroup::render(
        "settings-tabs-focus",
        active_tab.id().into(),
        visible_tabs.iter().map(|tab| tab.id().into()),
        window,
        cx,
    );
    let scroll = window
        .use_keyed_state(
            format!("settings-panel-{}-scroll", active_tab.id()),
            cx,
            |_, _| gpui::ScrollHandle::new(),
        )
        .read(cx)
        .clone();
    let key_scroll = scroll.clone();
    let key_focus = focuses.handle(active_tab.id());
    let mut tab_rail = crate::app::components::frame_tabs_list(palette)
        .id("settings-tab-list")
        .role(gpui::Role::TabList)
        .aria_label("Settings sections")
        .flex()
        .items_center()
        .justify_start();
    for tab in &visible_tabs {
        tab_rail = tab_rail.child(settings_tab_button(
            *tab,
            &focuses,
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
        .on_key_down(move |event, window, cx| {
            // Read-only panels remain keyboard-scrollable from their selected
            // tab, without inserting a container into the Tab sequence.
            if key_focus.is_focused(window) {
                crate::app::accessibility::handle_scroll_area_key(event, &key_scroll, window, cx);
            }
        })
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
                .debug_selector(|| "settings-panel-body".into())
                .role(gpui::Role::TabPanel)
                .aria_label(active_tab.label())
                .flex_1()
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .p(theme::ui_rem(SETTINGS_PANEL_PADDING))
                .child(settings_tab_content(active_tab, settings, window, cx)),
        )
}

pub(in crate::app) fn settings_tab_button(
    tab: SettingsTab,
    focuses: &TabFocusGroup,
    visible_tabs: &[SettingsTab],
    tooltip_visible_id: Option<&str>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let selected = focuses.selected(tab.id());
    let focus = focuses.handle(tab.id());
    let click_focus = focus.clone();
    let keyboard_focuses: Vec<_> = visible_tabs
        .iter()
        .map(|tab| (*tab, focuses.handle(tab.id())))
        .collect();
    let tab_id = format!("settings-tab-{}", tab.id());
    let keyboard_tabs = visible_tabs.to_vec();

    let button = div()
        .id(tab_id.clone())
        .debug_selector({
            let id = tab_id.clone();
            move || id
        })
        .role(gpui::Role::Tab)
        .aria_label(tab.label())
        .aria_selected(selected)
        .track_focus(&focus)
        .tab_stop(selected)
        .map(|element| tab_focus_base(element, palette))
        .focus_visible_style(move |style| tab_focus_ring(style, palette))
        .transitions(|t| {
            t.ring(crate::app::motion::interaction_motion())
                .outline_color(crate::app::motion::interaction_motion())
                .bg(crate::app::motion::interaction_motion())
                .text_color(crate::app::motion::interaction_motion())
                .border_color(crate::app::motion::interaction_motion())
        })
        .group(tab_id)
        .w(theme::ui_rem(SETTINGS_TAB_BUTTON_SIZE))
        .h(theme::ui_rem(theme::metrics::TAB_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .tab_surface(selected, palette)
        .hover_style(move |style| {
            style
                .cursor_default()
                .text_color(color(palette.text_primary))
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
            click_focus.focus(window, cx);
            root.settings_ui.active_tab = tab;
            cx.stop_propagation();
            cx.notify();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let Some(next_tab) =
                    settings_tab_for_key(tab, &keyboard_tabs, event.keystroke.key.as_str())
                else {
                    return;
                };
                if let Some((_, focus)) = keyboard_focuses.iter().find(|(tab, _)| *tab == next_tab)
                {
                    focus.focus(window, cx);
                }
                root.settings_ui.active_tab = next_tab;
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .child(
            gpui::svg()
                .path(settings_tab_icon(tab))
                .size(theme::ui_rem(SETTINGS_TAB_ICON_SIZE)),
        );
    let button = button.on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
        button_mouse_down(true, window, cx);
    });

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
    let content = crate::app::components::frame_field_group()
        .flex_shrink_0()
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
                resolution_locked: settings.crop_editing,
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
            settings.settings_disabled,
            settings.crop_editing,
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
    frame_section(theme::ui_text(label), palette)
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
