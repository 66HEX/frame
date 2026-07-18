use super::{FrameTextInputKind, Range};

pub(super) fn sanitize_number_input(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

pub(super) fn sanitize_replacement_text(kind: FrameTextInputKind, value: &str) -> String {
    match kind {
        FrameTextInputKind::MaxConcurrency
        | FrameTextInputKind::AudioBitrate
        | FrameTextInputKind::VideoCustomWidth
        | FrameTextInputKind::VideoCustomHeight
        | FrameTextInputKind::VideoBitrate
        | FrameTextInputKind::GifLoop => sanitize_number_input(value),
        FrameTextInputKind::PreviewStartTime | FrameTextInputKind::PreviewEndTime => {
            sanitize_number_input(value)
        }
        FrameTextInputKind::OutputName
        | FrameTextInputKind::MetadataTitle
        | FrameTextInputKind::MetadataArtist
        | FrameTextInputKind::MetadataAlbum
        | FrameTextInputKind::MetadataGenre
        | FrameTextInputKind::MetadataDate
        | FrameTextInputKind::MetadataComment
        | FrameTextInputKind::PresetName => value.chars().filter(|ch| !ch.is_control()).collect(),
        FrameTextInputKind::SubtitleFontColorHex | FrameTextInputKind::SubtitleOutlineColorHex => {
            value
                .chars()
                .filter(|ch| *ch == '#' || ch.is_ascii_hexdigit())
                .collect()
        }
    }
}

pub(super) struct MaskedTimecodeEdit {
    pub(super) value: String,
    pub(super) cursor: usize,
}

pub(super) fn timecode_cursor_at_or_after(value: &str, offset: usize) -> usize {
    value
        .bytes()
        .enumerate()
        .skip(offset.min(value.len()))
        .find_map(|(index, byte)| byte.is_ascii_digit().then_some(index))
        .unwrap_or(value.len())
}

pub(super) fn previous_timecode_cursor(value: &str, offset: usize) -> usize {
    value
        .bytes()
        .enumerate()
        .take(offset.min(value.len()))
        .rev()
        .find_map(|(index, byte)| byte.is_ascii_digit().then_some(index))
        .unwrap_or(0)
}

pub(super) fn next_timecode_cursor(value: &str, offset: usize) -> usize {
    let search_from = offset.saturating_add(1).min(value.len());
    timecode_cursor_at_or_after(value, search_from)
}

pub(super) fn replace_timecode_mask(
    value: &str,
    range: &Range<usize>,
    replacement: &str,
) -> Option<MaskedTimecodeEdit> {
    if !value.is_ascii() {
        return None;
    }

    let range = clamp_text_range(value, range);
    let mut bytes = value.as_bytes().to_vec();
    let first_selected_digit = bytes
        .iter()
        .enumerate()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .find_map(|(index, byte)| byte.is_ascii_digit().then_some(index));

    if replacement.is_empty() && range.is_empty() {
        return None;
    }

    for byte in bytes
        .iter_mut()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
    {
        if byte.is_ascii_digit() {
            *byte = b'0';
        }
    }

    let insertion_start =
        first_selected_digit.unwrap_or_else(|| timecode_cursor_at_or_after(value, range.start));
    let mut replacement_digits = replacement.bytes().filter(u8::is_ascii_digit);
    let mut last_replaced_digit = None;

    for (index, byte) in bytes.iter_mut().enumerate().skip(insertion_start) {
        if !byte.is_ascii_digit() {
            continue;
        }
        let Some(digit) = replacement_digits.next() else {
            break;
        };
        *byte = digit;
        last_replaced_digit = Some(index);
    }

    let cursor =
        last_replaced_digit.map_or(insertion_start, |index| next_timecode_cursor(value, index));
    let value = String::from_utf8(bytes).ok()?;

    Some(MaskedTimecodeEdit { value, cursor })
}

pub(super) fn sanitize_hex_draft(value: &str) -> String {
    let mut next = String::from("#");
    next.extend(
        value
            .chars()
            .filter(char::is_ascii_hexdigit)
            .take(6)
            .map(|ch| ch.to_ascii_uppercase()),
    );
    next
}

pub(super) fn clamp_text_offset(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub(super) fn clamp_text_range(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = clamp_text_offset(text, range.start);
    let end = clamp_text_offset(text, range.end);
    start.min(end)..start.max(end)
}

pub(super) fn previous_text_boundary(text: &str, offset: usize) -> usize {
    let offset = clamp_text_offset(text, offset);
    text[..offset]
        .char_indices()
        .last()
        .map_or(0, |(index, _)| index)
}

pub(super) fn next_text_boundary(text: &str, offset: usize) -> usize {
    let offset = clamp_text_offset(text, offset);
    if offset >= text.len() {
        return text.len();
    }

    text[offset..]
        .char_indices()
        .find_map(|(index, _)| (index > 0).then_some(offset + index))
        .unwrap_or(text.len())
}

pub(super) fn text_offset_to_utf16(text: &str, offset: usize) -> usize {
    text[..clamp_text_offset(text, offset)]
        .encode_utf16()
        .count()
}

pub(super) fn text_offset_from_utf16(text: &str, offset_utf16: usize) -> usize {
    let mut utf16_count = 0;
    let mut utf8_offset = 0;

    for ch in text.chars() {
        if utf16_count >= offset_utf16 {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }

    clamp_text_offset(text, utf8_offset)
}

pub(super) fn text_range_to_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    text_offset_to_utf16(text, range.start)..text_offset_to_utf16(text, range.end)
}

pub(super) fn text_range_from_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = text_offset_from_utf16(text, range.start);
    let end = text_offset_from_utf16(text, range.end);
    clamp_text_range(text, &(start..end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecode_cursor_moves_forward_across_separators() {
        assert_eq!(next_timecode_cursor("00:00:00.000", 1), 3);
    }

    #[test]
    fn timecode_cursor_moves_backward_across_separators() {
        assert_eq!(previous_timecode_cursor("00:00:00.000", 3), 1);
    }

    #[test]
    fn timecode_mask_replaces_only_digit_positions() {
        let edit = replace_timecode_mask("00:00:00.000", &(0..12), "011500000")
            .expect("masked replacement should succeed");

        assert_eq!(edit.value, "01:15:00.000");
    }

    #[test]
    fn timecode_mask_clear_preserves_fixed_separators() {
        let edit = replace_timecode_mask("12:34:56.789", &(0..12), "")
            .expect("masked clear should succeed");

        assert_eq!(edit.value, "00:00:00.000");
    }

    #[test]
    fn timecode_mask_supports_more_than_two_hour_digits() {
        let edit = replace_timecode_mask("100:00:00.000", &(0..13), "1234567890")
            .expect("wide hour mask should succeed");

        assert_eq!(edit.value, "123:45:67.890");
    }
}
