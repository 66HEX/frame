use crate::app::accessibility::FrameFocusKey;
use crate::app::inspector::subtitles::state::SettingsSubtitlePopover;
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, ProcessingMode, SettingsTab, subtitle_font_options,
    subtitle_font_size_options,
};
use gpui::{Context, FocusHandle, Window};

#[expect(
    clippy::struct_field_names,
    reason = "Names match the extracted render bindings so the focus registration order remains reviewable."
)]
pub(super) struct SubtitleFocusHandles {
    pub(super) subtitle_burn_file_focus: FocusHandle,
    pub(super) subtitle_add_external_focus: FocusHandle,
    pub(super) subtitle_font_trigger_focus: FocusHandle,
    pub(super) subtitle_font_panel_focus: FocusHandle,
    pub(super) subtitle_font_first_focus: FocusHandle,
    pub(super) subtitle_font_last_focus: FocusHandle,
    pub(super) subtitle_size_trigger_focus: FocusHandle,
    pub(super) subtitle_size_panel_focus: FocusHandle,
    pub(super) subtitle_size_first_focus: FocusHandle,
    pub(super) subtitle_size_last_focus: FocusHandle,
    pub(super) subtitle_font_color_trigger_focus: FocusHandle,
    pub(super) subtitle_font_color_panel_focus: FocusHandle,
    pub(super) subtitle_font_color_sv_focus: FocusHandle,
    pub(super) subtitle_font_color_hue_focus: FocusHandle,
    pub(super) subtitle_outline_color_trigger_focus: FocusHandle,
    pub(super) subtitle_outline_color_panel_focus: FocusHandle,
    pub(super) subtitle_outline_color_sv_focus: FocusHandle,
    pub(super) subtitle_outline_color_hue_focus: FocusHandle,
}

#[derive(Clone, Copy)]
pub(super) struct SubtitleFocusContext<'a> {
    pub(super) selected_config_snapshot: &'a ConversionConfig,
    pub(super) app_root_focus: &'a FocusHandle,
    pub(super) subtitle_font_color_focus: &'a FocusHandle,
    pub(super) subtitle_outline_color_focus: &'a FocusHandle,
}

impl FrameRoot {
    #[expect(
        clippy::too_many_lines,
        reason = "Keep the existing focus registration and element append order explicit during structural extraction."
    )]
    pub(super) fn prepare_subtitle_focus(
        &mut self,
        input: SubtitleFocusContext<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SubtitleFocusHandles {
        let SubtitleFocusContext {
            selected_config_snapshot,
            app_root_focus,
            subtitle_font_color_focus,
            subtitle_outline_color_focus,
        } = input;
        let subtitles_copy_mode = selected_config_snapshot.processing_mode == ProcessingMode::Copy;
        let subtitles_tab_active = self.settings_ui.active_tab == SettingsTab::Output
            && self
                .settings_ui
                .expanded_output_sections
                .contains(&crate::settings::OutputSection::Subtitles)
            && crate::settings::visible_output_sections(
                selected_config_snapshot,
                self.selected_source_metadata().as_ref(),
            )
            .contains(&crate::settings::OutputSection::Subtitles);
        let subtitles_enabled =
            subtitles_tab_active && !self.file_queue.selected_file_locked() && !subtitles_copy_mode;
        let external_subtitles_enabled =
            subtitles_tab_active && !self.file_queue.selected_file_locked();
        let subtitle_font_option_count = subtitle_font_options(
            selected_config_snapshot,
            &self.subtitle_font_families,
            !subtitles_enabled,
        )
        .len();
        let subtitle_font_select_enabled = subtitles_enabled && subtitle_font_option_count > 0;
        let subtitle_font_size_option_count =
            subtitle_font_size_options(selected_config_snapshot, !subtitles_enabled).len();
        let subtitle_burn_file_focus = self.ensure_focus(
            FrameFocusKey::Control("settings-subtitle-burn-file".to_string()),
            subtitles_enabled,
            cx,
        );
        let subtitle_add_external_focus = self.ensure_focus(
            FrameFocusKey::Control("settings-subtitle-add-external".to_string()),
            external_subtitles_enabled,
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
        if subtitle_font_popover_active && !subtitle_font_panel_focus.contains_focused(window, cx) {
            subtitle_font_first_focus.focus(window, cx);
        }
        if subtitle_size_popover_active && !subtitle_size_panel_focus.contains_focused(window, cx) {
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
        SubtitleFocusHandles {
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
        }
    }
}
