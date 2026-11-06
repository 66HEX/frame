use crate::app::inspector::state::SettingsVideoInputFocuses;
use crate::app::inspector::video::encoding::{
    settings_video_encoder_section, settings_video_hw_section, settings_video_nvenc_section,
    settings_video_pixel_format_section, settings_video_preset_section,
    settings_video_quality_section, settings_video_videotoolbox_section,
};
use crate::app::inspector::video::geometry::{
    settings_video_fps_section, settings_video_gif_colors_section,
    settings_video_gif_dither_section, settings_video_gif_loop_section,
    settings_video_resolution_section, settings_video_scaling_section,
};
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, is_gif_container, is_hardware_video_codec, is_nvenc_video_codec,
    is_videotoolbox_video_codec,
};
use crate::theme;
use frame_core::capabilities::AvailableEncoders;
use gpui::prelude::*;
use gpui::{Context, Window};

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
    let resolution = settings_video_resolution_section(
        config,
        settings_disabled || focuses.resolution_locked,
        focuses.width,
        focuses.height,
        palette,
        window,
        cx,
    );
    let content = crate::app::inspector::panel::settings_fields_grid()
        .child(settings_video_scaling_section(
            config,
            settings_disabled,
            palette,
            window,
            cx,
        ))
        .child(settings_video_fps_section(
            config,
            settings_disabled,
            palette,
            window,
            cx,
        ));

    if is_gif_mode {
        let controls = content
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
        return crate::app::components::frame_field_group()
            .child(resolution)
            .child(controls);
    }

    let controls = content
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
        );
    crate::app::components::frame_field_group()
        .child(resolution)
        .child(controls)
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
