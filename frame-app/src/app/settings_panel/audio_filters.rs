use super::*;
use crate::settings::{
    AudioScalarFilter, FilterStrength, apply_audio_compressor, apply_audio_scalar_filter,
    reset_audio_filters,
};
use frame_core::capabilities::AvailableFilters;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AudioFilterRangeTarget {
    Volume,
    Limiter,
    Bass,
    Treble,
    HighPass,
    LowPass,
    NoiseReduction,
    DeEsser,
    StereoWidth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AudioFilterRangeDrag {
    target: AudioFilterRangeTarget,
    min: i32,
    max: i32,
}

struct AudioFilterRangeDragPreview;

impl Render for AudioFilterRangeDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The tab render function keeps the approved filter section order visible."
)]
pub(in crate::app) fn settings_audio_filters_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_filters: &AvailableFilters,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let filters = config.audio_filters;
    let controls_disabled = settings_disabled || config.processing_mode == ProcessingMode::Copy;

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            settings_section("Audio Filters").child(settings_audio_filters_reset_all(
                controls_disabled,
                window,
                cx,
            )),
        )
        .child(
            settings_section("Level")
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Volume,
                        true,
                        i32::try_from(config.audio_volume).unwrap_or(200),
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                ))
                .child(settings_audio_normalize_control(
                    config.audio_normalize,
                    controls_disabled || !available_filters.loudnorm,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Limiter,
                        filters.limiter.enabled,
                        filters.limiter.value,
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                )),
        )
        .child(
            settings_section("Dynamics").child(settings_audio_compressor_control(
                filters.compressor_enabled,
                filters.compressor_strength,
                controls_disabled || !available_filters.acompressor,
                window,
                cx,
            )),
        )
        .child(
            settings_section("Tone")
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Bass,
                        filters.bass.enabled,
                        filters.bass.value,
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Treble,
                        filters.treble.enabled,
                        filters.treble.value,
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::HighPass,
                        filters.high_pass.enabled,
                        i32::try_from(filters.high_pass.value).unwrap_or(80),
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::LowPass,
                        filters.low_pass.enabled,
                        i32::try_from(filters.low_pass.value).unwrap_or(16_000),
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                )),
        )
        .child(
            settings_section("Cleanup")
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::NoiseReduction,
                        filters.noise_reduction.enabled,
                        i32::try_from(filters.noise_reduction.value).unwrap_or(12),
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::DeEsser,
                        filters.de_esser.enabled,
                        i32::try_from(filters.de_esser.value).unwrap_or(35),
                        available_filters,
                    ),
                    controls_disabled,
                    window,
                    cx,
                )),
        )
        .child(
            settings_section("Stereo").child(settings_audio_filter_range_field(
                audio_filter_spec(
                    AudioFilterRangeTarget::StereoWidth,
                    filters.stereo_width.enabled,
                    i32::try_from(filters.stereo_width.value).unwrap_or(100),
                    available_filters,
                ),
                controls_disabled,
                window,
                cx,
            )),
        )
}

struct AudioFilterRangeSpec {
    target: AudioFilterRangeTarget,
    label: &'static str,
    value_label: String,
    enabled: bool,
    available: bool,
    value: i32,
    default_value: i32,
    min: i32,
    max: i32,
}

fn audio_filter_spec(
    target: AudioFilterRangeTarget,
    enabled: bool,
    value: i32,
    available_filters: &AvailableFilters,
) -> AudioFilterRangeSpec {
    let available = audio_filter_available(target, available_filters);
    match target {
        AudioFilterRangeTarget::Volume => {
            audio_spec(target, "Volume", true, available, value, 100, 0, 200, "%")
        }
        AudioFilterRangeTarget::Limiter => audio_spec(
            target, "Limiter", enabled, available, value, -1, -12, 0, " dB",
        ),
        AudioFilterRangeTarget::Bass => {
            audio_spec(target, "Bass", enabled, available, value, 0, -20, 20, " dB")
        }
        AudioFilterRangeTarget::Treble => audio_spec(
            target, "Treble", enabled, available, value, 0, -20, 20, " dB",
        ),
        AudioFilterRangeTarget::HighPass => audio_spec(
            target,
            "High-pass",
            enabled,
            available,
            value,
            80,
            20,
            2000,
            " Hz",
        ),
        AudioFilterRangeTarget::LowPass => audio_spec(
            target, "Low-pass", enabled, available, value, 16_000, 1000, 20_000, " Hz",
        ),
        AudioFilterRangeTarget::NoiseReduction => audio_spec(
            target,
            "Noise Reduction",
            enabled,
            available,
            value,
            12,
            1,
            30,
            " dB",
        ),
        AudioFilterRangeTarget::DeEsser => audio_spec(
            target, "De-esser", enabled, available, value, 35, 0, 100, "%",
        ),
        AudioFilterRangeTarget::StereoWidth => audio_spec(
            target,
            "Stereo Width",
            enabled,
            available,
            value,
            100,
            0,
            200,
            "%",
        ),
    }
}

const fn audio_filter_available(
    target: AudioFilterRangeTarget,
    available_filters: &AvailableFilters,
) -> bool {
    match target {
        AudioFilterRangeTarget::Volume => available_filters.volume,
        AudioFilterRangeTarget::Limiter => available_filters.alimiter,
        AudioFilterRangeTarget::Bass => available_filters.bass,
        AudioFilterRangeTarget::Treble => available_filters.treble,
        AudioFilterRangeTarget::HighPass => available_filters.highpass,
        AudioFilterRangeTarget::LowPass => available_filters.lowpass,
        AudioFilterRangeTarget::NoiseReduction => available_filters.afftdn,
        AudioFilterRangeTarget::DeEsser => available_filters.deesser,
        AudioFilterRangeTarget::StereoWidth => available_filters.stereotools,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Filter specs are compact data literals for the settings UI."
)]
fn audio_spec(
    target: AudioFilterRangeTarget,
    label: &'static str,
    enabled: bool,
    available: bool,
    value: i32,
    default_value: i32,
    min: i32,
    max: i32,
    suffix: &'static str,
) -> AudioFilterRangeSpec {
    AudioFilterRangeSpec {
        target,
        label,
        value_label: format!("{value}{suffix}"),
        enabled,
        available,
        value,
        default_value,
        min,
        max,
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Range specs are short-lived UI values built inline at call sites."
)]
fn settings_audio_filter_range_field(
    spec: AudioFilterRangeSpec,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let is_volume = spec.target == AudioFilterRangeTarget::Volume;
    let unavailable = !spec.available;
    let control_disabled = settings_disabled || unavailable;
    let slider_disabled = control_disabled || (!is_volume && !spec.enabled);

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_end()
                .justify_between()
                .gap_2()
                .child(
                    frame_checkbox_row(
                        format!(
                            "settings-audio-filter-{}-toggle",
                            audio_target_id(spec.target)
                        ),
                        spec.label,
                        if unavailable {
                            "This FFmpeg runtime does not provide the required filter."
                        } else {
                            ""
                        },
                        spec.enabled,
                        control_disabled || is_volume,
                        cx,
                        move |root, _event, _window, cx| {
                            if control_disabled || is_volume {
                                return;
                            }
                            if root.update_selected_config(|config| {
                                apply_audio_filter_range(
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
                        .child(settings_value_badge(spec.value_label.clone()))
                        .child(settings_audio_filter_reset(
                            spec.target,
                            spec.default_value,
                            control_disabled,
                            window,
                            cx,
                        )),
                ),
        )
        .child(settings_audio_filter_range_slider(
            spec.value,
            spec.min,
            spec.max,
            slider_disabled,
            spec.target,
            cx,
        ))
}

fn settings_audio_filter_range_slider(
    value: i32,
    min: i32,
    max: i32,
    disabled: bool,
    target: AudioFilterRangeTarget,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = signed_range_fraction(value, min, max);
    let drag = AudioFilterRangeDrag { target, min, max };
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    frame_slider(
        audio_slider_id(target),
        audio_slider_label(target),
        fraction,
        disabled,
    )
    .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
        if disabled {
            return;
        }
        owner.update(cx, move |root, cx| {
            if let Some(next) = signed_range_value_for_key(value, min, max, "right")
                && root.update_selected_config(|config| {
                    apply_audio_filter_range(config, target, true, next)
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
                    apply_audio_filter_range(config, target, true, next)
                })
            {
                root.defer_filter_preview_reconfigure(cx);
                cx.notify();
            }
        });
    })
    .when(!disabled, |slider| {
        slider.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| AudioFilterRangeDragPreview)
        })
    })
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<AudioFilterRangeDrag>, _window, cx| {
            let drag = *event.drag(cx);
            let fraction = timeline_slider_percent_from_bounds(event.event.position, event.bounds);
            let value = signed_range_value_from_fraction(fraction, drag.min, drag.max);
            if root.update_selected_config(|config| {
                apply_audio_filter_range(config, drag.target, true, value)
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
                apply_audio_filter_range(config, target, true, value)
            }) {
                root.defer_filter_preview_reconfigure(cx);
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
    .child(
        frame_slider_handle(audio_handle_id(target), fraction, !disabled).when(
            !disabled,
            |handle| {
                handle.on_drag(drag, |_drag, _position, _window, cx| {
                    cx.new(|_| AudioFilterRangeDragPreview)
                })
            },
        ),
    )
}

fn settings_audio_filter_reset(
    target: AudioFilterRangeTarget,
    default_value: i32,
    disabled: bool,
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

fn settings_audio_normalize_control(
    checked: bool,
    disabled: bool,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(
        "settings-audio-normalize-row",
        "Normalize audio",
        "",
        checked,
        disabled,
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

fn settings_audio_compressor_control(
    enabled: bool,
    strength: FilterStrength,
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).mt_1().gap_2();
    for (candidate, label) in [
        (FilterStrength::Low, "Gentle"),
        (FilterStrength::Medium, "Balanced"),
        (FilterStrength::High, "Strong"),
    ] {
        grid = grid.child(
            frame_choice_button(
                format!(
                    "settings-audio-compressor-{}",
                    filter_strength_id(candidate)
                ),
                label,
                enabled && strength == candidate,
                !disabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_audio_compressor(config, true, candidate)
                }) {
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
            "settings-audio-compressor-toggle",
            "Compressor",
            "",
            enabled,
            disabled,
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

fn settings_audio_filters_reset_all(
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "settings-audio-filters-reset-all",
        "Reset audio filters",
        ButtonVariant::Secondary,
        false,
        !disabled,
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

fn apply_audio_filter_range(
    config: &mut ConversionConfig,
    target: AudioFilterRangeTarget,
    enabled: bool,
    value: i32,
) -> bool {
    match target {
        AudioFilterRangeTarget::Volume => {
            let Ok(value) = u32::try_from(value) else {
                return false;
            };
            apply_audio_volume(config, value)
        }
        AudioFilterRangeTarget::Limiter => {
            apply_audio_scalar_filter(config, AudioScalarFilter::Limiter, enabled, value)
        }
        AudioFilterRangeTarget::Bass => {
            apply_audio_scalar_filter(config, AudioScalarFilter::Bass, enabled, value)
        }
        AudioFilterRangeTarget::Treble => {
            apply_audio_scalar_filter(config, AudioScalarFilter::Treble, enabled, value)
        }
        AudioFilterRangeTarget::HighPass => {
            apply_audio_scalar_filter(config, AudioScalarFilter::HighPass, enabled, value)
        }
        AudioFilterRangeTarget::LowPass => {
            apply_audio_scalar_filter(config, AudioScalarFilter::LowPass, enabled, value)
        }
        AudioFilterRangeTarget::NoiseReduction => {
            apply_audio_scalar_filter(config, AudioScalarFilter::NoiseReduction, enabled, value)
        }
        AudioFilterRangeTarget::DeEsser => {
            apply_audio_scalar_filter(config, AudioScalarFilter::DeEsser, enabled, value)
        }
        AudioFilterRangeTarget::StereoWidth => {
            apply_audio_scalar_filter(config, AudioScalarFilter::StereoWidth, enabled, value)
        }
    }
}

fn signed_range_fraction(value: i32, min: i32, max: i32) -> f32 {
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

fn signed_range_value_from_fraction(fraction: f64, min: i32, max: i32) -> i32 {
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

fn signed_range_value_for_key(value: i32, min: i32, max: i32, key: &str) -> Option<i32> {
    let offset = u32::try_from(value.clamp(min, max) - min).ok()?;
    let span = u32::try_from(max - min).ok()?;
    let next = range_value_for_key(offset, 0, span, key)?;
    i32::try_from(next).ok().map(|next| next + min)
}

const fn audio_target_id(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "volume",
        AudioFilterRangeTarget::Limiter => "limiter",
        AudioFilterRangeTarget::Bass => "bass",
        AudioFilterRangeTarget::Treble => "treble",
        AudioFilterRangeTarget::HighPass => "high-pass",
        AudioFilterRangeTarget::LowPass => "low-pass",
        AudioFilterRangeTarget::NoiseReduction => "noise-reduction",
        AudioFilterRangeTarget::DeEsser => "de-esser",
        AudioFilterRangeTarget::StereoWidth => "stereo-width",
    }
}

const fn audio_slider_id(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "settings-audio-filter-volume-slider",
        AudioFilterRangeTarget::Limiter => "settings-audio-filter-limiter-slider",
        AudioFilterRangeTarget::Bass => "settings-audio-filter-bass-slider",
        AudioFilterRangeTarget::Treble => "settings-audio-filter-treble-slider",
        AudioFilterRangeTarget::HighPass => "settings-audio-filter-high-pass-slider",
        AudioFilterRangeTarget::LowPass => "settings-audio-filter-low-pass-slider",
        AudioFilterRangeTarget::NoiseReduction => "settings-audio-filter-noise-reduction-slider",
        AudioFilterRangeTarget::DeEsser => "settings-audio-filter-de-esser-slider",
        AudioFilterRangeTarget::StereoWidth => "settings-audio-filter-stereo-width-slider",
    }
}

const fn audio_handle_id(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "settings-audio-filter-volume-handle",
        AudioFilterRangeTarget::Limiter => "settings-audio-filter-limiter-handle",
        AudioFilterRangeTarget::Bass => "settings-audio-filter-bass-handle",
        AudioFilterRangeTarget::Treble => "settings-audio-filter-treble-handle",
        AudioFilterRangeTarget::HighPass => "settings-audio-filter-high-pass-handle",
        AudioFilterRangeTarget::LowPass => "settings-audio-filter-low-pass-handle",
        AudioFilterRangeTarget::NoiseReduction => "settings-audio-filter-noise-reduction-handle",
        AudioFilterRangeTarget::DeEsser => "settings-audio-filter-de-esser-handle",
        AudioFilterRangeTarget::StereoWidth => "settings-audio-filter-stereo-width-handle",
    }
}

const fn audio_slider_label(target: AudioFilterRangeTarget) -> &'static str {
    match target {
        AudioFilterRangeTarget::Volume => "Volume",
        AudioFilterRangeTarget::Limiter => "Limiter",
        AudioFilterRangeTarget::Bass => "Bass",
        AudioFilterRangeTarget::Treble => "Treble",
        AudioFilterRangeTarget::HighPass => "High-pass",
        AudioFilterRangeTarget::LowPass => "Low-pass",
        AudioFilterRangeTarget::NoiseReduction => "Noise reduction",
        AudioFilterRangeTarget::DeEsser => "De-esser",
        AudioFilterRangeTarget::StereoWidth => "Stereo width",
    }
}

const fn filter_strength_id(strength: FilterStrength) -> &'static str {
    match strength {
        FilterStrength::Low => "low",
        FilterStrength::Medium => "medium",
        FilterStrength::High => "high",
    }
}
