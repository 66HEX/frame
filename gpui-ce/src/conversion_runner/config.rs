use frame_core::{
    media_rules,
    types::{ConversionConfig as CoreConversionConfig, ConversionTask, CropConfig, MetadataConfig},
};

use crate::{
    file_queue::FileItem,
    settings::{
        ConversionConfig as GpuiConversionConfig, CropSettings, DEFAULT_AUDIO_BITRATE,
        DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS, DEFAULT_AUDIO_QUALITY,
    },
};

const DEFAULT_VIDEO_BITRATE: &str = "5000";
const DEFAULT_CRF: u8 = 23;
const DEFAULT_QUALITY: u32 = 50;
const DEFAULT_PRESET: &str = "medium";

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
        video_codec: default_video_codec_for_container(&config.container),
        video_bitrate_mode: "crf".to_string(),
        video_bitrate: DEFAULT_VIDEO_BITRATE.to_string(),
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
        subtitle_burn_path: None,
        subtitle_font_name: None,
        subtitle_font_size: None,
        subtitle_font_color: None,
        subtitle_outline_color: None,
        subtitle_position: None,
        resolution: "original".to_string(),
        custom_width: None,
        custom_height: None,
        scaling_algorithm: "bicubic".to_string(),
        fps: "original".to_string(),
        crf: DEFAULT_CRF,
        quality: DEFAULT_QUALITY,
        preset: DEFAULT_PRESET.to_string(),
        start_time: config.start_time.clone(),
        end_time: config.end_time.clone(),
        metadata: MetadataConfig::default(),
        rotation: config.rotation.clone(),
        flip_horizontal: config.flip_horizontal,
        flip_vertical: config.flip_vertical,
        ml_upscale: None,
        crop: config.crop.as_ref().map(core_crop_from_gpui),
        overlay: None,
        nvenc_spatial_aq: false,
        nvenc_temporal_aq: false,
        videotoolbox_allow_sw: false,
        hw_decode: false,
        pixel_format: "auto".to_string(),
        gif_colors: 256,
        gif_dither: "sierra2_4a".to_string(),
        gif_loop: 0,
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
        .unwrap_or_else(|| "libx264".to_string())
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
