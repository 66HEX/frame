use super::*;

pub(in crate::app) fn settings_audio_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    audio_bitrate_focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut channels_section = settings_section("Channels / bitrate")
        .child(settings_audio_channels_grid(
            config,
            settings_disabled,
            window,
            cx,
        ))
        .child(settings_audio_encoding_controls(
            config,
            settings_disabled,
            audio_bitrate_focus,
            window,
            cx,
        ));
    if config.processing_mode == ProcessingMode::Copy {
        channels_section = channels_section.child(settings_hint_text(
            "Stream copy keeps source audio settings.",
        ));
    }

    let content = div()
        .flex()
        .flex_col()
        .gap_4()
        .child(channels_section)
        .child(settings_section("Codec").child(settings_audio_codec_list(
            config,
            available_encoders,
            settings_disabled,
            window,
            cx,
        )));

    let track_options = audio_track_options(config, metadata, settings_disabled);
    if track_options.is_empty() {
        return content.child(
            settings_section("Source tracks").child(settings_hint_text("No audio tracks.")),
        );
    }

    let mut list = div().flex().flex_col().gap_2();
    for option in track_options {
        list = list.child(settings_audio_track_button(option, window, cx));
    }

    content.child(settings_section("Source tracks").child(list))
}

pub(in crate::app) fn settings_audio_channels_grid(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).gap_2();
    for option in audio_channel_options(config, settings_disabled) {
        let channels = option.id;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            frame_choice_button(
                format!("audio-channels-{channels}"),
                option.label,
                option.is_selected,
                is_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_audio_channels(config, channels)) {
                    cx.notify();
                }
            })),
        );
    }

    grid
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsAudioRangeTarget {
    Quality,
    Volume,
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
        div().w(px(0.0)).h(px(0.0))
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
            .child(settings_field_label("Quality control"))
            .child(settings_audio_bitrate_mode_grid(
                config,
                controls_disabled,
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
                cx,
            ));
        }
    } else {
        controls = controls.child(settings_audio_bitrate_field(
            config,
            controls_disabled || is_lossless,
            is_lossless,
            audio_bitrate_focus,
            window,
            cx,
        ));
    }

    controls
        .child(settings_audio_range_field(
            SettingsAudioRangeSpec {
                label: "Volume",
                value_label: format!("{}%", config.audio_volume),
                value: config.audio_volume,
                min: 0,
                max: 200,
                lower_label: "Muted",
                upper_label: "Max volume",
                target: SettingsAudioRangeTarget::Volume,
            },
            controls_disabled,
            cx,
        ))
        .child(settings_audio_normalize_toggle(
            config.audio_normalize,
            controls_disabled,
            cx,
        ))
}

fn settings_audio_bitrate_mode_grid(
    config: &ConversionConfig,
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for (mode, label) in [("bitrate", "Target Bitrate"), ("vbr", "Variable Bitrate")] {
        let selected = config.audio_bitrate_mode == mode;
        let enabled =
            !disabled && (mode == "bitrate" || audio_codec_supports_vbr(&config.audio_codec));
        grid = grid.child(
            frame_choice_button(
                format!("audio-bitrate-mode-{mode}"),
                label,
                selected,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_audio_bitrate_mode(config, mode)) {
                    cx.notify();
                }
            })),
        );
    }

    grid
}

fn settings_audio_bitrate_field(
    config: &ConversionConfig,
    disabled: bool,
    is_lossless: bool,
    audio_bitrate_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("Bitrate (KB/s)"))
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
            window,
            cx,
        ))
}

fn settings_audio_range_field(
    spec: SettingsAudioRangeSpec,
    disabled: bool,
    cx: &Context<FrameRoot>,
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
                .child(settings_field_label(spec.label))
                .child(settings_value_badge(spec.value_label)),
        )
        .child(settings_audio_range_slider(
            spec.value,
            spec.min,
            spec.max,
            disabled,
            spec.target,
            cx,
        ))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(theme::ui_text(spec.lower_label))
                .child(theme::ui_text(spec.upper_label)),
        )
}

fn settings_audio_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsAudioRangeTarget,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = range_fraction(value, min, max);
    let drag = SettingsAudioRangeDrag { target, min, max };

    frame_slider(
        match target {
            SettingsAudioRangeTarget::Quality => "settings-audio-quality-slider",
            SettingsAudioRangeTarget::Volume => "settings-audio-volume-slider",
        },
        fraction,
        disabled,
    )
    .when(!disabled, |slider| {
        slider.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsAudioRangeDragPreview)
        })
    })
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<SettingsAudioRangeDrag>, _window, cx| {
            let drag = *event.drag(cx);
            let fraction = timeline_slider_percent_from_bounds(event.event.position, event.bounds);
            let value = range_value_from_fraction(fraction, drag.min, drag.max);
            let changed = root.update_selected_config(|config| match drag.target {
                SettingsAudioRangeTarget::Quality => {
                    apply_audio_quality(config, &value.to_string())
                }
                SettingsAudioRangeTarget::Volume => apply_audio_volume(config, value),
            });
            if changed {
                cx.notify();
            }
        },
    ))
    .child(settings_audio_range_handle(fraction, drag, !disabled))
}

fn settings_audio_range_handle(
    fraction: f32,
    drag: SettingsAudioRangeDrag,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle = frame_slider_handle(
        match drag.target {
            SettingsAudioRangeTarget::Quality => "settings-audio-quality-handle",
            SettingsAudioRangeTarget::Volume => "settings-audio-volume-handle",
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

fn settings_audio_normalize_toggle(
    checked: bool,
    disabled: bool,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(
        "settings-audio-normalize-row",
        "Normalize audio",
        "Smooth out loudness differences.",
        checked,
        disabled,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if disabled {
            return;
        }
        if root.update_selected_config(|config| apply_audio_normalize(config, !checked)) {
            cx.notify();
        }
    }))
}

pub(in crate::app) fn settings_audio_codec_list(
    config: &ConversionConfig,
    available_encoders: &AvailableEncoders,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in audio_codec_options(config, available_encoders, settings_disabled) {
        list = list.child(settings_audio_codec_button(option, window, cx));
    }

    list
}

pub(in crate::app) fn settings_audio_codec_button(
    option: crate::settings::AudioCodecOption,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let codec = option.codec;
    let is_enabled = !option.is_disabled;
    let caption = option.disabled_reason.unwrap_or(option.label);

    frame_list_item_with_caption(
        format!("audio-codec-{codec}"),
        codec,
        caption,
        option.is_selected,
        is_enabled,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !is_enabled {
            return;
        }
        if root.update_selected_config(|config| apply_audio_codec(config, codec)) {
            cx.notify();
        }
    }))
}

pub(in crate::app) fn settings_audio_track_button(
    option: crate::settings::AudioTrackOption,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let index = option.index;
    let is_enabled = !option.is_disabled;

    frame_track_list_item(
        format!("audio-track-{index}"),
        FrameTrackListItemText {
            index_label: option.index_label,
            primary: option.codec,
            detail: option.detail,
        },
        option.is_selected,
        is_enabled,
        FrameTrackListItemLayout::Stacked,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !is_enabled {
            return;
        }
        if root.update_selected_config(|config| toggle_audio_track_selection(config, index)) {
            cx.notify();
        }
    }))
}
