use crate::app::components::frame_checkbox_row;
use crate::app::components::{FrameOption, frame_option_field, frame_select_field};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::panel::settings_field;
use crate::app::inspector::shared::settings_field_label;
use crate::app::inspector::video::quality::{SettingsVideoRangeTarget, settings_video_range_field};
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, apply_hw_decode, apply_nvenc_spatial_aq, apply_nvenc_temporal_aq,
    apply_pixel_format, apply_video_bitrate_mode, apply_video_codec, apply_video_preset,
    apply_videotoolbox_allow_sw, is_hardware_video_codec, video_codec_options,
    video_pixel_format_options, video_preset_options,
};
use crate::theme;
use frame_core::capabilities::AvailableEncoders;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, Window, div};

pub(super) fn settings_video_encoder_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_encoders: &AvailableEncoders,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = Vec::new();
    for option in video_codec_options(config, available_encoders, settings_disabled) {
        let codec = option.codec;
        let enabled = !option.is_disabled;
        list.push(FrameOption::with_caption(
            format!("video-codec-{codec}"),
            codec,
            option.disabled_reason.unwrap_or(option.label).to_string(),
            option.is_selected,
            enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_codec(config, codec)) {
                    cx.notify();
                }
            },
        ));
    }
    let list = frame_select_field(
        "video-encoder-select",
        "Video encoder",
        list,
        palette,
        window,
        cx,
    );

    settings_field("Video encoder", palette).child(list)
}

pub(super) fn settings_video_pixel_format_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = Vec::new();
    for option in video_pixel_format_options(config) {
        let pixel_format = option.id;
        let enabled = !settings_disabled && !option.is_disabled;
        list.push(FrameOption::with_caption(
            format!("video-pixel-format-{pixel_format}"),
            option.label,
            option.caption,
            option.is_selected,
            enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_pixel_format(config, pixel_format)) {
                    cx.notify();
                }
            },
        ));
    }
    let list = frame_option_field(
        "video-pixel-format-select",
        "Pixel format",
        list,
        palette,
        window,
        cx,
    );

    settings_field("Pixel format", palette).child(list)
}

pub(super) fn settings_video_preset_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = Vec::new();
    for option in video_preset_options(config, settings_disabled) {
        let preset = option.preset;
        let enabled = !option.is_disabled;
        list.push(FrameOption::with_caption(
            format!("video-preset-{preset}"),
            option.label,
            option.caption,
            option.is_selected,
            enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_preset(config, preset)) {
                    cx.notify();
                }
            },
        ));
    }
    let list = frame_option_field(
        "video-preset-select",
        "Encoding speed",
        list,
        palette,
        window,
        cx,
    );

    settings_field(
        if config.video_codec == "libsvtav1" {
            "AV1 preset"
        } else {
            "Encoding speed"
        },
        palette,
    )
    .child(list)
}

pub(super) fn settings_video_quality_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_bitrate_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_field("Quality control", palette);
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
                "Encoding quality"
            } else {
                "Quality factor"
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
                "Low quality"
            } else {
                "Lossless"
            },
            if is_hardware {
                "Best quality"
            } else {
                "Smallest"
            },
            if is_hardware {
                SettingsVideoRangeTarget::Quality
            } else {
                SettingsVideoRangeTarget::Crf
            },
            settings_disabled,
            palette,
            window,
            cx,
        ));
    } else {
        section = section.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .pt(theme::ui_rem(4.0))
                .child(settings_field_label("Target bitrate (kbps)", palette))
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

pub(super) fn settings_video_bitrate_mode_grid(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = Vec::new();
    for (mode, label) in [("crf", "Constant Quality"), ("bitrate", "Target Bitrate")] {
        grid.push(FrameOption::new(
            format!("video-bitrate-mode-{mode}"),
            label,
            config.video_bitrate_mode == mode,
            !disabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_video_bitrate_mode(config, mode)) {
                    cx.notify();
                }
            },
        ));
    }
    frame_option_field(
        "video-bitrate-mode-select",
        "Quality control",
        grid,
        palette,
        window,
        cx,
    )
}

pub(super) fn settings_video_nvenc_section(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_field("NVENC options", palette)
        .child(settings_video_checkbox_row(
            "video-nvenc-spatial-aq",
            "Spatial AQ",
            "Improves detail in scenes with high complexity",
            config.nvenc_spatial_aq,
            disabled,
            palette,
            window,
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
            "Temporal AQ",
            "Stabilizes quality between frames",
            config.nvenc_temporal_aq,
            disabled,
            palette,
            window,
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

pub(super) fn settings_video_videotoolbox_section(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_field("VideoToolbox options", palette).child(settings_video_checkbox_row(
        "video-videotoolbox-allow-sw",
        "Allow software fallback",
        "Drop back to CPU encoding if hardware fails",
        config.videotoolbox_allow_sw,
        disabled,
        palette,
        window,
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

pub(super) fn settings_video_hw_section(
    config: &ConversionConfig,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_field("Hardware acceleration", palette).child(settings_video_checkbox_row(
        "video-hw-decode",
        "Hardware decoding",
        "Use GPU for decoding input video (faster)",
        config.hw_decode,
        disabled,
        palette,
        window,
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
pub(super) fn settings_video_checkbox_row(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    checked: bool,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row(
        id, label, hint, checked, disabled, palette, window, cx, action,
    )
}

pub(super) fn resolution_label(resolution: &str) -> &'static str {
    match resolution {
        "custom" => "Custom",
        "1080p" => "1080p",
        "720p" => "720p",
        "480p" => "480p",
        _ => "Original",
    }
}

pub(super) fn scaling_algorithm_label(algorithm: &str) -> &'static str {
    match algorithm {
        "lanczos" => "Lanczos",
        "bilinear" => "Bilinear",
        "nearest" => "Nearest",
        _ => "Bicubic",
    }
}

pub(super) fn fps_label(fps: &str) -> String {
    if fps == "original" {
        "Same as source".to_string()
    } else {
        format!("{fps} fps")
    }
}

pub(super) fn gif_dither_label(dither: &str) -> &'static str {
    match dither {
        "floyd_steinberg" => "Floyd-Steinberg",
        "bayer" => "Bayer",
        "none" => "None",
        _ => "Sierra2_4a",
    }
}
