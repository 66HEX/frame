use frame_core::{
    media_rules,
    types::{
        AudioFiltersConfig as CoreAudioFiltersConfig, ConversionConfig as CoreConversionConfig,
        ConversionTask, CropConfig, DeinterlaceMode as CoreDeinterlaceMode,
        FilterStrength as CoreFilterStrength, FilterValue as CoreFilterValue,
        MetadataConfig as CoreMetadataConfig, MetadataMode as CoreMetadataMode, OverlayConfig,
        VideoColorFiltersConfig as CoreVideoColorFiltersConfig,
        VideoFiltersConfig as CoreVideoFiltersConfig,
    },
};

use crate::{
    file_queue::FileItem,
    settings::{
        AudioFiltersConfig as GpuiAudioFiltersConfig, ConversionConfig as GpuiConversionConfig,
        CropSettings, DEFAULT_AUDIO_BITRATE, DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS,
        DEFAULT_AUDIO_QUALITY, DEFAULT_FPS, DEFAULT_GIF_COLORS, DEFAULT_GIF_DITHER,
        DEFAULT_PIXEL_FORMAT, DEFAULT_PRESET, DEFAULT_RESOLUTION, DEFAULT_SCALING_ALGORITHM,
        DEFAULT_VIDEO_BITRATE, DEFAULT_VIDEO_BITRATE_MODE, DEFAULT_VIDEO_CODEC,
        DeinterlaceMode as GpuiDeinterlaceMode, FilterStrength as GpuiFilterStrength,
        FilterValue as GpuiFilterValue, MetadataConfig as GpuiMetadataConfig,
        MetadataMode as GpuiMetadataMode, OverlaySettings,
        VideoColorFiltersConfig as GpuiVideoColorFiltersConfig,
        VideoFiltersConfig as GpuiVideoFiltersConfig,
    },
};

#[must_use]
pub fn conversion_task_from_file(file: &FileItem) -> ConversionTask {
    let output_name = crate::settings::sanitize_output_name(&file.output_name);

    ConversionTask {
        id: file.id.clone(),
        file_path: file.path.clone(),
        output_name: (!output_name.is_empty()).then_some(output_name),
        config: core_config_from_gpui(&file.config),
    }
}

#[must_use]
pub fn core_config_from_gpui(config: &GpuiConversionConfig) -> CoreConversionConfig {
    CoreConversionConfig {
        processing_mode: config.processing_mode.id().to_string(),
        container: config.container.clone(),
        video_codec: if config.video_codec.is_empty() {
            default_video_codec_for_container(&config.container)
        } else {
            config.video_codec.clone()
        },
        video_bitrate_mode: non_empty_or(&config.video_bitrate_mode, DEFAULT_VIDEO_BITRATE_MODE),
        video_bitrate: non_empty_or(&config.video_bitrate, DEFAULT_VIDEO_BITRATE),
        audio_codec: config.audio_codec.clone(),
        audio_bitrate: non_empty_or(&config.audio_bitrate, DEFAULT_AUDIO_BITRATE),
        audio_bitrate_mode: non_empty_or(&config.audio_bitrate_mode, DEFAULT_AUDIO_BITRATE_MODE),
        audio_quality: non_empty_or(&config.audio_quality, DEFAULT_AUDIO_QUALITY),
        audio_channels: non_empty_or(&config.audio_channels, DEFAULT_AUDIO_CHANNELS),
        audio_volume: f64::from(config.audio_volume.min(200)),
        audio_normalize: config.audio_normalize,
        video_filters: core_video_filters_from_gpui(&config.video_filters),
        audio_filters: core_audio_filters_from_gpui(&config.audio_filters),
        selected_audio_tracks: config.selected_audio_tracks.clone(),
        selected_subtitle_tracks: config.selected_subtitle_tracks.clone(),
        subtitle_burn_path: config.subtitle_burn_path.clone(),
        subtitle_font_name: config.subtitle_font_name.clone(),
        subtitle_font_size: config.subtitle_font_size.clone(),
        subtitle_font_color: config.subtitle_font_color.clone(),
        subtitle_outline_color: config.subtitle_outline_color.clone(),
        subtitle_position: config.subtitle_position.clone(),
        resolution: non_empty_or(&config.resolution, DEFAULT_RESOLUTION),
        custom_width: config.custom_width.clone(),
        custom_height: config.custom_height.clone(),
        scaling_algorithm: non_empty_or(&config.scaling_algorithm, DEFAULT_SCALING_ALGORITHM),
        fps: non_empty_or(&config.fps, DEFAULT_FPS),
        crf: config.crf.min(51),
        quality: config.quality.clamp(1, 100),
        preset: non_empty_or(&config.preset, DEFAULT_PRESET),
        start_time: config.start_time.clone(),
        end_time: config.end_time.clone(),
        metadata: core_metadata_from_gpui(&config.metadata),
        rotation: config.rotation.clone(),
        flip_horizontal: config.flip_horizontal,
        flip_vertical: config.flip_vertical,
        crop: config.crop.as_ref().map(core_crop_from_gpui),
        overlay: config.overlay.as_ref().map(core_overlay_from_gpui),
        nvenc_spatial_aq: config.nvenc_spatial_aq,
        nvenc_temporal_aq: config.nvenc_temporal_aq,
        videotoolbox_allow_sw: config.videotoolbox_allow_sw,
        hw_decode: config.hw_decode,
        pixel_format: non_empty_or(&config.pixel_format, DEFAULT_PIXEL_FORMAT),
        image_jpeg_quality: config.image_jpeg_quality.clamp(1, 100),
        image_jpeg_huffman: config.image_jpeg_huffman.clone(),
        image_webp_lossless: config.image_webp_lossless,
        image_webp_quality: config.image_webp_quality.min(100),
        image_webp_compression: config.image_webp_compression.min(6),
        image_webp_preset: config.image_webp_preset.clone(),
        image_png_compression: config.image_png_compression.min(9),
        image_png_prediction: config.image_png_prediction.clone(),
        image_tiff_compression: config.image_tiff_compression.clone(),
        gif_colors: config.gif_colors.clamp(2, DEFAULT_GIF_COLORS),
        gif_dither: non_empty_or(&config.gif_dither, DEFAULT_GIF_DITHER),
        gif_loop: config.gif_loop,
    }
}

#[must_use]
pub fn core_video_filters_from_gpui(filters: &GpuiVideoFiltersConfig) -> CoreVideoFiltersConfig {
    CoreVideoFiltersConfig {
        color: core_video_color_filters_from_gpui(&filters.color),
        hue: core_filter_value_from_gpui(filters.hue),
        temperature: core_filter_value_from_gpui(filters.temperature),
        sharpen: core_filter_value_from_gpui(filters.sharpen),
        gaussian_blur: core_filter_value_from_gpui(filters.gaussian_blur),
        denoise_enabled: filters.denoise_enabled,
        denoise_strength: core_filter_strength_from_gpui(filters.denoise_strength),
        deband: core_filter_value_from_gpui(filters.deband),
        vignette: core_filter_value_from_gpui(filters.vignette),
        grayscale: filters.grayscale,
        deinterlace: core_deinterlace_mode_from_gpui(filters.deinterlace),
    }
}

fn core_video_color_filters_from_gpui(
    filters: &GpuiVideoColorFiltersConfig,
) -> CoreVideoColorFiltersConfig {
    CoreVideoColorFiltersConfig {
        brightness: core_filter_value_from_gpui(filters.brightness),
        contrast: core_filter_value_from_gpui(filters.contrast),
        saturation: core_filter_value_from_gpui(filters.saturation),
        gamma: core_filter_value_from_gpui(filters.gamma),
    }
}

#[must_use]
pub fn core_audio_filters_from_gpui(filters: &GpuiAudioFiltersConfig) -> CoreAudioFiltersConfig {
    CoreAudioFiltersConfig {
        compressor_enabled: filters.compressor_enabled,
        compressor_strength: core_filter_strength_from_gpui(filters.compressor_strength),
        limiter: core_filter_value_from_gpui(filters.limiter),
        bass: core_filter_value_from_gpui(filters.bass),
        treble: core_filter_value_from_gpui(filters.treble),
        high_pass: core_filter_value_from_gpui(filters.high_pass),
        low_pass: core_filter_value_from_gpui(filters.low_pass),
        noise_reduction: core_filter_value_from_gpui(filters.noise_reduction),
        de_esser: core_filter_value_from_gpui(filters.de_esser),
        stereo_width: core_filter_value_from_gpui(filters.stereo_width),
    }
}

fn core_filter_value_from_gpui<T>(value: GpuiFilterValue<T>) -> CoreFilterValue<T> {
    CoreFilterValue {
        enabled: value.enabled,
        value: value.value,
    }
}

const fn core_filter_strength_from_gpui(strength: GpuiFilterStrength) -> CoreFilterStrength {
    match strength {
        GpuiFilterStrength::Low => CoreFilterStrength::Low,
        GpuiFilterStrength::Medium => CoreFilterStrength::Medium,
        GpuiFilterStrength::High => CoreFilterStrength::High,
    }
}

const fn core_deinterlace_mode_from_gpui(mode: GpuiDeinterlaceMode) -> CoreDeinterlaceMode {
    match mode {
        GpuiDeinterlaceMode::Off => CoreDeinterlaceMode::Off,
        GpuiDeinterlaceMode::Auto => CoreDeinterlaceMode::Auto,
        GpuiDeinterlaceMode::On => CoreDeinterlaceMode::On,
    }
}

fn non_empty_or(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn core_metadata_from_gpui(metadata: &GpuiMetadataConfig) -> CoreMetadataConfig {
    CoreMetadataConfig {
        mode: match metadata.mode {
            GpuiMetadataMode::Preserve => CoreMetadataMode::Preserve,
            GpuiMetadataMode::Clean => CoreMetadataMode::Clean,
            GpuiMetadataMode::Replace => CoreMetadataMode::Replace,
        },
        title: metadata.title.clone(),
        artist: metadata.artist.clone(),
        album: metadata.album.clone(),
        genre: metadata.genre.clone(),
        date: metadata.date.clone(),
        comment: metadata.comment.clone(),
    }
}

fn default_video_codec_for_container(container: &str) -> String {
    if media_rules::is_gif_container(container) {
        return "gif".to_string();
    }

    media_rules::video_codec_fallback_order()
        .iter()
        .find(|codec| media_rules::is_video_codec_allowed(container, codec))
        .cloned()
        .or_else(|| {
            media_rules::video_codecs_for_container(container)
                .and_then(|codecs| codecs.first().cloned())
        })
        .unwrap_or_else(|| DEFAULT_VIDEO_CODEC.to_string())
}

fn core_crop_from_gpui(crop: &CropSettings) -> CropConfig {
    CropConfig {
        enabled: crop.enabled,
        x: f64::from(crop.x),
        y: f64::from(crop.y),
        width: f64::from(crop.width),
        height: f64::from(crop.height),
        source_width: crop.source_width.map(f64::from),
        source_height: crop.source_height.map(f64::from),
        aspect_ratio: crop.aspect_ratio.clone(),
    }
}

fn core_overlay_from_gpui(overlay: &OverlaySettings) -> OverlayConfig {
    OverlayConfig {
        enabled: overlay.enabled,
        path: overlay.path.clone(),
        x: overlay.x,
        y: overlay.y,
        width: overlay.width,
        opacity: overlay.opacity,
        anchor: overlay.anchor.clone(),
    }
}
