use crate::elements::div::ScrollHandle;
use crate::elements::text::{
    TruncationCandidate, text_layout_fits, truncate_with_measured_candidates,
};
use crate::{
    App, AvailableSpace, Bounds, Display, InlineBidiScope, InlineBoxRequest, InlineLayout,
    InlineLayoutRequest, InlineTextMetrics, InlineTextStyle, LayoutId, Pixels, Point, Position,
    SharedString, Size, Style, TextLayout, TextLayoutTruncation, TextRun, TextStyle, UnicodeBidi,
    Window, WindowTextSystem, place_inline_layout, size,
};

use collections::FxHashMap;
use gpui_util::ResultExt;
use smallvec::SmallVec;
use std::{
    cell::{Ref, RefCell},
    ops::Range,
    rc::Rc,
    sync::Arc,
};
use unicode_segmentation::UnicodeSegmentation;

/// Resolved content published by the element's ordinary layout request. Wrappers that return
/// the same layout ID automatically retain this content and the element's normal lifecycle.
pub(crate) enum InlineContent {
    Text {
        text: SharedString,
        runs: Arc<[TextRun]>,
        font_size: Pixels,
        line_height: Pixels,
    },

    Container {
        children: SmallVec<[LayoutId; 2]>,
    },

    /// Content with its own interaction and text layout, such as InteractiveText.
    Atomic,
}

struct InlineSpan {
    layout_id: LayoutId,
    text_range: Range<usize>,
    box_range: Range<usize>,
    direction: crate::taffy::LayoutDirectionHandle,
    unicode_bidi: UnicodeBidi,
}

#[derive(Default)]
struct InlineDocument {
    text: String,
    runs: Vec<TextRun>,
    text_styles: Vec<InlineTextStyle>,
    boxes: Vec<InlineBoxRequest>,
    box_layout_ids: Vec<LayoutId>,
    spans: Vec<InlineSpan>,
}

impl InlineDocument {
    fn bidi_scopes(&self) -> Vec<InlineBidiScope> {
        self.spans
            .iter()
            .filter(|span| span.unicode_bidi != UnicodeBidi::Normal)
            .map(|span| InlineBidiScope {
                range: span.text_range.clone(),
                direction: span.direction.get(),
                unicode_bidi: span.unicode_bidi,
            })
            .collect()
    }

    fn layout(
        self: &Rc<Self>,
        request: InlineLayoutRequest<'_>,
        truncation: &TextLayoutTruncation,
        text_system: &WindowTextSystem,
    ) -> (Rc<Self>, Arc<InlineLayout>) {
        // Removing embedded widgets also requires suppressing their painting and hit regions.
        let Some(width) = truncation.width.filter(|_width| self.boxes.is_empty()) else {
            return (self.clone(), text_system.layout_inline(request));
        };

        let max_lines = request.options.line_clamp;
        let request = InlineLayoutRequest {
            options: crate::TextLayoutOptions {
                line_clamp: None,
                ..request.options
            },
            ..request
        };
        let probe = text_system.layout_inline(request);
        let fits = |layout: &InlineLayout| text_layout_fits(&layout.layout, width, max_lines);

        if fits(&probe) {
            return (self.clone(), probe);
        }

        let boundaries = self
            .text
            .grapheme_indices(true)
            .map(|(index, _grapheme)| index)
            .chain(std::iter::once(self.text.len()));

        let (_candidate, measurement) = truncate_with_measured_candidates(
            &self.text,
            boundaries,
            None,
            &truncation.affix,
            &self.runs,
            truncation.source,
            |candidate| {
                let document = self.truncated(candidate);
                let bidi_scopes = document.bidi_scopes();
                let layout = text_system.layout_inline(InlineLayoutRequest {
                    text: &document.text,
                    runs: &document.runs,
                    text_styles: &document.text_styles,
                    bidi_scopes: &bidi_scopes,
                    ..request
                });
                let fits = fits(&layout);

                ((Rc::new(document), layout), fits)
            },
        );

        measurement
    }

    fn truncated(&self, candidate: &TruncationCandidate) -> Self {
        let text_styles = self
            .text_styles
            .iter()
            .flat_map(|style| {
                candidate
                    .display_ranges(&style.range)
                    .into_iter()
                    .map(|range| InlineTextStyle {
                        range,
                        ..style.clone()
                    })
            })
            .collect();
        let spans = self
            .spans
            .iter()
            .flat_map(|span| {
                candidate
                    .display_ranges(&span.text_range)
                    .into_iter()
                    .map(|text_range| InlineSpan {
                        layout_id: span.layout_id,
                        text_range,
                        box_range: 0..0,
                        direction: span.direction.clone(),
                        unicode_bidi: span.unicode_bidi,
                    })
            })
            .collect();

        Self {
            text: candidate.text.to_string(),
            runs: candidate.runs.clone(),
            text_styles,
            spans,
            ..Self::default()
        }
    }
}

struct InlineParagraphMeasurement {
    truncate_width: Option<Pixels>,
    options: crate::TextLayoutOptions,
    bidi_scopes: Vec<InlineBidiScope>,
    document: Rc<InlineDocument>,
    layout: Arc<InlineLayout>,
}

struct InlineParagraph {
    layout_id: LayoutId,
    document: Rc<InlineDocument>,
    measurement: Rc<RefCell<Option<InlineParagraphMeasurement>>>,
    paint_origin: Point<Pixels>,
}

impl InlineParagraph {
    fn measurement(&self) -> Option<Ref<'_, InlineParagraphMeasurement>> {
        Ref::filter_map(self.measurement.borrow(), Option::as_ref).ok()
    }
}

#[derive(Default)]
pub(super) struct InlineDivFrameState {
    paragraphs: Vec<InlineParagraph>,
    /// Text and inline containers whose bounds come from paragraph fragments.
    span_layout_ids: Vec<LayoutId>,
    /// Anonymous paragraphs and separate children, including absolute children.
    flow_child_layout_ids: Vec<LayoutId>,
}

/// Collects text, nested inline spans, and atomic boxes into paragraphs.
/// Block children finish the current paragraph and remain separate flow children.
struct InlineParagraphCollector<'a> {
    frame_state: InlineDivFrameState,
    current_document: InlineDocument,
    current_span_indices: FxHashMap<LayoutId, usize>,
    open_span_layout_ids: Vec<LayoutId>,
    text_style: TextStyle,
    unicode_bidi: UnicodeBidi,
    window: &'a mut Window,
    cx: &'a mut App,
}

impl InlineParagraphCollector<'_> {
    fn collect_element(&mut self, layout_id: LayoutId) {
        let Some((display, position)) = self.window.layout_display_and_position(layout_id) else {
            return;
        };

        if display == Display::None {
            return;
        } else if position == Position::Absolute {
            self.frame_state.flow_child_layout_ids.push(layout_id);
            return;
        }

        let content = self.window.inline_content(layout_id);

        match content.as_deref() {
            Some(InlineContent::Text {
                text,
                runs,
                font_size,
                line_height,
            }) => {
                self.frame_state.span_layout_ids.push(layout_id);

                let text_start = self.current_document.text.len();
                let box_start = self.current_document.boxes.len();

                self.current_document.text.push_str(text);
                self.current_document.runs.extend(runs.iter().cloned());
                self.current_document.text_styles.push(InlineTextStyle {
                    range: text_start..self.current_document.text.len(),
                    font_size: *font_size,
                    line_height: *line_height,
                });

                self.record_span_ranges(layout_id, text_start, box_start);
                self.record_open_span_ranges(text_start, box_start);
            }

            Some(InlineContent::Container { children }) if display == Display::Inline => {
                self.frame_state.span_layout_ids.push(layout_id);
                self.open_span_layout_ids.push(layout_id);

                for child in children {
                    self.collect_element(*child);
                }

                self.open_span_layout_ids.pop();
            }

            _ if !matches!(display, Display::Inline | Display::InlineFlex) => {
                self.finish_paragraph();
                self.frame_state.flow_child_layout_ids.push(layout_id);
            }

            _ => {
                // Measure atomic contents before Taffy enters a measurement callback. The
                // layout engine is temporarily absent from Window during those callbacks.
                self.window.compute_layout(
                    layout_id,
                    size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                    self.cx,
                );

                let bounds = self.window.layout_bounds(layout_id);
                let box_start = self.current_document.boxes.len();
                let text_start = self.current_document.text.len();

                self.current_document.boxes.push(InlineBoxRequest {
                    id: box_start as u64,
                    index: text_start,
                    size: bounds.size,
                    vertical_align: self
                        .window
                        .layout_vertical_align(layout_id)
                        .unwrap_or_default(),
                });

                self.current_document.box_layout_ids.push(layout_id);
                self.record_open_span_ranges(text_start, box_start);
            }
        }
    }

    fn record_open_span_ranges(&mut self, text_start: usize, box_start: usize) {
        for index in 0..self.open_span_layout_ids.len() {
            self.record_span_ranges(self.open_span_layout_ids[index], text_start, box_start);
        }
    }

    fn record_span_ranges(&mut self, layout_id: LayoutId, text_start: usize, box_start: usize) {
        let text_end = self.current_document.text.len();
        let box_end = self.current_document.boxes.len();

        if let Some(span_idx) = self.current_span_indices.get(&layout_id).copied() {
            let span = &mut self.current_document.spans[span_idx];
            span.text_range.end = text_end;
            span.box_range.end = box_end;
        } else {
            self.current_span_indices
                .insert(layout_id, self.current_document.spans.len());
            self.current_document.spans.push(InlineSpan {
                layout_id,
                text_range: text_start..text_end,
                box_range: box_start..box_end,
                direction: self.window.layout_direction_handle(layout_id),
                unicode_bidi: self.window.layout_directionality(layout_id).1,
            });
        }
    }

    fn finish_paragraph(&mut self) {
        if self.current_document.text.is_empty() && self.current_document.boxes.is_empty() {
            return;
        }

        let document = Rc::new(std::mem::take(&mut self.current_document));
        self.current_span_indices.clear();
        let measurement = Rc::new(RefCell::new(None));

        let text_style = self.text_style.clone();
        let font_size = text_style.font_size.to_pixels(self.window.rem_size());
        let line_height = self.window.pixel_snap(
            text_style
                .line_height
                .to_pixels(font_size.into(), self.window.rem_size()),
        );

        let font_id = self.window.text_system().resolve_font(&text_style.font());

        let text_metrics = InlineTextMetrics {
            ascent: self.window.text_system().ascent(font_id, font_size),
            descent: self.window.text_system().descent(font_id, font_size),
            x_height: self.window.text_system().x_height(font_id, font_size),
        };

        let measured_document = document.clone();
        let measurement_cache = measurement.clone();
        let unicode_bidi = self.unicode_bidi;

        let layout_id = self.window.request_measured_layout(
            Style {
                display: Display::Block,
                ..Style::default()
            },
            move |known_dimensions, available_space, window, _cx| {
                let (options, truncation) = TextLayout::layout_options(
                    &text_style,
                    known_dimensions,
                    available_space,
                    window.resolved_direction(),
                    unicode_bidi,
                );

                let bidi_scopes = measured_document.bidi_scopes();

                if let Some(measurement) =
                    measurement_cache.borrow().as_ref() as Option<&InlineParagraphMeasurement>
                    && measurement.truncate_width == truncation.width
                    && measurement.options == options
                    && measurement.bidi_scopes == bidi_scopes
                {
                    return measurement.layout.size;
                }

                let request = InlineLayoutRequest {
                    text: &measured_document.text,
                    runs: &measured_document.runs,
                    text_styles: &measured_document.text_styles,
                    boxes: &measured_document.boxes,
                    font_size,
                    line_height,
                    text_metrics,
                    options,
                    bidi_scopes: &bidi_scopes,
                };
                let (document, layout) =
                    measured_document.layout(request, &truncation, window.text_system());

                let size = layout.size;
                measurement_cache
                    .borrow_mut()
                    .replace(InlineParagraphMeasurement {
                        truncate_width: truncation.width,
                        options,
                        bidi_scopes,
                        document,
                        layout,
                    });

                size
            },
        );

        self.frame_state.flow_child_layout_ids.push(layout_id);
        self.frame_state.paragraphs.push(InlineParagraph {
            layout_id,
            document,
            measurement,
            paint_origin: Point::default(),
        });
    }
}

impl InlineDivFrameState {
    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn measured_paragraphs(&self) -> Vec<(SharedString, Arc<InlineLayout>)> {
        self.paragraphs
            .iter()
            .filter_map(|paragraph| {
                paragraph.measurement().map(|measurement| {
                    (
                        measurement.document.text.clone().into(),
                        measurement.layout.clone(),
                    )
                })
            })
            .collect()
    }

    pub(super) fn request_layout(
        style: &Style,
        children: &[LayoutId],
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self) {
        let mut paragraph_collector = InlineParagraphCollector {
            frame_state: Self::default(),
            current_document: InlineDocument::default(),
            current_span_indices: FxHashMap::default(),
            open_span_layout_ids: Vec::new(),
            text_style: window.text_style(),
            unicode_bidi: style.effective_unicode_bidi(),
            window,
            cx,
        };

        for child in children {
            paragraph_collector.collect_element(*child);
        }

        paragraph_collector.finish_paragraph();

        let node_id = paragraph_collector.window.request_layout(
            style.clone(),
            paragraph_collector
                .frame_state
                .flow_child_layout_ids
                .iter()
                .copied(),
            paragraph_collector.cx,
        );

        (node_id, paragraph_collector.frame_state)
    }

    pub(super) fn prepare_layout(
        &self,
        bounds: Bounds<Pixels>,
        scroll_handle: Option<&ScrollHandle>,
        children: &[LayoutId],
        window: &mut Window,
    ) -> Size<Pixels> {
        let mut fragments: FxHashMap<LayoutId, Vec<Bounds<Pixels>>> = self
            .span_layout_ids
            .iter()
            .map(|node_id| (*node_id, Vec::new()))
            .collect();

        for paragraph in &self.paragraphs {
            let Some(measurement) = paragraph.measurement() else {
                continue;
            };
            let origin = window.layout_bounds(paragraph.layout_id).origin;
            let layout = &measurement.layout;
            let document = &measurement.document;

            let placement = place_inline_layout(origin, layout.alignment_offset, window);
            let origin = origin + placement.delta;

            for inline_box in &layout.boxes {
                window.place_inline(
                    paragraph.document.box_layout_ids[inline_box.id as usize],
                    Bounds::new(
                        window.pixel_snap_point(origin + inline_box.bounds.origin),
                        inline_box.bounds.size,
                    ),
                    None,
                );
            }

            let text_ranges = document
                .spans
                .iter()
                .map(|span| span.text_range.clone())
                .collect::<Vec<_>>();
            let text_geometry = layout
                .layout
                .platform_layout
                .inline_geometry_for_ranges(&text_ranges);
            let mut boxes_by_id = vec![None; paragraph.document.box_layout_ids.len()];

            for inline_box in &layout.boxes {
                if let Some(slot) = boxes_by_id.get_mut(inline_box.id as usize) {
                    *slot = Some(inline_box);
                }
            }

            for (span, text_regions) in document.spans.iter().zip(text_geometry) {
                let regions = fragments.get_mut(&span.layout_id).unwrap();

                for geometry in text_regions {
                    regions.push(Bounds::new(
                        origin + geometry.bounds.origin,
                        geometry.bounds.size,
                    ));
                }

                for box_index in span.box_range.clone() {
                    if let Some(inline_box) = boxes_by_id.get(box_index).and_then(|slot| *slot) {
                        regions.push(Bounds::new(
                            origin + inline_box.bounds.origin,
                            inline_box.bounds.size,
                        ));
                    }
                }
            }
        }

        for (node_id, mut regions) in fragments {
            for region in &mut regions {
                *region = Bounds::from_corners(
                    window.pixel_snap_point(region.origin),
                    window.pixel_snap_point(region.bottom_right()),
                );
            }

            merge_fragments(&mut regions);

            let union = regions
                .iter()
                .copied()
                .reduce(|left, right| left.union(&right))
                .unwrap_or_else(|| Bounds::new(bounds.origin, Size::default()));

            window.place_inline(node_id, union, Some(regions));
        }

        if let Some(scroll_handle) = scroll_handle {
            scroll_handle.0.borrow_mut().child_bounds = children
                .iter()
                .map(|node_id| window.layout_bounds(*node_id))
                .collect();
        }

        self.flow_child_layout_ids
            .iter()
            .map(|node_id| window.layout_bounds(*node_id))
            .reduce(|left, right| left.union(&right))
            .map_or(Size::default(), |right| right.size)
    }

    pub(super) fn record_paragraph_origins(&mut self, window: &mut Window) {
        for paragraph in &mut self.paragraphs {
            paragraph.paint_origin = window.layout_bounds(paragraph.layout_id).origin;
        }
    }

    pub(super) fn paint_paragraphs(&self, window: &mut Window, cx: &mut App) {
        for paragraph in &self.paragraphs {
            let Some(measurement) = paragraph.measurement() else {
                continue;
            };
            let layout = &measurement.layout;

            layout
                .paint_background(paragraph.paint_origin, window, cx)
                .log_err();

            layout.paint(paragraph.paint_origin, window, cx).log_err();
        }
    }
}

fn merge_fragments(regions: &mut Vec<Bounds<Pixels>>) {
    regions.sort_by(|left, right| {
        left.origin
            .y
            .partial_cmp(&right.origin.y)
            .unwrap()
            .then_with(|| left.origin.x.partial_cmp(&right.origin.x).unwrap())
    });

    let mut merged: Vec<Bounds<Pixels>> = Vec::with_capacity(regions.len());

    for region in regions.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.origin.y == region.origin.y
            && last.size.height == region.size.height
            && region.origin.x <= last.right()
        {
            *last = last.union(&region);
        } else {
            merged.push(region);
        }
    }

    *regions = merged;
}
