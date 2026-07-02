use frame_core::media_rules;
use serde::{Deserialize, Serialize};

pub const DEFAULT_VIDEO_CODEC: &str = "libx264";
pub const DEFAULT_VIDEO_BITRATE_MODE: &str = "crf";
pub const DEFAULT_VIDEO_BITRATE: &str = "5000";
pub const DEFAULT_RESOLUTION: &str = "original";
pub const DEFAULT_SCALING_ALGORITHM: &str = "bicubic";
pub const DEFAULT_FPS: &str = "original";
pub const DEFAULT_CRF: u8 = 23;
pub const DEFAULT_QUALITY: u32 = 50;
pub const DEFAULT_PRESET: &str = "medium";
pub const DEFAULT_PIXEL_FORMAT: &str = "auto";
pub const DEFAULT_IMAGE_JPEG_QUALITY: u32 = 85;
pub const DEFAULT_IMAGE_JPEG_HUFFMAN: &str = "optimal";
pub const DEFAULT_IMAGE_WEBP_QUALITY: u32 = 75;
pub const DEFAULT_IMAGE_WEBP_COMPRESSION: u32 = 4;
pub const DEFAULT_IMAGE_WEBP_PRESET: &str = "default";
pub const DEFAULT_IMAGE_PNG_COMPRESSION: u32 = 9;
pub const DEFAULT_IMAGE_PNG_PREDICTION: &str = "paeth";
pub const DEFAULT_IMAGE_TIFF_COMPRESSION: &str = "packbits";
pub const DEFAULT_GIF_COLORS: u16 = 256;
pub const DEFAULT_GIF_DITHER: &str = "sierra2_4a";
pub const DEFAULT_GIF_LOOP: u16 = 0;
pub const DEFAULT_AUDIO_BITRATE: &str = "128";
pub const DEFAULT_AUDIO_BITRATE_MODE: &str = "bitrate";
pub const DEFAULT_AUDIO_QUALITY: &str = "4";
pub const DEFAULT_AUDIO_CHANNELS: &str = "original";
pub const DEFAULT_AUDIO_VOLUME: u32 = 100;
pub const DEFAULT_METADATA_MODE: MetadataMode = MetadataMode::Preserve;
pub const DEFAULT_SUBTITLE_FONT_COLOR: &str = "#ffffff";
pub const DEFAULT_SUBTITLE_OUTLINE_COLOR: &str = "#000000";
pub const DEFAULT_SUBTITLE_POSITION: SubtitlePosition = SubtitlePosition::Bottom;
pub(super) const MAX_AUDIO_VOLUME: u32 = 200;
pub(super) const MAX_GIF_LOOP: u16 = 65_535;
pub(super) const MAX_GIF_COLORS: u16 = 256;
pub(super) const MAX_IMAGE_JPEG_QUALITY: u32 = 100;
pub(super) const MAX_IMAGE_WEBP_QUALITY: u32 = 100;
pub(super) const MAX_IMAGE_WEBP_COMPRESSION: u32 = 6;
pub(super) const MAX_IMAGE_PNG_COMPRESSION: u32 = 9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsTab {
    Source,
    Output,
    Video,
    Images,
    Audio,
    Subtitles,
    Metadata,
    Presets,
}

impl SettingsTab {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Output => "Output",
            Self::Video => "Video",
            Self::Images => "Images",
            Self::Audio => "Audio",
            Self::Subtitles => "Subtitles",
            Self::Metadata => "Metadata",
            Self::Presets => "Presets",
        }
    }

    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Output => "output",
            Self::Video => "video",
            Self::Images => "images",
            Self::Audio => "audio",
            Self::Subtitles => "subtitles",
            Self::Metadata => "metadata",
            Self::Presets => "presets",
        }
    }
}

pub const ALL_SETTINGS_TABS: [SettingsTab; 8] = [
    SettingsTab::Source,
    SettingsTab::Output,
    SettingsTab::Video,
    SettingsTab::Images,
    SettingsTab::Audio,
    SettingsTab::Subtitles,
    SettingsTab::Metadata,
    SettingsTab::Presets,
];

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessingMode {
    #[default]
    Reencode,
    Copy,
}

impl ProcessingMode {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Reencode => "reencode",
            Self::Copy => "copy",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reencode => "Re-encode",
            Self::Copy => "Cut / Stream Copy",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Reencode => {
                "Decodes and encodes media so all filters and codec settings are available."
            }
            Self::Copy => {
                "Fast trim/remux without re-encoding. Cut precision depends on keyframes."
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MetadataMode {
    #[default]
    Preserve,
    Clean,
    Replace,
}

impl MetadataMode {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::Clean => "clean",
            Self::Replace => "replace",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Preserve => "Preserve",
            Self::Clean => "Clean",
            Self::Replace => "Replace",
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Preserve => {
                "Keeps original metadata. Values entered below will overwrite specific fields."
            }
            Self::Clean => "Removes all metadata tags from the output file.",
            Self::Replace => "Removes original metadata and adds only the values entered below.",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "preserve" => Some(Self::Preserve),
            "clean" => Some(Self::Clean),
            "replace" => Some(Self::Replace),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetadataConfig {
    pub mode: MetadataMode,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            mode: DEFAULT_METADATA_MODE,
            title: None,
            artist: None,
            album: None,
            genre: None,
            date: None,
            comment: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataField {
    Title,
    Artist,
    Album,
    Genre,
    Date,
    Comment,
}

impl MetadataField {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Genre => "genre",
            Self::Date => "date",
            Self::Comment => "comment",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::Genre => "Genre",
            Self::Date => "Date / Year",
            Self::Comment => "Comment",
        }
    }

    #[must_use]
    pub const fn visible_for_image(self) -> bool {
        !matches!(self, Self::Album | Self::Genre)
    }
}

pub const METADATA_MODES: [MetadataMode; 3] = [
    MetadataMode::Preserve,
    MetadataMode::Clean,
    MetadataMode::Replace,
];

pub const METADATA_FIELDS: [MetadataField; 6] = [
    MetadataField::Title,
    MetadataField::Artist,
    MetadataField::Album,
    MetadataField::Genre,
    MetadataField::Date,
    MetadataField::Comment,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubtitlePosition {
    Bottom,
    Middle,
    Top,
}

impl SubtitlePosition {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Bottom => "bottom",
            Self::Middle => "middle",
            Self::Top => "top",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bottom => "Bottom",
            Self::Middle => "Middle",
            Self::Top => "Top",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "bottom" => Some(Self::Bottom),
            "middle" => Some(Self::Middle),
            "top" => Some(Self::Top),
            _ => None,
        }
    }
}

pub const SUBTITLE_POSITIONS: [SubtitlePosition; 3] = [
    SubtitlePosition::Bottom,
    SubtitlePosition::Middle,
    SubtitlePosition::Top,
];

pub const SUBTITLE_FONT_SIZES: [&str; 14] = [
    "8", "10", "12", "14", "16", "18", "20", "22", "24", "28", "32", "36", "42", "48",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataModeOption {
    pub mode: MetadataMode,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataFieldOption {
    pub field: MetadataField,
    pub id: &'static str,
    pub label: &'static str,
    pub value: String,
    pub placeholder: String,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtitleFontOption {
    pub name: String,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubtitleFontSizeOption {
    pub size: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubtitlePositionOption {
    pub position: SubtitlePosition,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtitleTrackOption {
    pub index: u32,
    pub index_label: String,
    pub codec: String,
    pub detail: String,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDefinition {
    pub id: String,
    pub name: String,
    pub config: ConversionConfig,
    #[serde(default)]
    pub built_in: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetOption {
    pub preset: PresetDefinition,
    pub is_selected: bool,
    pub is_compatible: bool,
    pub status: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresetNoticeTone {
    Success,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetNotice {
    pub text: String,
    pub tone: PresetNoticeTone,
}

impl PresetDefinition {
    #[must_use]
    pub fn built_in(id: &str, name: &str, config: ConversionConfig) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            config,
            built_in: true,
        }
    }

    #[must_use]
    pub const fn custom(id: String, name: String, config: ConversionConfig) -> Self {
        Self {
            id,
            name,
            config,
            built_in: false,
        }
    }
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputModeOption {
    pub mode: ProcessingMode,
    pub label: &'static str,
    pub hint: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputContainerOption {
    pub container: String,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioCodecOption {
    pub codec: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioChannelOption {
    pub id: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoCodecOption {
    pub codec: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoPixelFormatOption {
    pub id: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub caption: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoPresetOption {
    pub preset: &'static str,
    pub label: &'static str,
    pub caption: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageEncodingOption {
    pub id: &'static str,
    pub label: &'static str,
    pub caption: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioQualityRange {
    pub min: u32,
    pub max: u32,
    pub lower_is_better: bool,
    pub default_value: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInfoRow {
    pub label: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTrackSection {
    pub label: String,
    pub rows: Vec<SourceInfoRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceInfoSection {
    Rows {
        title: &'static str,
        rows: Vec<SourceInfoRow>,
    },
    Tracks {
        title: &'static str,
        tracks: Vec<SourceTrackSection>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioTrackOption {
    pub index: u32,
    pub index_label: String,
    pub codec: String,
    pub detail: String,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Video,
    Audio,
    Image,
}

impl SourceKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Image => "Image",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioTrack {
    pub index: u32,
    pub codec: String,
    pub channels: Option<String>,
    pub language: Option<String>,
    pub label: Option<String>,
    pub bitrate_kbps: Option<f64>,
    pub sample_rate: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
}

impl SourceTags {
    #[must_use]
    pub fn value(&self, field: MetadataField) -> Option<&str> {
        match field {
            MetadataField::Title => self.title.as_deref(),
            MetadataField::Artist => self.artist.as_deref(),
            MetadataField::Album => self.album.as_deref(),
            MetadataField::Genre => self.genre.as_deref(),
            MetadataField::Date => self.date.as_deref(),
            MetadataField::Comment => self.comment.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMetadata {
    pub media_kind: Option<SourceKind>,
    pub duration: Option<String>,
    pub bitrate: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub frame_rate: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub tags: Option<SourceTags>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_primaries: Option<String>,
    pub profile: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AudioCodecDefinition {
    pub(super) codec: &'static str,
    pub(super) label: &'static str,
}

pub(super) const AUDIO_CODEC_DEFINITIONS: [AudioCodecDefinition; 7] = [
    AudioCodecDefinition {
        codec: "aac",
        label: "AAC / Stereo",
    },
    AudioCodecDefinition {
        codec: "ac3",
        label: "Dolby Digital",
    },
    AudioCodecDefinition {
        codec: "libopus",
        label: "Opus",
    },
    AudioCodecDefinition {
        codec: "mp3",
        label: "MP3",
    },
    AudioCodecDefinition {
        codec: "alac",
        label: "ALAC (Lossless)",
    },
    AudioCodecDefinition {
        codec: "flac",
        label: "FLAC (Lossless)",
    },
    AudioCodecDefinition {
        codec: "pcm_s16le",
        label: "PCM / WAV",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OptionalAudioCodecDefinition {
    pub(super) codec: &'static str,
    pub(super) label: &'static str,
}

pub(super) const OPTIONAL_AUDIO_CODEC_DEFINITIONS: [OptionalAudioCodecDefinition; 1] =
    [OptionalAudioCodecDefinition {
        codec: "libfdk_aac",
        label: "AAC (Fraunhofer FDK)",
    }];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AudioChannelDefinition {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
}

pub(super) const AUDIO_CHANNEL_DEFINITIONS: [AudioChannelDefinition; 3] = [
    AudioChannelDefinition {
        id: "original",
        label: "Original",
    },
    AudioChannelDefinition {
        id: "stereo",
        label: "Stereo",
    },
    AudioChannelDefinition {
        id: "mono",
        label: "Mono",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct VideoCodecDefinition {
    pub(super) codec: &'static str,
    pub(super) label: &'static str,
    pub(super) capability: Option<VideoCodecCapability>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum VideoCodecCapability {
    H264Videotoolbox,
    H264Nvenc,
    HevcVideotoolbox,
    HevcNvenc,
    Av1Nvenc,
}

pub(super) const VIDEO_CODEC_DEFINITIONS: [VideoCodecDefinition; 11] = [
    VideoCodecDefinition {
        codec: "libx264",
        label: "H.264 / AVC",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "libx265",
        label: "H.265 / HEVC",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "vp9",
        label: "VP9 / Web",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "prores",
        label: "Apple ProRes",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "libsvtav1",
        label: "AV1 / SVT",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "gif",
        label: "GIF / Palette",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "h264_videotoolbox",
        label: "H.264 (Apple Silicon)",
        capability: Some(VideoCodecCapability::H264Videotoolbox),
    },
    VideoCodecDefinition {
        codec: "h264_nvenc",
        label: "H.264 (NVIDIA)",
        capability: Some(VideoCodecCapability::H264Nvenc),
    },
    VideoCodecDefinition {
        codec: "hevc_videotoolbox",
        label: "H.265 (Apple Silicon)",
        capability: Some(VideoCodecCapability::HevcVideotoolbox),
    },
    VideoCodecDefinition {
        codec: "hevc_nvenc",
        label: "H.265 (NVIDIA)",
        capability: Some(VideoCodecCapability::HevcNvenc),
    },
    VideoCodecDefinition {
        codec: "av1_nvenc",
        label: "AV1 (NVIDIA)",
        capability: Some(VideoCodecCapability::Av1Nvenc),
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct VideoPixelFormatDefinition {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
}

pub(super) const VIDEO_PIXEL_FORMAT_DEFINITIONS: [VideoPixelFormatDefinition; 7] = [
    VideoPixelFormatDefinition {
        id: "auto",
        label: "Auto",
    },
    VideoPixelFormatDefinition {
        id: "yuv420p",
        label: "YUV 4:2:0 (8-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv422p",
        label: "YUV 4:2:2 (8-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv444p",
        label: "YUV 4:4:4 (8-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv420p10le",
        label: "YUV 4:2:0 (10-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv422p10le",
        label: "YUV 4:2:2 (10-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv444p10le",
        label: "YUV 4:4:4 (10-bit)",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ImageEncodingOptionDefinition {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) caption: &'static str,
}

pub(super) const IMAGE_JPEG_HUFFMAN_OPTIONS: [ImageEncodingOptionDefinition; 2] = [
    ImageEncodingOptionDefinition {
        id: "optimal",
        label: "Optimized",
        caption: "Smaller files, slower encode",
    },
    ImageEncodingOptionDefinition {
        id: "default",
        label: "Default",
        caption: "Fast standard tables",
    },
];

pub(super) const IMAGE_WEBP_PRESET_OPTIONS: [ImageEncodingOptionDefinition; 6] = [
    ImageEncodingOptionDefinition {
        id: "default",
        label: "Default",
        caption: "Balanced encoder defaults",
    },
    ImageEncodingOptionDefinition {
        id: "picture",
        label: "Picture",
        caption: "Portraits and indoor shots",
    },
    ImageEncodingOptionDefinition {
        id: "photo",
        label: "Photo",
        caption: "Natural outdoor images",
    },
    ImageEncodingOptionDefinition {
        id: "drawing",
        label: "Drawing",
        caption: "Lines and high contrast",
    },
    ImageEncodingOptionDefinition {
        id: "icon",
        label: "Icon",
        caption: "Small colorful graphics",
    },
    ImageEncodingOptionDefinition {
        id: "text",
        label: "Text",
        caption: "Text-like imagery",
    },
];

pub(super) const IMAGE_PNG_PREDICTION_OPTIONS: [ImageEncodingOptionDefinition; 6] = [
    ImageEncodingOptionDefinition {
        id: "paeth",
        label: "Paeth",
        caption: "PNG default",
    },
    ImageEncodingOptionDefinition {
        id: "mixed",
        label: "Mixed",
        caption: "Adaptive prediction",
    },
    ImageEncodingOptionDefinition {
        id: "sub",
        label: "Sub",
        caption: "Horizontal predictor",
    },
    ImageEncodingOptionDefinition {
        id: "up",
        label: "Up",
        caption: "Vertical predictor",
    },
    ImageEncodingOptionDefinition {
        id: "avg",
        label: "Average",
        caption: "Average predictor",
    },
    ImageEncodingOptionDefinition {
        id: "none",
        label: "None",
        caption: "No prediction",
    },
];

pub(super) const IMAGE_TIFF_COMPRESSION_OPTIONS: [ImageEncodingOptionDefinition; 4] = [
    ImageEncodingOptionDefinition {
        id: "packbits",
        label: "PackBits",
        caption: "Fast lossless default",
    },
    ImageEncodingOptionDefinition {
        id: "lzw",
        label: "LZW",
        caption: "Broad lossless support",
    },
    ImageEncodingOptionDefinition {
        id: "deflate",
        label: "Deflate",
        caption: "Smaller lossless output",
    },
    ImageEncodingOptionDefinition {
        id: "raw",
        label: "Raw",
        caption: "Uncompressed pixels",
    },
];

pub(super) const VIDEO_PRESETS: [&str; 9] = [
    "ultrafast",
    "superfast",
    "veryfast",
    "faster",
    "fast",
    "medium",
    "slow",
    "slower",
    "veryslow",
];

pub(super) const RESOLUTION_OPTIONS: [&str; 5] = ["original", "1080p", "720p", "480p", "custom"];
pub(super) const SCALING_ALGORITHM_OPTIONS: [&str; 4] =
    ["bicubic", "lanczos", "bilinear", "nearest"];
pub(super) const FPS_OPTIONS: [&str; 4] = ["original", "24", "30", "60"];
pub(super) const GIF_FPS_OPTIONS: [&str; 8] = ["original", "8", "10", "12", "15", "20", "24", "30"];
pub(super) const GIF_COLOR_OPTIONS: [u16; 4] = [32, 64, 128, 256];
pub(super) const GIF_DITHER_OPTIONS: [&str; 4] = ["sierra2_4a", "floyd_steinberg", "bayer", "none"];

impl SourceMetadata {
    #[must_use]
    pub fn source_kind(&self) -> SourceKind {
        self.media_kind.unwrap_or_else(|| {
            if self.video_codec.is_some() {
                SourceKind::Video
            } else {
                SourceKind::Audio
            }
        })
    }
}
