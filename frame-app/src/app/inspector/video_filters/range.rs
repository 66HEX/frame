use crate::app::components::{
    frame_checkbox_row, frame_slider, frame_slider_fraction_from_bounds, frame_slider_handle,
};
use crate::app::inspector::shared::{range_value_for_key, settings_value_badge};
use crate::app::inspector::video_filters::controls::settings_video_filter_reset;
use crate::app::inspector::video_filters::spec::{
    VideoFilterRangeSpec, apply_video_filter_range, video_handle_id, video_slider_id,
    video_slider_label, video_target_id,
};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, DragMoveEvent, IntoElement, Render, Window, div};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum VideoFilterRangeTarget {
    Brightness,
    Contrast,
    Saturation,
    Gamma,
    Hue,
    Temperature,
    Sharpen,
    GaussianBlur,
    Deband,
    Vignette,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct VideoFilterRangeDrag {
    pub(super) target: VideoFilterRangeTarget,
    pub(super) min: i32,
    pub(super) max: i32,
}

pub(super) struct VideoFilterRangeDragPreview;

impl Render for VideoFilterRangeDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(theme::ui_rem(0.0)).h(theme::ui_rem(0.0))
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Range specs are short-lived UI values built inline at call sites."
)]
pub(super) fn settings_video_filter_range_field(
    spec: VideoFilterRangeSpec,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let unavailable = !spec.available;
    let control_disabled = settings_disabled || unavailable;
    let slider_disabled = control_disabled || !spec.enabled;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_2()
                .child(
                    frame_checkbox_row(
                        format!(
                            "settings-video-filter-{}-toggle",
                            video_target_id(spec.target)
                        ),
                        spec.label,
                        if unavailable {
                            "This FFmpeg runtime does not provide the required filter."
                        } else {
                            ""
                        },
                        spec.enabled,
                        control_disabled,
                        palette,
                        cx,
                        move |root, _event, _window, cx| {
                            if control_disabled {
                                return;
                            }
                            if root.update_selected_config(|config| {
                                apply_video_filter_range(
                                    config,
                                    spec.target,
                                    !spec.enabled,
                                    spec.value,
                                )
                            }) {
                                cx.notify();
                            }
                        },
                    )
                    .flex_1(),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(settings_value_badge(spec.value_label.clone(), palette))
                        .child(settings_video_filter_reset(
                            spec.target,
                            spec.default_value,
                            control_disabled,
                            palette,
                            window,
                            cx,
                        )),
                ),
        )
        .child(settings_video_filter_range_slider(
            spec.value,
            spec.min,
            spec.max,
            slider_disabled,
            spec.target,
            palette,
            window,
            cx,
        ))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Range controls require domain limits, target, state and rendering context."
)]
pub(super) fn settings_video_filter_range_slider(
    value: i32,
    min: i32,
    max: i32,
    disabled: bool,
    target: VideoFilterRangeTarget,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = signed_range_fraction(value, min, max);
    let drag = VideoFilterRangeDrag { target, min, max };
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    frame_slider(
        video_slider_id(target),
        video_slider_label(target),
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
            if let Some(next) = signed_range_value_for_key(value, min, max, "right")
                && root.update_selected_config(|config| {
                    apply_video_filter_range(config, target, true, next)
                })
            {
                root.defer_filter_preview_reconfigure(cx);
                cx.notify();
            }
        });
    })
    .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
        if disabled {
            return;
        }
        decrement_owner.update(cx, move |root, cx| {
            if let Some(next) = signed_range_value_for_key(value, min, max, "left")
                && root.update_selected_config(|config| {
                    apply_video_filter_range(config, target, true, next)
                })
            {
                root.defer_filter_preview_reconfigure(cx);
                cx.notify();
            }
        });
    })
    .when(!disabled, |slider| {
        slider.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| VideoFilterRangeDragPreview)
        })
    })
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<VideoFilterRangeDrag>, window, cx| {
            let drag = *event.drag(cx);
            let fraction = frame_slider_fraction_from_bounds(
                event.event.position,
                event.bounds,
                window.rem_size(),
            );
            let value = signed_range_value_from_fraction(fraction, drag.min, drag.max);
            if root.update_selected_config(|config| {
                apply_video_filter_range(config, drag.target, true, value)
            }) {
                root.defer_filter_preview_reconfigure(cx);
                cx.notify();
            }
        },
    ))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
            let Some(value) =
                signed_range_value_for_key(value, min, max, event.keystroke.key.as_str())
            else {
                return;
            };
            if root.update_selected_config(|config| {
                apply_video_filter_range(config, target, true, value)
            }) {
                root.defer_filter_preview_reconfigure(cx);
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
    .child(
        frame_slider_handle(video_handle_id(target), fraction, !disabled).when(
            !disabled,
            |handle| {
                handle.on_drag(drag, |_drag, _position, _window, cx| {
                    cx.new(|_| VideoFilterRangeDragPreview)
                })
            },
        ),
    )
}

pub(super) fn signed_range_fraction(value: i32, min: i32, max: i32) -> f32 {
    if max <= min {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "Filter slider ranges are small bounded UI values."
    )]
    {
        ((value.clamp(min, max) - min) as f32) / ((max - min) as f32)
    }
}

pub(super) fn signed_range_value_from_fraction(fraction: f64, min: i32, max: i32) -> i32 {
    if max <= min {
        return min;
    }
    let span = f64::from(max - min);
    let value = fraction
        .clamp(0.0, 1.0)
        .mul_add(span, f64::from(min))
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Value is rounded and clamped into i32 range before casting."
    )]
    {
        value as i32
    }
}

pub(super) fn signed_range_value_for_key(value: i32, min: i32, max: i32, key: &str) -> Option<i32> {
    let offset = u32::try_from(value.clamp(min, max) - min).ok()?;
    let span = u32::try_from(max - min).ok()?;
    let next = range_value_for_key(offset, 0, span, key)?;
    i32::try_from(next).ok().map(|next| next + min)
}
