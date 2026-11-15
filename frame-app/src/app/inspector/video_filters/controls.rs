use crate::app::components::{
    FRAME_ICON_BUTTON_SM_SIZE, FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant,
    frame_checkbox_row, frame_icon_button, frame_text_button,
};
use crate::app::components::{FrameOption, frame_option_field};
use crate::app::inspector::video_filters::range::VideoFilterRangeTarget;
use crate::app::inspector::video_filters::spec::{
    apply_video_filter_range, deinterlace_id, filter_strength_id, video_target_id,
};
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::settings::{
    DeinterlaceMode, FilterStrength, apply_video_deinterlace, apply_video_denoise,
    apply_video_grayscale, apply_video_stabilization, reset_video_filters,
};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, KeyDownEvent, Window, div};

pub(super) fn settings_video_stabilization_control(
    checked: bool,
    available: bool,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let disabled = settings_disabled || (!available && !checked);
    let focus = window
        .use_keyed_state("settings-video-stabilization-focus", cx, |_, cx| {
            cx.focus_handle()
        })
        .read(cx)
        .clone()
        .tab_stop(!disabled);
    crate::app::components::frame_checkbox_row_with_focus(
        "settings-video-stabilization-toggle",
        "Stabilize video",
        "",
        checked,
        disabled,
        &focus,
        palette,
        window,
        cx,
        move |root, _event, _window, cx| {
            toggle_stabilization(root, checked, disabled, cx);
        },
    )
    .debug_selector(|| "settings-video-stabilization-toggle".into())
    .on_key_down(cx.listener(move |root, event: &KeyDownEvent, window, cx| {
        if focus.is_focused(window) && matches!(event.keystroke.key.as_str(), "space" | "enter") {
            cx.stop_propagation();
            toggle_stabilization(root, checked, disabled, cx);
        }
    }))
}

fn toggle_stabilization(
    root: &mut FrameRoot,
    checked: bool,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) {
    if disabled || (!checked && !root.available_filters.deshake) {
        return;
    }
    if root.update_selected_config(|config| apply_video_stabilization(config, !checked)) {
        cx.notify();
    }
}

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
    let mut grid = Vec::new();
    for (candidate, label) in [
        (FilterStrength::Low, "Low"),
        (FilterStrength::Medium, "Medium"),
        (FilterStrength::High, "High"),
    ] {
        grid.push(FrameOption::new(
            format!("settings-video-denoise-{}", filter_strength_id(candidate)),
            label,
            strength == candidate,
            !disabled,
            move |root, _window, cx| {
                if disabled {
                    return;
                }
                if root
                    .update_selected_config(|config| apply_video_denoise(config, true, candidate))
                {
                    cx.notify();
                }
            },
        ));
    }
    let grid = frame_option_field(
        "video-denoise-control-select",
        "Video denoise control",
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
            "settings-video-denoise-toggle",
            "Denoise",
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(
        "settings-video-grayscale-toggle",
        "Grayscale",
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
    let mut grid = Vec::new();
    for (candidate, label) in [
        (DeinterlaceMode::Off, "Off"),
        (DeinterlaceMode::Auto, "Auto"),
        (DeinterlaceMode::On, "On"),
    ] {
        grid.push(FrameOption::new(
            format!("settings-video-deinterlace-{}", deinterlace_id(candidate)),
            label,
            mode == candidate,
            !disabled,
            move |root, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_deinterlace(config, candidate))
                {
                    cx.notify();
                }
            },
        ));
    }
    frame_option_field(
        "video-deinterlace-control-select",
        "Video deinterlace control",
        grid,
        palette,
        window,
        cx,
    )
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
        if root.update_selected_config(reset_video_filters) {
            cx.notify();
        }
    }))
}
