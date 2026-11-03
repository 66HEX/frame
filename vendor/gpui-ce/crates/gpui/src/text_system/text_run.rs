use crate::{Font, Pixels, StrikethroughStyle, UnderlineStyle};
use palette::Hsla;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// A styled run of text, for use in [`crate::TextLayout`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TextRun {
    /// A number of utf8 bytes
    pub len: usize,
    /// The font to use for this run.
    pub font: Font,
    /// The color
    pub color: Hsla,
    /// The background color (if any)
    pub background_color: Option<Hsla>,
    /// The underline style (if any)
    pub underline: Option<UnderlineStyle>,
    /// The strikethrough style (if any)
    pub strikethrough: Option<StrikethroughStyle>,
    /// Letter spacing applied between glyphs, in pixels.
    pub letter_spacing: Option<Pixels>,
}

impl Eq for TextRun {}

impl Hash for TextRun {
    fn hash<H: Hasher>(&self, state: &mut H) {
        fn hash_float<H: Hasher>(value: f32, state: &mut H) {
            if value == 0.0 {
                0.0f32.to_bits().hash(state);
            } else {
                value.to_bits().hash(state);
            }
        }

        fn hash_color<H: Hasher>(color: Hsla, state: &mut H) {
            let color = crate::hsla_to_rgba(color);
            hash_float(color.color.red, state);
            hash_float(color.color.green, state);
            hash_float(color.color.blue, state);
            hash_float(color.alpha, state);
        }

        self.len.hash(state);
        self.font.hash(state);
        hash_color(self.color, state);
        if let Some(color) = self.background_color {
            hash_color(color, state);
        }
        self.background_color.is_some().hash(state);
        self.underline.is_some().hash(state);
        if let Some(underline) = self.underline {
            hash_float(underline.thickness.into(), state);
            underline.color.is_some().hash(state);
            if let Some(color) = underline.color {
                hash_color(color, state);
            }
            underline.wavy.hash(state);
        }
        self.strikethrough.is_some().hash(state);
        if let Some(strikethrough) = self.strikethrough {
            hash_float(strikethrough.thickness.into(), state);
            strikethrough.color.is_some().hash(state);
            if let Some(color) = strikethrough.color {
                hash_color(color, state);
            }
        }
        self.letter_spacing.is_some().hash(state);
        if let Some(letter_spacing) = self.letter_spacing {
            hash_float(letter_spacing.into(), state);
        }
    }
}
