use frame_core::media_rules;

pub const DEFAULT_AUDIO_BITRATE: &str = "128";
pub const DEFAULT_AUDIO_BITRATE_MODE: &str = "bitrate";
pub const DEFAULT_AUDIO_QUALITY: &str = "4";
pub const DEFAULT_AUDIO_CHANNELS: &str = "original";
pub const DEFAULT_AUDIO_VOLUME: u32 = 100;
pub(super) const MAX_AUDIO_VOLUME: u32 = 200;

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionConfig {
    pub processing_mode: ProcessingMode,
    pub container: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
    pub audio_bitrate_mode: String,
    pub audio_quality: String,
    pub audio_channels: String,
    pub audio_volume: u32,
    pub audio_normalize: bool,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub rotation: String,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub crop: Option<CropSettings>,
    pub selected_audio_tracks: Vec<u32>,
    pub selected_subtitle_tracks: Vec<u32>,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            processing_mode: ProcessingMode::Reencode,
            container: "mp4".to_string(),
            audio_codec: media_rules::default_audio_codec_for_container("mp4").to_string(),
            audio_bitrate: DEFAULT_AUDIO_BITRATE.to_string(),
            audio_bitrate_mode: DEFAULT_AUDIO_BITRATE_MODE.to_string(),
            audio_quality: DEFAULT_AUDIO_QUALITY.to_string(),
            audio_channels: DEFAULT_AUDIO_CHANNELS.to_string(),
            audio_volume: DEFAULT_AUDIO_VOLUME,
            audio_normalize: false,
            start_time: None,
            end_time: None,
            rotation: "0".to_string(),
            flip_horizontal: false,
            flip_vertical: false,
            crop: None,
            selected_audio_tracks: Vec::new(),
            selected_subtitle_tracks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
pub struct AudioQualityRange {
    pub min: u32,
    pub max: u32,
    pub lower_is_better: bool,
    pub default_value: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceInfoRow {
    pub label: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceTrackSection {
    pub label: String,
    pub rows: Vec<SourceInfoRow>,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
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
