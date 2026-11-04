use crate::settings::model::defaults::DEFAULT_METADATA_MODE;
use serde::{Deserialize, Serialize};

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
    pub service_name: Option<String>,
    pub service_provider: Option<String>,
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
            service_name: None,
            service_provider: None,
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
    ServiceName,
    ServiceProvider,
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
            Self::ServiceName => "serviceName",
            Self::ServiceProvider => "serviceProvider",
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
            Self::ServiceName => "Service name",
            Self::ServiceProvider => "Service provider",
        }
    }

    #[must_use]
    pub const fn visible_for_image(self) -> bool {
        !matches!(
            self,
            Self::Album | Self::Genre | Self::ServiceName | Self::ServiceProvider
        )
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
