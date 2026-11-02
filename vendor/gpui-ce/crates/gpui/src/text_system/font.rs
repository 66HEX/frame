use crate::{FontFallbacks, FontFeatures, FontStyle, FontWeight, FontWidth};
use gpui_shared_string::SharedString;

/// An opaque identifier for a specific font.
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
#[repr(C)]
pub struct FontId(pub usize);

/// The configuration details for identifying a specific font.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Font {
    /// The font family name.
    ///
    /// The special name ".SystemUIFont" is used to identify the system UI font, which varies based on platform.
    pub family: SharedString,

    /// The font features to use.
    pub features: FontFeatures,

    /// The fallbacks fonts to use.
    pub fallbacks: Option<FontFallbacks>,

    /// The font weight.
    pub weight: FontWeight,

    /// The font width. Exhaustive font literals must supply this field.
    pub width: FontWidth,

    /// The font style.
    pub style: FontStyle,
}

impl Default for Font {
    fn default() -> Self {
        font(".SystemUIFont")
    }
}

/// Get a [`Font`] for a given name.
pub fn font(family: impl Into<SharedString>) -> Font {
    Font {
        family: family.into(),
        features: FontFeatures::default(),
        weight: FontWeight::default(),
        width: FontWidth::default(),
        style: FontStyle::default(),
        fallbacks: None,
    }
}

impl Font {
    /// Set this Font to be bold
    pub fn bold(mut self) -> Self {
        self.weight = FontWeight::BOLD;
        self
    }

    /// Set this Font to be italic
    pub fn italic(mut self) -> Self {
        self.style = FontStyle::Italic;
        self
    }
}
