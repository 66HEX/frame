use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_CONCURRENCY: usize = 2;
pub const VOLUME_EPSILON: f64 = 0.01;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    pub index: u32,
    pub codec: String,
    pub channels: String,
    pub language: Option<String>,
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<f64>,
    pub sample_rate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMetadata {
    pub duration: Option<String>,
    pub bitrate: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_bitrate_kbps: Option<f64>,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    #[serde(default)]
    pub tags: Option<FfprobeTags>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_primaries: Option<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConversionConfig {
    pub container: String,
    pub video_codec: String,
    pub video_bitrate_mode: String,
    pub video_bitrate: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
    pub audio_channels: String,
    #[serde(default = "default_audio_volume")]
    pub audio_volume: f64,
    #[serde(default)]
    pub audio_normalize: bool,
    pub selected_audio_tracks: Vec<u32>,
    pub selected_subtitle_tracks: Vec<u32>,
    pub subtitle_burn_path: Option<String>,
    pub resolution: String,
    pub custom_width: Option<String>,
    pub custom_height: Option<String>,
    pub scaling_algorithm: String,
    pub fps: String,
    pub crf: u8,
    #[serde(default = "default_quality")]
    pub quality: u32,
    pub preset: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    #[serde(default)]
    pub metadata: MetadataConfig,
    #[serde(default = "default_rotation")]
    pub rotation: String,
    #[serde(default)]
    pub flip_horizontal: bool,
    #[serde(default)]
    pub flip_vertical: bool,
    #[serde(default)]
    pub ml_upscale: Option<String>,
    #[serde(default)]
    pub crop: Option<CropConfig>,
    #[serde(default)]
    pub nvenc_spatial_aq: bool,
    #[serde(default)]
    pub nvenc_temporal_aq: bool,
    #[serde(default)]
    pub videotoolbox_allow_sw: bool,
    #[serde(default = "default_hw_decode")]
    pub hw_decode: bool,
}

fn default_rotation() -> String {
    "0".to_string()
}

fn default_quality() -> u32 {
    50
}

fn default_audio_volume() -> f64 {
    100.0
}

fn default_hw_decode() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CropConfig {
    pub enabled: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataConfig {
    pub mode: MetadataMode,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MetadataMode {
    #[default]
    Preserve,
    Clean,
    Replace,
}

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
}

#[derive(Clone, Serialize)]
pub struct StartedPayload {
    pub id: String,
}

#[derive(Clone, Serialize)]
pub struct CompletedPayload {
    pub id: String,
    pub output_path: String,
}

#[derive(Clone, Serialize)]
pub struct ErrorPayload {
    pub id: String,
    pub error: String,
}

#[derive(Clone, Serialize)]
pub struct LogPayload {
    pub id: String,
    pub line: String,
}

#[derive(Deserialize)]
pub struct FfprobeOutput {
    pub streams: Vec<FfprobeStream>,
    pub format: FfprobeFormat,
}

#[derive(Deserialize)]
pub struct FfprobeStream {
    pub index: u32,
    pub codec_type: String,
    pub codec_name: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub channels: Option<i32>,
    pub bit_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    #[allow(dead_code)]
    pub channel_layout: Option<String>,
    pub tags: Option<FfprobeTags>,
    pub pix_fmt: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_primaries: Option<String>,
    pub profile: Option<String>,
    pub sample_rate: Option<String>,
}

#[derive(Deserialize)]
pub struct FfprobeFormat {
    pub duration: Option<String>,
    pub bit_rate: Option<String>,
    pub tags: Option<FfprobeTags>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FfprobeTags {
    #[serde(alias = "TITLE")]
    pub title: Option<String>,
    #[serde(alias = "ARTIST")]
    pub artist: Option<String>,
    #[serde(alias = "ALBUM")]
    pub album: Option<String>,
    #[serde(alias = "GENRE")]
    pub genre: Option<String>,
    #[serde(alias = "DATE")]
    pub date: Option<String>,
    #[serde(rename = "creation_time")]
    pub creation_time: Option<String>,
    pub language: Option<String>,
    #[serde(alias = "COMMENT")]
    pub comment: Option<String>,
    #[serde(rename = "DESCRIPTION")]
    pub description_upper: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConversionTask {
    pub id: String,
    pub file_path: String,
    pub output_name: Option<String>,
    pub config: ConversionConfig,
}
