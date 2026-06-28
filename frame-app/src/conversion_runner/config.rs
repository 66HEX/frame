use frame_core::{
    media_rules,
    types::{
        ConversionConfig as CoreConversionConfig, ConversionTask, CropConfig,
        MetadataConfig as CoreMetadataConfig, MetadataMode as CoreMetadataMode, OverlayConfig,
    },
};

use crate::{
    file_queue::FileItem,
    settings::{
        ConversionConfig as GpuiConversionConfig, CropSettings, DEFAULT_AUDIO_BITRATE,
        DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS, DEFAULT_AUDIO_QUALITY, DEFAULT_FPS,
        DEFAULT_GIF_COLORS, DEFAULT_GIF_DITHER, DEFAULT_PIXEL_FORMAT, DEFAULT_PRESET,
        DEFAULT_RESOLUTION, DEFAULT_SCALING_ALGORITHM, DEFAULT_VIDEO_BITRATE,
        DEFAULT_VIDEO_BITRATE_MODE, DEFAULT_VIDEO_CODEC, MetadataConfig as GpuiMetadataConfig,
        MetadataMode as GpuiMetadataMode, OverlaySettings,
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
        video_bitrate_mode: if config.video_bitrate_mode.is_empty() {
            DEFAULT_VIDEO_BITRATE_MODE.to_string()
        } else {
            config.video_bitrate_mode.clone()
        },
        video_bitrate: if config.video_bitrate.is_empty() {
            DEFAULT_VIDEO_BITRATE.to_string()
        } else {
            config.video_bitrate.clone()
        },
        audio_codec: config.audio_codec.clone(),
        audio_bitrate: if config.audio_bitrate.is_empty() {
            DEFAULT_AUDIO_BITRATE.to_string()
        } else {
            config.audio_bitrate.clone()
        },
        audio_bitrate_mode: if config.audio_bitrate_mode.is_empty() {
            DEFAULT_AUDIO_BITRATE_MODE.to_string()
        } else {
            config.audio_bitrate_mode.clone()
        },
        audio_quality: if config.audio_quality.is_empty() {
            DEFAULT_AUDIO_QUALITY.to_string()
        } else {
            config.audio_quality.clone()
        },
        audio_channels: if config.audio_channels.is_empty() {
            DEFAULT_AUDIO_CHANNELS.to_string()
        } else {
            config.audio_channels.clone()
        },
        audio_volume: f64::from(config.audio_volume.min(200)),
        audio_normalize: config.audio_normalize,
        selected_audio_tracks: config.selected_audio_tracks.clone(),
        selected_subtitle_tracks: config.selected_subtitle_tracks.clone(),
        subtitle_burn_path: config.subtitle_burn_path.clone(),
        subtitle_font_name: config.subtitle_font_name.clone(),
        subtitle_font_size: config.subtitle_font_size.clone(),
        subtitle_font_color: config.subtitle_font_color.clone(),
        subtitle_outline_color: config.subtitle_outline_color.clone(),
        subtitle_position: config.subtitle_position.clone(),
        resolution: if config.resolution.is_empty() {
            DEFAULT_RESOLUTION.to_string()
        } else {
            config.resolution.clone()
        },
        custom_width: config.custom_width.clone(),
        custom_height: config.custom_height.clone(),
        scaling_algorithm: if config.scaling_algorithm.is_empty() {
            DEFAULT_SCALING_ALGORITHM.to_string()
        } else {
            config.scaling_algorithm.clone()
        },
        fps: if config.fps.is_empty() {
            DEFAULT_FPS.to_string()
        } else {
            config.fps.clone()
        },
        crf: config.crf.min(51),
        quality: config.quality.clamp(1, 100),
        preset: if config.preset.is_empty() {
            DEFAULT_PRESET.to_string()
        } else {
            config.preset.clone()
        },
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
        pixel_format: if config.pixel_format.is_empty() {
            DEFAULT_PIXEL_FORMAT.to_string()
        } else {
            config.pixel_format.clone()
        },
        gif_colors: config.gif_colors.clamp(2, DEFAULT_GIF_COLORS),
        gif_dither: if config.gif_dither.is_empty() {
            DEFAULT_GIF_DITHER.to_string()
        } else {
            config.gif_dither.clone()
        },
        gif_loop: config.gif_loop,
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
