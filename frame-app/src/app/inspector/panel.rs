use crate::app::accessibility::TabFocusGroup;
use crate::app::components::{
    FrameTabIndicator, frame_collapsible, frame_field, frame_label, frame_section,
    frame_tab_trigger,
};
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
use crate::app::layout::{PANEL_HEADER_HEIGHT, SETTINGS_PANEL_PADDING, SETTINGS_TAB_ICON_SIZE};
use crate::app::primitives::{FrameSurface, button_mouse_down, color, panel_bottom_separator};
use crate::app::root::FrameRoot;
use crate::settings::{
    ALL_SETTINGS_TABS, OutputSection, SettingsTab, SourceKind, source_kind_for,
    visible_output_sections,
};
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{ClickEvent, Context, InteractiveElement, StatefulInteractiveElement, Window, div};

pub(in crate::app) fn settings_panel(
    settings: &SettingsRenderState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = settings.palette;
    let active_tab = settings.active_tab;
    let visible_tabs = ALL_SETTINGS_TABS;
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
    let indicator = FrameTabIndicator::new("settings-tab-list", window, cx);
    let mut tab_rail = crate::app::components::frame_tabs_list(palette)
        .id("settings-tab-list")
        .debug_selector(|| "settings-tab-list".to_owned())
        .role(gpui::Role::TabList)
        .aria_label("Inspector")
        .w_full()
        .flex()
        .items_center()
        .justify_start()
        .child(indicator.background(palette));
    for tab in &visible_tabs {
        tab_rail = tab_rail.child(settings_tab_button(
            *tab,
            &focuses,
            &visible_tabs,
            palette,
            &indicator,
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
                .when(active_tab != SettingsTab::Output, |body| {
                    body.p(theme::ui_rem(SETTINGS_PANEL_PADDING))
                })
                .child(settings_tab_content(active_tab, settings, window, cx)),
        )
}

pub(in crate::app) fn settings_tab_button(
    tab: SettingsTab,
    focuses: &TabFocusGroup,
    visible_tabs: &[SettingsTab],
    palette: &'static theme::ThemePalette,
    indicator: &FrameTabIndicator,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let selected = focuses.selected(tab.id());
    let focus = focuses.handle(tab.id());
    let click_focus = focus.clone();
    let keyboard_focuses: Vec<_> = visible_tabs
        .iter()
        .map(|tab| (*tab, focuses.handle(tab.id())))
        .collect();
    let tab_id = format!("settings-tab-{}", tab.id());
    let keyboard_tabs = visible_tabs.to_vec();

    let button = frame_tab_trigger(
        tab_id,
        tab.label().to_owned(),
        selected,
        true,
        &focus,
        palette,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
        click_focus.focus(window, cx);
        root.settings_ui.active_tab = tab;
        root.subtitle_ui.popover = None;
        root.subtitle_ui.rendered_popover = None;
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
            if let Some((_, focus)) = keyboard_focuses.iter().find(|(tab, _)| *tab == next_tab) {
                focus.focus(window, cx);
            }
            root.settings_ui.active_tab = next_tab;
            root.subtitle_ui.popover = None;
            root.subtitle_ui.rendered_popover = None;
            cx.stop_propagation();
            cx.notify();
        }),
    )
    .child(
        gpui::svg()
            .path(settings_tab_icon(tab))
            .size(theme::ui_rem(SETTINGS_TAB_ICON_SIZE)),
    )
    .child(tab.label());
    indicator
        .track(button, selected.then(|| tab.id().to_owned()))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
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
        SettingsTab::Metadata => content.child(settings_source_tab(
            settings.metadata,
            settings.metadata_status,
            settings.metadata_error,
            palette,
        )),

        SettingsTab::Output => {
            let mut sections = div().flex().flex_col().w_full();
            if settings.apply_all != super::apply_all::ApplyAllAvailability::Hidden {
                sections = sections.child(super::apply_all::apply_all_control(
                    settings.apply_all,
                    palette,
                    window,
                    cx,
                ));
            }
            for section in visible_output_sections(settings.config, settings.metadata) {
                let open = settings.expanded_output_sections.contains(&section);
                // Do not construct controls for closed sections: no hidden focus stops.
                let body = open.then(|| settings_output_section(section, settings, window, cx));
                sections = sections.child(frame_collapsible(
                    format!("output-section-{}", section.id()),
                    section.label(),
                    open,
                    body,
                    move |root, _, _| {
                        if !root.settings_ui.expanded_output_sections.remove(&section) {
                            root.settings_ui.expanded_output_sections.insert(section);
                        }
                        root.subtitle_ui.popover = None;
                        root.subtitle_ui.rendered_popover = None;
                    },
                    palette,
                    window,
                    cx,
                ));
            }
            content.gap_0().child(sections)
        }
        SettingsTab::Presets => content.child(settings_presets_tab(
            SettingsPresetsTabState {
                config: settings.config,
                metadata: settings.metadata,
                settings_disabled: settings.settings_disabled,
                preset_name: settings.preset_name,
                preset_name_focus: settings.preset_name_focus,
                presets: settings.presets,
                palette,
            },
            window,
            cx,
        )),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Settings tab dispatch is intentionally explicit for routing clarity."
)]
fn settings_output_section(
    section: OutputSection,
    settings: &SettingsRenderState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = settings.palette;
    let content = crate::app::components::frame_field_group();
    match section {
        OutputSection::General => content.child(settings_output_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.output_name,
            settings.output_name_focus,
            palette,
            window,
            cx,
        )),
        OutputSection::Video => content.child(settings_video_tab(
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
        OutputSection::VideoFilters => content.child(settings_video_filters_tab(
            settings.config,
            settings.settings_disabled,
            source_kind_for(settings.metadata) == SourceKind::Image,
            settings.available_filters,
            palette,
            window,
            cx,
        )),
        OutputSection::Images => content.child(settings_images_tab(
            settings.config,
            settings.settings_disabled,
            settings.crop_editing,
            settings.video_width_focus,
            settings.video_height_focus,
            palette,
            window,
            cx,
        )),
        OutputSection::Audio => content.child(settings_audio_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.available_encoders,
            settings.audio_bitrate_focus,
            palette,
            window,
            cx,
        )),
        OutputSection::AudioFilters => content.child(settings_audio_filters_tab(
            settings.config,
            settings.settings_disabled,
            settings.available_filters,
            palette,
            window,
            cx,
        )),
        OutputSection::Subtitles => content.child(settings_subtitles_tab(
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
        OutputSection::Metadata => content.child(settings_metadata_tab(
            settings.config,
            settings.metadata,
            settings.settings_disabled,
            settings.metadata_focuses,
            palette,
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

/// Compact labelled fields inside Output; sections own their outer spacing.
pub(in crate::app) fn settings_field(
    label: &'static str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    frame_field()
        .min_w_0()
        .child(frame_label(theme::ui_text(label), palette))
}
pub(in crate::app) fn settings_fields_grid() -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap(theme::ui_rem(theme::metrics::GROUP_GAP))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABS: &[SettingsTab] = &[
        SettingsTab::Metadata,
        SettingsTab::Output,
        SettingsTab::Presets,
    ];

    #[test]
    fn settings_tab_for_key_handles_the_navigation_matrix() {
        let cases = [
            (SettingsTab::Metadata, "left", Some(SettingsTab::Presets)),
            (SettingsTab::Presets, "right", Some(SettingsTab::Metadata)),
            (SettingsTab::Output, "home", Some(SettingsTab::Metadata)),
            (SettingsTab::Output, "end", Some(SettingsTab::Presets)),
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
