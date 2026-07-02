use super::{
    ClickEvent, Context, ConversionConfig, DragMoveEvent, FocusHandle, FrameRoot, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, apply_image_jpeg_huffman,
    apply_image_jpeg_quality, apply_image_png_compression, apply_image_png_prediction,
    apply_image_tiff_compression, apply_image_webp_compression, apply_image_webp_lossless,
    apply_image_webp_preset, apply_image_webp_quality, apply_pixel_format, color, div,
    frame_choice_button, frame_list_item_with_caption, frame_slider, frame_slider_handle,
    image_jpeg_huffman_options, image_png_prediction_options, image_tiff_compression_options,
    image_webp_preset_options, px, range_fraction, range_value_from_fraction, settings_field_label,
    settings_hint_text, settings_section, settings_value_badge, settings_video_resolution_section,
    settings_video_scaling_section, theme, timeline_slider_percent_from_bounds,
    video_pixel_format_options,
};
use gpui::{AppContext, InteractiveElement, prelude::FluentBuilder};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsImageRangeTarget {
    JpegQuality,
    WebpQuality,
    WebpCompression,
    PngCompression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SettingsImageRangeDrag {
    target: SettingsImageRangeTarget,
    min: u32,
    max: u32,
}

struct SettingsImageRangeDragPreview;

impl Render for SettingsImageRangeDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

pub(in crate::app) fn settings_images_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_width_focus: Option<&FocusHandle>,
    video_height_focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(settings_video_resolution_section(
            config,
            settings_disabled,
            video_width_focus,
            video_height_focus,
            window,
            cx,
        ))
        .child(settings_video_scaling_section(
            config,
            settings_disabled,
            window,
            cx,
        ))
        .child(settings_images_pixel_format_section(
            config,
            settings_disabled,
            window,
            cx,
        ))
        .child(settings_images_encoding_section(
            config,
            settings_disabled,
            window,
            cx,
        ))
}

fn settings_images_pixel_format_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in video_pixel_format_options(config) {
        let pixel_format = option.id;
        let enabled = !settings_disabled && !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("images-pixel-format-{pixel_format}"),
                option.label,
                option.caption,
                option.is_selected,
                enabled,
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

    settings_section("Pixel format").child(list)
}

fn settings_images_encoding_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    match config.container.as_str() {
        "jpg" => settings_image_jpeg_section(config, settings_disabled, window, cx),
        "webp" => settings_image_webp_section(config, settings_disabled, window, cx),
        "png" => settings_image_png_section(config, settings_disabled, window, cx),
        "tiff" => settings_image_tiff_section(config, settings_disabled, window, cx),
        "bmp" => settings_section("BMP encoding")
            .child(settings_hint_text("BMP output is uncompressed.")),
        _ => settings_section("Image encoding").child(settings_hint_text(
            "Select an image format to tune encoding.",
        )),
    }
}

fn settings_image_jpeg_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("JPEG encoding")
        .child(settings_image_range_field(
            "Quality",
            format!("{}%", config.image_jpeg_quality),
            config.image_jpeg_quality,
            1,
            100,
            "Smallest",
            "Best quality",
            SettingsImageRangeTarget::JpegQuality,
            settings_disabled,
            cx,
        ))
        .child(settings_image_option_list(
            image_jpeg_huffman_options(config, settings_disabled),
            "image-jpeg-huffman",
            window,
            cx,
            apply_image_jpeg_huffman,
        ))
}

fn settings_image_webp_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("WebP encoding")
        .child(settings_image_webp_mode_grid(
            config,
            settings_disabled,
            window,
            cx,
        ))
        .child(settings_image_range_field(
            if config.image_webp_lossless {
                "Effort"
            } else {
                "Quality"
            },
            format!("{}%", config.image_webp_quality),
            config.image_webp_quality,
            0,
            100,
            if config.image_webp_lossless {
                "Fastest"
            } else {
                "Smallest"
            },
            if config.image_webp_lossless {
                "Smallest"
            } else {
                "Best quality"
            },
            SettingsImageRangeTarget::WebpQuality,
            settings_disabled,
            cx,
        ))
        .child(settings_image_range_field(
            "Compression effort",
            config.image_webp_compression.to_string(),
            config.image_webp_compression,
            0,
            6,
            "Fastest",
            "Smallest",
            SettingsImageRangeTarget::WebpCompression,
            settings_disabled,
            cx,
        ))
        .child(settings_image_option_list(
            image_webp_preset_options(config, settings_disabled),
            "image-webp-preset",
            window,
            cx,
            apply_image_webp_preset,
        ))
}

fn settings_image_png_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("PNG compression")
        .child(settings_image_range_field(
            "Compression level",
            config.image_png_compression.to_string(),
            config.image_png_compression,
            0,
            9,
            "Fastest",
            "Smallest",
            SettingsImageRangeTarget::PngCompression,
            settings_disabled,
            cx,
        ))
        .child(settings_image_option_list(
            image_png_prediction_options(config, settings_disabled),
            "image-png-prediction",
            window,
            cx,
            apply_image_png_prediction,
        ))
}

fn settings_image_tiff_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    settings_section("TIFF compression").child(settings_image_option_list(
        image_tiff_compression_options(config, settings_disabled),
        "image-tiff-compression",
        window,
        cx,
        apply_image_tiff_compression,
    ))
}

fn settings_image_webp_mode_grid(
    config: &ConversionConfig,
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for (lossless, label) in [(false, "Lossy"), (true, "Lossless")] {
        grid = grid.child(
            frame_choice_button(
                format!("image-webp-mode-{label}"),
                label,
                config.image_webp_lossless == lossless,
                !disabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_image_webp_lossless(config, lossless))
                {
                    cx.notify();
                }
            })),
        );
    }

    grid
}

#[expect(
    clippy::too_many_arguments,
    reason = "keeps slider construction close to the existing settings slider contract"
)]
fn settings_image_range_field(
    label: &'static str,
    value_label: String,
    value: u32,
    min: u32,
    max: u32,
    lower_label: &'static str,
    upper_label: &'static str,
    target: SettingsImageRangeTarget,
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
                .child(settings_field_label(label))
                .child(settings_value_badge(value_label)),
        )
        .child(settings_image_range_slider(
            value, min, max, disabled, target, cx,
        ))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(theme::ui_text(lower_label))
                .child(theme::ui_text(upper_label)),
        )
}

fn settings_image_range_slider(
    value: u32,
    min: u32,
    max: u32,
    disabled: bool,
    target: SettingsImageRangeTarget,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let fraction = range_fraction(value, min, max);
    let drag = SettingsImageRangeDrag { target, min, max };

    frame_slider(settings_image_range_slider_id(target), fraction, disabled)
        .when(!disabled, |slider| {
            slider.on_drag(drag, |_drag, _position, _window, cx| {
                cx.new(|_| SettingsImageRangeDragPreview)
            })
        })
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<SettingsImageRangeDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let fraction =
                    timeline_slider_percent_from_bounds(event.event.position, event.bounds);
                let value = range_value_from_fraction(fraction, drag.min, drag.max);
                let changed = root.update_selected_config(|config| match drag.target {
                    SettingsImageRangeTarget::JpegQuality => {
                        apply_image_jpeg_quality(config, value)
                    }
                    SettingsImageRangeTarget::WebpQuality => {
                        apply_image_webp_quality(config, value)
                    }
                    SettingsImageRangeTarget::WebpCompression => {
                        apply_image_webp_compression(config, value)
                    }
                    SettingsImageRangeTarget::PngCompression => {
                        apply_image_png_compression(config, value)
                    }
                });
                if changed {
                    cx.notify();
                }
            },
        ))
        .child(settings_image_range_handle(fraction, drag, !disabled))
}

fn settings_image_range_handle(
    fraction: f32,
    drag: SettingsImageRangeDrag,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle = frame_slider_handle(
        settings_image_range_handle_id(drag.target),
        fraction,
        enabled,
    );

    if enabled {
        handle.on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsImageRangeDragPreview)
        })
    } else {
        handle
    }
}

const fn settings_image_range_slider_id(target: SettingsImageRangeTarget) -> &'static str {
    match target {
        SettingsImageRangeTarget::JpegQuality => "settings-image-jpeg-quality-slider",
        SettingsImageRangeTarget::WebpQuality => "settings-image-webp-quality-slider",
        SettingsImageRangeTarget::WebpCompression => "settings-image-webp-compression-slider",
        SettingsImageRangeTarget::PngCompression => "settings-image-png-compression-slider",
    }
}

const fn settings_image_range_handle_id(target: SettingsImageRangeTarget) -> &'static str {
    match target {
        SettingsImageRangeTarget::JpegQuality => "settings-image-jpeg-quality-handle",
        SettingsImageRangeTarget::WebpQuality => "settings-image-webp-quality-handle",
        SettingsImageRangeTarget::WebpCompression => "settings-image-webp-compression-handle",
        SettingsImageRangeTarget::PngCompression => "settings-image-png-compression-handle",
    }
}

fn settings_image_option_list(
    options: Vec<crate::settings::ImageEncodingOption>,
    id_prefix: &'static str,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
    apply: fn(&mut ConversionConfig, &str) -> bool,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in options {
        let id = option.id;
        let enabled = !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("{id_prefix}-{id}"),
                option.label,
                option.caption,
                option.is_selected,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply(config, id)) {
                    cx.notify();
                }
            })),
        );
    }

    list
}
