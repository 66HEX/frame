use crate::app::components::{
    FRAME_ICON_BUTTON_SM_SIZE, FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant,
    frame_checkbox_row, frame_icon_button, frame_text_button,
};
use crate::app::components::{FrameOption, frame_option_field};
use crate::app::inspector::audio_filters::range::AudioFilterRangeTarget;
use crate::app::inspector::audio_filters::spec::{
    apply_audio_filter_range, audio_target_id, filter_strength_id,
};
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::settings::{
    FilterStrength, apply_audio_compressor, apply_audio_normalize, reset_audio_filters,
};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, Window, div};

pub(super) fn settings_audio_filter_reset(
    target: AudioFilterRangeTarget,
    default_value: i32,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_icon_button(
        format!("settings-audio-filter-{}-reset", audio_target_id(target)),
        assets::ICON_REFRESH,
        "Reset filter",
        FrameIconButtonVariant::Ghost,
        !disabled,
        FrameIconButtonSize {
            button: FRAME_ICON_BUTTON_SM_SIZE,
            icon: FRAME_ICON_SM_SIZE,
        },
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        if disabled {
            return;
        }
        if root.update_selected_config(|config| {
            apply_audio_filter_range(config, target, false, default_value)
        }) {
            cx.notify();
        }
    }))
}

pub(super) fn settings_audio_normalize_control(
    checked: bool,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(
        "settings-audio-normalize-row",
        "Normalize audio",
        "",
        checked,
        disabled,
        palette,
        window,
        cx,
        move |root, _event, _window, cx| {
            if disabled {
                return;
            }
            if root.update_selected_config(|config| apply_audio_normalize(config, !checked)) {
                cx.notify();
            }
        },
    )
}

pub(super) fn settings_audio_compressor_control(
    enabled: bool,
    strength: FilterStrength,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = Vec::new();
    for (candidate, label) in [
        (FilterStrength::Low, "Gentle"),
        (FilterStrength::Medium, "Balanced"),
        (FilterStrength::High, "Strong"),
    ] {
        grid.push(FrameOption::new(
            format!(
                "settings-audio-compressor-{}",
                filter_strength_id(candidate)
            ),
            label,
            strength == candidate,
            !disabled,
            move |root, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_audio_compressor(config, true, candidate)
                }) {
                    cx.notify();
                }
            },
        ));
    }
    let grid = frame_option_field(
        "audio-compressor-control-select",
        "Audio compressor control",
        grid,
        palette,
        window,
        cx,
    );

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(frame_checkbox_row(
            "settings-audio-compressor-toggle",
            "Compressor",
            "",
            enabled,
            disabled,
            palette,
            window,
            cx,
            move |root, _event, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_audio_compressor(config, !enabled, strength)
                }) {
                    cx.notify();
                }
            },
        ))
        .child(grid)
}

pub(super) fn settings_audio_filters_reset_all(
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "settings-audio-filters-reset-all",
        "Reset audio filters",
        if palette.is_light() {
            ButtonVariant::Accent
        } else {
            ButtonVariant::Secondary
        },
        false,
        !disabled,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        if disabled {
            return;
        }
        if root.update_selected_config(reset_audio_filters) {
            cx.notify();
        }
    }))
}
