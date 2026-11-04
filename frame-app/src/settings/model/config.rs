use crate::settings::model::defaults::{
    DEFAULT_AUDIO_BITRATE, DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS,
    DEFAULT_AUDIO_QUALITY, DEFAULT_AUDIO_VOLUME, DEFAULT_CRF, DEFAULT_FPS, DEFAULT_GIF_COLORS,
    DEFAULT_GIF_DITHER, DEFAULT_GIF_LOOP, DEFAULT_IMAGE_JPEG_HUFFMAN, DEFAULT_IMAGE_JPEG_QUALITY,
    DEFAULT_IMAGE_PNG_COMPRESSION, DEFAULT_IMAGE_PNG_PREDICTION, DEFAULT_IMAGE_TIFF_COMPRESSION,
    DEFAULT_IMAGE_WEBP_COMPRESSION, DEFAULT_IMAGE_WEBP_PRESET, DEFAULT_IMAGE_WEBP_QUALITY,
    DEFAULT_PIXEL_FORMAT, DEFAULT_PRESET, DEFAULT_QUALITY, DEFAULT_RESOLUTION,
    DEFAULT_SCALING_ALGORITHM, DEFAULT_VIDEO_BITRATE, DEFAULT_VIDEO_BITRATE_MODE,
    DEFAULT_VIDEO_CODEC,
};
use crate::settings::model::filters::{AudioFiltersConfig, VideoFiltersConfig};
use crate::settings::model::metadata::MetadataConfig;
use crate::settings::model::processing::ProcessingMode;
use crate::settings::model::subtitles::ExternalSubtitleTrack;
use frame_core::media_rules;
use serde::{Deserialize, Serialize};

#[expect(
    clippy::struct_excessive_bools,
    reason = "ConversionConfig is the persisted settings model and keeps booleans as stable fields."
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConversionConfig {
    pub processing_mode: ProcessingMode,
    pub container: String,
    pub video_codec: String,
    pub video_bitrate_mode: String,
    pub video_bitrate: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
    pub audio_bitrate_mode: String,
    pub audio_quality: String,
    pub audio_channels: String,
    pub audio_volume: u32,
    pub audio_normalize: bool,
    pub video_filters: VideoFiltersConfig,
    pub audio_filters: AudioFiltersConfig,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub metadata: MetadataConfig,
    pub subtitle_burn_path: Option<String>,
    pub subtitle_font_name: Option<String>,
    pub subtitle_font_size: Option<String>,
    pub subtitle_font_color: Option<String>,
    pub subtitle_outline_color: Option<String>,
    pub subtitle_position: Option<String>,
    pub rotation: String,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub crop: Option<CropSettings>,
    pub overlay: Option<OverlaySettings>,
    pub selected_audio_tracks: Vec<u32>,
    pub selected_subtitle_tracks: Vec<u32>,
    pub external_subtitle_tracks: Vec<ExternalSubtitleTrack>,
    pub resolution: String,
    pub custom_width: Option<String>,
    pub custom_height: Option<String>,
    pub scaling_algorithm: String,
    pub fps: String,
    pub crf: u8,
    pub quality: u32,
    pub preset: String,
    pub pixel_format: String,
    pub image_jpeg_quality: u32,
    pub image_jpeg_huffman: String,
    pub image_webp_lossless: bool,
    pub image_webp_quality: u32,
    pub image_webp_compression: u32,
    pub image_webp_preset: String,
    pub image_png_compression: u32,
    pub image_png_prediction: String,
    pub image_tiff_compression: String,
    pub gif_colors: u16,
    pub gif_dither: String,
    pub gif_loop: u16,
    pub nvenc_spatial_aq: bool,
    pub nvenc_temporal_aq: bool,
    pub videotoolbox_allow_sw: bool,
    pub hw_decode: bool,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            processing_mode: ProcessingMode::Reencode,
            container: "mp4".to_string(),
            video_codec: DEFAULT_VIDEO_CODEC.to_string(),
            video_bitrate_mode: DEFAULT_VIDEO_BITRATE_MODE.to_string(),
            video_bitrate: DEFAULT_VIDEO_BITRATE.to_string(),
            audio_codec: media_rules::default_audio_codec_for_container("mp4").to_string(),
            audio_bitrate: DEFAULT_AUDIO_BITRATE.to_string(),
            audio_bitrate_mode: DEFAULT_AUDIO_BITRATE_MODE.to_string(),
            audio_quality: DEFAULT_AUDIO_QUALITY.to_string(),
            audio_channels: DEFAULT_AUDIO_CHANNELS.to_string(),
            audio_volume: DEFAULT_AUDIO_VOLUME,
            audio_normalize: false,
            video_filters: VideoFiltersConfig::default(),
            audio_filters: AudioFiltersConfig::default(),
            start_time: None,
            end_time: None,
            metadata: MetadataConfig::default(),
            subtitle_burn_path: None,
            subtitle_font_name: None,
            subtitle_font_size: None,
            subtitle_font_color: None,
            subtitle_outline_color: None,
            subtitle_position: None,
            rotation: "0".to_string(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
            overlay: None,
            selected_audio_tracks: Vec::new(),
            selected_subtitle_tracks: Vec::new(),
            external_subtitle_tracks: Vec::new(),
            resolution: DEFAULT_RESOLUTION.to_string(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: DEFAULT_SCALING_ALGORITHM.to_string(),
            fps: DEFAULT_FPS.to_string(),
            crf: DEFAULT_CRF,
            quality: DEFAULT_QUALITY,
            preset: DEFAULT_PRESET.to_string(),
            pixel_format: DEFAULT_PIXEL_FORMAT.to_string(),
            image_jpeg_quality: DEFAULT_IMAGE_JPEG_QUALITY,
            image_jpeg_huffman: DEFAULT_IMAGE_JPEG_HUFFMAN.to_string(),
            image_webp_lossless: false,
            image_webp_quality: DEFAULT_IMAGE_WEBP_QUALITY,
            image_webp_compression: DEFAULT_IMAGE_WEBP_COMPRESSION,
            image_webp_preset: DEFAULT_IMAGE_WEBP_PRESET.to_string(),
            image_png_compression: DEFAULT_IMAGE_PNG_COMPRESSION,
            image_png_prediction: DEFAULT_IMAGE_PNG_PREDICTION.to_string(),
            image_tiff_compression: DEFAULT_IMAGE_TIFF_COMPRESSION.to_string(),
            gif_colors: DEFAULT_GIF_COLORS,
            gif_dither: DEFAULT_GIF_DITHER.to_string(),
            gif_loop: DEFAULT_GIF_LOOP,
            nvenc_spatial_aq: false,
            nvenc_temporal_aq: false,
            videotoolbox_allow_sw: false,
            hw_decode: false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CropSettings {
    pub enabled: bool,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub aspect_ratio: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    pub enabled: bool,
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub opacity: f64,
    pub anchor: String,
}

impl Eq for OverlaySettings {}
