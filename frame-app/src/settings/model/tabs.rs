#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsTab {
    Metadata,
    Output,
    Presets,
}
impl SettingsTab {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Metadata => "Metadata",
            Self::Output => "Output",
            Self::Presets => "Presets",
        }
    }
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::Output => "output",
            Self::Presets => "presets",
        }
    }
}
pub const ALL_SETTINGS_TABS: [SettingsTab; 3] = [
    SettingsTab::Metadata,
    SettingsTab::Output,
    SettingsTab::Presets,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OutputSection {
    General,
    Video,
    VideoFilters,
    Images,
    Audio,
    AudioFilters,
    Subtitles,
    Metadata,
}
impl OutputSection {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Video => "Video",
            Self::VideoFilters => "Video filters",
            Self::Images => "Images",
            Self::Audio => "Audio",
            Self::AudioFilters => "Audio filters",
            Self::Subtitles => "Subtitles",
            Self::Metadata => "Metadata",
        }
    }
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Video => "video",
            Self::VideoFilters => "video-filters",
            Self::Images => "images",
            Self::Audio => "audio",
            Self::AudioFilters => "audio-filters",
            Self::Subtitles => "subtitles",
            Self::Metadata => "metadata",
        }
    }
}
pub const ALL_OUTPUT_SECTIONS: [OutputSection; 8] = [
    OutputSection::General,
    OutputSection::Video,
    OutputSection::Images,
    OutputSection::VideoFilters,
    OutputSection::Audio,
    OutputSection::AudioFilters,
    OutputSection::Subtitles,
    OutputSection::Metadata,
];
