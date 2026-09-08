use super::*;
use crate::numeric::u32_to_u8;

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
        div().w(theme::ui_rem(0.0)).h(theme::ui_rem(0.0))
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The declarative video tab keeps its conditional sections in the same order as the UI."
)]
pub(in crate::app) fn settings_video_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    focuses: SettingsVideoInputFocuses<'_>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
            palette,
            window,
            cx,
        ))
        .child(settings_video_scaling_section(
            config,
            settings_disabled,
            palette,
            window,
            cx,
        ));

    content = content.child(settings_video_fps_section(
        config,
        settings_disabled,
        palette,
        window,
        cx,
    ));

    if is_gif_mode {
        return content
            .child(settings_video_gif_colors_section(
                config,
                settings_disabled,
                palette,
                window,
                cx,
            ))
            .child(settings_video_gif_dither_section(
                config,
                settings_disabled,
                palette,
                window,
                cx,
            ))
            .child(settings_video_gif_loop_section(
                config,
                settings_disabled,
                focuses.gif_loop,
                palette,
                window,
                cx,
            ));
    }

    content
        .child(settings_video_encoder_section(
            config,
            settings_disabled,
            available_encoders,
            palette,
            window,
            cx,
        ))
        .child(settings_video_pixel_format_section(
            config,
            settings_disabled,
            palette,
            window,
            cx,
        ))
        .when(
            !is_videotoolbox_video_codec(&config.video_codec) && config.video_codec != "mpeg2video",
            |this| {
                this.child(settings_video_preset_section(
                    config,
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
            },
        )
        .child(settings_video_quality_section(
            config,
            settings_disabled,
            focuses.bitrate,
            palette,
            window,
            cx,
        ))
        .when(is_nvenc_video_codec(&config.video_codec), |this| {
            this.child(settings_video_nvenc_section(
                config,
                settings_disabled,
                palette,
                cx,
            ))
        })
        .when(is_videotoolbox_video_codec(&config.video_codec), |this| {
            this.child(settings_video_videotoolbox_section(
                config,
                settings_disabled,
                palette,
                cx,
            ))
        })
        .when(is_hardware_video_codec(&config.video_codec), |this| {
            this.child(settings_video_hw_section(
                config,
                settings_disabled,
                palette,
                cx,
            ))
        })
}

pub(in crate::app) fn settings_video_resolution_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_width_focus: Option<&FocusHandle>,
    video_height_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_section("分辨率与帧率", palette).child(
        settings_resolution_grid(config, settings_disabled, palette, window, cx),
    );

    if config.resolution == "custom" {
        section = section.child(settings_custom_dimensions_grid(
            config,
            settings_disabled,
            video_width_focus,
            video_height_focus,
            palette,
            window,
            cx,
        ));
    }

    section
}

pub(in crate::app) fn settings_resolution_grid(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap_2()
        .pt(theme::ui_rem(4.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_field_label("宽度", palette))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-video-width-field",
                        value: config.custom_width.as_deref().unwrap_or_default(),
                        placeholder: "1920",
                        disabled,
                        focus: video_width_focus,
                        kind: FrameTextInputKind::VideoCustomWidth,
                    },
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_field_label("高度", palette))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-video-height-field",
                        value: config.custom_height.as_deref().unwrap_or_default(),
                        placeholder: "1080",
                        disabled,
                        focus: video_height_focus,
                        kind: FrameTextInputKind::VideoCustomHeight,
                    },
                    palette,
                    window,
                    cx,
                )),
        )
}

pub(in crate::app) fn settings_video_scaling_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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

    settings_section("缩放算法", palette).child(grid)
}

pub(in crate::app) fn settings_video_fps_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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

    settings_section("帧率", palette).child(grid)
}

pub(in crate::app) fn settings_video_gif_colors_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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

    settings_section("调色板颜色", palette).child(grid)
}

pub(in crate::app) fn settings_video_gif_dither_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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

    settings_section("抖动", palette).child(list)
}

fn settings_video_gif_loop_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    gif_loop_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    settings_section("循环次数", palette)
        .child(frame_text_input(
            FrameTextInputSpec {
                id: "settings-gif-loop-field",
                value: &config.gif_loop.to_string(),
                placeholder: "0",
                disabled: settings_disabled,
                focus: gif_loop_focus,
                kind: FrameTextInputKind::GifLoop,
            },
            palette,
            window,
            cx,
        ))
        .child(settings_hint_text("设为 0 表示无限循环。", palette))
}

fn settings_video_encoder_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in video_codec_options(config, available_encoders, settings_disabled) {
        let codec = option.codec;
        let enabled = !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("video-codec-{codec}"),
                codec,
                option.disabled_reason.unwrap_or(option.label).to_string(),
                option.is_selected,
                enabled,
                palette,
                window,
                cx,
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

    settings_section("视频编码器", palette).child(list)
}

fn settings_video_pixel_format_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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

    settings_section("像素格式", palette).child(list)
}

fn settings_video_preset_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
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
                palette,
                window,
                cx,
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

    settings_section("编码速度·Preset", palette).child(list)
}

fn settings_video_quality_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_bitrate_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_section("质量控制", palette);
    if config.video_codec != "mpeg2video" {
        section = section.child(settings_video_bitrate_mode_grid(
            config,
            settings_disabled,
            palette,
            window,
            cx,
        ));
    }

    if config.video_bitrate_mode == "crf" && config.video_codec != "mpeg2video" {
        let is_hardware = is_hardware_video_codec(&config.video_codec);
        section = section.child(settings_video_range_field(
            if is_hardware {
                "编码质量"
            } else {
                "质量因子"
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
            u32::from(is_hardware),
            if is_hardware { 100 } else { 51 },
            if is_hardware {
                "低质量"
            } else {
                "无损"
            },
            if is_hardware {
                "最佳质量"
            } else {
                "最小"
            },
            if is_hardware {
                SettingsVideoRangeTarget::Quality
            } else {
                SettingsVideoRangeTarget::Crf
            },
            settings_disabled,
            palette,
            cx,
        ));
    } else {
        section = section.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .pt(theme::ui_rem(4.0))
                .child(settings_field_label("指定码率 (kbps)", palette))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-video-bitrate-field",
                        value: &config.video_bitrate,
                        placeholder: "5000",
                        disabled: settings_disabled,
                        focus: video_bitrate_focus,
                        kind: FrameTextInputKind::VideoBitrate,
                    },
                    palette,
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
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for (mode, label) in [("crf", "恒定质量"), ("bitrate", "指定码率")] {
        grid = grid.child(
            frame_choice_button(
                format!("video-bitrate-mode-{mode}"),
                label,
                config.video_bitrate_mode == mode,
                !disabled,
                palette,
                window,
                cx,
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
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
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
            value, min, max, disabled, target, palette, cx,
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

fn settings_video_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsVideoRangeTarget,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
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
            SettingsVideoRangeTarget::Crf => "视频 CRF",
            SettingsVideoRangeTarget::Quality => "视频质量",
        },
        fraction,
        disabled,
        palette,
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
        |root, event: &DragMoveEvent<SettingsVideoRangeDrag>, _window, cx| {
            let drag = *event.drag(cx);
            let fraction = timeline_slider_percent_from_bounds(event.event.position, event.bounds);
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

fn apply_settings_video_range_value(
    config: &mut ConversionConfig,
    target: SettingsVideoRangeTarget,
    value: u32,
) -> bool {
    match target {
        SettingsVideoRangeTarget::Crf => apply_crf(config, u32_to_u8(value)),
        SettingsVideoRangeTarget::Quality => apply_quality(config, value),
    }
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
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    settings_section("NVENC 选项", palette)
        .child(settings_video_checkbox_row(
            "video-nvenc-spatial-aq",
            "空间 AQ",
            "提升高复杂度场景的细节",
            config.nvenc_spatial_aq,
            disabled,
            palette,
            cx,
            move |root, _event, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_nvenc_spatial_aq(config, !config.nvenc_spatial_aq)
                }) {
                    cx.notify();
                }
            },
        ))
        .child(settings_video_checkbox_row(
            "video-nvenc-temporal-aq",
            "时间 AQ",
            "稳定帧间质量",
            config.nvenc_temporal_aq,
            disabled,
            palette,
            cx,
            move |root, _event, _window, cx| {
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| {
                    apply_nvenc_temporal_aq(config, !config.nvenc_temporal_aq)
                }) {
                    cx.notify();
                }
            },
        ))
}

fn settings_video_videotoolbox_section(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    settings_section("VideoToolbox 选项", palette).child(settings_video_checkbox_row(
        "video-videotoolbox-allow-sw",
        "允许软件回退",
        "硬件失败时回退到 CPU 编码",
        config.videotoolbox_allow_sw,
        disabled,
        palette,
        cx,
        move |root, _event, _window, cx| {
            if disabled {
                return;
            }
            if root.update_selected_config(|config| {
                apply_videotoolbox_allow_sw(config, !config.videotoolbox_allow_sw)
            }) {
                cx.notify();
            }
        },
    ))
}

fn settings_video_hw_section(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    settings_section("硬件加速", palette).child(settings_video_checkbox_row(
        "video-hw-decode",
        "硬件解码",
        "使用 GPU 解码输入视频（更快）",
        config.hw_decode,
        disabled,
        palette,
        cx,
        move |root, _event, _window, cx| {
            if disabled {
                return;
            }
            if root.update_selected_config(|config| apply_hw_decode(config, !config.hw_decode)) {
                cx.notify();
            }
        },
    ))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Video checkbox rows keep semantics, state, palette, root context, and action explicit."
)]
fn settings_video_checkbox_row(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    checked: bool,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(id, label, hint, checked, disabled, palette, cx, action)
}

fn resolution_label(resolution: &str) -> &'static str {
    match resolution {
        "custom" => "自定义",
        "1080p" => "1080p",
        "720p" => "720p",
        "480p" => "480p",
        _ => "原始",
    }
}

fn scaling_algorithm_label(algorithm: &str) -> &'static str {
    match algorithm {
        "lanczos" => "Lanczos",
        "bilinear" => "Bilinear",
        "nearest" => "Nearest",
        _ => "Bicubic",
    }
}

fn fps_label(fps: &str) -> String {
    if fps == "original" {
        "与源相同".to_string()
    } else {
        format!("{fps} fps")
    }
}

fn gif_dither_label(dither: &str) -> &'static str {
    match dither {
        "floyd_steinberg" => "Floyd-Steinberg",
        "bayer" => "Bayer",
        "none" => "无",
        _ => "Sierra2_4a",
    }
}
