use crate::app::components::frame_choice_button;
use crate::app::inspector::panel::settings_section;
use crate::app::inspector::shared::settings_hint_text;
use crate::app::inspector::subtitles::burn_in::{
    settings_subtitle_burn_file_row, settings_subtitle_load_button,
    settings_subtitle_style_controls,
};
use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorInputFocuses, SettingsSubtitleColorPopoverFocuses,
    SettingsSubtitleColorTarget, SettingsSubtitleFocuses, SettingsSubtitleHsv,
    SettingsSubtitleMode, SettingsSubtitlePopover, SettingsSubtitleSelectFocuses,
};
use crate::app::inspector::subtitles::tracks::{
    settings_external_subtitles_section, settings_subtitle_track_button,
};
use crate::app::root::FrameRoot;
use crate::settings::{ConversionConfig, ProcessingMode, SourceMetadata, subtitle_track_options};
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, ScrollHandle, Window, div};

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsSubtitlesTabState<'a> {
    pub(in crate::app) config: &'a ConversionConfig,
    pub(in crate::app) metadata: Option<&'a SourceMetadata>,
    pub(in crate::app) settings_disabled: bool,
    pub(in crate::app) available_encoders: &'a frame_core::capabilities::AvailableEncoders,
    pub(in crate::app) subtitle_fonts: &'a [String],
    pub(in crate::app) focuses: SettingsSubtitleFocuses<'a>,
    pub(in crate::app) external_language_focus: Option<&'a FocusHandle>,
    pub(in crate::app) external_title_focus: Option<&'a FocusHandle>,
    pub(in crate::app) external_track_index: Option<usize>,
    pub(in crate::app) mode: SettingsSubtitleMode,
    pub(in crate::app) color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    pub(in crate::app) active_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) font_select_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) font_size_select_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) font_color_draft: &'a str,
    pub(in crate::app) outline_color_draft: &'a str,
    pub(in crate::app) font_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) outline_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) palette: &'static theme::ThemePalette,
}

#[derive(Clone, Copy)]
pub(super) struct SettingsSubtitleStyleState<'a> {
    pub(super) config: &'a ConversionConfig,
    pub(super) disabled: bool,
    pub(super) subtitle_fonts: &'a [String],
    pub(super) focuses: SettingsSubtitleFocuses<'a>,
    pub(super) color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    pub(super) active_popover: Option<SettingsSubtitlePopover>,
    pub(super) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(super) font_select_scroll_handle: &'a ScrollHandle,
    pub(super) font_size_select_scroll_handle: &'a ScrollHandle,
    pub(super) font_color_draft: &'a str,
    pub(super) outline_color_draft: &'a str,
    pub(super) font_color_hsv_draft: SettingsSubtitleHsv,
    pub(super) outline_color_hsv_draft: SettingsSubtitleHsv,
    pub(super) palette: &'static theme::ThemePalette,
}

pub(super) struct SettingsSubtitleColorFieldSpec<'a> {
    pub(super) label: &'static str,
    pub(super) id: &'static str,
    pub(super) value: String,
    pub(super) disabled: bool,
    pub(super) target: SettingsSubtitleColorTarget,
    pub(super) popover_focuses: SettingsSubtitleColorPopoverFocuses<'a>,
    pub(super) focus: Option<&'a FocusHandle>,
    pub(super) active_popover: Option<SettingsSubtitlePopover>,
    pub(super) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(super) draft: &'a str,
    pub(super) hsv: SettingsSubtitleHsv,
    pub(super) palette: &'static theme::ThemePalette,
}

#[derive(Clone, Copy)]
pub(super) struct SettingsSubtitleFontSelectState<'a> {
    pub(super) config: &'a ConversionConfig,
    pub(super) disabled: bool,
    pub(super) subtitle_fonts: &'a [String],
    pub(super) active_popover: Option<SettingsSubtitlePopover>,
    pub(super) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(super) popover: SettingsSubtitlePopover,
    pub(super) scroll_handle: &'a ScrollHandle,
    pub(super) focuses: SettingsSubtitleSelectFocuses<'a>,
    pub(super) palette: &'static theme::ThemePalette,
}

#[derive(Clone, Copy)]
pub(super) struct SettingsSubtitleFontSizeSelectState<'a> {
    pub(super) config: &'a ConversionConfig,
    pub(super) disabled: bool,
    pub(super) active_popover: Option<SettingsSubtitlePopover>,
    pub(super) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(super) popover: SettingsSubtitlePopover,
    pub(super) scroll_handle: &'a ScrollHandle,
    pub(super) focuses: SettingsSubtitleSelectFocuses<'a>,
    pub(super) palette: &'static theme::ThemePalette,
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "Tab render state is a short-lived bundle of references and copyable focus handles consumed during render."
)]
pub(in crate::app) fn settings_subtitles_tab(
    state: SettingsSubtitlesTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let content = div().flex().flex_col().gap_4().child(
        settings_section("Subtitle type", palette)
            .child(settings_subtitle_mode_grid(state.mode, palette, window, cx)),
    );

    match state.mode {
        SettingsSubtitleMode::Selectable => {
            content.child(settings_selectable_subtitles_content(&state, window, cx))
        }
        SettingsSubtitleMode::BurnIn => {
            content.child(settings_burn_in_subtitles_content(&state, window, cx))
        }
    }
}

pub(super) fn settings_subtitle_mode_grid(
    mode: SettingsSubtitleMode,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap_2()
        .child(
            frame_choice_button(
                "settings-subtitle-mode-selectable",
                "Selectable",
                mode == SettingsSubtitleMode::Selectable,
                true,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.select_subtitle_mode(SettingsSubtitleMode::Selectable) {
                    cx.notify();
                }
            })),
        )
        .child(
            frame_choice_button(
                "settings-subtitle-mode-burn-in",
                "Burn-in",
                mode == SettingsSubtitleMode::BurnIn,
                true,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.select_subtitle_mode(SettingsSubtitleMode::BurnIn) {
                    cx.notify();
                }
            })),
        )
}

pub(super) fn settings_selectable_subtitles_content(
    state: &SettingsSubtitlesTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let content = div()
        .flex()
        .flex_col()
        .gap_4()
        .child(settings_external_subtitles_section(state, window, cx));

    let track_options = subtitle_track_options(
        state.config,
        state.metadata,
        state.settings_disabled,
        state.available_encoders,
    );
    if track_options.is_empty() {
        return content.child(
            settings_section("Source tracks", palette)
                .child(settings_hint_text("No subtitles", palette)),
        );
    }

    let mut list = div().grid().grid_cols(1).gap_2();
    for option in track_options {
        list = list.child(settings_subtitle_track_button(option, palette, window, cx));
    }

    content.child(settings_section("Source tracks", palette).child(list))
}

pub(super) fn settings_burn_in_subtitles_content(
    state: &SettingsSubtitlesTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let config = state.config;
    let palette = state.palette;
    let copy_mode = config.processing_mode == ProcessingMode::Copy;
    let burn_in_disabled = state.settings_disabled || copy_mode;
    let mut burn_in_section = settings_section("Burn-in file", palette)
        .child(settings_subtitle_load_button(
            burn_in_disabled,
            state.focuses.burn_file,
            palette,
            window,
            cx,
        ))
        .child(settings_hint_text(
            if copy_mode {
                "Burn-in subtitles are disabled in stream copy mode."
            } else {
                "Burning in subtitles will force video re-encoding."
            },
            palette,
        ));

    if config.subtitle_burn_path.is_some() {
        burn_in_section = burn_in_section.child(settings_subtitle_burn_file_row(
            config,
            burn_in_disabled,
            palette,
            window,
            cx,
        ));
    }

    let content = div().flex().flex_col().gap_4().child(burn_in_section);

    if copy_mode {
        return content;
    }

    content.child(
        settings_section("Style", palette).child(settings_subtitle_style_controls(
            SettingsSubtitleStyleState {
                config,
                disabled: burn_in_disabled,
                subtitle_fonts: state.subtitle_fonts,
                focuses: state.focuses,
                color_focuses: state.color_focuses,
                active_popover: state.active_popover,
                rendered_popover: state.rendered_popover,
                font_select_scroll_handle: state.font_select_scroll_handle,
                font_size_select_scroll_handle: state.font_size_select_scroll_handle,
                font_color_draft: state.font_color_draft,
                outline_color_draft: state.outline_color_draft,
                font_color_hsv_draft: state.font_color_hsv_draft,
                outline_color_hsv_draft: state.outline_color_hsv_draft,
                palette,
            },
            window,
            cx,
        )),
    )
}
