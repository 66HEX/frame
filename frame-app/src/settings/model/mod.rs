//! Conversion settings data, defaults and option definitions by domain.

mod codecs;
mod config;
mod defaults;
mod filters;
mod images;
mod metadata;
mod options;
mod presets;
mod processing;
mod source;
mod subtitles;
mod tabs;

pub(super) use codecs::{
    AUDIO_CHANNEL_DEFINITIONS, AUDIO_CODEC_DEFINITIONS, FPS_OPTIONS, GIF_COLOR_OPTIONS,
    GIF_DITHER_OPTIONS, GIF_FPS_OPTIONS, OPTIONAL_AUDIO_CODEC_DEFINITIONS, RESOLUTION_OPTIONS,
    SCALING_ALGORITHM_OPTIONS, VIDEO_CODEC_DEFINITIONS, VIDEO_PIXEL_FORMAT_DEFINITIONS,
    VIDEO_PRESETS, VideoCodecCapability,
};
pub use config::{ConversionConfig, CropSettings, OverlaySettings};
pub use defaults::{
    DEFAULT_AUDIO_BITRATE, DEFAULT_AUDIO_BITRATE_MODE, DEFAULT_AUDIO_CHANNELS,
    DEFAULT_AUDIO_FILTER_DE_ESSER, DEFAULT_AUDIO_FILTER_HIGH_PASS, DEFAULT_AUDIO_FILTER_LIMITER,
    DEFAULT_AUDIO_FILTER_LOW_PASS, DEFAULT_AUDIO_FILTER_NOISE_REDUCTION, DEFAULT_AUDIO_QUALITY,
    DEFAULT_AUDIO_VOLUME, DEFAULT_CRF, DEFAULT_FPS, DEFAULT_GIF_COLORS, DEFAULT_GIF_DITHER,
    DEFAULT_GIF_LOOP, DEFAULT_IMAGE_JPEG_HUFFMAN, DEFAULT_IMAGE_JPEG_QUALITY,
    DEFAULT_IMAGE_PNG_COMPRESSION, DEFAULT_IMAGE_PNG_PREDICTION, DEFAULT_IMAGE_TIFF_COMPRESSION,
    DEFAULT_IMAGE_WEBP_COMPRESSION, DEFAULT_IMAGE_WEBP_PRESET, DEFAULT_IMAGE_WEBP_QUALITY,
    DEFAULT_METADATA_MODE, DEFAULT_PIXEL_FORMAT, DEFAULT_PRESET, DEFAULT_QUALITY,
    DEFAULT_RESOLUTION, DEFAULT_SCALING_ALGORITHM, DEFAULT_SUBTITLE_FONT_COLOR,
    DEFAULT_SUBTITLE_OUTLINE_COLOR, DEFAULT_SUBTITLE_POSITION, DEFAULT_VIDEO_BITRATE,
    DEFAULT_VIDEO_BITRATE_MODE, DEFAULT_VIDEO_CODEC, DEFAULT_VIDEO_FILTER_BLUR,
    DEFAULT_VIDEO_FILTER_DEBAND, DEFAULT_VIDEO_FILTER_SHARPEN, DEFAULT_VIDEO_FILTER_TEMPERATURE,
    DEFAULT_VIDEO_FILTER_VIGNETTE,
};
pub(super) use defaults::{
    MAX_AUDIO_VOLUME, MAX_GIF_COLORS, MAX_GIF_LOOP, MAX_IMAGE_JPEG_QUALITY,
    MAX_IMAGE_PNG_COMPRESSION, MAX_IMAGE_WEBP_COMPRESSION, MAX_IMAGE_WEBP_QUALITY,
};
pub use filters::{
    AudioFiltersConfig, DeinterlaceMode, FilterStrength, FilterValue, VideoColorFiltersConfig,
    VideoFiltersConfig,
};
pub(super) use images::{
    IMAGE_JPEG_HUFFMAN_OPTIONS, IMAGE_PNG_PREDICTION_OPTIONS, IMAGE_TIFF_COMPRESSION_OPTIONS,
    IMAGE_WEBP_PRESET_OPTIONS, ImageEncodingOptionDefinition,
};
pub use metadata::{
    METADATA_FIELDS, METADATA_MODES, MetadataConfig, MetadataField, MetadataFieldOption,
    MetadataMode, MetadataModeOption,
};
pub use options::{
    AudioChannelOption, AudioCodecOption, AudioQualityRange, AudioTrackOption, ImageEncodingOption,
    OutputContainerOption, OutputModeOption, VideoCodecOption, VideoPixelFormatOption,
    VideoPresetOption,
};
pub use presets::{PresetDefinition, PresetNotice, PresetNoticeTone, PresetOption};
pub use processing::ProcessingMode;
pub use source::{
    AudioTrack, SourceInfoRow, SourceInfoSection, SourceKind, SourceMetadata, SourceTags,
    SourceTrackSection, SubtitleTrack,
};
pub use subtitles::{
    ExternalSubtitleTrack, SUBTITLE_FONT_SIZES, SUBTITLE_POSITIONS, SubtitleFontOption,
    SubtitleFontSizeOption, SubtitlePosition, SubtitlePositionOption, SubtitleTrackOption,
};
pub use tabs::{ALL_OUTPUT_SECTIONS, ALL_SETTINGS_TABS, OutputSection, SettingsTab};
