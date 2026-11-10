use crate::app::components::{
    frame_slider, frame_slider_fraction_from_bounds, frame_slider_handle,
};
use crate::app::inspector::shared::{
    range_fraction, range_value_for_key, range_value_from_fraction, settings_field_label,
    settings_value_badge,
};
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::numeric::u32_to_u8;
use crate::settings::{ConversionConfig, apply_crf, apply_quality};
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, DragMoveEvent, IntoElement, Render, Window, div};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SettingsVideoRangeTarget {
    Crf,
    Quality,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SettingsVideoRangeDrag {
    pub(super) target: SettingsVideoRangeTarget,
    pub(super) min: u32,
    pub(super) max: u32,
}

pub(super) struct SettingsVideoRangeDragPreview;

impl Render for SettingsVideoRangeDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(theme::ui_rem(0.0)).h(theme::ui_rem(0.0))
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "keeps slider construction close to visual contract"
)]
pub(super) fn settings_video_range_field(
    label: &'static str,
    value_label: String,
    value: u32,
    min: u32,
    max: u32,
    lower_label: &'static str,
    upper_label: &'static str,
    target: SettingsVideoRangeTarget,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .pt(theme::ui_rem(4.0))
        .child(
            div()
                .flex()
                .items_end()
                .justify_between()
                .child(settings_field_label(label, palette))
                .child(settings_value_badge(value_label, palette)),
        )
        .child(settings_video_range_slider(
            value, min, max, disabled, target, palette, window, cx,
        ))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.text_muted))
                .child(theme::ui_text(lower_label))
                .child(theme::ui_text(upper_label)),
        )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Range controls require domain limits, target, state and rendering context."
)]
pub(super) fn settings_video_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsVideoRangeTarget,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = range_fraction(value, min, max);
    let drag = SettingsVideoRangeDrag { target, min, max };
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    frame_slider(
        match target {
            SettingsVideoRangeTarget::Crf => "settings-video-crf-slider",
            SettingsVideoRangeTarget::Quality => "settings-video-quality-slider",
        },
        match target {
            SettingsVideoRangeTarget::Crf => "Video CRF",
            SettingsVideoRangeTarget::Quality => "Video quality",
        },
        fraction,
        disabled,
        palette,
        window,
        cx,
    )
    .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
        if disabled {
            return;
        }
        owner.update(cx, move |root, cx| {
            if let Some(value) = range_value_for_key(value, min, max, "right")
                && root.update_selected_config(|config| {
                    apply_settings_video_range_value(config, target, value)
                })
            {
                cx.notify();
            }
        });
    })
    .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
        if disabled {
            return;
        }
        decrement_owner.update(cx, move |root, cx| {
            if let Some(value) = range_value_for_key(value, min, max, "left")
                && root.update_selected_config(|config| {
                    apply_settings_video_range_value(config, target, value)
                })
            {
                cx.notify();
            }
        });
    })
    .when(!disabled, |slider| {
        slider.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsVideoRangeDragPreview)
        })
    })
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<SettingsVideoRangeDrag>, window, cx| {
            let drag = *event.drag(cx);
            let fraction = frame_slider_fraction_from_bounds(
                event.event.position,
                event.bounds,
                window.rem_size(),
            );
            let value = range_value_from_fraction(fraction, drag.min, drag.max);
            let changed = root.update_selected_config(|config| {
                apply_settings_video_range_value(config, drag.target, value)
            });
            if changed {
                cx.notify();
            }
        },
    ))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
            let Some(value) = range_value_for_key(value, min, max, event.keystroke.key.as_str())
            else {
                return;
            };
            if root.update_selected_config(|config| {
                apply_settings_video_range_value(config, target, value)
            }) {
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
    .child(settings_video_range_handle(fraction, drag, !disabled))
}

pub(super) fn apply_settings_video_range_value(
    config: &mut ConversionConfig,
    target: SettingsVideoRangeTarget,
    value: u32,
) -> bool {
    match target {
        SettingsVideoRangeTarget::Crf => apply_crf(config, u32_to_u8(value)),
        SettingsVideoRangeTarget::Quality => apply_quality(config, value),
    }
}

pub(super) fn settings_video_range_handle(
    fraction: f32,
    drag: SettingsVideoRangeDrag,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle = frame_slider_handle(
        match drag.target {
            SettingsVideoRangeTarget::Crf => "settings-video-crf-handle",
            SettingsVideoRangeTarget::Quality => "settings-video-quality-handle",
        },
        fraction,
        enabled,
    );

    if enabled {
        handle.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsVideoRangeDragPreview)
        })
    } else {
        handle
    }
}
