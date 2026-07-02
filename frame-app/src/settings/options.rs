use frame_core::{capabilities::AvailableEncoders, media_rules};

use super::{
    model::{
        AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, AudioChannelOption, AudioCodecOption,
        AudioTrackOption, ConversionConfig, FPS_OPTIONS, GIF_COLOR_OPTIONS, GIF_DITHER_OPTIONS,
        GIF_FPS_OPTIONS, IMAGE_JPEG_HUFFMAN_OPTIONS, IMAGE_PNG_PREDICTION_OPTIONS,
        IMAGE_TIFF_COMPRESSION_OPTIONS, IMAGE_WEBP_PRESET_OPTIONS, ImageEncodingOption,
        METADATA_FIELDS, METADATA_MODES, MetadataConfig, MetadataField, MetadataFieldOption,
        MetadataMode, MetadataModeOption, OPTIONAL_AUDIO_CODEC_DEFINITIONS, OutputContainerOption,
        OutputModeOption, PresetDefinition, PresetOption, ProcessingMode, RESOLUTION_OPTIONS,
        SCALING_ALGORITHM_OPTIONS, SUBTITLE_FONT_SIZES, SUBTITLE_POSITIONS, SourceKind,
        SourceMetadata, SubtitleFontOption, SubtitleFontSizeOption, SubtitlePosition,
        SubtitlePositionOption, SubtitleTrackOption, VIDEO_CODEC_DEFINITIONS,
        VIDEO_PIXEL_FORMAT_DEFINITIONS, VIDEO_PRESETS, VideoCodecCapability, VideoCodecOption,
        VideoPixelFormatOption, VideoPresetOption,
    },
    rules::{
        is_audio_codec_allowed_for_container, is_audio_only_container,
        is_audio_stream_codec_allowed_for_container, is_gif_container, is_image_container,
        is_subtitle_codec_allowed_for_container, is_video_codec_allowed_for_container,
        is_video_pixel_format_allowed_for_container, is_video_stream_codec_allowed_for_container,
        source_kind_for,
    },
    source_info::{audio_track_detail, display_source_value},
};

#[must_use]
pub fn output_processing_mode_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> [OutputModeOption; 2] {
    let is_source_image = source_kind_for(metadata) == SourceKind::Image;
    [
        output_mode_option(ProcessingMode::Reencode, config, disabled),
        output_mode_option(ProcessingMode::Copy, config, disabled || is_source_image),
    ]
}

#[must_use]
pub fn visible_output_containers(metadata: Option<&SourceMetadata>) -> Vec<String> {
    let is_source_image = source_kind_for(metadata) == SourceKind::Image;

    media_rules::all_containers()
        .iter()
        .filter(|container| {
            if is_source_image {
                is_image_container(container) || is_gif_container(container)
            } else {
                !is_image_container(container)
            }
        })
        .cloned()
        .collect()
}

#[must_use]
pub fn output_container_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<OutputContainerOption> {
    visible_output_containers(metadata)
        .into_iter()
        .map(|container| {
            let disabled_reason =
                output_container_disabled_reason(config, metadata, &container, disabled);
            OutputContainerOption {
                is_selected: config.container.eq_ignore_ascii_case(&container),
                is_disabled: disabled_reason.is_some(),
                disabled_reason,
                container,
            }
        })
        .collect()
}

#[must_use]
pub fn audio_codec_options(
    config: &ConversionConfig,
    available_encoders: &AvailableEncoders,
    disabled: bool,
) -> Vec<AudioCodecOption> {
    let encode_controls_disabled = disabled || config.processing_mode == ProcessingMode::Copy;

    AUDIO_CODEC_DEFINITIONS
        .iter()
        .map(|definition| (definition.codec, definition.label))
        .chain(
            OPTIONAL_AUDIO_CODEC_DEFINITIONS
                .iter()
                .filter(|definition| {
                    definition.codec != "libfdk_aac" || available_encoders.libfdk_aac
                })
                .map(|definition| (definition.codec, definition.label)),
        )
        .map(|definition| {
            let is_compatible =
                is_audio_codec_allowed_for_container(&config.container, definition.0);
            AudioCodecOption {
                codec: definition.0,
                label: definition.1,
                is_selected: config.audio_codec.eq_ignore_ascii_case(definition.0),
                is_disabled: encode_controls_disabled || !is_compatible,
                disabled_reason: (!is_compatible).then_some("Incompatible container"),
            }
        })
        .collect()
}

#[must_use]
pub fn audio_channel_options(config: &ConversionConfig, disabled: bool) -> [AudioChannelOption; 3] {
    let disabled = disabled || config.processing_mode == ProcessingMode::Copy;

    AUDIO_CHANNEL_DEFINITIONS.map(|definition| AudioChannelOption {
        id: definition.id,
        label: definition.label,
        is_selected: config.audio_channels.eq_ignore_ascii_case(definition.id),
        is_disabled: disabled,
    })
}

#[must_use]
pub fn audio_track_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<AudioTrackOption> {
    metadata
        .map(|metadata| {
            metadata
                .audio_tracks
                .iter()
                .map(|track| AudioTrackOption {
                    index: track.index,
                    index_label: format!("#{}", track.index),
                    codec: display_source_value(Some(&track.codec)),
                    detail: audio_track_detail(track),
                    is_selected: config.selected_audio_tracks.contains(&track.index),
                    is_disabled: disabled,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[must_use]
pub fn subtitle_track_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<SubtitleTrackOption> {
    metadata
        .map(|metadata| {
            metadata
                .subtitle_tracks
                .iter()
                .map(|track| SubtitleTrackOption {
                    index: track.index,
                    index_label: format!("#{}", track.index),
                    codec: display_source_value(Some(&track.codec)),
                    detail: subtitle_track_detail(
                        track.language.as_deref(),
                        track.label.as_deref(),
                    ),
                    is_selected: config.selected_subtitle_tracks.contains(&track.index),
                    is_disabled: disabled,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[must_use]
pub fn subtitle_font_options(
    config: &ConversionConfig,
    fonts: &[String],
    disabled: bool,
) -> Vec<SubtitleFontOption> {
    let selected = config.subtitle_font_name.as_deref().unwrap_or_default();
    fonts
        .iter()
        .map(|font| SubtitleFontOption {
            name: font.clone(),
            is_selected: selected == font,
            is_disabled: disabled,
        })
        .collect()
}

#[must_use]
pub fn subtitle_font_size_options(
    config: &ConversionConfig,
    disabled: bool,
) -> [SubtitleFontSizeOption; 14] {
    let selected = config.subtitle_font_size.as_deref().unwrap_or_default();
    SUBTITLE_FONT_SIZES.map(|size| SubtitleFontSizeOption {
        size,
        is_selected: selected == size,
        is_disabled: disabled,
    })
}

#[must_use]
pub fn subtitle_position_options(
    config: &ConversionConfig,
    disabled: bool,
) -> [SubtitlePositionOption; 3] {
    let selected = subtitle_position(config);
    SUBTITLE_POSITIONS.map(|position| SubtitlePositionOption {
        position,
        label: position.label(),
        is_selected: selected == position,
        is_disabled: disabled,
    })
}

#[must_use]
pub fn subtitle_position(config: &ConversionConfig) -> SubtitlePosition {
    config
        .subtitle_position
        .as_deref()
        .and_then(SubtitlePosition::from_id)
        .unwrap_or(super::model::DEFAULT_SUBTITLE_POSITION)
}

#[must_use]
pub fn subtitle_burn_file_label(config: &ConversionConfig) -> String {
    config
        .subtitle_burn_path
        .as_deref()
        .and_then(|path| path.rsplit(['/', '\\']).next())
        .filter(|name| !name.is_empty())
        .map_or_else(
            || "Select .srt or .ass file".to_string(),
            ToString::to_string,
        )
}

#[must_use]
pub fn subtitle_color_value(value: Option<&String>, fallback: &str) -> String {
    value
        .and_then(|value| normalized_hex_color(value.as_str()))
        .unwrap_or_else(|| fallback.to_string())
}

#[must_use]
pub fn normalized_hex_color(value: &str) -> Option<String> {
    let source = value.trim().trim_start_matches('#');
    if source.len() == 3 && source.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let mut expanded = String::from("#");
        for ch in source.chars() {
            expanded.push(ch.to_ascii_lowercase());
            expanded.push(ch.to_ascii_lowercase());
        }
        return Some(expanded);
    }
    if source.len() == 6 && source.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Some(format!("#{}", source.to_ascii_lowercase()));
    }

    None
}

#[must_use]
pub fn metadata_mode_options(config: &ConversionConfig, disabled: bool) -> [MetadataModeOption; 3] {
    METADATA_MODES.map(|mode| MetadataModeOption {
        mode,
        label: mode.label(),
        is_selected: config.metadata.mode == mode,
        is_disabled: disabled,
    })
}

#[must_use]
pub fn metadata_field_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<MetadataFieldOption> {
    metadata_fields_for_source(metadata)
        .iter()
        .copied()
        .map(|field| MetadataFieldOption {
            field,
            id: field.id(),
            label: field.label(),
            value: metadata_field_value(config, field)
                .unwrap_or_default()
                .to_string(),
            placeholder: metadata_field_placeholder(config, metadata, field),
            is_disabled: disabled,
        })
        .collect()
}

#[must_use]
pub fn metadata_fields_for_source(metadata: Option<&SourceMetadata>) -> Vec<MetadataField> {
    let is_image = source_kind_for(metadata) == SourceKind::Image;
    METADATA_FIELDS
        .iter()
        .copied()
        .filter(|field| !is_image || field.visible_for_image())
        .collect()
}

#[must_use]
pub fn metadata_field_value(config: &ConversionConfig, field: MetadataField) -> Option<&str> {
    match field {
        MetadataField::Title => config.metadata.title.as_deref(),
        MetadataField::Artist => config.metadata.artist.as_deref(),
        MetadataField::Album => config.metadata.album.as_deref(),
        MetadataField::Genre => config.metadata.genre.as_deref(),
        MetadataField::Date => config.metadata.date.as_deref(),
        MetadataField::Comment => config.metadata.comment.as_deref(),
    }
}

#[must_use]
pub fn metadata_field_placeholder(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    field: MetadataField,
) -> String {
    if config.metadata.mode != MetadataMode::Preserve {
        return String::new();
    }

    metadata
        .and_then(|metadata| metadata.tags.as_ref())
        .and_then(|tags| tags.value(field))
        .filter(|value| !value.trim().is_empty())
        .map_or_else(
            || "Leave empty to keep original".to_string(),
            ToString::to_string,
        )
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "Preset definitions are data declarations and are easier to compare when kept as one table."
)]
pub fn default_presets() -> Vec<PresetDefinition> {
    vec![
        PresetDefinition::built_in("balanced-mp4", "Balanced MP4", preset_config("mp4")),
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
    PresetDefinition::custom(
        id,
        if name.trim().is_empty() {
            "Untitled Preset".to_string()
        } else {
            name.trim().to_string()
        },
        config.clone(),
    )
}

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
pub fn image_jpeg_huffman_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_JPEG_HUFFMAN_OPTIONS,
        &config.image_jpeg_huffman,
        disabled,
    )
}

#[must_use]
pub fn image_webp_preset_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_WEBP_PRESET_OPTIONS,
        &config.image_webp_preset,
        disabled,
    )
}

#[must_use]
pub fn image_png_prediction_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_PNG_PREDICTION_OPTIONS,
        &config.image_png_prediction,
        disabled,
    )
}

#[must_use]
pub fn image_tiff_compression_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_TIFF_COMPRESSION_OPTIONS,
        &config.image_tiff_compression,
        disabled,
    )
}

fn image_encoding_options(
    definitions: &[super::model::ImageEncodingOptionDefinition],
    selected_id: &str,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    definitions
        .iter()
        .map(|definition| ImageEncodingOption {
            id: definition.id,
            label: definition.label,
            caption: definition.caption,
            is_selected: selected_id == definition.id,
            is_disabled: disabled,
        })
        .collect()
}

#[must_use]
pub fn video_codec_options(
    config: &ConversionConfig,
    available_encoders: &AvailableEncoders,
    disabled: bool,
) -> Vec<VideoCodecOption> {
    VIDEO_CODEC_DEFINITIONS
        .iter()
        .filter(|definition| {
            definition.capability.is_none_or(|capability| {
                video_codec_capability_available(available_encoders, capability)
            })
        })
        .map(|definition| {
            let allowed = is_video_codec_allowed_for_container(&config.container, definition.codec);
            VideoCodecOption {
                codec: definition.codec,
                label: definition.label,
                is_selected: allowed && config.video_codec.eq_ignore_ascii_case(definition.codec),
                is_disabled: disabled || !allowed,
                disabled_reason: (!allowed).then_some("Incompatible container"),
            }
        })
        .collect()
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

#[must_use]
pub fn video_preset_options(config: &ConversionConfig, disabled: bool) -> Vec<VideoPresetOption> {
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

#[must_use]
pub fn is_container_compatible_for_stream_copy(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    container: &str,
) -> bool {
    if config.processing_mode != ProcessingMode::Copy {
        return true;
    }
    if source_kind_for(metadata) == SourceKind::Image {
        return false;
    }
    if is_image_container(container) || is_gif_container(container) {
        return false;
    }

    let Some(metadata) = metadata else {
        return true;
    };

    let selected_audio_codecs = selected_audio_codecs(config, metadata);
    if is_audio_only_container(container) {
        return !selected_audio_codecs.is_empty()
            && selected_audio_codecs
                .iter()
                .all(|codec| is_audio_stream_codec_allowed_for_container(container, codec));
    }

    let Some(video_codec) = metadata.video_codec.as_deref() else {
        return false;
    };
    if !is_video_stream_codec_allowed_for_container(container, video_codec) {
        return false;
    }

    let audio_codecs_allowed = selected_audio_codecs
        .iter()
        .all(|codec| is_audio_stream_codec_allowed_for_container(container, codec));
    let subtitle_codecs_allowed = selected_subtitle_codecs(config, metadata)
        .iter()
        .all(|codec| is_subtitle_codec_allowed_for_container(container, codec));

    audio_codecs_allowed && subtitle_codecs_allowed
}

fn output_mode_option(
    mode: ProcessingMode,
    config: &ConversionConfig,
    is_disabled: bool,
) -> OutputModeOption {
    OutputModeOption {
        mode,
        label: mode.label(),
        hint: mode.hint(),
        is_selected: config.processing_mode == mode,
        is_disabled,
    }
}

fn preset_config(container: &str) -> ConversionConfig {
    ConversionConfig {
        container: container.to_string(),
        ..ConversionConfig::default()
    }
}

fn gif_preset_config(
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

fn audio_preset_config(
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

fn social_preset_config(
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

fn youtube_preset_config(
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

fn subtitle_track_detail(language: Option<&str>, label: Option<&str>) -> String {
    [language, label]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" • ")
}

fn output_container_disabled_reason(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    container: &str,
    disabled: bool,
) -> Option<&'static str> {
    let source_kind = source_kind_for(metadata);

    if disabled {
        return Some("Locked");
    }
    if source_kind == SourceKind::Audio && !is_audio_only_container(container) {
        return Some("Video container unavailable for audio sources");
    }
    if source_kind == SourceKind::Image && is_audio_only_container(container) {
        return Some("Audio container unavailable for image sources");
    }
    if !is_container_compatible_for_stream_copy(config, metadata, container) {
        return Some("Incompatible container");
    }

    None
}

fn selected_audio_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    if metadata.audio_tracks.is_empty() {
        return Vec::new();
    }
    if config.selected_audio_tracks.is_empty() {
        return metadata
            .audio_tracks
            .iter()
            .map(|track| track.codec.as_str())
            .collect();
    }

    metadata
        .audio_tracks
        .iter()
        .filter(|track| config.selected_audio_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}

fn selected_subtitle_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    if metadata.subtitle_tracks.is_empty() {
        return Vec::new();
    }
    if config.selected_subtitle_tracks.is_empty() {
        return metadata
            .subtitle_tracks
            .iter()
            .map(|track| track.codec.as_str())
            .collect();
    }

    metadata
        .subtitle_tracks
        .iter()
        .filter(|track| config.selected_subtitle_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}

const fn video_codec_capability_available(
    available_encoders: &AvailableEncoders,
    capability: VideoCodecCapability,
) -> bool {
    match capability {
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

fn video_codec_available(codec: &str, available_encoders: Option<&AvailableEncoders>) -> bool {
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
