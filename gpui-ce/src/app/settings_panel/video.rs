use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsVideoRangeTarget {
    Crf,
    Quality,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SettingsVideoRangeDrag {
    target: SettingsVideoRangeTarget,
    min: u32,
    max: u32,
}

struct SettingsVideoRangeDragPreview;

impl Render for SettingsVideoRangeDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

pub(in crate::app) fn settings_video_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    focuses: SettingsVideoInputFocuses<'_>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let is_gif_mode = is_gif_container(&config.container);
    let mut content = div()
        .flex()
        .flex_col()
        .gap_4()
        .child(settings_video_resolution_section(
            config,
            settings_disabled,
            focuses.width,
            focuses.height,
            window,
            cx,
        ))
        .child(settings_video_scaling_section(
            config,
            settings_disabled,
            cx,
        ));

    content = content.child(settings_video_fps_section(config, settings_disabled, cx));

    if is_gif_mode {
        return content
            .child(settings_video_gif_colors_section(
                config,
                settings_disabled,
                cx,
            ))
            .child(settings_video_gif_dither_section(
                config,
                settings_disabled,
                cx,
            ))
            .child(settings_video_gif_loop_section(
                config,
                settings_disabled,
                focuses.gif_loop,
                window,
                cx,
            ));
    }

    content
        .child(settings_video_encoder_section(
            config,
            settings_disabled,
            available_encoders,
            cx,
        ))
        .child(settings_video_pixel_format_section(
            config,
            settings_disabled,
            cx,
        ))
        .when(!is_videotoolbox_video_codec(&config.video_codec), |this| {
            this.child(settings_video_preset_section(config, settings_disabled, cx))
        })
        .child(settings_video_quality_section(
            config,
            settings_disabled,
            focuses.bitrate,
            window,
            cx,
        ))
        .when(is_nvenc_video_codec(&config.video_codec), |this| {
            this.child(settings_video_nvenc_section(config, settings_disabled, cx))
        })
        .when(is_videotoolbox_video_codec(&config.video_codec), |this| {
            this.child(settings_video_videotoolbox_section(
                config,
                settings_disabled,
                cx,
            ))
        })
        .when(is_hardware_video_codec(&config.video_codec), |this| {
            this.child(settings_video_hw_section(config, settings_disabled, cx))
        })
}

pub(in crate::app) fn settings_video_resolution_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_width_focus: Option<&FocusHandle>,
    video_height_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_section("RESOLUTION & FRAMERATE").child(settings_resolution_grid(
        config,
        settings_disabled,
        cx,
    ));

    if config.resolution == "custom" {
        section = section.child(settings_custom_dimensions_grid(
            config,
            settings_disabled,
            video_width_focus,
            video_height_focus,
            window,
            cx,
        ));
    }

    section
}

pub(in crate::app) fn settings_resolution_grid(
    config: &ConversionConfig,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for resolution in resolution_options() {
        let selected = config.resolution == *resolution;
        let label = resolution_label(resolution);
        grid = grid.child(
            frame_choice_button(
                format!("video-resolution-{resolution}"),
                label,
                selected,
                !disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_resolution(config, resolution)) {
                    cx.notify();
                }
            })),
        );
    }
    grid
}

fn settings_custom_dimensions_grid(
    config: &ConversionConfig,
    disabled: bool,
    video_width_focus: Option<&FocusHandle>,
    video_height_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap_2()
        .pt(px(4.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_field_label("WIDTH"))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-video-width-field",
                        value: config.custom_width.as_deref().unwrap_or_default(),
                        placeholder: "1920",
                        disabled,
                        focus: video_width_focus,
                        kind: FrameTextInputKind::VideoCustomWidth,
                    },
                    window,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_field_label("HEIGHT"))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-video-height-field",
                        value: config.custom_height.as_deref().unwrap_or_default(),
                        placeholder: "1080",
                        disabled,
                        focus: video_height_focus,
                        kind: FrameTextInputKind::VideoCustomHeight,
                    },
                    window,
                    cx,
                )),
        )
}

pub(in crate::app) fn settings_video_scaling_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let disabled = settings_disabled || config.resolution == "original";
    let mut grid = div().grid().grid_cols(2).gap_2();
    for algorithm in scaling_algorithm_options() {
        grid = grid.child(
            frame_choice_button(
                format!("video-scaling-{algorithm}"),
                scaling_algorithm_label(algorithm),
                config.scaling_algorithm == *algorithm,
                !disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_scaling_algorithm(config, algorithm))
                {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("SCALING ALGORITHM").child(grid)
}

pub(in crate::app) fn settings_video_fps_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let is_gif = is_gif_container(&config.container);
    let mut grid = div().grid().grid_cols(2).gap_2();
    for fps in fps_options(is_gif) {
        grid = grid.child(
            frame_choice_button(
                format!("video-fps-{fps}"),
                fps_label(fps),
                config.fps == *fps,
                !settings_disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if settings_disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_fps(config, fps)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("FRAMERATE").child(grid)
}

pub(in crate::app) fn settings_video_gif_colors_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for colors in gif_color_options() {
        grid = grid.child(
            frame_choice_button(
                format!("video-gif-colors-{colors}"),
                colors.to_string(),
                config.gif_colors == *colors,
                !settings_disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if settings_disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_gif_colors(config, *colors)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("PALETTE COLORS").child(grid)
}

pub(in crate::app) fn settings_video_gif_dither_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for dither in gif_dither_options() {
        let dither = *dither;
        list = list.child(
            frame_list_item_with_caption(
                format!("video-gif-dither-{dither}"),
                gif_dither_label(dither),
                dither.to_string(),
                config.gif_dither == dither,
                !settings_disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if settings_disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_gif_dither(config, dither)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("DITHERING").child(list)
}

fn settings_video_gif_loop_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    gif_loop_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("LOOP COUNT")
        .child(frame_text_input(
            FrameTextInputSpec {
                id: "settings-gif-loop-field",
                value: &config.gif_loop.to_string(),
                placeholder: "0",
                disabled: settings_disabled,
                focus: gif_loop_focus,
                kind: FrameTextInputKind::GifLoop,
            },
            window,
            cx,
        ))
        .child(settings_hint_text("Use 0 for infinite looping."))
}

fn settings_video_encoder_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in video_codec_options(config, available_encoders, settings_disabled) {
        let codec = option.codec;
        let enabled = !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("video-codec-{codec}"),
                codec.to_uppercase(),
                option.disabled_reason.unwrap_or(option.label).to_string(),
                option.is_selected,
                enabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_codec(config, codec)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("VIDEO ENCODER").child(list)
}

fn settings_video_pixel_format_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in video_pixel_format_options(config) {
        let pixel_format = option.id;
        let enabled = !settings_disabled && !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("video-pixel-format-{pixel_format}"),
                option.label,
                option.caption,
                option.is_selected,
                enabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_pixel_format(config, pixel_format)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("PIXEL FORMAT").child(list)
}

fn settings_video_preset_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in video_preset_options(config, settings_disabled) {
        let preset = option.preset;
        let enabled = !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("video-preset-{preset}"),
                option.label,
                option.caption,
                option.is_selected,
                enabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_preset(config, preset)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("ENCODING SPEED").child(list)
}

fn settings_video_quality_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_bitrate_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_section("QUALITY CONTROL").child(settings_video_bitrate_mode_grid(
        config,
        settings_disabled,
        cx,
    ));

    if config.video_bitrate_mode == "crf" {
        let is_hardware = is_hardware_video_codec(&config.video_codec);
        section = section.child(settings_video_range_field(
            if is_hardware {
                "ENCODING QUALITY"
            } else {
                "QUALITY FACTOR"
            },
            if is_hardware {
                format!("Q {}", config.quality)
            } else {
                format!("CRF {}", config.crf)
            },
            if is_hardware {
                config.quality
            } else {
                u32::from(config.crf)
            },
            if is_hardware { 1 } else { 0 },
            if is_hardware { 100 } else { 51 },
            if is_hardware {
                "LOW QUALITY"
            } else {
                "LOSSLESS"
            },
            if is_hardware {
                "BEST QUALITY"
            } else {
                "SMALLEST"
            },
            if is_hardware {
                SettingsVideoRangeTarget::Quality
            } else {
                SettingsVideoRangeTarget::Crf
            },
            settings_disabled,
            cx,
        ));
    } else {
        section = section.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .pt(px(4.0))
                .child(settings_field_label("TARGET BITRATE (KBPS)"))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-video-bitrate-field",
                        value: &config.video_bitrate,
                        placeholder: "5000",
                        disabled: settings_disabled,
                        focus: video_bitrate_focus,
                        kind: FrameTextInputKind::VideoBitrate,
                    },
                    window,
                    cx,
                )),
        );
    }

    section
}

fn settings_video_bitrate_mode_grid(
    config: &ConversionConfig,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for (mode, label) in [("crf", "Constant Quality"), ("bitrate", "Target Bitrate")] {
        grid = grid.child(
            frame_choice_button(
                format!("video-bitrate-mode-{mode}"),
                label,
                config.video_bitrate_mode == mode,
                !disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_bitrate_mode(config, mode)) {
                    cx.notify();
                }
            })),
        );
    }
    grid
}

#[expect(
    clippy::too_many_arguments,
    reason = "keeps slider construction close to visual contract"
)]
fn settings_video_range_field(
    label: &'static str,
    value_label: String,
    value: u32,
    min: u32,
    max: u32,
    lower_label: &'static str,
    upper_label: &'static str,
    target: SettingsVideoRangeTarget,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .pt(px(4.0))
        .child(
            div()
                .flex()
                .items_end()
                .justify_between()
                .child(settings_field_label(label))
                .child(settings_value_badge(value_label)),
        )
        .child(settings_video_range_slider(
            value, min, max, disabled, target, cx,
        ))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(lower_label)
                .child(upper_label),
        )
}

fn settings_video_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsVideoRangeTarget,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = range_fraction(value, min, max);
    let drag = SettingsVideoRangeDrag { target, min, max };

    frame_slider(
        match target {
            SettingsVideoRangeTarget::Crf => "settings-video-crf-slider",
            SettingsVideoRangeTarget::Quality => "settings-video-quality-slider",
        },
        fraction,
        disabled,
    )
    .on_drag_move(cx.listener(
        |root, event: &DragMoveEvent<SettingsVideoRangeDrag>, _window, cx| {
            let drag = *event.drag(cx);
            let fraction = timeline_slider_percent_from_bounds(event.event.position, event.bounds);
            let value = range_value_from_fraction(fraction, drag.min, drag.max);
            let changed = root.update_selected_config(|config| match drag.target {
                SettingsVideoRangeTarget::Crf => apply_crf(config, value as u8),
                SettingsVideoRangeTarget::Quality => apply_quality(config, value),
            });
            if changed {
                cx.notify();
            }
        },
    ))
    .child(settings_video_range_handle(fraction, drag, !disabled))
}

fn settings_video_range_handle(
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

fn settings_video_nvenc_section(
    config: &ConversionConfig,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("NVENC OPTIONS")
        .child(
            settings_video_checkbox_row(
                "video-nvenc-spatial-aq",
                "SPATIAL AQ",
                "Improves detail in scenes with high complexity",
                config.nvenc_spatial_aq,
                disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_nvenc_spatial_aq(config, !config.nvenc_spatial_aq)
                }) {
                    cx.notify();
                }
            })),
        )
        .child(
            settings_video_checkbox_row(
                "video-nvenc-temporal-aq",
                "TEMPORAL AQ",
                "Stabilizes quality between frames",
                config.nvenc_temporal_aq,
                disabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_nvenc_temporal_aq(config, !config.nvenc_temporal_aq)
                }) {
                    cx.notify();
                }
            })),
        )
}

fn settings_video_videotoolbox_section(
    config: &ConversionConfig,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("VIDEOTOOLBOX OPTIONS").child(
        settings_video_checkbox_row(
            "video-videotoolbox-allow-sw",
            "ALLOW SOFTWARE FALLBACK",
            "Drop back to CPU encoding if hardware fails",
            config.videotoolbox_allow_sw,
            disabled,
        )
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            if root.update_selected_config(|config| {
                apply_videotoolbox_allow_sw(config, !config.videotoolbox_allow_sw)
            }) {
                cx.notify();
            }
        })),
    )
}

fn settings_video_hw_section(
    config: &ConversionConfig,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("HARDWARE ACCELERATION").child(
        settings_video_checkbox_row(
            "video-hw-decode",
            "HARDWARE DECODING",
            "Use GPU for decoding input video (faster)",
            config.hw_decode,
            disabled,
        )
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            if root.update_selected_config(|config| apply_hw_decode(config, !config.hw_decode)) {
                cx.notify();
            }
        })),
    )
}

fn settings_video_checkbox_row(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    checked: bool,
    disabled: bool,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(id, label, hint, checked, disabled)
}

fn resolution_label(resolution: &str) -> &'static str {
    match resolution {
        "original" => "Original",
        "custom" => "Custom",
        "1080p" => "1080p",
        "720p" => "720p",
        "480p" => "480p",
        _ => "Original",
    }
}

fn scaling_algorithm_label(algorithm: &str) -> &'static str {
    match algorithm {
        "bicubic" => "Bicubic",
        "lanczos" => "Lanczos",
        "bilinear" => "Bilinear",
        "nearest" => "Nearest",
        _ => "Bicubic",
    }
}

fn fps_label(fps: &str) -> String {
    if fps == "original" {
        "Same as source".to_string()
    } else {
        format!("{fps} fps")
    }
}

fn gif_dither_label(dither: &str) -> &'static str {
    match dither {
        "sierra2_4a" => "Sierra2_4a",
        "floyd_steinberg" => "Floyd-Steinberg",
        "bayer" => "Bayer",
        "none" => "None",
        _ => "Sierra2_4a",
    }
}
