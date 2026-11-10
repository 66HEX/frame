use crate::settings::model::config::ConversionConfig;
use serde::{Deserialize, Serialize};

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
