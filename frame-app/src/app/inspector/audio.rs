use crate::app::components::{
    FrameOption, frame_boolean_field, frame_option_field, frame_select_field,
};
use crate::app::components::{
    frame_slider, frame_slider_fraction_from_bounds, frame_slider_handle,
};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::panel::settings_field;
use crate::app::inspector::shared::{
    is_lossless_audio_codec, parse_audio_value, range_fraction, range_value_for_key,
    range_value_from_fraction, settings_field_label, settings_hint_text, settings_value_badge,
};
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, ProcessingMode, SourceMetadata, apply_audio_bitrate_mode,
    apply_audio_channels, apply_audio_codec, apply_audio_quality, apply_opus_vbr,
    audio_channel_options, audio_codec_options, audio_codec_supports_vbr, audio_quality_range,
    audio_track_options, mp2_original_channels_are_unsupported, toggle_audio_track_selection,
};
use crate::theme;
use frame_core::capabilities::AvailableEncoders;
use gpui::prelude::*;
use gpui::{Context, DragMoveEvent, FocusHandle, IntoElement, Render, Window, div};

#[expect(
    clippy::too_many_arguments,
    reason = "The audio tab explicitly receives conversion state, capabilities, focus, palette, and render context."
)]
pub(in crate::app) fn settings_audio_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    audio_bitrate_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut encoding = settings_audio_encoding_controls(
        config,
        settings_disabled,
        audio_bitrate_focus,
        palette,
        window,
        cx,
    );
    if config.processing_mode == ProcessingMode::Copy {
        encoding = encoding.child(settings_hint_text(
            "Stream copy keeps source audio settings.",
            palette,
        ));
    } else if mp2_original_channels_are_unsupported(config, metadata) {
        encoding = encoding.child(settings_hint_text(
            "MP2 supports at most two channels; multichannel source tracks are exported as stereo.",
            palette,
        ));
    }
    let content = crate::app::components::frame_field_group()
        .child(
            settings_field("Codec", palette).child(settings_audio_codec_list(
                config,
                available_encoders,
                settings_disabled,
                palette,
                window,
                cx,
            )),
        )
        .child(
            settings_field("Channels", palette).child(settings_audio_channels_grid(
                config,
                metadata,
                settings_disabled,
                palette,
                window,
                cx,
            )),
        )
        .child(encoding);

    let track_options = audio_track_options(config, metadata, settings_disabled);
    if track_options.is_empty() {
        return content.child(
            settings_field("Source tracks", palette)
                .child(settings_hint_text("No audio tracks.", palette)),
        );
    }

    let mut list = crate::app::components::frame_list_group();
    for option in track_options {
        list = list.child(settings_audio_track_button(&option, palette, window, cx));
    }

    content.child(settings_field("Source tracks", palette).child(list))
}

pub(in crate::app) fn settings_audio_channels_grid(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = Vec::new();
    for option in audio_channel_options(config, metadata, settings_disabled) {
        let channels = option.id;
        let is_enabled = !option.is_disabled;
        grid.push(FrameOption::new(
            format!("audio-channels-{channels}"),
            option.label,
            option.is_selected,
            is_enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_audio_channels(config, channels)) {
                    cx.notify();
                }
            },
        ));
    }
    frame_option_field(
        "audio-channels-select",
        "Channels",
        grid,
        palette,
        window,
        cx,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsAudioRangeTarget {
    Quality,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SettingsAudioRangeDrag {
    target: SettingsAudioRangeTarget,
    min: u32,
    max: u32,
}

struct SettingsAudioRangeDragPreview;

impl Render for SettingsAudioRangeDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(theme::ui_rem(0.0)).h(theme::ui_rem(0.0))
    }
}

struct SettingsAudioRangeSpec {
    label: &'static str,
    value_label: String,
    value: u32,
    min: u32,
    max: u32,
    lower_label: &'static str,
    upper_label: &'static str,
    target: SettingsAudioRangeTarget,
}

fn settings_audio_encoding_controls(
    config: &ConversionConfig,
    settings_disabled: bool,
    audio_bitrate_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let controls_disabled = settings_disabled || config.processing_mode == ProcessingMode::Copy;
    let is_lossless = is_lossless_audio_codec(&config.audio_codec);
    let show_vbr_toggle = !is_lossless && audio_codec_supports_vbr(&config.audio_codec);
    let is_vbr = show_vbr_toggle && config.audio_bitrate_mode == "vbr";

    let mut controls = div().flex().flex_col().gap_3();
    if show_vbr_toggle {
        controls = controls
            .child(settings_field_label("Quality control", palette))
            .child(settings_audio_bitrate_mode_grid(
                config,
                controls_disabled,
                palette,
                window,
                cx,
            ));
    }

    if is_vbr {
        if let Some(range) = audio_quality_range(&config.audio_codec) {
            let value = parse_audio_value(&config.audio_quality, range.default_value)
                .clamp(range.min, range.max);
            let lower_label = if range.lower_is_better {
                "Best"
            } else {
                "Smallest"
            };
            let upper_label = if range.lower_is_better {
                "Smallest"
            } else {
                "Best"
            };
            controls = controls.child(settings_audio_range_field(
                SettingsAudioRangeSpec {
                    label: "Quality level",
                    value_label: format!("Q {value}"),
                    value,
                    min: range.min,
                    max: range.max,
                    lower_label,
                    upper_label,
                    target: SettingsAudioRangeTarget::Quality,
                },
                controls_disabled,
                palette,
                window,
                cx,
            ));
        }
    } else {
        controls = controls.child(settings_audio_bitrate_field(
            config,
            controls_disabled || is_lossless,
            is_lossless,
            audio_bitrate_focus,
            palette,
            window,
            cx,
        ));
    }

    if config.audio_codec == "libopus" {
        controls = controls.child(settings_field("Variable bitrate", palette).child(
            frame_boolean_field(
                "audio-opus-vbr",
                "Variable bitrate",
                config.opus_vbr,
                !controls_disabled,
                ["No", "Yes"],
                |root, value, _window, cx| {
                    if root.update_selected_config(|config| apply_opus_vbr(config, value)) {
                        cx.notify();
                    }
                },
                palette,
                window,
                cx,
            ),
        ));
    }

    controls
}

fn settings_audio_bitrate_mode_grid(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = Vec::new();
    for (mode, label) in [("bitrate", "Target Bitrate"), ("vbr", "Variable Bitrate")] {
        let selected = config.audio_bitrate_mode == mode;
        let enabled =
            !disabled && (mode == "bitrate" || audio_codec_supports_vbr(&config.audio_codec));
        grid.push(FrameOption::new(
            format!("audio-bitrate-mode-{mode}"),
            label,
            selected,
            enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_audio_bitrate_mode(config, mode)) {
                    cx.notify();
                }
            },
        ));
    }
    frame_option_field(
        "audio-bitrate-mode-select",
        "Quality control",
        grid,
        palette,
        window,
        cx,
    )
}

fn settings_audio_bitrate_field(
    config: &ConversionConfig,
    disabled: bool,
    is_lossless: bool,
    audio_bitrate_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    crate::app::components::frame_field()
        .child(settings_field_label("Bitrate (kbps)", palette))
        .child(frame_text_input(
            FrameTextInputSpec {
                id: "settings-audio-bitrate-field",
                value: if is_lossless {
                    ""
                } else {
                    &config.audio_bitrate
                },
                placeholder: if is_lossless {
                    "Bitrate ignored"
                } else {
                    "128"
                },
                disabled,
                focus: audio_bitrate_focus,
                kind: FrameTextInputKind::AudioBitrate,
            },
            palette,
            window,
            cx,
        ))
}

fn settings_audio_range_field(
    spec: SettingsAudioRangeSpec,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_end()
                .justify_between()
                .child(settings_field_label(spec.label, palette))
                .child(settings_value_badge(spec.value_label, palette)),
        )
        .child(settings_audio_range_slider(
            spec.value,
            spec.min,
            spec.max,
            disabled,
            spec.target,
            palette,
            window,
            cx,
        ))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.text_muted))
                .child(theme::ui_text(spec.lower_label))
                .child(theme::ui_text(spec.upper_label)),
        )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Range controls require domain limits, target, state and rendering context."
)]
fn settings_audio_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsAudioRangeTarget,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = range_fraction(value, min, max);
    let drag = SettingsAudioRangeDrag { target, min, max };
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    frame_slider(
        match target {
            SettingsAudioRangeTarget::Quality => "settings-audio-quality-slider",
        },
        match target {
            SettingsAudioRangeTarget::Quality => "Audio quality",
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
                    apply_settings_audio_range_value(config, target, value)
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
                    apply_settings_audio_range_value(config, target, value)
                })
            {
                cx.notify();
            }
        });
    })
    .when(!disabled, |slider| {
        slider.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsAudioRangeDragPreview)
        })
    })
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<SettingsAudioRangeDrag>, window, cx| {
            let drag = *event.drag(cx);
            let fraction = frame_slider_fraction_from_bounds(
                event.event.position,
                event.bounds,
                window.rem_size(),
            );
            let value = range_value_from_fraction(fraction, drag.min, drag.max);
            let changed = root.update_selected_config(|config| {
                apply_settings_audio_range_value(config, drag.target, value)
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
                apply_settings_audio_range_value(config, target, value)
            }) {
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
    .child(settings_audio_range_handle(fraction, drag, !disabled))
}

fn apply_settings_audio_range_value(
    config: &mut ConversionConfig,
    target: SettingsAudioRangeTarget,
    value: u32,
) -> bool {
    match target {
        SettingsAudioRangeTarget::Quality => apply_audio_quality(config, &value.to_string()),
    }
}

fn settings_audio_range_handle(
    fraction: f32,
    drag: SettingsAudioRangeDrag,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle = frame_slider_handle(
        match drag.target {
            SettingsAudioRangeTarget::Quality => "settings-audio-quality-handle",
        },
        fraction,
        enabled,
    );

    if enabled {
        handle.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsAudioRangeDragPreview)
        })
    } else {
        handle
    }
}

pub(in crate::app) fn settings_audio_codec_list(
    config: &ConversionConfig,
    available_encoders: &AvailableEncoders,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let choices = audio_codec_options(config, available_encoders, settings_disabled)
        .into_iter()
        .map(|option| {
            let codec = option.codec;
            FrameOption::with_caption(
                format!("audio-codec-{codec}"),
                codec,
                option.disabled_reason.unwrap_or(option.label),
                option.is_selected,
                !option.is_disabled,
                move |root, _, cx| {
                    if root.update_selected_config(|config| apply_audio_codec(config, codec)) {
                        cx.notify();
                    }
                },
            )
        })
        .collect();
    frame_select_field(
        "audio-codec-select",
        "Audio codec",
        choices,
        palette,
        window,
        cx,
    )
}

pub(in crate::app) fn settings_audio_track_button(
    option: &crate::settings::AudioTrackOption,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let index = option.index;
    crate::app::inspector::shared::settings_track_choice(
        format!("audio-track-{index}"),
        format!("{} · {}", option.index_label, option.codec),
        format!("{} · {}", option.detail, option.bitrate),
        option.is_selected,
        !option.is_disabled,
        move |root, _, cx| {
            if root.update_selected_config(|config| toggle_audio_track_selection(config, index)) {
                cx.notify();
            }
        },
        palette,
        window,
        cx,
    )
}
