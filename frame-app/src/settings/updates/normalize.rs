use crate::file_filters::is_supported_selectable_subtitle_path_for_container;
use crate::settings::model::{
    ConversionConfig, DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS, DEFAULT_AUDIO_VOLUME,
    DEFAULT_FPS, DEFAULT_GIF_DITHER, DEFAULT_IMAGE_JPEG_HUFFMAN, DEFAULT_IMAGE_PNG_PREDICTION,
    DEFAULT_IMAGE_TIFF_COMPRESSION, DEFAULT_IMAGE_WEBP_PRESET, DEFAULT_PIXEL_FORMAT,
    DEFAULT_RESOLUTION, DEFAULT_VIDEO_BITRATE_MODE, GIF_DITHER_OPTIONS, IMAGE_JPEG_HUFFMAN_OPTIONS,
    IMAGE_PNG_PREDICTION_OPTIONS, IMAGE_TIFF_COMPRESSION_OPTIONS, IMAGE_WEBP_PRESET_OPTIONS,
    MAX_AUDIO_VOLUME, MAX_GIF_COLORS, MAX_IMAGE_JPEG_QUALITY, MAX_IMAGE_PNG_COMPRESSION,
    MAX_IMAGE_WEBP_COMPRESSION, MAX_IMAGE_WEBP_QUALITY, MetadataMode, ProcessingMode, SourceKind,
    SourceMetadata,
};
use crate::settings::options::{
    first_allowed_video_codec, first_allowed_video_pixel_format, first_allowed_video_preset,
    is_hardware_video_codec, is_nvenc_video_codec, is_video_preset_allowed,
    is_videotoolbox_video_codec, mp2_original_channels_are_unsupported, stereo_width_available,
};
use crate::settings::rules::{
    container_supports_audio, container_supports_subtitles, default_audio_codec_for_container,
    is_audio_codec_allowed_for_container, is_audio_only_container, is_gif_container,
    is_image_container, is_video_codec_allowed_for_container,
    is_video_pixel_format_allowed_for_container, source_kind_for,
};
use crate::settings::updates::audio::{
    audio_codec_supports_vbr, is_known_audio_channels, normalized_audio_quality,
};
use crate::settings::updates::images::image_option_is_known;

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
        reset_video_filter_settings(config);
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
        reset_audio_filter_settings(config);
    }

    if container_supports_subtitles(&config.container) {
        config.external_subtitle_tracks.retain(|track| {
            is_supported_selectable_subtitle_path_for_container(
                std::path::Path::new(&track.path),
                &config.container,
            )
        });
        match frame_core::container::transport_stream_profile(&config.container) {
            Some(frame_core::container::TransportStreamProfile::M2ts192) => {
                for track in &mut config.external_subtitle_tracks {
                    if config.metadata.mode == MetadataMode::Preserve {
                        track.language = None;
                        track.title = None;
                    }
                    track.is_default = false;
                    track.is_forced = false;
                }
            }
            Some(frame_core::container::TransportStreamProfile::MpegTs188) => {
                for track in &mut config.external_subtitle_tracks {
                    if config.metadata.mode == MetadataMode::Preserve {
                        track.title = None;
                    }
                    track.is_default = false;
                    track.is_forced = false;
                }
            }
            None => {}
        }
        // Keep explicit selections so a saved or queued configuration cannot
        // silently drop a track after its output compatibility changes. The UI
        // permits deselecting incompatible tracks and core explains the error.
    } else {
        reset_subtitle_settings(config);
    }

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
    }
    normalize_audio_encoding_settings(config);
    if config.audio_channels == "original"
        && mp2_original_channels_are_unsupported(config, metadata)
    {
        config.audio_channels = "stereo".to_string();
    }
    if config.audio_filters.stereo_width.value != 100
        && metadata.is_some()
        && !config.selected_audio_tracks.is_empty()
        && !stereo_width_available(config, metadata)
    {
        config.audio_filters.stereo_width.enabled = false;
    }
    normalize_video_config_values(config, metadata);

    before != *config
}

pub fn initialize_output_config(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    normalize_output_config(config, metadata);

    if container_supports_audio(&config.container)
        && config.selected_audio_tracks.is_empty()
        && let Some(first_track) = metadata.and_then(|metadata| metadata.audio_tracks.first())
    {
        config.selected_audio_tracks.push(first_track.index);
    }

    before != *config
}

pub fn normalize_video_config(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    normalize_video_config_values(config, metadata);

    before != *config
}

fn normalize_video_config_values(config: &mut ConversionConfig, metadata: Option<&SourceMetadata>) {
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
        config.video_filters.deinterlace = crate::settings::model::DeinterlaceMode::Off;
        reset_subtitle_settings(config);
    }

    if (is_audio_container || is_gif_output || config.processing_mode == ProcessingMode::Copy)
        && let Some(overlay) = config.overlay.as_mut()
    {
        overlay.enabled = false;
    }

    if is_audio_container {
        config.pixel_format = DEFAULT_PIXEL_FORMAT.to_string();
        config.selected_subtitle_tracks.clear();
        reset_video_filter_settings(config);
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

    if config.video_codec == "vp9" {
        config.preset = frame_core::utils::map_vp9_speed(&config.preset);
    } else if config.video_codec != "prores"
        && !is_video_preset_allowed(&config.video_codec, &config.preset)
    {
        config.preset = first_allowed_video_preset(&config.video_codec).to_string();
    }
    if config.video_codec == "mpeg2video" {
        config.video_bitrate_mode = "bitrate".to_string();
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

    normalize_image_encoding_settings(config);

    config.gif_colors = config
        .gif_colors
        .clamp(frame_core::types::MIN_GIF_COLORS, MAX_GIF_COLORS);
    if !GIF_DITHER_OPTIONS.contains(&config.gif_dither.as_str()) {
        config.gif_dither = DEFAULT_GIF_DITHER.to_string();
    }
}

pub(super) fn normalize_audio_encoding_settings(config: &mut ConversionConfig) {
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
    if config.audio_codec == "mp2"
        && !matches!(
            config.audio_bitrate.as_str(),
            "64" | "96" | "112" | "128" | "160" | "192" | "224" | "256" | "320" | "384"
        )
    {
        config.audio_bitrate = "192".to_string();
    }
    config.audio_volume = config.audio_volume.min(MAX_AUDIO_VOLUME);
}

pub(super) fn normalize_image_encoding_settings(config: &mut ConversionConfig) {
    config.image_jpeg_quality = config.image_jpeg_quality.clamp(1, MAX_IMAGE_JPEG_QUALITY);
    if !image_option_is_known(&IMAGE_JPEG_HUFFMAN_OPTIONS, &config.image_jpeg_huffman) {
        config.image_jpeg_huffman = DEFAULT_IMAGE_JPEG_HUFFMAN.to_string();
    }

    config.image_webp_quality = config.image_webp_quality.min(MAX_IMAGE_WEBP_QUALITY);
    config.image_webp_compression = config
        .image_webp_compression
        .min(MAX_IMAGE_WEBP_COMPRESSION);
    if !image_option_is_known(&IMAGE_WEBP_PRESET_OPTIONS, &config.image_webp_preset) {
        config.image_webp_preset = DEFAULT_IMAGE_WEBP_PRESET.to_string();
    }

    config.image_png_compression = config.image_png_compression.min(MAX_IMAGE_PNG_COMPRESSION);
    if !image_option_is_known(&IMAGE_PNG_PREDICTION_OPTIONS, &config.image_png_prediction) {
        config.image_png_prediction = DEFAULT_IMAGE_PNG_PREDICTION.to_string();
    }

    if !image_option_is_known(
        &IMAGE_TIFF_COMPRESSION_OPTIONS,
        &config.image_tiff_compression,
    ) {
        config.image_tiff_compression = DEFAULT_IMAGE_TIFF_COMPRESSION.to_string();
    }
}

pub(super) fn reset_audio_filter_settings(config: &mut ConversionConfig) {
    config.audio_normalize = false;
    config.audio_volume = DEFAULT_AUDIO_VOLUME;
    config.audio_filters = crate::settings::model::AudioFiltersConfig::default();
}

pub(super) fn reset_subtitle_settings(config: &mut ConversionConfig) {
    config.selected_subtitle_tracks.clear();
    config.external_subtitle_tracks.clear();
    config.subtitle_burn_path = None;
    config.subtitle_font_name = None;
    config.subtitle_font_size = None;
    config.subtitle_font_color = None;
    config.subtitle_outline_color = None;
    config.subtitle_position = None;
}

pub(super) fn reset_video_filter_settings(config: &mut ConversionConfig) {
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
    config.video_filters = crate::settings::model::VideoFiltersConfig::default();
}
