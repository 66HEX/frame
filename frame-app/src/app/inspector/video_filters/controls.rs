use crate::app::components::{
    FRAME_ICON_BUTTON_SM_SIZE, FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant,
    frame_checkbox_row, frame_choice_button, frame_icon_button, frame_text_button,
};
use crate::app::inspector::video_filters::range::VideoFilterRangeTarget;
use crate::app::inspector::video_filters::spec::{
    apply_video_filter_range, deinterlace_id, filter_strength_id, video_target_id,
};
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::settings::{
    DeinterlaceMode, FilterStrength, apply_video_deinterlace, apply_video_denoise,
    apply_video_grayscale, reset_video_filters,
};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, Window, div};

pub(super) fn settings_video_filter_reset(
    target: VideoFilterRangeTarget,
    default_value: i32,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_icon_button(
        format!("settings-video-filter-{}-reset", video_target_id(target)),
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
            apply_video_filter_range(config, target, false, default_value)
        }) {
            cx.notify();
        }
    }))
}

pub(super) fn settings_video_denoise_control(
    enabled: bool,
    strength: FilterStrength,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).mt_1().gap_2();
    for (candidate, label) in [
        (FilterStrength::Low, "Low"),
        (FilterStrength::Medium, "Medium"),
        (FilterStrength::High, "High"),
    ] {
        grid = grid.child(
            frame_choice_button(
                format!("settings-video-denoise-{}", filter_strength_id(candidate)),
                label,
                enabled && strength == candidate,
                !disabled,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if disabled {
                    return;
                }
                if root
                    .update_selected_config(|config| apply_video_denoise(config, true, candidate))
                {
                    cx.notify();
                }
            })),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(frame_checkbox_row(
            "settings-video-denoise-toggle",
            "Denoise",
            "",
            enabled,
            disabled,
            palette,
            cx,
            move |root, _event, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_video_denoise(config, !enabled, strength)
                }) {
                    cx.notify();
                }
            },
        ))
        .child(grid)
}

pub(super) fn settings_video_grayscale_control(
    checked: bool,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(
        "settings-video-grayscale-toggle",
        "Grayscale",
        "",
        checked,
        disabled,
        palette,
        cx,
        move |root, _event, _window, cx| {
            if disabled {
                return;
            }
            if root.update_selected_config(|config| apply_video_grayscale(config, !checked)) {
                cx.notify();
            }
        },
    )
}

pub(super) fn settings_video_deinterlace_control(
    mode: DeinterlaceMode,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).gap_2();
    for (candidate, label) in [
        (DeinterlaceMode::Off, "Off"),
        (DeinterlaceMode::Auto, "Auto"),
        (DeinterlaceMode::On, "On"),
    ] {
        grid = grid.child(
            frame_choice_button(
                format!("settings-video-deinterlace-{}", deinterlace_id(candidate)),
                label,
                mode == candidate,
                !disabled,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_deinterlace(config, candidate))
                {
                    cx.notify();
                }
            })),
        );
    }
    grid
}

pub(super) fn settings_video_filters_reset_all(
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "settings-video-filters-reset-all",
        "Reset video filters",
        ButtonVariant::Secondary,
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
        if root.update_selected_config(reset_video_filters) {
            cx.notify();
        }
    }))
}
