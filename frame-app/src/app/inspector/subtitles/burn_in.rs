use crate::app::components::{
    FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant, frame_choice_button,
    frame_icon_button,
};
use crate::app::inspector::shared::{settings_field_label, settings_hint_text};
use crate::app::inspector::subtitles::color_picker::settings_subtitle_color_field;
use crate::app::inspector::subtitles::font_select::settings_subtitle_font_select;
use crate::app::inspector::subtitles::size_select::settings_subtitle_font_size_select;
use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorTarget, SettingsSubtitlePopover,
};
use crate::app::inspector::subtitles::view::{
    SettingsSubtitleColorFieldSpec, SettingsSubtitleFontSelectState,
    SettingsSubtitleFontSizeSelectState, SettingsSubtitleStyleState,
};
use crate::app::layout::SETTINGS_CONTROL_HEIGHT;
use crate::app::primitives::{ButtonVariant, button_colors, color};
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, DEFAULT_SUBTITLE_FONT_COLOR, DEFAULT_SUBTITLE_OUTLINE_COLOR,
    apply_subtitle_burn_path, apply_subtitle_position, subtitle_burn_file_label,
    subtitle_color_value, subtitle_position_options,
};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, Window, div};

pub(super) fn settings_subtitle_burn_file_row(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let label = subtitle_burn_file_label(config);
    let colors = button_colors(ButtonVariant::Secondary, false, !disabled, palette);

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .rounded(theme::ui_rem(theme::RADIUS_MD))
                .px(theme::ui_rem(theme::metrics::CONTROL_PADDING_X))
                .bg(color(colors.background))
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(colors.foreground))
                .opacity(colors.opacity)
                .child(div().min_w_0().truncate().child(label)),
        )
        .child(settings_subtitle_clear_button(
            disabled, palette, window, cx,
        ))
}

pub(super) fn settings_subtitle_clear_button(
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_icon_button(
        "settings-subtitle-clear-file",
        assets::ICON_TRASH,
        "Clear subtitle file",
        FrameIconButtonVariant::DestructiveGhost,
        !disabled,
        FrameIconButtonSize {
            button: SETTINGS_CONTROL_HEIGHT,
            icon: FRAME_ICON_SM_SIZE,
        },
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if disabled {
            return;
        }
        if root.update_selected_config(|config| apply_subtitle_burn_path(config, None)) {
            cx.notify();
        }
    }))
}

#[expect(
    clippy::large_types_passed_by_value,
    clippy::too_many_lines,
    reason = "Style render state is a short-lived bundle of references and copyable focus handles consumed during render."
)]
pub(super) fn settings_subtitle_style_controls(
    state: SettingsSubtitleStyleState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(settings_subtitle_font_select(
                    SettingsSubtitleFontSelectState {
                        config: state.config,
                        disabled: state.disabled,
                        subtitle_fonts: state.subtitle_fonts,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        popover: SettingsSubtitlePopover::FontName,
                        scroll_handle: state.font_select_scroll_handle,
                        focuses: state.focuses.font_select,
                        palette: state.palette,
                    },
                    window,
                    cx,
                ))
                .child(settings_subtitle_font_size_select(
                    SettingsSubtitleFontSizeSelectState {
                        config: state.config,
                        disabled: state.disabled,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        popover: SettingsSubtitlePopover::FontSize,
                        scroll_handle: state.font_size_select_scroll_handle,
                        focuses: state.focuses.font_size_select,
                        palette: state.palette,
                    },
                    window,
                    cx,
                )),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(settings_subtitle_color_field(
                    SettingsSubtitleColorFieldSpec {
                        label: "Text color",
                        id: "settings-subtitle-font-color",
                        value: subtitle_color_value(
                            state.config.subtitle_font_color.as_ref(),
                            DEFAULT_SUBTITLE_FONT_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Font,
                        popover_focuses: state.focuses.font_color,
                        focus: state.color_focuses.font,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        draft: state.font_color_draft,
                        hsv: state.font_color_hsv_draft,
                        palette: state.palette,
                    },
                    window,
                    cx,
                ))
                .child(settings_subtitle_color_field(
                    SettingsSubtitleColorFieldSpec {
                        label: "Outline color",
                        id: "settings-subtitle-outline-color",
                        value: subtitle_color_value(
                            state.config.subtitle_outline_color.as_ref(),
                            DEFAULT_SUBTITLE_OUTLINE_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Outline,
                        popover_focuses: state.focuses.outline_color,
                        focus: state.color_focuses.outline,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        draft: state.outline_color_draft,
                        hsv: state.outline_color_hsv_draft,
                        palette: state.palette,
                    },
                    window,
                    cx,
                )),
        )
        .child(
            crate::app::components::frame_field()
                .child(settings_field_label("Position", state.palette))
                .child(settings_subtitle_position_grid(
                    state.config,
                    state.disabled,
                    state.palette,
                    window,
                    cx,
                )),
        )
        .child(settings_hint_text(
            "Style applies to burned-in subtitles only.",
            state.palette,
        ))
}

pub(super) fn settings_subtitle_position_grid(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let choice_focus = crate::app::components::FrameChoiceFocus::new(
        "settings_subtitle_position_grid",
        window,
        cx,
    );
    let mut grid = div().grid().grid_cols(3).gap_2();
    for option in subtitle_position_options(config, disabled) {
        let position = option.position;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            frame_choice_button(
                &choice_focus,
                format!("subtitle-position-{}", position.id()),
                option.label,
                option.is_selected,
                is_enabled,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_subtitle_position(config, position)) {
                    cx.notify();
                }
            })),
        );
    }

    grid
}
