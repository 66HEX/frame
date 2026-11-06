use crate::app::inspector::panel::settings_field;
use crate::app::inspector::video_filters::controls::{
    settings_video_deinterlace_control, settings_video_denoise_control,
    settings_video_filters_reset_all, settings_video_grayscale_control,
};
use crate::app::inspector::video_filters::range::{
    VideoFilterRangeTarget, settings_video_filter_range_field,
};
use crate::app::inspector::video_filters::spec::video_filter_spec;
use crate::app::root::FrameRoot;
use crate::settings::ConversionConfig;
use crate::theme;
use frame_core::capabilities::AvailableFilters;
use gpui::prelude::*;
use gpui::{Context, Window, div};

#[expect(
    clippy::too_many_lines,
    reason = "The tab render function keeps the approved filter section order visible."
)]
pub(in crate::app) fn settings_video_filters_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    is_image_source: bool,
    available_filters: &AvailableFilters,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let filters = config.video_filters;
    let mut content = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(settings_video_filters_reset_all(
            settings_disabled,
            palette,
            window,
            cx,
        ))
        .child(
            settings_field("Color Adjustments", palette)
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Brightness,
                        filters.color.brightness.enabled,
                        filters.color.brightness.value,
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Contrast,
                        filters.color.contrast.enabled,
                        i32::try_from(filters.color.contrast.value).unwrap_or(100),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Saturation,
                        filters.color.saturation.enabled,
                        i32::try_from(filters.color.saturation.value).unwrap_or(100),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Gamma,
                        filters.color.gamma.enabled,
                        i32::try_from(filters.color.gamma.value).unwrap_or(100),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Tone", palette)
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Hue,
                        filters.hue.enabled,
                        filters.hue.value,
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Temperature,
                        filters.temperature.enabled,
                        i32::try_from(filters.temperature.value).unwrap_or(6500),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Detail", palette)
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Sharpen,
                        filters.sharpen.enabled,
                        i32::try_from(filters.sharpen.value).unwrap_or(25),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::GaussianBlur,
                        filters.gaussian_blur.enabled,
                        i32::try_from(filters.gaussian_blur.value).unwrap_or(20),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Cleanup", palette)
                .child(settings_video_denoise_control(
                    filters.denoise_enabled,
                    filters.denoise_strength,
                    settings_disabled || !available_filters.hqdn3d,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Deband,
                        filters.deband.enabled,
                        i32::try_from(filters.deband.value).unwrap_or(25),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Style", palette)
                .child(settings_video_filter_range_field(
                    video_filter_spec(
                        VideoFilterRangeTarget::Vignette,
                        filters.vignette.enabled,
                        i32::try_from(filters.vignette.value).unwrap_or(35),
                        available_filters,
                    ),
                    settings_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_video_grayscale_control(
                    filters.grayscale,
                    settings_disabled || !available_filters.hue,
                    palette,
                    window,
                    cx,
                )),
        );

    if !is_image_source {
        content = content.child(settings_field("Interlace", palette).child(
            settings_video_deinterlace_control(
                filters.deinterlace,
                settings_disabled || !available_filters.bwdif,
                palette,
                window,
                cx,
            ),
        ));
    }

    content
}
