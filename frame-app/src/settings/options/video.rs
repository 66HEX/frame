use crate::capabilities::{HardwareCapabilities, HardwareEncoder, HardwareProbePolicy};
use crate::settings::model::{
    ConversionConfig, FPS_OPTIONS, GIF_COLOR_OPTIONS, GIF_DITHER_OPTIONS, GIF_FPS_OPTIONS,
    RESOLUTION_OPTIONS, SCALING_ALGORITHM_OPTIONS, VIDEO_CODEC_DEFINITIONS,
    VIDEO_PIXEL_FORMAT_DEFINITIONS, VIDEO_PRESETS, VideoCodecCapability, VideoCodecOption,
    VideoPixelFormatOption, VideoPresetOption,
};
use crate::settings::rules::{
    is_video_codec_allowed_for_container, is_video_pixel_format_allowed_for_container,
};
use frame_core::capabilities::AvailableEncoders;
use frame_core::media_rules;

#[must_use]
pub const fn resolution_options() -> &'static [&'static str] {
    &RESOLUTION_OPTIONS
}

#[must_use]
pub const fn scaling_algorithm_options() -> &'static [&'static str] {
    &SCALING_ALGORITHM_OPTIONS
}

#[must_use]
pub const fn fps_options(is_gif: bool) -> &'static [&'static str] {
    if is_gif {
        &GIF_FPS_OPTIONS
    } else {
        &FPS_OPTIONS
    }
}

#[must_use]
pub const fn gif_color_options() -> &'static [u16] {
    &GIF_COLOR_OPTIONS
}

#[must_use]
pub const fn gif_dither_options() -> &'static [&'static str] {
    &GIF_DITHER_OPTIONS
}

#[must_use]
pub fn video_codec_options(
    config: &ConversionConfig,
    available_encoders: &AvailableEncoders,
    hardware: &HardwareCapabilities,
    disabled: bool,
) -> Vec<VideoCodecOption> {
    let mut options = Vec::new();
    for definition in &VIDEO_CODEC_DEFINITIONS {
        let encoder = HardwareEncoder::from_name(definition.codec);
        let selected = config.video_codec.eq_ignore_ascii_case(definition.codec);
        let compiled = definition.capability.is_none_or(|capability| {
            video_codec_capability_available(available_encoders, capability)
        });
        if !(compiled || selected && encoder.is_some()) {
            continue;
        }
        let modes: &[bool] = if encoder.is_some_and(HardwareEncoder::videotoolbox) {
            &[false, true]
        } else {
            &[false]
        };
        for &allow_software_fallback in modes {
            let allowed = is_video_codec_allowed_for_container(&config.container, definition.codec);
            let hardware_reason = encoder.and_then(|encoder| match hardware.inventory {
                crate::capabilities::EncoderInventory::Failed => {
                    Some("Encoder list could not be read")
                }
                crate::capabilities::EncoderInventory::Unknown => {
                    Some("Encoder availability not checked")
                }
                crate::capabilities::EncoderInventory::Complete
                    if !encoder.compiled(available_encoders) =>
                {
                    Some("Encoder is not included in this FFmpeg")
                }
                crate::capabilities::EncoderInventory::Complete => hardware
                    .disabled_reason(HardwareProbePolicy::new(encoder, allow_software_fallback)),
            });
            let policy_selected = !encoder.is_some_and(HardwareEncoder::videotoolbox)
                || config.videotoolbox_allow_sw == allow_software_fallback;
            options.push(VideoCodecOption {
                codec: definition.codec,
                label: definition.label,
                is_selected: allowed && selected && policy_selected,
                is_disabled: disabled || !allowed || hardware_reason.is_some(),
                disabled_reason: if allowed {
                    hardware_reason
                } else {
                    Some("Incompatible container")
                },
                allow_software_fallback,
            });
        }
    }
    options
}

#[must_use]
pub fn video_pixel_format_options(config: &ConversionConfig) -> Vec<VideoPixelFormatOption> {
    VIDEO_PIXEL_FORMAT_DEFINITIONS
        .iter()
        .map(|definition| {
            let allowed = is_video_pixel_format_allowed_for_container(
                &config.container,
                &config.video_codec,
                definition.id,
            );
            VideoPixelFormatOption {
                id: definition.id,
                label: definition.label,
                is_selected: allowed && config.pixel_format.eq_ignore_ascii_case(definition.id),
                is_disabled: !allowed,
                caption: if definition.id == "auto" {
                    "Encoder default"
                } else if allowed {
                    definition.id
                } else {
                    "Incompatible codec"
                },
            }
        })
        .collect()
}

pub(super) const SVT_AV1_PRESETS: [(&str, &str); 14] = [
    ("0", "Preset 0"),
    ("1", "Preset 1"),
    ("2", "Preset 2"),
    ("3", "Preset 3"),
    ("4", "Preset 4"),
    ("5", "Preset 5"),
    ("6", "Preset 6"),
    ("7", "Preset 7"),
    ("8", "Preset 8"),
    ("9", "Preset 9"),
    ("10", "Preset 10"),
    ("11", "Preset 11"),
    ("12", "Preset 12"),
    ("13", "Preset 13"),
];

pub(super) const VP9_SPEEDS: [(&str, &str); 6] = [
    ("0", "Speed 0"),
    ("1", "Speed 1"),
    ("2", "Speed 2"),
    ("3", "Speed 3"),
    ("4", "Speed 4"),
    ("5", "Speed 5"),
];

#[must_use]
pub fn video_preset_options(config: &ConversionConfig, disabled: bool) -> Vec<VideoPresetOption> {
    if config.video_codec == "vp9" {
        let selected = frame_core::utils::map_vp9_speed(&config.preset);
        return VP9_SPEEDS
            .iter()
            .map(|&(preset, label)| VideoPresetOption {
                preset,
                label,
                caption: match preset {
                    "0" => "Slowest encode, highest compression",
                    "5" => "Fastest encode, lowest compression",
                    "1" => "Default encoding speed",
                    _ => "Lower numbers favor compression; higher numbers favor speed",
                },
                is_selected: selected == preset,
                is_disabled: disabled,
            })
            .collect();
    }
    if config.video_codec == "libsvtav1" {
        let selected = frame_core::utils::map_svt_av1_preset(&config.preset);
        return SVT_AV1_PRESETS
            .iter()
            .map(|&(preset, label)| VideoPresetOption {
                preset,
                label,
                caption: match preset {
                    "0" => "Slowest encode, highest compression",
                    "13" => "Fastest encode, lowest compression",
                    "8" => "Balanced encoding speed",
                    _ => "Lower numbers favor compression; higher numbers favor speed",
                },
                is_selected: selected == preset,
                is_disabled: disabled,
            })
            .collect();
    }
    VIDEO_PRESETS
        .iter()
        .map(|preset| {
            let allowed = is_video_preset_allowed(&config.video_codec, preset);
            VideoPresetOption {
                preset,
                label: video_preset_label(preset),
                caption: if allowed {
                    video_preset_caption(preset)
                } else {
                    "Incompatible preset"
                },
                is_selected: allowed && config.preset == *preset,
                is_disabled: disabled || !allowed,
            }
        })
        .collect()
}

pub(super) const fn video_codec_capability_available(
    available_encoders: &AvailableEncoders,
    capability: VideoCodecCapability,
) -> bool {
    match capability {
        VideoCodecCapability::Mpeg2video => available_encoders.mpeg2video,
        VideoCodecCapability::H264Videotoolbox => available_encoders.h264_videotoolbox,
        VideoCodecCapability::H264Nvenc => available_encoders.h264_nvenc,
        VideoCodecCapability::HevcVideotoolbox => available_encoders.hevc_videotoolbox,
        VideoCodecCapability::HevcNvenc => available_encoders.hevc_nvenc,
        VideoCodecCapability::Av1Nvenc => available_encoders.av1_nvenc,
    }
}

#[must_use]
pub fn is_nvenc_video_codec(codec: &str) -> bool {
    matches!(codec, "h264_nvenc" | "hevc_nvenc" | "av1_nvenc")
}

#[must_use]
pub fn is_videotoolbox_video_codec(codec: &str) -> bool {
    matches!(codec, "h264_videotoolbox" | "hevc_videotoolbox")
}

#[must_use]
pub fn is_hardware_video_codec(codec: &str) -> bool {
    is_nvenc_video_codec(codec) || is_videotoolbox_video_codec(codec)
}

#[must_use]
pub fn is_video_preset_allowed(codec: &str, preset: &str) -> bool {
    if codec == "vp9" {
        return VP9_SPEEDS.iter().any(|(value, _)| *value == preset);
    }
    if codec == "libsvtav1" && SVT_AV1_PRESETS.iter().any(|(value, _)| *value == preset) {
        return true;
    }
    if is_videotoolbox_video_codec(codec) {
        return true;
    }
    if is_nvenc_video_codec(codec) {
        return matches!(preset, "fast" | "medium" | "slow");
    }

    VIDEO_PRESETS.contains(&preset)
}

#[must_use]
pub fn first_allowed_video_preset(codec: &str) -> &'static str {
    if codec == "vp9" {
        return "1";
    }
    VIDEO_PRESETS
        .iter()
        .copied()
        .find(|preset| is_video_preset_allowed(codec, preset))
        .unwrap_or("medium")
}

#[must_use]
pub fn first_allowed_video_codec(
    container: &str,
    available_encoders: Option<&AvailableEncoders>,
) -> String {
    let candidates = media_rules::video_codec_fallback_order().iter();
    let first = candidates
        .filter(|codec| video_codec_available(codec, available_encoders))
        .find(|codec| is_video_codec_allowed_for_container(container, codec))
        .cloned();

    first
        .or_else(|| {
            media_rules::video_codecs_for_container(container).and_then(|codecs| {
                codecs
                    .iter()
                    .find(|codec| video_codec_available(codec, available_encoders))
                    .cloned()
            })
        })
        .unwrap_or_else(|| {
            media_rules::video_codec_fallback_order()
                .first()
                .cloned()
                .unwrap_or_else(|| "libx264".to_string())
        })
}

pub(super) fn video_codec_available(
    codec: &str,
    available_encoders: Option<&AvailableEncoders>,
) -> bool {
    available_encoders.is_none_or(|encoders| {
        VIDEO_CODEC_DEFINITIONS
            .iter()
            .find(|definition| definition.codec == codec)
            .and_then(|definition| definition.capability)
            .is_none_or(|capability| video_codec_capability_available(encoders, capability))
    })
}

#[must_use]
pub fn first_allowed_video_pixel_format(container: &str, encoder: &str) -> &'static str {
    VIDEO_PIXEL_FORMAT_DEFINITIONS
        .iter()
        .find(|definition| {
            is_video_pixel_format_allowed_for_container(container, encoder, definition.id)
        })
        .map_or("auto", |definition| definition.id)
}

#[must_use]
pub fn video_preset_label(preset: &str) -> &'static str {
    match preset {
        "ultrafast" => "Ultrafast",
        "superfast" => "Superfast",
        "veryfast" => "Very Fast",
        "faster" => "Faster",
        "fast" => "Fast",
        "slow" => "Slow",
        "slower" => "Slower",
        "veryslow" => "Very Slow",
        _ => "Medium",
    }
}

#[must_use]
pub fn video_preset_caption(preset: &str) -> &'static str {
    match preset {
        "ultrafast" => "Fastest encode, largest file",
        "superfast" => "Very fast encode",
        "veryfast" => "Fast encode",
        "faster" => "Faster than default",
        "fast" => "Fast with reasonable compression",
        "slow" => "Smaller file, slower encode",
        "slower" => "High compression",
        "veryslow" => "Smallest file, slowest encode",
        _ => "Balanced default",
    }
}
