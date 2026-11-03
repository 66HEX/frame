pub use gpui::TextBoundary;
use gpui::{NavigationDirection, utf8_to_utf16_offset, utf16_to_utf8_offset};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Implement this trait to create a storage medium that can be used as the content of EditableText elements.
/// Default implementation is [`StringStorage`].
pub trait UnicodeTextStorage {
    /// Returns the version/generation of the content, which should be incremented ever time the
    /// content is changed so that rendering elements can reprocess the contents via the text layout engine.
    fn version(&self) -> u16;

    /// Returns a reference to the utf8 string.
    fn content_utf8(&self) -> &str;

    /// Returns the UTF-16 length of the content.
    fn len_utf16(&self) -> usize;

    /// Replace contents within the provided range with the given str slice.
    fn replace_range(&mut self, range: Range<usize>, text: &str);

    /// Returns the utf16 position equivalent of the provided utf8 character position.
    fn utf_offset_8to16(&self, pos_uft8: usize) -> usize {
        // Fast path: if offset is 0, return 0
        if pos_uft8 == 0 {
            return 0;
        }

        // Fast path: if offset is at or past end, return cached length
        if pos_uft8 >= self.content_utf8().len() {
            return self.len_utf16();
        }

        utf8_to_utf16_offset(self.content_utf8(), pos_uft8)
    }

    /// Returns the utf8 position equivalent of the provided utf16 character position.
    fn utf_offset_16to8(&self, pos_utf16: usize) -> usize {
        // Fast path: if offset is 0, return 0
        if pos_utf16 == 0 {
            return 0;
        }

        utf16_to_utf8_offset(self.content_utf8(), pos_utf16)
    }

    /// Converts a utf8 character range into a utf16 character range.
    fn utf_range_8to16(&self, range_utf8: &Range<usize>) -> Range<usize> {
        self.utf_offset_8to16(range_utf8.start)..self.utf_offset_8to16(range_utf8.end)
    }

    /// Converts a utf16 character range into a utf8 character range.
    fn utf_range_16to8(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.utf_offset_16to8(range_utf16.start)..self.utf_offset_16to8(range_utf16.end)
    }

    /// Builds a utf8 character range based on a caret position within the storage,
    /// the direction to traverse, and the boundary to stop at.
    /// The start of the range will be the earlier position (destination if Back, caret if Forward),
    /// and the end will be the later position (caret if Back, destination if Forward).
    fn range_from_caret(
        &self,
        caret: usize,
        direction: NavigationDirection,
        magnitude: TextBoundary,
    ) -> Range<usize> {
        let offset = self.offset_from_caret(caret, direction, magnitude);
        match direction {
            NavigationDirection::Back => offset..caret,
            NavigationDirection::Forward => caret..offset,
        }
    }

    /// Finds the next location from the caret based on the direction to traverse and the boundary to stop at.
    fn offset_from_caret(
        &self,
        caret: usize,
        direction: NavigationDirection,
        boundary: TextBoundary,
    ) -> usize {
        use NavigationDirection::*;
        use TextBoundary::*;
        match (direction, boundary) {
            (Back, Cluster) => {
                if caret == 0 {
                    return 0;
                }

                let str = self.content_utf8();
                let iter = str[..caret.min(str.len())].grapheme_indices(true);
                iter.map(|(i, _)| i).next_back().unwrap_or(0)
            }
            (Forward, Cluster) => {
                let str = self.content_utf8();
                let len_utf8 = str.len();
                if caret >= len_utf8 {
                    return len_utf8;
                }

                let mut iter = str[caret..].grapheme_indices(true);
                iter.nth(1).map(|(i, _)| caret + i).unwrap_or(len_utf8)
            }
            (Back, Word) => {
                let text = self.content_utf8();
                let prefix = &text[..caret.min(text.len())];

                prefix
                    .unicode_word_indices()
                    .next_back()
                    .map_or(0, |(idx, _word)| idx)
            }
            (Forward, Word) => {
                let text = self.content_utf8();
                let len_utf8 = text.len();

                if caret >= len_utf8 {
                    return len_utf8;
                }

                text[caret..]
                    .unicode_word_indices()
                    .next()
                    .map_or(len_utf8, |(idx, word)| caret + idx + word.len())
            }
            // Returns the utf-8 character position of first character after the first new-line
            // preceding the character at the provided utf-8 character position.
            (Back, VisualLine | HardLine) => {
                let str = self.content_utf8();
                let iter = str[..caret.min(str.len())].rfind('\n');
                iter.map(|pos| pos + 1).unwrap_or(0)
            }
            // Returns the utf-8 character position of the character immediately before the first
            // new-line character after the character at the provided utf-8 character position.
            (Forward, VisualLine | HardLine) => {
                let str = self.content_utf8();
                let iter = str[caret.min(str.len())..].find('\n');
                iter.map(|pos| caret + pos).unwrap_or(str.len())
            }
            (Back, Document) => 0,
            (Forward, Document) => self.content_utf8().len(),
        }
    }

    /// Returns the start and end of the word the position resides within.
    fn word_range_at(&self, position: usize) -> Range<usize> {
        let offset = position.min(self.content_utf8().len());

        for (idx, word) in self.content_utf8().unicode_word_indices() {
            let word_end = idx + word.len();
            if offset >= idx && offset <= word_end {
                return idx..word_end;
            }
        }

        offset..offset
    }
}

/// [`UnicodeTextStorage`] implementation for [`String`].
/// This is not the most performant, especially for large text documents.
/// Its a decent default for editable text fields though.
#[derive(Clone, Default)]
pub struct StringStorage {
    value: String,
    version: u16,
}
impl<S> From<S> for StringStorage
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        Self {
            value: value.into(),
            version: u16::default(),
        }
    }
}
impl UnicodeTextStorage for StringStorage {
    fn version(&self) -> u16 {
        self.version
    }

    fn content_utf8(&self) -> &str {
        self.value.as_str()
    }

    fn len_utf16(&self) -> usize {
        self.value.chars().map(|c| c.len_utf16()).sum()
    }

    fn replace_range(&mut self, range: Range<usize>, text: &str) {
        self.value.replace_range(range, &text);
        self.version = self.version.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_navigation_handles_boundaries_and_scripts() {
        for (text, caret, direction, expected) in [
            ("", 0, NavigationDirection::Back, 0),
            ("", 0, NavigationDirection::Forward, 0),
            ("   ", 3, NavigationDirection::Back, 0),
            ("   ", 0, NavigationDirection::Forward, 3),
            ("hello world", 11, NavigationDirection::Back, 6),
            ("hello world", 0, NavigationDirection::Forward, 5),
            ("hello world", 2, NavigationDirection::Forward, 5),
            (", hi!", 5, NavigationDirection::Back, 2),
            (", hi!", 0, NavigationDirection::Forward, 4),
            ("a 日本語 b", 11, NavigationDirection::Back, 8),
            ("a 日本語 b", 2, NavigationDirection::Forward, 5),
            ("hi", 100, NavigationDirection::Back, 0),
            ("hi", 100, NavigationDirection::Forward, 2),
        ] {
            let storage = StringStorage::from(text);

            assert_eq!(
                storage.offset_from_caret(caret, direction, TextBoundary::Word),
                expected,
                "text {text:?}, caret {caret}, direction {direction:?}",
            );
        }
    }
}
