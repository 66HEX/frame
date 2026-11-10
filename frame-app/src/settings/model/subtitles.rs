use serde::{Deserialize, Serialize};

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
    pub disabled_reason: Option<String>,
}

/// A sidecar subtitle file embedded as a selectable output track.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExternalSubtitleTrack {
    pub path: String,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
    pub is_forced: bool,
}
