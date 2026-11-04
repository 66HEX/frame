#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsTab {
    Source,
    Output,
    Video,
    VideoFilters,
    Images,
    Audio,
    AudioFilters,
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
            Self::VideoFilters => "Video Filters",
            Self::Images => "Images",
            Self::Audio => "Audio",
            Self::AudioFilters => "Audio Filters",
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
            Self::VideoFilters => "video-filters",
            Self::Images => "images",
            Self::Audio => "audio",
            Self::AudioFilters => "audio-filters",
            Self::Subtitles => "subtitles",
            Self::Metadata => "metadata",
            Self::Presets => "presets",
        }
    }
}

pub const ALL_SETTINGS_TABS: [SettingsTab; 10] = [
    SettingsTab::Source,
    SettingsTab::Output,
    SettingsTab::Video,
    SettingsTab::VideoFilters,
    SettingsTab::Images,
    SettingsTab::Audio,
    SettingsTab::AudioFilters,
    SettingsTab::Subtitles,
    SettingsTab::Metadata,
    SettingsTab::Presets,
];
