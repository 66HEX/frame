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
        FrameTextInputKind::PreviewStartTime | FrameTextInputKind::PreviewEndTime => value
            .chars()
            .filter(|ch| ch.is_ascii_digit() || matches!(*ch, ':' | '.' | ','))
            .collect(),
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
