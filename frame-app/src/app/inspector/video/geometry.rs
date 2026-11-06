use crate::app::components::{FrameOption, frame_option_field, frame_option_tabs};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::panel::settings_field;
use crate::app::inspector::shared::{settings_field_label, settings_hint_text};
use crate::app::inspector::video::encoding::{
    fps_label, gif_dither_label, resolution_label, scaling_algorithm_label,
};
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, apply_fps, apply_gif_colors, apply_gif_dither, apply_resolution,
    apply_scaling_algorithm, fps_options, gif_color_options, gif_dither_options, is_gif_container,
    resolution_options, scaling_algorithm_options,
};
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, FocusHandle, Window, div};

pub(in crate::app) fn settings_video_resolution_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_width_focus: Option<&FocusHandle>,
    video_height_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_field("Resolution", palette).child(settings_resolution_grid(
        config,
        settings_disabled,
        palette,
        window,
        cx,
    ));

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
    let mut grid = Vec::new();
    for resolution in resolution_options() {
        let selected = config.resolution == *resolution;
        let label = resolution_label(resolution);
        grid.push(FrameOption::new(
            format!("video-resolution-{resolution}"),
            label,
            selected,
            !disabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_resolution(config, resolution)) {
                    cx.notify();
                }
            },
        ));
    }
    frame_option_tabs("resolution-tabs", "Resolution", grid, palette, window, cx)
}

pub(super) fn settings_custom_dimensions_grid(
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
                .child(settings_field_label("Width", palette))
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
                .child(settings_field_label("Height", palette))
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
    let mut grid = Vec::new();
    for algorithm in scaling_algorithm_options() {
        grid.push(FrameOption::new(
            format!("video-scaling-{algorithm}"),
            scaling_algorithm_label(algorithm),
            config.scaling_algorithm == *algorithm,
            !disabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_scaling_algorithm(config, algorithm))
                {
                    cx.notify();
                }
            },
        ));
    }
    let grid = frame_option_field(
        "video-scaling-select",
        "Scaling algorithm",
        grid,
        palette,
        window,
        cx,
    );

    settings_field("Scaling algorithm", palette).child(grid)
}

pub(in crate::app) fn settings_video_fps_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let is_gif = is_gif_container(&config.container);
    let mut grid = Vec::new();
    for fps in fps_options(is_gif) {
        grid.push(FrameOption::new(
            format!("video-fps-{fps}"),
            fps_label(fps),
            config.fps == *fps,
            !settings_disabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if settings_disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_fps(config, fps)) {
                    cx.notify();
                }
            },
        ));
    }
    let grid = frame_option_field("video-fps-select", "Framerate", grid, palette, window, cx);

    settings_field("Framerate", palette).child(grid)
}

pub(in crate::app) fn settings_video_gif_colors_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = Vec::new();
    for colors in gif_color_options() {
        grid.push(FrameOption::new(
            format!("video-gif-colors-{colors}"),
            colors.to_string(),
            config.gif_colors == *colors,
            !settings_disabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if settings_disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_gif_colors(config, *colors)) {
                    cx.notify();
                }
            },
        ));
    }
    let grid = frame_option_field(
        "video-gif-colors-select",
        "Palette colors",
        grid,
        palette,
        window,
        cx,
    );

    settings_field("Palette colors", palette).child(grid)
}

pub(in crate::app) fn settings_video_gif_dither_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = Vec::new();
    for dither in gif_dither_options() {
        let dither = *dither;
        list.push(FrameOption::with_caption(
            format!("video-gif-dither-{dither}"),
            gif_dither_label(dither),
            dither.to_string(),
            config.gif_dither == dither,
            !settings_disabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if settings_disabled {
                    return;
                }
                if root.update_selected_config(|config| apply_gif_dither(config, dither)) {
                    cx.notify();
                }
            },
        ));
    }
    let list = frame_option_field(
        "video-gif-dither-select",
        "Dithering",
        list,
        palette,
        window,
        cx,
    );

    settings_field("Dithering", palette).child(list)
}

pub(super) fn settings_video_gif_loop_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    gif_loop_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    settings_field("Loop count", palette)
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
        .child(settings_hint_text("Use 0 for infinite looping.", palette))
}
