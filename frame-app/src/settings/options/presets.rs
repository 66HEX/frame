use crate::settings::model::{
    ConversionConfig, MetadataConfig, MetadataMode, PresetDefinition, PresetOption, SourceKind,
    SourceMetadata,
};
use crate::settings::rules::{
    is_audio_only_container, is_gif_container, is_image_container, source_kind_for,
};

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "Preset definitions are data declarations and are easier to compare when kept as one table."
)]
pub fn default_presets() -> Vec<PresetDefinition> {
    vec![
        PresetDefinition::built_in("balanced-mp4", "Balanced MP4", preset_config("mp4")),
        PresetDefinition::built_in(
            "broadcast-m2t",
            "M2T Broadcast (188-byte TS)",
            ConversionConfig {
                container: "m2t".to_string(),
                video_codec: "mpeg2video".to_string(),
                video_bitrate_mode: "bitrate".to_string(),
                video_bitrate: "18000".to_string(),
                audio_codec: "mp2".to_string(),
                audio_bitrate: "192".to_string(),
                audio_channels: "stereo".to_string(),
                pixel_format: "yuv420p".to_string(),
                ..preset_config("m2t")
            },
        ),
        PresetDefinition::built_in(
            "m2ts-h264",
            "M2TS H.264 (192-byte)",
            ConversionConfig {
                container: "m2ts".to_string(),
                video_codec: "libx264".to_string(),
                video_bitrate_mode: "bitrate".to_string(),
                video_bitrate: "18000".to_string(),
                audio_codec: "ac3".to_string(),
                audio_bitrate: "384".to_string(),
                pixel_format: "yuv420p".to_string(),
                ..preset_config("m2ts")
            },
        ),
        PresetDefinition::built_in(
            "archive-hq",
            "Archive H.265",
            ConversionConfig {
                container: "mkv".to_string(),
                video_codec: "libx265".to_string(),
                video_bitrate: "8000".to_string(),
                audio_codec: "ac3".to_string(),
                audio_bitrate: "192".to_string(),
                scaling_algorithm: "lanczos".to_string(),
                crf: 18,
                quality: 60,
                preset: "slow".to_string(),
                ..preset_config("mkv")
            },
        ),
        PresetDefinition::built_in(
            "web-share",
            "Web Share",
            ConversionConfig {
                container: "webm".to_string(),
                video_codec: "vp9".to_string(),
                video_bitrate: "2500".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: "96".to_string(),
                audio_channels: "stereo".to_string(),
                resolution: "720p".to_string(),
                crf: 30,
                quality: 40,
                ..preset_config("webm")
            },
        ),
        PresetDefinition::built_in(
            "gif-web-small",
            "GIF Web Small",
            gif_preset_config("custom", Some("640"), Some("360"), "12", 128, "sierra2_4a"),
        ),
        PresetDefinition::built_in(
            "gif-quality",
            "GIF High Quality",
            gif_preset_config("720p", None, None, "15", 256, "floyd_steinberg"),
        ),
        PresetDefinition::built_in(
            "audio-only",
            "Audio MP3",
            audio_preset_config("mp3", "mp3", "128", "stereo"),
        ),
        PresetDefinition::built_in(
            "audio-flac",
            "Audio FLAC (Lossless)",
            audio_preset_config("flac", "flac", "0", "original"),
        ),
        PresetDefinition::built_in(
            "audio-alac",
            "Audio ALAC (Apple)",
            audio_preset_config("m4a", "alac", "0", "original"),
        ),
        PresetDefinition::built_in(
            "audio-wav",
            "Audio WAV (Lossless)",
            audio_preset_config("wav", "pcm_s16le", "0", "original"),
        ),
        PresetDefinition::built_in(
            "social-tiktok",
            "Social (TikTok/Reels)",
            social_preset_config("6000", "custom", Some("1080"), Some("1920"), "30", "slow"),
        ),
        PresetDefinition::built_in(
            "yt-1080p",
            "YouTube 1080p",
            youtube_preset_config("10000", "1080p", None, None),
        ),
        PresetDefinition::built_in(
            "yt-4k",
            "YouTube 4K",
            youtube_preset_config("40000", "custom", Some("3840"), Some("2160")),
        ),
        PresetDefinition::built_in(
            "x-landscape",
            "X (Landscape)",
            social_preset_config("2500", "720p", None, None, "30", "medium"),
        ),
        PresetDefinition::built_in(
            "x-portrait",
            "X (Mobile/Portrait)",
            social_preset_config("2000", "custom", Some("720"), Some("1280"), "30", "medium"),
        ),
        PresetDefinition::built_in(
            "discord",
            "Discord",
            ConversionConfig {
                video_bitrate_mode: "bitrate".to_string(),
                video_bitrate: "1000".to_string(),
                audio_bitrate: "64".to_string(),
                audio_channels: "stereo".to_string(),
                audio_normalize: true,
                resolution: "720p".to_string(),
                fps: "30".to_string(),
                preset: "veryfast".to_string(),
                metadata: MetadataConfig {
                    mode: MetadataMode::Clean,
                    ..MetadataConfig::default()
                },
                pixel_format: "yuv420p".to_string(),
                ..preset_config("mp4")
            },
        ),
    ]
}

#[must_use]
pub fn preset_options(
    config: &ConversionConfig,
    presets: &[PresetDefinition],
    metadata: Option<&SourceMetadata>,
) -> Vec<PresetOption> {
    presets
        .iter()
        .map(|preset| {
            let is_compatible = preset_is_compatible(preset, metadata);
            let is_selected = configs_match(config, &preset.config);
            PresetOption {
                preset: preset.clone(),
                is_selected,
                is_compatible,
                status: if !is_compatible {
                    Some("Incompatible container")
                } else if is_selected {
                    Some("Applied")
                } else {
                    None
                },
            }
        })
        .collect()
}

#[must_use]
pub fn configs_match(a: &ConversionConfig, b: &ConversionConfig) -> bool {
    if a.container != b.container
        || a.video_codec != b.video_codec
        || a.audio_codec != b.audio_codec
        || a.resolution != b.resolution
        || a.preset != b.preset
        || a.video_bitrate_mode != b.video_bitrate_mode
    {
        return false;
    }

    if a.video_bitrate_mode == "crf" {
        if a.crf != b.crf || a.quality != b.quality {
            return false;
        }
    } else if a.video_bitrate != b.video_bitrate {
        return false;
    }

    if a.resolution == "custom" {
        return a.custom_width == b.custom_width && a.custom_height == b.custom_height;
    }

    true
}

#[must_use]
pub fn preset_is_compatible(preset: &PresetDefinition, metadata: Option<&SourceMetadata>) -> bool {
    match source_kind_for(metadata) {
        SourceKind::Image => {
            is_image_container(&preset.config.container)
                || is_gif_container(&preset.config.container)
        }
        SourceKind::Audio => is_audio_only_container(&preset.config.container),
        SourceKind::Video => true,
    }
}

#[must_use]
pub fn create_custom_preset(id: String, name: &str, config: &ConversionConfig) -> PresetDefinition {
    let mut preset_config = config.clone();
    // Source-bound selections must not leak into a reusable preset.
    preset_config.selected_audio_tracks.clear();
    preset_config.selected_subtitle_tracks.clear();
    preset_config.external_subtitle_tracks.clear();
    preset_config.subtitle_burn_path = None;

    PresetDefinition::custom(
        id,
        if name.trim().is_empty() {
            "Untitled Preset".to_string()
        } else {
            name.trim().to_string()
        },
        preset_config,
    )
}

pub(super) fn preset_config(container: &str) -> ConversionConfig {
    ConversionConfig {
        container: container.to_string(),
        ..ConversionConfig::default()
    }
}

pub(super) fn gif_preset_config(
    resolution: &str,
    custom_width: Option<&str>,
    custom_height: Option<&str>,
    fps: &str,
    colors: u16,
    dither: &str,
) -> ConversionConfig {
    ConversionConfig {
        container: "gif".to_string(),
        video_codec: "gif".to_string(),
        video_bitrate: "0".to_string(),
        audio_bitrate: "0".to_string(),
        resolution: resolution.to_string(),
        custom_width: custom_width.map(str::to_string),
        custom_height: custom_height.map(str::to_string),
        scaling_algorithm: "lanczos".to_string(),
        fps: fps.to_string(),
        metadata: MetadataConfig {
            mode: MetadataMode::Clean,
            ..MetadataConfig::default()
        },
        gif_colors: colors,
        gif_dither: dither.to_string(),
        ..ConversionConfig::default()
    }
}

pub(super) fn audio_preset_config(
    container: &str,
    codec: &str,
    bitrate: &str,
    channels: &str,
) -> ConversionConfig {
    ConversionConfig {
        container: container.to_string(),
        video_bitrate: "0".to_string(),
        audio_codec: codec.to_string(),
        audio_bitrate: bitrate.to_string(),
        audio_channels: channels.to_string(),
        ..ConversionConfig::default()
    }
}

pub(super) fn social_preset_config(
    bitrate: &str,
    resolution: &str,
    custom_width: Option<&str>,
    custom_height: Option<&str>,
    fps: &str,
    preset: &str,
) -> ConversionConfig {
    ConversionConfig {
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: bitrate.to_string(),
        audio_channels: "stereo".to_string(),
        audio_normalize: true,
        resolution: resolution.to_string(),
        custom_width: custom_width.map(str::to_string),
        custom_height: custom_height.map(str::to_string),
        scaling_algorithm: "lanczos".to_string(),
        fps: fps.to_string(),
        preset: preset.to_string(),
        metadata: MetadataConfig {
            mode: MetadataMode::Clean,
            ..MetadataConfig::default()
        },
        ..preset_config("mp4")
    }
}

pub(super) fn youtube_preset_config(
    bitrate: &str,
    resolution: &str,
    custom_width: Option<&str>,
    custom_height: Option<&str>,
) -> ConversionConfig {
    ConversionConfig {
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: bitrate.to_string(),
        audio_bitrate: "320".to_string(),
        audio_channels: "stereo".to_string(),
        audio_normalize: true,
        resolution: resolution.to_string(),
        custom_width: custom_width.map(str::to_string),
        custom_height: custom_height.map(str::to_string),
        scaling_algorithm: "lanczos".to_string(),
        preset: "slow".to_string(),
        ..preset_config("mp4")
    }
}
