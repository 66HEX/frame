use serde::{Deserialize, Serialize};

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
