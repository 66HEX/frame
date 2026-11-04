use crate::settings::model::metadata::MetadataMode;
use crate::settings::model::subtitles::SubtitlePosition;

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

pub const DEFAULT_VIDEO_FILTER_TEMPERATURE: u32 = 6500;

pub const DEFAULT_VIDEO_FILTER_SHARPEN: u32 = 25;

pub const DEFAULT_VIDEO_FILTER_BLUR: u32 = 20;

pub const DEFAULT_VIDEO_FILTER_DEBAND: u32 = 25;

pub const DEFAULT_VIDEO_FILTER_VIGNETTE: u32 = 35;

pub const DEFAULT_AUDIO_FILTER_LIMITER: i32 = -1;

pub const DEFAULT_AUDIO_FILTER_HIGH_PASS: u32 = 80;

pub const DEFAULT_AUDIO_FILTER_LOW_PASS: u32 = 16_000;

pub const DEFAULT_AUDIO_FILTER_NOISE_REDUCTION: u32 = 12;

pub const DEFAULT_AUDIO_FILTER_DE_ESSER: u32 = 35;

pub const DEFAULT_METADATA_MODE: MetadataMode = MetadataMode::Preserve;

pub const DEFAULT_SUBTITLE_FONT_COLOR: &str = "#ffffff";

pub const DEFAULT_SUBTITLE_OUTLINE_COLOR: &str = "#000000";

pub const DEFAULT_SUBTITLE_POSITION: SubtitlePosition = SubtitlePosition::Bottom;

pub(in crate::settings) const MAX_AUDIO_VOLUME: u32 = 200;

pub(in crate::settings) const MAX_GIF_LOOP: u16 = 65_535;

pub(in crate::settings) const MAX_GIF_COLORS: u16 = 256;

pub(in crate::settings) const MAX_IMAGE_JPEG_QUALITY: u32 = 100;

pub(in crate::settings) const MAX_IMAGE_WEBP_QUALITY: u32 = 100;

pub(in crate::settings) const MAX_IMAGE_WEBP_COMPRESSION: u32 = 6;

pub(in crate::settings) const MAX_IMAGE_PNG_COMPRESSION: u32 = 9;
