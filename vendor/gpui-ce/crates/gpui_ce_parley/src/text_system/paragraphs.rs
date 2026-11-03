use gpui::{
    Bounds, CaretMovement, CaretPosition, InlineRangeGeometry, Pixels, PlatformTextLayout, Point,
    Size, TextBoundary as Boundary, TextDirection as Direction, TextMovement, TextSelectionKind,
    VisualDirection, is_paragraph_separator, point, px,
};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation as _;

#[derive(Debug)]
pub(super) struct ParagraphRange {
    pub content: Range<usize>,
    pub separator: Range<usize>,
}

pub(super) fn paragraph_ranges(text: &str) -> Vec<ParagraphRange> {
    let mut paragraphs = Vec::new();
    let mut start = 0;
    let mut characters = text.char_indices().peekable();

    while let Some((idx, character)) = characters.next() {
        if !is_paragraph_separator(character) {
            continue;
        }

        let mut end = idx + character.len_utf8();

        if character == '\r' && characters.peek().is_some_and(|(_idx, next)| *next == '\n') {
            characters.next();
            end += 1;
        }

        paragraphs.push(ParagraphRange {
            content: start..idx,
            separator: idx..end,
        });
        start = end;
    }

    paragraphs.push(ParagraphRange {
        content: start..text.len(),
        separator: text.len()..text.len(),
    });

    paragraphs
}

pub(super) fn local_range(source: &Range<usize>, content: &Range<usize>) -> Option<Range<usize>> {
    let start = source.start.max(content.start);
    let end = source.end.min(content.end);

    if start >= end {
        return None;
    }

    Some(start - content.start..end - content.start)
}

#[derive(Debug)]
pub(super) struct ParagraphLayout {
    pub source: ParagraphRange,
    pub first_line: usize,
    pub block_offset: Pixels,
    pub native: Arc<dyn PlatformTextLayout>,
    pub newline: Range<Pixels>,
}

impl ParagraphLayout {
    fn local_caret(&self, caret: CaretPosition) -> CaretPosition {
        CaretPosition {
            index: caret
                .index
                .saturating_sub(self.source.content.start)
                .min(self.native.len()),
            affinity: caret.affinity,
        }
    }

    fn global_caret(&self, caret: CaretPosition) -> CaretPosition {
        CaretPosition {
            index: self.source.content.start + caret.index,
            affinity: caret.affinity,
        }
    }

    fn local_point(&self, mut position: Point<Pixels>, line_height: Pixels) -> Point<Pixels> {
        position.y -= line_height * self.first_line;

        position
    }

    fn edge(&self, direction: VisualDirection) -> CaretPosition {
        let position = match direction {
            VisualDirection::Left => point(px(f32::MAX), px(self.native.line_count() as f32 - 0.5)),
            VisualDirection::Right => point(px(-f32::MAX), px(0.5)),
        };
        let caret = self
            .native
            .caret_from_pixel_point(position, px(1.0))
            .unwrap_or_else(|caret| caret);

        self.global_caret(caret)
    }
}

#[derive(Debug)]
pub(super) struct ParleyDocumentLayout {
    paragraphs: Vec<ParagraphLayout>,
    size: Size<Pixels>,
    graphemes: Vec<Range<usize>>,
}

impl ParleyDocumentLayout {
    pub fn new(paragraphs: Vec<ParagraphLayout>, text: &str, size: Size<Pixels>) -> Self {
        let graphemes = text
            .grapheme_indices(true)
            .map(|(start, grapheme)| start..start + grapheme.len())
            .collect();

        Self {
            paragraphs,
            size,
            graphemes,
        }
    }

    fn paragraph_for_index(&self, idx: usize) -> usize {
        self.paragraphs
            .partition_point(|paragraph| paragraph.source.content.start <= idx)
            .saturating_sub(1)
    }

    fn paragraph_for_point(
        &self,
        position: Point<Pixels>,
        line_height: Pixels,
    ) -> &ParagraphLayout {
        let line_index = if line_height > Pixels::ZERO && position.y >= Pixels::ZERO {
            (position.y / line_height) as usize
        } else {
            0
        };
        let paragraph_idx = self
            .paragraphs
            .partition_point(|paragraph| paragraph.first_line <= line_index)
            .saturating_sub(1);

        &self.paragraphs[paragraph_idx]
    }

    fn adjacent_edge(
        &self,
        paragraph_idx: usize,
        direction: VisualDirection,
    ) -> Option<CaretPosition> {
        let next_idx = match direction {
            VisualDirection::Left => paragraph_idx.checked_sub(1)?,
            VisualDirection::Right => paragraph_idx + 1,
        };

        Some(self.paragraphs.get(next_idx)?.edge(direction))
    }
}

impl PlatformTextLayout for ParleyDocumentLayout {
    fn len(&self) -> usize {
        self.paragraphs.last().unwrap().source.separator.end
    }

    fn line_count(&self) -> usize {
        let paragraph = self.paragraphs.last().unwrap();

        paragraph.first_line + paragraph.native.line_count()
    }

    fn size(&self) -> Size<Pixels> {
        self.size
    }

    fn byte_index_from_pixel_point(
        &self,
        pixel_point: Point<Pixels>,
        line_height: Pixels,
    ) -> Result<usize, usize> {
        let paragraph = self.paragraph_for_point(pixel_point, line_height);

        paragraph
            .native
            .byte_index_from_pixel_point(
                paragraph.local_point(pixel_point, line_height),
                line_height,
            )
            .map(|idx| idx + paragraph.source.content.start)
            .map_err(|idx| idx + paragraph.source.content.start)
    }

    fn caret_from_pixel_point(
        &self,
        pixel_point: Point<Pixels>,
        line_height: Pixels,
    ) -> Result<CaretPosition, CaretPosition> {
        let paragraph = self.paragraph_for_point(pixel_point, line_height);

        paragraph
            .native
            .caret_from_pixel_point(paragraph.local_point(pixel_point, line_height), line_height)
            .map(|caret| paragraph.global_caret(caret))
            .map_err(|caret| paragraph.global_caret(caret))
    }

    fn caret_bounds(&self, caret: CaretPosition, line_height: Pixels) -> Option<Bounds<Pixels>> {
        if caret.index > self.len() {
            return None;
        }

        let paragraph = &self.paragraphs[self.paragraph_for_index(caret.index)];
        let mut bounds = paragraph
            .native
            .caret_bounds(paragraph.local_caret(caret), line_height)?;
        bounds.origin.y += line_height * paragraph.first_line;

        Some(bounds)
    }

    fn normalized_caret(&self, caret: CaretPosition) -> CaretPosition {
        let paragraph = &self.paragraphs[self.paragraph_for_index(caret.index)];

        paragraph.global_caret(
            paragraph
                .native
                .normalized_caret(paragraph.local_caret(caret)),
        )
    }

    fn adjacent_visual_caret(
        &self,
        caret: CaretPosition,
        direction: VisualDirection,
    ) -> Option<CaretPosition> {
        let paragraph_idx = self.paragraph_for_index(caret.index);
        let paragraph = &self.paragraphs[paragraph_idx];

        paragraph
            .native
            .adjacent_visual_caret(paragraph.local_caret(caret), direction)
            .map(|caret| paragraph.global_caret(caret))
            .or_else(|| self.adjacent_edge(paragraph_idx, direction))
    }

    fn selection_bounds(
        &self,
        byte_range: Range<usize>,
        line_height: Pixels,
    ) -> Vec<Bounds<Pixels>> {
        let mut regions = Vec::new();

        for paragraph in &self.paragraphs {
            if let Some(local) = local_range(&byte_range, &paragraph.source.content) {
                for mut bounds in paragraph.native.selection_bounds(local, line_height) {
                    bounds.origin.y += line_height * paragraph.first_line;
                    regions.push(bounds);
                }
            }

            if local_range(&byte_range, &paragraph.source.separator).is_none() {
                continue;
            }

            let line_index = paragraph.first_line + paragraph.native.line_count() - 1;
            regions.push(Bounds::from_corners(
                point(paragraph.newline.start, line_height * line_index),
                point(paragraph.newline.end, line_height * (line_index + 1)),
            ));
        }

        regions
    }

    fn inline_geometry(&self, range: Range<usize>) -> Option<Vec<InlineRangeGeometry>> {
        if range.is_empty() {
            return None;
        }

        Some(
            self.inline_geometry_for_ranges(std::slice::from_ref(&range))
                .pop()
                .unwrap(),
        )
    }

    fn inline_geometry_for_ranges(&self, ranges: &[Range<usize>]) -> Vec<Vec<InlineRangeGeometry>> {
        let mut paragraph_requests = vec![Vec::new(); self.paragraphs.len()];

        for (range_idx, range) in ranges.iter().enumerate() {
            if range.is_empty() {
                continue;
            }

            let first_paragraph = self.paragraph_for_index(range.start);

            for (paragraph_idx, paragraph) in
                self.paragraphs.iter().enumerate().skip(first_paragraph)
            {
                if paragraph.source.content.start >= range.end {
                    break;
                }

                if let Some(local) = local_range(range, &paragraph.source.content) {
                    paragraph_requests[paragraph_idx].push((range_idx, local));
                }
            }
        }

        let mut output = vec![Vec::new(); ranges.len()];

        for (paragraph, requests) in self.paragraphs.iter().zip(paragraph_requests) {
            if requests.is_empty() {
                continue;
            }

            let local_ranges = requests
                .iter()
                .map(|(_range_idx, range)| range.clone())
                .collect::<Vec<_>>();
            let local_geometry = paragraph.native.inline_geometry_for_ranges(&local_ranges);

            for ((range_idx, _range), regions) in requests.into_iter().zip(local_geometry) {
                output[range_idx].extend(regions.into_iter().map(|mut geometry| {
                    geometry.bounds.origin.y += paragraph.block_offset;
                    geometry.visual_line_index += paragraph.first_line;

                    geometry
                }));
            }
        }

        output
    }

    fn logical_cluster_before(&self, caret: CaretPosition) -> Option<Range<usize>> {
        self.graphemes
            .iter()
            .rev()
            .find(|range| range.start < caret.index)
            .cloned()
    }

    fn logical_cluster_after(&self, caret: CaretPosition) -> Option<Range<usize>> {
        self.graphemes
            .iter()
            .find(|range| range.end > caret.index)
            .cloned()
    }

    fn caret_movement(
        &self,
        caret: CaretPosition,
        movement: TextMovement,
        vertical_navigation_x: Option<Pixels>,
    ) -> CaretMovement {
        let caret = self.normalized_caret(caret);
        let direction = match movement.direction {
            Direction::Left => Some(VisualDirection::Left),
            Direction::Right => Some(VisualDirection::Right),
            _ => None,
        };

        if movement.boundary == Boundary::Cluster {
            if let Some(direction) = direction {
                return CaretMovement {
                    result: self
                        .adjacent_visual_caret(caret, direction)
                        .unwrap_or(caret),
                    vertical_navigation_x: None,
                };
            }
        }

        if movement.boundary == Boundary::Document {
            let caret = match movement.direction {
                Direction::Start => CaretPosition::attached_to_next_cluster(0),
                Direction::End => CaretPosition::attached_to_previous_cluster(self.len()),
                _ => caret,
            };

            return CaretMovement {
                result: self.normalized_caret(caret),
                vertical_navigation_x: None,
            };
        }

        if movement.boundary == Boundary::VisualLine
            && matches!(movement.direction, Direction::Up | Direction::Down)
        {
            let geometry = self.caret_bounds(caret, px(1.0)).unwrap();
            let delta = if movement.direction == Direction::Up {
                -1
            } else {
                1
            };
            let target_index = (f32::from(geometry.origin.y) as usize)
                .checked_add_signed(delta)
                .filter(|index| *index < self.line_count());
            let Some(target_index) = target_index else {
                let index = if delta < 0 { 0 } else { self.len() };

                return CaretMovement {
                    result: self.normalized_caret(CaretPosition {
                        index,
                        affinity: caret.affinity,
                    }),
                    vertical_navigation_x,
                };
            };
            let x = vertical_navigation_x.unwrap_or(geometry.origin.x);
            let moved = self
                .caret_from_pixel_point(point(x, px(target_index as f32 + 0.5)), px(1.0))
                .unwrap_or_else(|caret| caret);

            return CaretMovement {
                result: moved,
                vertical_navigation_x: Some(x),
            };
        }

        let paragraph_index = self.paragraph_for_index(caret.index);
        let paragraph = &self.paragraphs[paragraph_index];
        let local = paragraph.local_caret(caret);
        let CaretMovement {
            result: moved,
            vertical_navigation_x,
        } = paragraph
            .native
            .caret_movement(local, movement, vertical_navigation_x);

        if let Some(direction) = direction
            && paragraph.native.caret_bounds(moved, px(1.0))
                == paragraph.native.caret_bounds(local, px(1.0))
            && let Some(edge) = self.adjacent_edge(paragraph_index, direction)
        {
            return CaretMovement {
                result: edge,
                vertical_navigation_x,
            };
        }

        CaretMovement {
            result: paragraph.global_caret(moved),
            vertical_navigation_x,
        }
    }

    fn selection_from_pixel_point(
        &self,
        pixel_point: Point<Pixels>,
        line_height: Pixels,
        kind: TextSelectionKind,
    ) -> Range<usize> {
        let paragraph = self.paragraph_for_point(pixel_point, line_height);
        let local = paragraph.native.selection_from_pixel_point(
            paragraph.local_point(pixel_point, line_height),
            line_height,
            kind,
        );
        let start = local.start + paragraph.source.content.start;
        let mut end = local.end + paragraph.source.content.start;

        if matches!(
            kind,
            TextSelectionKind::VisualLine | TextSelectionKind::HardLine
        ) && end == paragraph.source.content.end
        {
            end = paragraph.source.separator.end;
        }

        start..end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_ranges_preserve_separators_empty_paragraphs_and_line_separators() {
        for (text, expected) in [
            ("", vec![(0..0, 0..0)]),
            (
                "אב\r\n\nabc\n",
                vec![
                    (0..4, 4..6),
                    (6..6, 6..7),
                    (7..10, 10..11),
                    (11..11, 11..11),
                ],
            ),
            ("a\u{2028}b\u{2029}c", vec![(0..5, 5..8), (8..9, 9..9)]),
            (
                "\r\u{0085}\u{001c}\u{001d}\u{001e}",
                vec![
                    (0..0, 0..1),
                    (1..1, 1..3),
                    (3..3, 3..4),
                    (4..4, 4..5),
                    (5..5, 5..6),
                    (6..6, 6..6),
                ],
            ),
        ] {
            let actual = paragraph_ranges(text)
                .into_iter()
                .map(|paragraph| (paragraph.content, paragraph.separator))
                .collect::<Vec<_>>();

            assert_eq!(actual, expected, "{text:?}");
        }
    }
}
