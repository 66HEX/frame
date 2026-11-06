use crate::app::inspector::audio_filters::controls::{
    settings_audio_compressor_control, settings_audio_filters_reset_all,
    settings_audio_normalize_control,
};
use crate::app::inspector::audio_filters::range::{
    AudioFilterRangeTarget, settings_audio_filter_range_field,
};
use crate::app::inspector::audio_filters::spec::audio_filter_spec;
use crate::app::inspector::panel::settings_field;
use crate::app::root::FrameRoot;
use crate::settings::{ConversionConfig, ProcessingMode};
use crate::theme;
use frame_core::capabilities::AvailableFilters;
use gpui::prelude::*;
use gpui::{Context, Window, div};

#[expect(
    clippy::too_many_lines,
    reason = "The tab render function keeps the approved filter section order visible."
)]
pub(in crate::app) fn settings_audio_filters_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    available_filters: &AvailableFilters,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let filters = config.audio_filters;
    let controls_disabled = settings_disabled || config.processing_mode == ProcessingMode::Copy;

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(settings_audio_filters_reset_all(
            controls_disabled,
            palette,
            window,
            cx,
        ))
        .child(
            settings_field("Level", palette)
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Volume,
                        true,
                        i32::try_from(config.audio_volume).unwrap_or(200),
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_audio_normalize_control(
                    config.audio_normalize,
                    controls_disabled || !available_filters.loudnorm,
                    palette,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Limiter,
                        filters.limiter.enabled,
                        filters.limiter.value,
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Dynamics", palette).child(settings_audio_compressor_control(
                filters.compressor_enabled,
                filters.compressor_strength,
                controls_disabled || !available_filters.acompressor,
                palette,
                window,
                cx,
            )),
        )
        .child(
            settings_field("Tone", palette)
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Bass,
                        filters.bass.enabled,
                        filters.bass.value,
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::Treble,
                        filters.treble.enabled,
                        filters.treble.value,
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::HighPass,
                        filters.high_pass.enabled,
                        i32::try_from(filters.high_pass.value).unwrap_or(80),
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::LowPass,
                        filters.low_pass.enabled,
                        i32::try_from(filters.low_pass.value).unwrap_or(16_000),
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Cleanup", palette)
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::NoiseReduction,
                        filters.noise_reduction.enabled,
                        i32::try_from(filters.noise_reduction.value).unwrap_or(12),
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                ))
                .child(settings_audio_filter_range_field(
                    audio_filter_spec(
                        AudioFilterRangeTarget::DeEsser,
                        filters.de_esser.enabled,
                        i32::try_from(filters.de_esser.value).unwrap_or(35),
                        available_filters,
                    ),
                    controls_disabled,
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            settings_field("Stereo", palette).child(settings_audio_filter_range_field(
                audio_filter_spec(
                    AudioFilterRangeTarget::StereoWidth,
                    filters.stereo_width.enabled,
                    i32::try_from(filters.stereo_width.value).unwrap_or(100),
                    available_filters,
                ),
                controls_disabled,
                palette,
                window,
                cx,
            )),
        )
}
