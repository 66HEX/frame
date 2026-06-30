use super::{
    model::{
        AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, AudioQualityRange, ConversionConfig,
        DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS, DEFAULT_AUDIO_QUALITY,
        DEFAULT_AUDIO_VOLUME, DEFAULT_FPS, DEFAULT_GIF_DITHER, DEFAULT_PIXEL_FORMAT,
        DEFAULT_RESOLUTION, DEFAULT_VIDEO_BITRATE_MODE, FPS_OPTIONS, GIF_DITHER_OPTIONS,
        GIF_FPS_OPTIONS, MAX_AUDIO_VOLUME, MAX_GIF_COLORS, MAX_GIF_LOOP, MetadataField,
        MetadataMode, PresetDefinition, ProcessingMode, RESOLUTION_OPTIONS,
        SCALING_ALGORITHM_OPTIONS, SUBTITLE_FONT_SIZES, SourceKind, SourceMetadata,
        SubtitlePosition, VIDEO_CODEC_DEFINITIONS, VIDEO_PIXEL_FORMAT_DEFINITIONS,
    },
    options::{
        first_allowed_video_codec, first_allowed_video_pixel_format, first_allowed_video_preset,
        is_hardware_video_codec, is_nvenc_video_codec, is_video_preset_allowed,
        is_videotoolbox_video_codec, normalized_hex_color,
    },
    rules::{
        container_supports_audio, container_supports_subtitles, default_audio_codec_for_container,
        is_audio_codec_allowed_for_container, is_audio_only_container, is_gif_container,
        is_image_container, is_video_codec_allowed_for_container,
        is_video_pixel_format_allowed_for_container, source_kind_for,
    },
};
use crate::numeric::u32_to_u16;

#[must_use]
pub fn sanitize_output_name(value: &str) -> String {
    let candidate = value.rsplit(['/', '\\']).next().unwrap_or_default().trim();

    if candidate == "." || candidate == ".." {
        String::new()
    } else {
        candidate.to_string()
    }
}

pub fn toggle_audio_track_selection(config: &mut ConversionConfig, index: u32) -> bool {
    if config.selected_audio_tracks.contains(&index) {
        config
            .selected_audio_tracks
            .retain(|selected_index| *selected_index != index);
    } else {
        config.selected_audio_tracks.push(index);
    }

    true
}

pub fn apply_audio_codec(config: &mut ConversionConfig, codec: &str) -> bool {
    let codec = codec.to_ascii_lowercase();
    if config.processing_mode == ProcessingMode::Copy
        || !is_known_audio_codec(&codec)
        || !container_supports_audio(&config.container)
        || !is_audio_codec_allowed_for_container(&config.container, &codec)
    {
        return false;
    }

    if config.audio_codec.eq_ignore_ascii_case(&codec) {
        return false;
    }

    config.audio_codec = codec;
    normalize_audio_encoding_settings(config);
    true
}

pub fn apply_audio_channels(config: &mut ConversionConfig, channels: &str) -> bool {
    let channels = channels.to_ascii_lowercase();
    if config.processing_mode == ProcessingMode::Copy || !is_known_audio_channels(&channels) {
        return false;
    }

    if config.audio_channels.eq_ignore_ascii_case(&channels) {
        return false;
    }

    config.audio_channels = channels;
    true
}

pub fn apply_audio_bitrate(config: &mut ConversionConfig, bitrate: &str) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    let bitrate: String = bitrate.chars().filter(char::is_ascii_digit).collect();
    if config.audio_bitrate == bitrate {
        return false;
    }

    config.audio_bitrate = bitrate;
    true
}

pub fn apply_audio_bitrate_mode(config: &mut ConversionConfig, mode: &str) -> bool {
    let mode = mode.to_ascii_lowercase();
    if config.processing_mode == ProcessingMode::Copy
        || !matches!(mode.as_str(), "bitrate" | "vbr")
        || (mode == "vbr" && !audio_codec_supports_vbr(&config.audio_codec))
    {
        return false;
    }

    if config.audio_bitrate_mode == mode {
        return false;
    }

    config.audio_bitrate_mode = mode;
    normalize_audio_encoding_settings(config);
    true
}

pub fn apply_audio_quality(config: &mut ConversionConfig, quality: &str) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    let quality = normalized_audio_quality(&config.audio_codec, quality);
    if config.audio_quality == quality {
        return false;
    }

    config.audio_quality = quality;
    true
}

pub fn apply_audio_volume(config: &mut ConversionConfig, volume: u32) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    let volume = volume.min(MAX_AUDIO_VOLUME);
    if config.audio_volume == volume {
        return false;
    }

    config.audio_volume = volume;
    true
}

pub fn apply_audio_normalize(config: &mut ConversionConfig, enabled: bool) -> bool {
    if config.processing_mode == ProcessingMode::Copy {
        return false;
    }

    if config.audio_normalize == enabled {
        return false;
    }

    config.audio_normalize = enabled;
    true
}

pub fn apply_metadata_mode(config: &mut ConversionConfig, mode: MetadataMode) -> bool {
    if config.metadata.mode == mode {
        return false;
    }

    config.metadata.mode = mode;
    true
}

pub fn apply_metadata_field(
    config: &mut ConversionConfig,
    field: MetadataField,
    value: &str,
) -> bool {
    let value = if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    };

    let target = match field {
        MetadataField::Title => &mut config.metadata.title,
        MetadataField::Artist => &mut config.metadata.artist,
        MetadataField::Album => &mut config.metadata.album,
        MetadataField::Genre => &mut config.metadata.genre,
        MetadataField::Date => &mut config.metadata.date,
        MetadataField::Comment => &mut config.metadata.comment,
    };

    if *target == value {
        return false;
    }

    *target = value;
    true
}

pub fn apply_subtitle_burn_path(config: &mut ConversionConfig, path: Option<String>) -> bool {
    let path = path
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());
    if config.subtitle_burn_path == path {
        return false;
    }

    config.subtitle_burn_path = path;
    true
}

pub fn apply_subtitle_font_name(config: &mut ConversionConfig, font: &str) -> bool {
    let font = font.trim();
    let font = if font.is_empty() {
        None
    } else {
        Some(font.to_string())
    };
    if config.subtitle_font_name == font {
        return false;
    }

    config.subtitle_font_name = font;
    true
}

pub fn apply_subtitle_font_size(config: &mut ConversionConfig, size: &str) -> bool {
    let size = size.trim();
    let size = if size.is_empty() {
        None
    } else if SUBTITLE_FONT_SIZES.contains(&size) {
        Some(size.to_string())
    } else {
        return false;
    };

    if config.subtitle_font_size == size {
        return false;
    }

    config.subtitle_font_size = size;
    true
}

pub fn apply_subtitle_font_color(config: &mut ConversionConfig, color: &str) -> bool {
    apply_subtitle_color(&mut config.subtitle_font_color, color)
}

pub fn apply_subtitle_outline_color(config: &mut ConversionConfig, color: &str) -> bool {
    apply_subtitle_color(&mut config.subtitle_outline_color, color)
}

pub fn apply_subtitle_position(config: &mut ConversionConfig, position: SubtitlePosition) -> bool {
    let position = Some(position.id().to_string());
    if config.subtitle_position == position {
        return false;
    }

    config.subtitle_position = position;
    true
}

pub fn toggle_subtitle_track_selection(config: &mut ConversionConfig, index: u32) -> bool {
    if config.selected_subtitle_tracks.contains(&index) {
        config
            .selected_subtitle_tracks
            .retain(|selected_index| *selected_index != index);
    } else {
        config.selected_subtitle_tracks.push(index);
    }

    true
}

pub fn apply_preset(
    config: &mut ConversionConfig,
    preset: &PresetDefinition,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    *config = preset.config.clone();
    normalize_output_config(config, metadata);

    before != *config
}

pub fn apply_resolution(config: &mut ConversionConfig, resolution: &str) -> bool {
    let resolution = resolution.to_ascii_lowercase();
    if !RESOLUTION_OPTIONS.contains(&resolution.as_str()) {
        return false;
    }

    if config.resolution == resolution {
        return false;
    }

    config.resolution = resolution;
    true
}

pub fn apply_custom_width(config: &mut ConversionConfig, width: &str) -> bool {
    let width = sanitized_optional_number(width);
    if config.custom_width == width {
        return false;
    }

    config.custom_width = width;
    true
}

pub fn apply_custom_height(config: &mut ConversionConfig, height: &str) -> bool {
    let height = sanitized_optional_number(height);
    if config.custom_height == height {
        return false;
    }

    config.custom_height = height;
    true
}

pub fn apply_scaling_algorithm(config: &mut ConversionConfig, algorithm: &str) -> bool {
    let algorithm = algorithm.to_ascii_lowercase();
    if !SCALING_ALGORITHM_OPTIONS.contains(&algorithm.as_str()) {
        return false;
    }

    if config.scaling_algorithm == algorithm {
        return false;
    }

    config.scaling_algorithm = algorithm;
    true
}

pub fn apply_fps(config: &mut ConversionConfig, fps: &str) -> bool {
    let valid = if is_gif_container(&config.container) {
        GIF_FPS_OPTIONS.contains(&fps)
    } else {
        FPS_OPTIONS.contains(&fps)
    };
    if !valid {
        return false;
    }

    if config.fps == fps {
        return false;
    }

    config.fps = fps.to_string();
    true
}

pub fn apply_gif_colors(config: &mut ConversionConfig, colors: u16) -> bool {
    let colors = colors.clamp(2, MAX_GIF_COLORS);
    if config.gif_colors == colors {
        return false;
    }

    config.gif_colors = colors;
    true
}

pub fn apply_gif_dither(config: &mut ConversionConfig, dither: &str) -> bool {
    let dither = dither.to_ascii_lowercase();
    if !GIF_DITHER_OPTIONS.contains(&dither.as_str()) {
        return false;
    }

    if config.gif_dither == dither {
        return false;
    }

    config.gif_dither = dither;
    true
}

pub fn apply_gif_loop(config: &mut ConversionConfig, loop_count: &str) -> bool {
    let parsed = loop_count
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse::<u32>()
        .map_or(0, |value| u32_to_u16(value.min(u32::from(MAX_GIF_LOOP))));

    if config.gif_loop == parsed {
        return false;
    }

    config.gif_loop = parsed;
    true
}

pub fn apply_video_codec(config: &mut ConversionConfig, codec: &str) -> bool {
    let codec = codec.to_ascii_lowercase();
    if !is_known_video_codec(&codec)
        || !is_video_codec_allowed_for_container(&config.container, &codec)
    {
        return false;
    }

    let changed = config.video_codec != codec;
    config.video_codec = codec;
    changed | normalize_video_config(config, None)
}

pub fn apply_pixel_format(config: &mut ConversionConfig, pixel_format: &str) -> bool {
    let pixel_format = pixel_format.to_ascii_lowercase();
    if !is_known_pixel_format(&pixel_format)
        || !is_video_pixel_format_allowed_for_container(
            &config.container,
            &config.video_codec,
            &pixel_format,
        )
    {
        return false;
    }

    if config.pixel_format == pixel_format {
        return false;
    }

    config.pixel_format = pixel_format;
    true
}

pub fn apply_video_preset(config: &mut ConversionConfig, preset: &str) -> bool {
    let preset = preset.to_ascii_lowercase();
    if !is_video_preset_allowed(&config.video_codec, &preset) {
        return false;
    }

    if config.preset == preset {
        return false;
    }

    config.preset = preset;
    true
}

pub fn apply_video_bitrate_mode(config: &mut ConversionConfig, mode: &str) -> bool {
    let mode = mode.to_ascii_lowercase();
    if !matches!(mode.as_str(), "crf" | "bitrate") {
        return false;
    }

    if config.video_bitrate_mode == mode {
        return false;
    }

    config.video_bitrate_mode = mode;
    true
}

pub fn apply_video_bitrate(config: &mut ConversionConfig, bitrate: &str) -> bool {
    let bitrate: String = bitrate.chars().filter(char::is_ascii_digit).collect();
    if config.video_bitrate == bitrate {
        return false;
    }

    config.video_bitrate = bitrate;
    true
}

pub fn apply_crf(config: &mut ConversionConfig, crf: u8) -> bool {
    let crf = crf.min(51);
    if config.crf == crf {
        return false;
    }

    config.crf = crf;
    true
}

pub fn apply_quality(config: &mut ConversionConfig, quality: u32) -> bool {
    let quality = quality.clamp(1, 100);
    if config.quality == quality {
        return false;
    }

    config.quality = quality;
    true
}

pub fn apply_nvenc_spatial_aq(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_nvenc_video_codec(&config.video_codec) || config.nvenc_spatial_aq == enabled {
        return false;
    }

    config.nvenc_spatial_aq = enabled;
    true
}

pub fn apply_nvenc_temporal_aq(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_nvenc_video_codec(&config.video_codec) || config.nvenc_temporal_aq == enabled {
        return false;
    }

    config.nvenc_temporal_aq = enabled;
    true
}

pub fn apply_videotoolbox_allow_sw(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_videotoolbox_video_codec(&config.video_codec) || config.videotoolbox_allow_sw == enabled
    {
        return false;
    }

    config.videotoolbox_allow_sw = enabled;
    true
}

pub fn apply_hw_decode(config: &mut ConversionConfig, enabled: bool) -> bool {
    if !is_hardware_video_codec(&config.video_codec) || config.hw_decode == enabled {
        return false;
    }

    config.hw_decode = enabled;
    true
}

pub fn apply_processing_mode(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
    mode: ProcessingMode,
) -> bool {
    if mode == ProcessingMode::Copy && source_kind_for(metadata) == SourceKind::Image {
        return false;
    }

    let changed = config.processing_mode != mode;
    config.processing_mode = mode;
    changed | normalize_output_config(config, metadata)
}

pub fn apply_output_container(config: &mut ConversionConfig, container: &str) -> bool {
    let changed = !config.container.eq_ignore_ascii_case(container);
    config.container = container.to_ascii_lowercase();

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
        normalize_audio_encoding_settings(config);
        return true;
    }

    normalize_audio_encoding_settings(config);
    normalize_video_config(config, None);
    changed
}

pub fn apply_trim_times(
    config: &mut ConversionConfig,
    start_time: Option<String>,
    end_time: Option<String>,
) -> bool {
    let start_time = normalize_optional_timecode(start_time);
    let end_time = normalize_optional_timecode(end_time);
    let changed = config.start_time != start_time || config.end_time != end_time;

    config.start_time = start_time;
    config.end_time = end_time;

    changed
}

fn normalize_optional_timecode(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn normalize_output_config(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    let source_kind = source_kind_for(metadata);

    if source_kind == SourceKind::Audio && !is_audio_only_container(&config.container) {
        config.container = "mp3".to_string();
    }

    if source_kind == SourceKind::Image
        && !is_image_container(&config.container)
        && !is_gif_container(&config.container)
    {
        config.container = "png".to_string();
    }

    if source_kind == SourceKind::Image {
        config.start_time = None;
        config.end_time = None;
        config.selected_audio_tracks.clear();
        config.selected_subtitle_tracks.clear();
        reset_subtitle_settings(config);
        config.metadata.album = None;
        config.metadata.genre = None;
        reset_audio_filter_settings(config);
    }

    if source_kind == SourceKind::Audio || is_audio_only_container(&config.container) {
        config.crop = None;
        reset_subtitle_settings(config);
    }

    if (source_kind == SourceKind::Image || is_gif_container(&config.container))
        && config.processing_mode == ProcessingMode::Copy
    {
        config.processing_mode = ProcessingMode::Reencode;
    }

    if config.processing_mode == ProcessingMode::Copy {
        reset_audio_filter_settings(config);
        reset_video_filter_settings(config);
        config.subtitle_burn_path = None;
    }

    if !container_supports_audio(&config.container) {
        config.selected_audio_tracks.clear();
        config.audio_normalize = false;
    }

    if !container_supports_subtitles(&config.container) {
        reset_subtitle_settings(config);
    }

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
    }
    normalize_audio_encoding_settings(config);
    normalize_video_config(config, metadata);

    before != *config
}

pub fn normalize_video_config(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    let source_kind = source_kind_for(metadata);
    let is_audio_container = is_audio_only_container(&config.container);
    let is_gif_output = is_gif_container(&config.container);

    if config.processing_mode == ProcessingMode::Copy {
        reset_video_filter_settings(config);
    }

    if source_kind == SourceKind::Image {
        config.processing_mode = ProcessingMode::Reencode;
        config.selected_audio_tracks.clear();
        config.selected_subtitle_tracks.clear();
        reset_subtitle_settings(config);
    }

    if is_audio_container {
        config.pixel_format = DEFAULT_PIXEL_FORMAT.to_string();
        config.selected_subtitle_tracks.clear();
        reset_subtitle_settings(config);
    }

    if is_gif_output {
        config.pixel_format = DEFAULT_PIXEL_FORMAT.to_string();
        config.video_codec = "gif".to_string();
        config.video_bitrate_mode = DEFAULT_VIDEO_BITRATE_MODE.to_string();
        config.hw_decode = false;
        config.nvenc_spatial_aq = false;
        config.nvenc_temporal_aq = false;
        config.videotoolbox_allow_sw = false;
    } else if !is_audio_container
        && !is_video_codec_allowed_for_container(&config.container, &config.video_codec)
    {
        config.video_codec = first_allowed_video_codec(&config.container, None);
    }

    if !is_video_pixel_format_allowed_for_container(
        &config.container,
        &config.video_codec,
        &config.pixel_format,
    ) {
        config.pixel_format =
            first_allowed_video_pixel_format(&config.container, &config.video_codec).to_string();
    }

    if !is_video_preset_allowed(&config.video_codec, &config.preset) {
        config.preset = first_allowed_video_preset(&config.video_codec).to_string();
    }

    if !is_nvenc_video_codec(&config.video_codec) {
        config.nvenc_spatial_aq = false;
        config.nvenc_temporal_aq = false;
    }
    if !is_videotoolbox_video_codec(&config.video_codec) {
        config.videotoolbox_allow_sw = false;
    }
    if !is_hardware_video_codec(&config.video_codec) {
        config.hw_decode = false;
    }

    config.gif_colors = config.gif_colors.clamp(2, MAX_GIF_COLORS);
    if !GIF_DITHER_OPTIONS.contains(&config.gif_dither.as_str()) {
        config.gif_dither = DEFAULT_GIF_DITHER.to_string();
    }

    before != *config
}

fn normalize_audio_encoding_settings(config: &mut ConversionConfig) {
    if !matches!(config.audio_bitrate_mode.as_str(), "bitrate" | "vbr") {
        config.audio_bitrate_mode = DEFAULT_AUDIO_BITRATE_MODE.to_string();
    }
    if config.audio_bitrate_mode == "vbr" && !audio_codec_supports_vbr(&config.audio_codec) {
        config.audio_bitrate_mode = DEFAULT_AUDIO_BITRATE_MODE.to_string();
    }
    if !is_known_audio_channels(&config.audio_channels) {
        config.audio_channels = DEFAULT_AUDIO_CHANNELS.to_string();
    }

    config.audio_quality = normalized_audio_quality(&config.audio_codec, &config.audio_quality);
    config.audio_volume = config.audio_volume.min(MAX_AUDIO_VOLUME);
}

fn reset_audio_filter_settings(config: &mut ConversionConfig) {
    config.audio_normalize = false;
    config.audio_volume = DEFAULT_AUDIO_VOLUME;
    config.audio_bitrate_mode = DEFAULT_AUDIO_BITRATE_MODE.to_string();
}

fn reset_subtitle_settings(config: &mut ConversionConfig) {
    config.selected_subtitle_tracks.clear();
    config.subtitle_burn_path = None;
    config.subtitle_font_name = None;
    config.subtitle_font_size = None;
    config.subtitle_font_color = None;
    config.subtitle_outline_color = None;
    config.subtitle_position = None;
}

fn reset_video_filter_settings(config: &mut ConversionConfig) {
    config.pixel_format = DEFAULT_PIXEL_FORMAT.to_string();
    config.resolution = DEFAULT_RESOLUTION.to_string();
    config.custom_width = None;
    config.custom_height = None;
    config.fps = DEFAULT_FPS.to_string();
    config.rotation = "0".to_string();
    config.flip_horizontal = false;
    config.flip_vertical = false;
    config.crop = None;
    config.hw_decode = false;
    config.nvenc_spatial_aq = false;
    config.nvenc_temporal_aq = false;
    config.videotoolbox_allow_sw = false;
}

fn apply_subtitle_color(target: &mut Option<String>, color: &str) -> bool {
    let Some(color) = normalized_hex_color(color) else {
        return false;
    };
    if target.as_deref() == Some(color.as_str()) {
        return false;
    }

    *target = Some(color);
    true
}

#[must_use]
pub fn audio_codec_supports_vbr(codec: &str) -> bool {
    matches!(codec, "mp3" | "libfdk_aac")
}

#[must_use]
pub fn audio_quality_range(codec: &str) -> Option<AudioQualityRange> {
    match codec {
        "mp3" => Some(AudioQualityRange {
            min: 0,
            max: 9,
            lower_is_better: true,
            default_value: 4,
        }),
        "libfdk_aac" => Some(AudioQualityRange {
            min: 1,
            max: 5,
            lower_is_better: false,
            default_value: 4,
        }),
        _ => None,
    }
}

fn normalized_audio_quality(codec: &str, quality: &str) -> String {
    let Some(range) = audio_quality_range(codec) else {
        return if quality.trim().is_empty() {
            DEFAULT_AUDIO_QUALITY.to_string()
        } else {
            quality.trim().to_string()
        };
    };

    let parsed = quality.trim().parse::<u32>().unwrap_or(range.default_value);
    parsed.clamp(range.min, range.max).to_string()
}

fn is_known_audio_codec(codec: &str) -> bool {
    AUDIO_CODEC_DEFINITIONS
        .iter()
        .any(|definition| definition.codec == codec)
}

fn is_known_audio_channels(channels: &str) -> bool {
    AUDIO_CHANNEL_DEFINITIONS
        .iter()
        .any(|definition| definition.id == channels)
}

fn is_known_video_codec(codec: &str) -> bool {
    VIDEO_CODEC_DEFINITIONS
        .iter()
        .any(|definition| definition.codec == codec)
}

fn is_known_pixel_format(pixel_format: &str) -> bool {
    VIDEO_PIXEL_FORMAT_DEFINITIONS
        .iter()
        .any(|definition| definition.id == pixel_format)
}

fn sanitized_optional_number(value: &str) -> Option<String> {
    let value: String = value.chars().filter(char::is_ascii_digit).collect();
    (!value.is_empty()).then_some(value)
}
