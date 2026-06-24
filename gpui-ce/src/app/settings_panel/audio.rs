use super::*;

pub(in crate::app) fn settings_audio_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    audio_bitrate_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut channels_section = settings_section("CHANNELS / BITRATE")
        .child(settings_audio_channels_grid(config, settings_disabled, cx))
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
        .child(settings_section("CODEC").child(settings_audio_codec_list(
            config,
            settings_disabled,
            cx,
        )));

    let track_options = audio_track_options(config, metadata, settings_disabled);
    if track_options.is_empty() {
        return content.child(
            settings_section("SOURCE TRACKS").child(settings_hint_text("No audio tracks.")),
        );
    }

    let mut list = div().flex().flex_col().gap_2();
    for option in track_options {
        list = list.child(settings_audio_track_button(option, cx));
    }

    content.child(settings_section("SOURCE TRACKS").child(list))
}

pub(in crate::app) fn settings_audio_channels_grid(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).gap_2();
    for option in audio_channel_options(config, settings_disabled) {
        let channels = option.id;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            settings_choice_button(
                format!("audio-channels-{channels}"),
                option.label,
                option.is_selected,
                is_enabled,
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
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let controls_disabled = settings_disabled || config.processing_mode == ProcessingMode::Copy;
    let is_lossless = is_lossless_audio_codec(&config.audio_codec);
    let show_vbr_toggle = !is_lossless && audio_codec_supports_vbr(&config.audio_codec);
    let is_vbr = show_vbr_toggle && config.audio_bitrate_mode == "vbr";

    let mut controls = div().flex().flex_col().gap_3();
    if show_vbr_toggle {
        controls = controls
            .child(settings_field_label("QUALITY CONTROL"))
            .child(settings_audio_bitrate_mode_grid(
                config,
                controls_disabled,
                cx,
            ));
    }

    if is_vbr {
        if let Some(range) = audio_quality_range(&config.audio_codec) {
            let value = parse_audio_value(&config.audio_quality, range.default_value)
                .clamp(range.min, range.max);
            let lower_label = if range.lower_is_better {
                "BEST"
            } else {
                "SMALLEST"
            };
            let upper_label = if range.lower_is_better {
                "SMALLEST"
            } else {
                "BEST"
            };
            controls = controls.child(settings_audio_range_field(
                SettingsAudioRangeSpec {
                    label: "QUALITY LEVEL",
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
                label: "VOLUME",
                value_label: format!("{}%", config.audio_volume),
                value: config.audio_volume,
                min: 0,
                max: 200,
                lower_label: "MUTED",
                upper_label: "MAX VOLUME",
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
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for (mode, label) in [("bitrate", "Target Bitrate"), ("vbr", "Variable Bitrate")] {
        let selected = config.audio_bitrate_mode == mode;
        let enabled =
            !disabled && (mode == "bitrate" || audio_codec_supports_vbr(&config.audio_codec));
        grid = grid.child(
            settings_choice_button(
                format!("audio-bitrate-mode-{mode}"),
                label,
                selected,
                enabled,
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
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("BITRATE (KB/S)"))
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
                .child(spec.lower_label)
                .child(spec.upper_label),
        )
}

fn settings_audio_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsAudioRangeTarget,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = range_fraction(value, min, max);
    let drag = SettingsAudioRangeDrag { target, min, max };

    div()
        .id(match target {
            SettingsAudioRangeTarget::Quality => "settings-audio-quality-slider",
            SettingsAudioRangeTarget::Volume => "settings-audio-volume-slider",
        })
        .relative()
        .h(px(20.0))
        .w_full()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(!disabled, |this| this.cursor_pointer())
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<SettingsAudioRangeDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let fraction =
                    timeline_slider_percent_from_bounds(event.event.position, event.bounds);
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
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(8.0))
                .h(px(4.0))
                .rounded(px(2.0))
                .bg(color(theme::FRAME_GRAY_100))
                .shadow(input_highlight_shadows()),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .top(px(8.0))
                .h(px(4.0))
                .w(relative(fraction))
                .rounded(px(2.0))
                .bg(color(theme::FOREGROUND)),
        )
        .child(settings_audio_range_handle(fraction, drag, !disabled))
}

fn settings_audio_range_handle(
    fraction: f32,
    drag: SettingsAudioRangeDrag,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle = div()
        .id(match drag.target {
            SettingsAudioRangeTarget::Quality => "settings-audio-quality-handle",
            SettingsAudioRangeTarget::Volume => "settings-audio-volume-handle",
        })
        .absolute()
        .left(relative(fraction))
        .top(px(3.0))
        .ml(px(-5.0))
        .w(px(10.0))
        .h(px(14.0))
        .rounded(px(5.0))
        .bg(color(theme::FOREGROUND))
        .shadow(button_highlight_shadows())
        .when(enabled, |this| this.cursor_ew_resize());

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
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let checkbox = div()
        .id("settings-audio-normalize")
        .w(px(14.0))
        .h(px(14.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .bg(color(theme::BACKGROUND))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .shadow(input_highlight_shadows())
        .child(
            div()
                .w(px(8.0))
                .h(px(8.0))
                .rounded(px(2.0))
                .bg(color(theme::FRAME_GRAY_600))
                .opacity(if checked { 1.0 } else { 0.0 }),
        );

    div()
        .id("settings-audio-normalize-row")
        .flex()
        .items_start()
        .gap_2()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(!disabled, |this| this.cursor_pointer())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            if root.update_selected_config(|config| apply_audio_normalize(config, !checked)) {
                cx.notify();
            }
        }))
        .child(checkbox)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_field_label("NORMALIZE AUDIO"))
                .child(settings_hint_text("Smooth out loudness differences.")),
        )
}

pub(in crate::app) fn settings_audio_codec_list(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in audio_codec_options(config, settings_disabled) {
        list = list.child(settings_audio_codec_button(option, cx));
    }

    list
}

pub(in crate::app) fn settings_audio_codec_button(
    option: crate::settings::AudioCodecOption,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(
        ButtonVariant::Secondary,
        option.is_selected,
        !option.is_disabled,
    );
    let codec = option.codec;
    let is_enabled = !option.is_disabled;
    let caption = option.disabled_reason.unwrap_or(option.label);

    div()
        .id(format!("audio-codec-{codec}"))
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(is_enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!is_enabled, |this| this.cursor_not_allowed())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if !is_enabled {
                return;
            }
            if root.update_selected_config(|config| apply_audio_codec(config, codec)) {
                cx.notify();
            }
        }))
        .child(
            div()
                .text_color(color(theme::FOREGROUND))
                .child(codec.to_uppercase()),
        )
        .child(
            div()
                .truncate()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(caption),
        )
}

pub(in crate::app) fn settings_audio_track_button(
    option: crate::settings::AudioTrackOption,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(
        ButtonVariant::Secondary,
        option.is_selected,
        !option.is_disabled,
    );
    let index = option.index;
    let is_enabled = !option.is_disabled;
    let is_selected = option.is_selected;

    div()
        .id(format!("audio-track-{index}"))
        .min_h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .py(px(6.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(is_enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!is_enabled, |this| this.cursor_not_allowed())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if !is_enabled {
                return;
            }
            if root.update_selected_config(|config| toggle_audio_track_selection(config, index)) {
                cx.notify();
            }
        }))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_color(color(theme::FRAME_GRAY_600))
                                .child(option.index_label),
                        )
                        .child(
                            div()
                                .text_color(color(theme::FOREGROUND))
                                .child(option.codec),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(option.detail),
                ),
        )
        .child(selection_dot(is_selected))
}
