use derive_more::{Add, FromStr, Sub};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
};

/// Font width as a percentage, with 100 representing normal width.
///
/// Selects static width variants or a variable font's `wdth` axis. Requests outside
/// a variable font's axis range are clamped to that range by the text backend.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Add, Sub, FromStr)]
#[serde(transparent)]
pub struct FontWidth(pub f32);

impl FontWidth {
    /// Ultra-condensed width, 50% of normal.
    pub const ULTRA_CONDENSED: Self = Self(50.0);
    /// Extra-condensed width, 62.5% of normal.
    pub const EXTRA_CONDENSED: Self = Self(62.5);
    /// Condensed width, 75% of normal.
    pub const CONDENSED: Self = Self(75.0);
    /// Semi-condensed width, 87.5% of normal.
    pub const SEMI_CONDENSED: Self = Self(87.5);
    /// Normal width, 100%.
    pub const NORMAL: Self = Self(100.0);
    /// Semi-expanded width, 112.5% of normal.
    pub const SEMI_EXPANDED: Self = Self(112.5);
    /// Expanded width, 125% of normal.
    pub const EXPANDED: Self = Self(125.0);
    /// Extra-expanded width, 150% of normal.
    pub const EXTRA_EXPANDED: Self = Self(150.0);
    /// Ultra-expanded width, 200% of normal.
    pub const ULTRA_EXPANDED: Self = Self(200.0);
}

impl From<f32> for FontWidth {
    fn from(width: f32) -> Self {
        Self(width)
    }
}

impl Default for FontWidth {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl Eq for FontWidth {}

impl Hash for FontWidth {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Display for FontWidth {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}%", self.0)
    }
}

impl JsonSchema for FontWidth {
    fn schema_name() -> Cow<'static, str> {
        "FontWidth".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "number",
            "exclusiveMinimum": 0,
            "default": Self::default(),
            "description": "Font width as a percentage of normal width (100)"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_widths_convert_parse_and_serialize() {
        for percentage in [0.5, 75.0, 82.5, 100.0, 250.0] {
            let width = FontWidth(percentage);
            let json = serde_json::to_string(&width).unwrap();

            assert_eq!(FontWidth::from(percentage), width);
            assert_eq!(json.parse::<FontWidth>().unwrap(), width);
            assert_eq!(serde_json::from_str::<FontWidth>(&json).unwrap(), width);
        }

        assert_eq!(FontWidth::default(), FontWidth::NORMAL);
    }
}
