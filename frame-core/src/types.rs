//! Shared conversion, probing, and event payload types.

use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_CONCURRENCY: usize = 2;
pub const VOLUME_EPSILON: f64 = 0.01;

/// A persisted filter parameter that preserves its draft value while disabled.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FilterValue<T> {
    /// Whether the filter parameter participates in the generated `FFmpeg` chain.
    pub enabled: bool,
    /// The UI-domain value. Core maps it to `FFmpeg` units during chain building.
    pub value: T,
}

impl<T: Default> Default for FilterValue<T> {
    fn default() -> Self {
        Self {
            enabled: false,
            value: T::default(),
        }
    }
}

/// Shared low/medium/high strength selector for choice-based filters.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FilterStrength {
    /// Lowest processing amount.
    Low,
    /// Balanced default processing amount.
    #[default]
    Medium,
    /// Highest processing amount.
    High,
}

/// Deinterlace behavior for video sources.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DeinterlaceMode {
    /// Do not emit a deinterlace filter.
    #[default]
    Off,
    /// Deinterlace frames marked as interlaced.
    Auto,
    /// Deinterlace all frames.
    On,
}

/// Color adjustment filters that are emitted as one combined `FFmpeg` `eq` filter.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoColorFiltersConfig {
    /// Brightness percentage in the UI range -100..100.
    pub brightness: FilterValue<i32>,
    /// Contrast percentage in the UI range 0..200.
    pub contrast: FilterValue<u32>,
    /// Saturation percentage in the UI range 0..300.
    pub saturation: FilterValue<u32>,
    /// Gamma percentage in the UI range 10..300.
    pub gamma: FilterValue<u32>,
}

impl Default for VideoColorFiltersConfig {
    fn default() -> Self {
        Self {
            brightness: FilterValue {
                enabled: false,
                value: 0,
            },
            contrast: FilterValue {
                enabled: false,
                value: 100,
            },
            saturation: FilterValue {
                enabled: false,
                value: 100,
            },
            gamma: FilterValue {
                enabled: false,
                value: 100,
            },
        }
    }
}

/// Video and image filter configuration in stable UI units.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoFiltersConfig {
    /// Combined brightness/contrast/saturation/gamma adjustment group.
    pub color: VideoColorFiltersConfig,
    /// Hue rotation in degrees, -180..180.
    pub hue: FilterValue<i32>,
    /// Color temperature in Kelvin, 2000..12000.
    pub temperature: FilterValue<u32>,
    /// Sharpen amount, 0..100.
    pub sharpen: FilterValue<u32>,
    /// Gaussian blur amount, 0..100.
    pub gaussian_blur: FilterValue<u32>,
    /// Enables `hqdn3d` with a fixed strength preset.
    pub denoise_enabled: bool,
    /// Denoise strength preset.
    pub denoise_strength: FilterStrength,
    /// Deband amount, 0..100.
    pub deband: FilterValue<u32>,
    /// Vignette amount, 0..100.
    pub vignette: FilterValue<u32>,
    /// Enables grayscale conversion.
    pub grayscale: bool,
    /// Deinterlace mode for video sources.
    pub deinterlace: DeinterlaceMode,
}

impl Default for VideoFiltersConfig {
    fn default() -> Self {
        Self {
            color: VideoColorFiltersConfig::default(),
            hue: FilterValue {
                enabled: false,
                value: 0,
            },
            temperature: FilterValue {
                enabled: false,
                value: 6500,
            },
            sharpen: FilterValue {
                enabled: false,
                value: 25,
            },
            gaussian_blur: FilterValue {
                enabled: false,
                value: 20,
            },
            denoise_enabled: false,
            denoise_strength: FilterStrength::Medium,
            deband: FilterValue {
                enabled: false,
                value: 25,
            },
            vignette: FilterValue {
                enabled: false,
                value: 35,
            },
            grayscale: false,
            deinterlace: DeinterlaceMode::Off,
        }
    }
}

/// Audio filter configuration in stable UI units.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AudioFiltersConfig {
    /// Enables `acompressor` with a fixed strength preset.
    pub compressor_enabled: bool,
    /// Compressor strength preset.
    pub compressor_strength: FilterStrength,
    /// Limiter ceiling in dB, -12..0.
    pub limiter: FilterValue<i32>,
    /// Bass gain in dB, -20..20.
    pub bass: FilterValue<i32>,
    /// Treble gain in dB, -20..20.
    pub treble: FilterValue<i32>,
    /// High-pass cutoff in Hz, 20..2000.
    pub high_pass: FilterValue<u32>,
    /// Low-pass cutoff in Hz, 1000..20000.
    pub low_pass: FilterValue<u32>,
    /// FFT noise reduction amount in dB, 1..30.
    pub noise_reduction: FilterValue<u32>,
    /// De-esser intensity, 0..100.
    pub de_esser: FilterValue<u32>,
    /// Stereo side width, 0..200.
    pub stereo_width: FilterValue<u32>,
}

impl Default for AudioFiltersConfig {
    fn default() -> Self {
        Self {
            compressor_enabled: false,
            compressor_strength: FilterStrength::Medium,
            limiter: FilterValue {
                enabled: false,
                value: -1,
            },
            bass: FilterValue {
                enabled: false,
                value: 0,
            },
            treble: FilterValue {
                enabled: false,
                value: 0,
            },
            high_pass: FilterValue {
                enabled: false,
                value: 80,
            },
            low_pass: FilterValue {
                enabled: false,
                value: 16_000,
            },
            noise_reduction: FilterValue {
                enabled: false,
                value: 12,
            },
            de_esser: FilterValue {
                enabled: false,
                value: 35,
            },
            stereo_width: FilterValue {
                enabled: false,
                value: 100,
            },
        }
    }
}

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
    #[serde(default = "default_media_kind")]
    pub media_kind: String,
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "conversion config mirrors UI toggles and serialized API fields"
)]
pub struct ConversionConfig {
    #[serde(default = "default_processing_mode")]
    pub processing_mode: String,
    pub container: String,
    pub video_codec: String,
    pub video_bitrate_mode: String,
    pub video_bitrate: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
    #[serde(default = "default_audio_bitrate_mode")]
    pub audio_bitrate_mode: String,
    #[serde(default = "default_audio_quality")]
    pub audio_quality: String,
    pub audio_channels: String,
    #[serde(default = "default_audio_volume")]
    pub audio_volume: f64,
    #[serde(default)]
    pub audio_normalize: bool,
    #[serde(default)]
    pub video_filters: VideoFiltersConfig,
    #[serde(default)]
    pub audio_filters: AudioFiltersConfig,
    pub selected_audio_tracks: Vec<u32>,
    pub selected_subtitle_tracks: Vec<u32>,
    pub subtitle_burn_path: Option<String>,
    #[serde(default)]
    pub subtitle_font_name: Option<String>,
    #[serde(default)]
    pub subtitle_font_size: Option<String>,
    #[serde(default)]
    pub subtitle_font_color: Option<String>,
    #[serde(default)]
    pub subtitle_outline_color: Option<String>,
    #[serde(default)]
    pub subtitle_position: Option<String>,
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
    pub crop: Option<CropConfig>,
    #[serde(default)]
    pub overlay: Option<OverlayConfig>,
    #[serde(default)]
    pub nvenc_spatial_aq: bool,
    #[serde(default)]
    pub nvenc_temporal_aq: bool,
    #[serde(default)]
    pub videotoolbox_allow_sw: bool,
    #[serde(default = "default_hw_decode")]
    pub hw_decode: bool,
    #[serde(default = "default_pixel_format")]
    pub pixel_format: String,
    #[serde(default = "default_image_jpeg_quality")]
    pub image_jpeg_quality: u32,
    #[serde(default = "default_image_jpeg_huffman")]
    pub image_jpeg_huffman: String,
    #[serde(default)]
    pub image_webp_lossless: bool,
    #[serde(default = "default_image_webp_quality")]
    pub image_webp_quality: u32,
    #[serde(default = "default_image_webp_compression")]
    pub image_webp_compression: u32,
    #[serde(default = "default_image_webp_preset")]
    pub image_webp_preset: String,
    #[serde(default = "default_image_png_compression")]
    pub image_png_compression: u32,
    #[serde(default = "default_image_png_prediction")]
    pub image_png_prediction: String,
    #[serde(default = "default_image_tiff_compression")]
    pub image_tiff_compression: String,
    #[serde(default = "default_gif_colors")]
    pub gif_colors: u16,
    #[serde(default = "default_gif_dither")]
    pub gif_dither: String,
    #[serde(default = "default_gif_loop")]
    pub gif_loop: u16,
}

fn default_rotation() -> String {
    "0".to_string()
}

fn default_media_kind() -> String {
    "video".to_string()
}

fn default_processing_mode() -> String {
    "reencode".to_string()
}

const fn default_quality() -> u32 {
    50
}

const fn default_audio_volume() -> f64 {
    100.0
}

fn default_audio_bitrate_mode() -> String {
    "bitrate".to_string()
}

fn default_audio_quality() -> String {
    "4".to_string()
}

const fn default_hw_decode() -> bool {
    false
}

fn default_pixel_format() -> String {
    "auto".to_string()
}

const fn default_image_jpeg_quality() -> u32 {
    85
}

fn default_image_jpeg_huffman() -> String {
    "optimal".to_string()
}

const fn default_image_webp_quality() -> u32 {
    75
}

const fn default_image_webp_compression() -> u32 {
    4
}

fn default_image_webp_preset() -> String {
    "default".to_string()
}

const fn default_image_png_compression() -> u32 {
    9
}

fn default_image_png_prediction() -> String {
    "paeth".to_string()
}

fn default_image_tiff_compression() -> String {
    "packbits".to_string()
}

const fn default_gif_colors() -> u16 {
    256
}

fn default_gif_dither() -> String {
    "sierra2_4a".to_string()
}

const fn default_gif_loop() -> u16 {
    0
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
pub struct OverlayConfig {
    pub enabled: bool,
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub opacity: f64,
    pub anchor: String,
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

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MetadataMode {
    #[default]
    Preserve,
    Clean,
    Replace,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StartedPayload {
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CancelledPayload {
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompletedPayload {
    pub id: String,
    pub output_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ErrorPayload {
    pub id: String,
    pub error: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
    pub format_name: Option<String>,
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
    pub output_directory: String,
    pub output_name: Option<String>,
    pub config: ConversionConfig,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn minimal_config_json() -> serde_json::Value {
        json!({
            "container": "mp4",
            "videoCodec": "libx264",
            "videoBitrateMode": "crf",
            "videoBitrate": "5000",
            "audioCodec": "aac",
            "audioBitrate": "128",
            "audioChannels": "original",
            "selectedAudioTracks": [1],
            "selectedSubtitleTracks": [],
            "subtitleBurnPath": null,
            "resolution": "original",
            "customWidth": null,
            "customHeight": null,
            "scalingAlgorithm": "bicubic",
            "fps": "original",
            "crf": 23,
            "preset": "medium",
            "startTime": null,
            "endTime": null
        })
    }

    #[test]
    fn conversion_config_deserializes_legacy_boundary_defaults() {
        let config: ConversionConfig = serde_json::from_value(minimal_config_json()).unwrap();

        assert_eq!(config.processing_mode, "reencode");
        assert_eq!(config.audio_bitrate_mode, "bitrate");
        assert_eq!(config.audio_quality, "4");
        assert!((config.audio_volume - 100.0).abs() < f64::EPSILON);
        assert_eq!(config.quality, 50);
        assert_eq!(config.rotation, "0");
        assert_eq!(config.pixel_format, "auto");
        assert_eq!(config.image_jpeg_quality, 85);
        assert_eq!(config.image_jpeg_huffman, "optimal");
        assert!(!config.image_webp_lossless);
        assert_eq!(config.image_webp_quality, 75);
        assert_eq!(config.image_webp_compression, 4);
        assert_eq!(config.image_webp_preset, "default");
        assert_eq!(config.image_png_compression, 9);
        assert_eq!(config.image_png_prediction, "paeth");
        assert_eq!(config.image_tiff_compression, "packbits");
        assert_eq!(config.gif_colors, 256);
        assert_eq!(config.gif_dither, "sierra2_4a");
        assert_eq!(config.gif_loop, 0);
        assert_eq!(config.metadata.mode, MetadataMode::Preserve);
    }

    #[test]
    fn conversion_config_serializes_camel_case_fields() {
        let config: ConversionConfig = serde_json::from_value(minimal_config_json()).unwrap();
        let serialized = serde_json::to_value(config).unwrap();

        assert_eq!(serialized["processingMode"], "reencode");
        assert_eq!(serialized["audioBitrateMode"], "bitrate");
        assert_eq!(serialized["imageJpegQuality"], 85);
        assert_eq!(serialized["imageWebpPreset"], "default");
        assert_eq!(serialized["imagePngPrediction"], "paeth");
        assert_eq!(serialized["metadata"]["mode"], "preserve");
        assert!(serialized.get("processing_mode").is_none());
    }

    #[test]
    fn probe_metadata_defaults_to_video_kind() {
        let metadata: ProbeMetadata = serde_json::from_value(json!({
            "audioTracks": [],
            "subtitleTracks": []
        }))
        .unwrap();

        assert_eq!(metadata.media_kind, "video");
    }

    #[test]
    fn ffprobe_tags_accept_uppercase_aliases() {
        let tags: FfprobeTags = serde_json::from_value(json!({
            "TITLE": "Clip",
            "ARTIST": "Frame",
            "DESCRIPTION": "Demo"
        }))
        .unwrap();

        assert_eq!(tags.title.as_deref(), Some("Clip"));
        assert_eq!(tags.artist.as_deref(), Some("Frame"));
        assert_eq!(tags.description_upper.as_deref(), Some("Demo"));
    }
}
