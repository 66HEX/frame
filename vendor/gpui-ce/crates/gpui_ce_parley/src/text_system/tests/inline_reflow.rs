use super::*;
use gpui::FontWidth;
use gpui_fonts::NOTO_SANS;

const SCALE_FACTOR: f32 = 1.5;

const INLINE: &str = "inline-translation-root";
const FIRST_BOX: &str = "inline-translation-first-box";
const SECOND_BOX: &str = "inline-translation-second-box";
const NESTED: &str = "inline-translation-nested";

fn first_group_color() -> Hsla {
    hsla(0.02, 0.75, 0.55, 1.0)
}

fn second_group_color() -> Hsla {
    hsla(0.58, 0.75, 0.55, 1.0)
}

fn highlighted(text: &'static str, color: Hsla) -> StyledText {
    StyledText::new(text).with_highlights([(
        0..text.len(),
        HighlightStyle {
            background_color: Some(color),
            ..Default::default()
        },
    )])
}

struct InlineTranslationView {
    width_offset: f32,
}

impl Render for InlineTranslationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().pl(px(11.)).pt(px(9.)).child(
            div()
                .block()
                .w(px(160. + self.width_offset))
                .p(px(7.))
                .border_1()
                .text_size(px(17.))
                .line_height(px(23.))
                .text_center()
                .debug_selector(|| INLINE.into())
                .child(highlighted("short line ", first_group_color()))
                .child(
                    div()
                        .inline_flex()
                        .w(px(13.))
                        .h(px(16.))
                        .align_middle()
                        .debug_selector(|| FIRST_BOX.into()),
                )
                .child("\n")
                .child(highlighted("secondline ", second_group_color()))
                .child(
                    div()
                        .inline_flex()
                        .items_center()
                        .justify_center()
                        .w(px(29.))
                        .h(px(16.))
                        .align_middle()
                        .debug_selector(|| SECOND_BOX.into())
                        .child(div().w(px(5.)).h(px(7.)).debug_selector(|| NESTED.into())),
                ),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    inline: Bounds<Pixels>,
    text_groups: [Bounds<Pixels>; 2],
    first_box: Bounds<Pixels>,
    second_box: Bounds<Pixels>,
    nested: Bounds<Pixels>,
}

impl Geometry {
    fn read(cx: &mut HeadlessAppContext, window: WindowHandle<InlineTranslationView>) -> Self {
        let any_window = window.into();
        let mut debug_bounds = |selector| {
            cx.debug_bounds(any_window, selector)
                .unwrap()
                .unwrap_or_else(|| panic!("missing debug bounds for {selector}"))
        };

        let inline = debug_bounds(INLINE);
        let first_box = debug_bounds(FIRST_BOX);
        let second_box = debug_bounds(SECOND_BOX);
        let nested = debug_bounds(NESTED);

        let text_groups = [
            text_group_bounds(cx, any_window, first_group_color()),
            text_group_bounds(cx, any_window, second_group_color()),
        ];

        Self {
            inline,
            text_groups,
            first_box,
            second_box,
            nested,
        }
    }

    fn participants(self) -> [Bounds<Pixels>; 5] {
        [
            self.text_groups[0],
            self.text_groups[1],
            self.first_box,
            self.second_box,
            self.nested,
        ]
    }

    fn nested_offset(self) -> Point<Pixels> {
        self.nested.origin - self.second_box.origin
    }

    fn assert_valid(self) {
        let epsilon = px(1.);

        assert!(self.text_groups[0].right() <= self.first_box.origin.x + epsilon);
        assert!(self.text_groups[1].right() <= self.second_box.origin.x + epsilon);
        assert!(self.text_groups[0].origin.y < self.text_groups[1].origin.y);

        for participant in self.participants() {
            assert!(participant.origin.x >= self.inline.origin.x);
            assert!(participant.origin.y >= self.inline.origin.y);
            assert!(participant.right() <= self.inline.right());
            assert!(participant.bottom() <= self.inline.bottom());
        }

        assert!(self.nested.origin.x >= self.second_box.origin.x);
        assert!(self.nested.origin.y >= self.second_box.origin.y);
        assert!(self.nested.right() <= self.second_box.right());
        assert!(self.nested.bottom() <= self.second_box.bottom());
    }

    fn assert_moved_as_one_group(self, before: Self, step: usize) {
        let expected_delta = self.text_groups[0].origin - before.text_groups[0].origin;

        for (participant, (actual, previous)) in self
            .participants()
            .into_iter()
            .zip(before.participants())
            .enumerate()
        {
            assert_point_close(
                actual.origin - previous.origin,
                expected_delta,
                &format!("participant {participant} at resize step {step}"),
            );
        }
    }
}

fn text_group_bounds(
    cx: &mut HeadlessAppContext,
    window: gpui::AnyWindowHandle,
    color: Hsla,
) -> Bounds<Pixels> {
    let bounds = cx.solid_quad_bounds(window, color).unwrap();
    assert_eq!(bounds.len(), 1, "expected one rendered quad for {color:?}");

    logical_bounds(bounds[0])
}

fn logical_bounds(bounds: Bounds<ScaledPixels>) -> Bounds<Pixels> {
    bounds.map(|value| px(value.as_f32() / SCALE_FACTOR))
}

fn assert_close(actual: Pixels, expected: Pixels, context: &str) {
    assert!(
        (actual - expected).abs() < px(0.01),
        "{context}: expected {actual:?} to equal {expected:?}"
    );
}

fn assert_point_close(actual: Point<Pixels>, expected: Point<Pixels>, context: &str) {
    assert_close(actual.x, expected.x, context);
    assert_close(actual.y, expected.y, context);
}

fn update_view<View: Render>(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<View>,
    edit: impl FnOnce(&mut View),
) {
    window
        .update(cx, |view, _window, cx| {
            edit(view);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
}

#[test]
fn centered_inline_lines_move_as_one_group_during_fractional_resize() {
    let text_system = ParleyTextSystem::new_with_system_font(SystemFonts::Skip, IBM_PLEX.family);
    text_system
        .add_fonts(vec![Cow::Borrowed(IBM_PLEX.data)])
        .unwrap();

    let mut cx = HeadlessAppContext::new(Arc::new(text_system));

    let window = cx
        .open_window(size(px(340.), px(180.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| InlineTranslationView { width_offset: 0. })
        })
        .unwrap();

    cx.run_until_parked();

    let initial = Geometry::read(&mut cx, window);
    initial.assert_valid();

    for step in 1..=64 {
        update_view(&mut cx, window, |view| {
            view.width_offset = step as f32 / 16.;
        });

        let translated = Geometry::read(&mut cx, window);
        translated.assert_valid();
        translated.assert_moved_as_one_group(initial, step);
        assert_point_close(
            translated.nested_offset(),
            initial.nested_offset(),
            &format!("nested offset at resize step {step}"),
        );
    }

    update_view(&mut cx, window, |view| view.width_offset = 0.);
    assert_eq!(Geometry::read(&mut cx, window), initial);
}

fn headless() -> HeadlessAppContext {
    let system = ParleyTextSystem::new_with_system_font(SystemFonts::Skip, IBM_PLEX.family);
    system
        .add_fonts(vec![
            Cow::Borrowed(IBM_PLEX.data),
            Cow::Borrowed(IBM_PLEX_SEMIBOLD.data),
            Cow::Borrowed(NOTO_SANS.data),
        ])
        .unwrap();
    HeadlessAppContext::new(Arc::new(system))
}

type CapturedParagraphs = Rc<RefCell<Vec<(gpui::SharedString, Arc<InlineLayout>)>>>;

struct ParagraphProbe {
    element: gpui::Div,
    painted: CapturedParagraphs,
    measurements: CapturedParagraphs,
    remeasure: bool,
}

impl IntoElement for ParagraphProbe {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for ParagraphProbe {
    type RequestLayoutState = gpui::DivFrameState;
    type PrepaintState = <gpui::Div as gpui::Element>::PrepaintState;

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let (layout_id, state) = self
            .element
            .request_layout(global_id, inspector_id, window, cx);

        if self.remeasure {
            self.measurements.borrow_mut().clear();

            for width in [80., 200., 2000., 80.] {
                window.compute_layout(
                    layout_id,
                    size(
                        gpui::AvailableSpace::Definite(px(width)),
                        gpui::AvailableSpace::MaxContent,
                    ),
                    cx,
                );
                self.measurements
                    .borrow_mut()
                    .extend(state.measured_inline_paragraphs());
            }
        }

        (layout_id, state)
    }

    fn prepaint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> Self::PrepaintState {
        self.element
            .prepaint(global_id, inspector_id, bounds, state, window, cx)
    }

    fn paint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        *self.painted.borrow_mut() = state.measured_inline_paragraphs();
        self.element
            .paint(global_id, inspector_id, bounds, state, prepaint, window, cx);
    }
}

const OVERFLOW_TEXT: &str =
    "Begin café e\u{301} 👩‍👩‍👧‍👦 and several words that need room before the ending";
const OVERFLOW_MIDDLE: &str = "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW";
const LINE_CLAMP_TEXT: &str = "A paragraph can wrap at word boundaries while keeping styled text, punctuation, and spacing together. Resize the window and this sample will follow the available width.";

struct OverflowParagraph {
    text: &'static str,
    font: Font,
    width: f32,
    direction: gpui::TruncateFrom,
    max_lines: Option<usize>,
    styled: bool,
    marker: bool,
    remeasure: bool,
    painted: CapturedParagraphs,
    measurements: CapturedParagraphs,
}

impl OverflowParagraph {
    fn new(direction: gpui::TruncateFrom) -> Self {
        Self {
            text: OVERFLOW_TEXT,
            font: font(IBM_PLEX.family),
            width: 180.,
            direction,
            max_lines: None,
            styled: false,
            marker: true,
            remeasure: false,
            painted: Rc::default(),
            measurements: Rc::default(),
        }
    }
}

impl Render for OverflowParagraph {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut paragraph = div()
            .block()
            .w_full()
            .min_w_0()
            .font(self.font.clone())
            .text_size(px(14.))
            .line_height(px(24.))
            .text_color(first_group_color());

        if self.marker {
            paragraph = match self.direction {
                gpui::TruncateFrom::End => paragraph.text_ellipsis(),
                gpui::TruncateFrom::Start => paragraph.text_ellipsis_start(),
                gpui::TruncateFrom::Middle => paragraph.text_ellipsis_middle(),
            };
        }

        paragraph = if let Some(count) = self.max_lines {
            paragraph.line_clamp(count)
        } else {
            paragraph.whitespace_nowrap()
        };

        paragraph = if self.styled {
            paragraph.child(
                div()
                    .inline()
                    .bg(first_group_color())
                    .child("begin ")
                    .child(
                        div()
                            .inline()
                            .text_size(px(30.))
                            .line_height(px(46.))
                            .text_color(second_group_color())
                            .bg(second_group_color())
                            .child(OVERFLOW_MIDDLE),
                    )
                    .child(" end"),
            )
        } else {
            paragraph.child(self.text)
        };

        div()
            .size_full()
            .items_start()
            .child(div().w(px(self.width)).child(ParagraphProbe {
                element: paragraph,
                painted: self.painted.clone(),
                measurements: self.measurements.clone(),
                remeasure: self.remeasure,
            }))
    }
}

fn assert_paragraph_fits(text: &str, layout: &InlineLayout, width: f32, max_lines: usize) {
    assert_eq!(layout.layout.len, text.len());
    assert!(
        layout.lines.len() <= max_lines,
        "{text:?}: {:?}",
        layout.lines
    );
    assert!(
        layout.layout.platform_layout.size().width <= px(width + 0.01),
        "{text:?}: {:?}",
        layout.layout.visual_lines
    );
}

fn open_overflow_paragraph(
    cx: &mut HeadlessAppContext,
    view: OverflowParagraph,
    viewport_width: f32,
    scale: f32,
) -> (WindowHandle<OverflowParagraph>, CapturedParagraphs) {
    let painted = view.painted.clone();
    let window = cx
        .open_window(size(px(viewport_width), px(180.)), |window, cx| {
            window.set_scale_factor(scale);

            cx.new(|_cx| view)
        })
        .unwrap();
    cx.run_until_parked();

    (window, painted)
}

#[test]
fn block_overflow_preserves_graphemes_marker_rows_and_unmarked_source_text() {
    let mut cx = headless();
    let boundaries = OVERFLOW_TEXT
        .grapheme_indices(true)
        .map(|(idx, _grapheme)| idx)
        .chain([OVERFLOW_TEXT.len()])
        .collect::<Vec<_>>();

    for (direction, max_lines, width, scale) in [
        (gpui::TruncateFrom::End, None, 180., SCALE_FACTOR),
        (gpui::TruncateFrom::Start, None, 180., SCALE_FACTOR),
        (gpui::TruncateFrom::Middle, None, 180., SCALE_FACTOR),
        (gpui::TruncateFrom::End, Some(2), 140., 1.0),
    ] {
        let mut view = OverflowParagraph::new(direction);
        view.max_lines = max_lines;
        view.width = width;
        let (window, painted) = open_overflow_paragraph(&mut cx, view, 360., scale);

        {
            let captured = painted.borrow();
            assert_eq!(captured.len(), 1);
            let (text, layout) = &captured[0];
            let expected_lines = max_lines.unwrap_or(1);
            assert_paragraph_fits(text, layout, width, expected_lines);
            assert_eq!(layout.lines.len(), expected_lines);
            let (prefix, suffix) = text
                .split_once('…')
                .expect("paragraph must paint the marker");
            assert!(OVERFLOW_TEXT.starts_with(prefix) && OVERFLOW_TEXT.ends_with(suffix));
            assert!(boundaries.contains(&prefix.len()));
            assert!(boundaries.contains(&(OVERFLOW_TEXT.len() - suffix.len())));

            match direction {
                gpui::TruncateFrom::End => assert!(suffix.is_empty() && !prefix.is_empty()),
                gpui::TruncateFrom::Start => assert!(prefix.is_empty() && !suffix.is_empty()),
                gpui::TruncateFrom::Middle => assert!(!prefix.is_empty() && !suffix.is_empty()),
            }

            if max_lines == Some(2) {
                let marker = text.len() - '…'.len_utf8()..text.len();
                assert!(
                    layout.layout.visual_lines[1]
                        .text_range
                        .contains(&marker.start)
                );
                let geometry = layout
                    .layout
                    .platform_layout
                    .inline_geometry_for_ranges(&[marker]);
                assert!(!geometry[0].is_empty());
                assert!(
                    geometry[0]
                        .iter()
                        .all(|geometry| geometry.visual_line_index == 1)
                );
            }

            assert!(
                !cx.glyph_bounds(window.into(), first_group_color())
                    .unwrap()
                    .is_empty()
            );
        }

        update_view(&mut cx, window, |view| view.marker = false);
        let captured = painted.borrow();
        assert_eq!(captured[0].0.as_ref(), OVERFLOW_TEXT);
        assert_eq!(captured[0].1.lines.len(), max_lines.unwrap_or(1));
    }
}

#[test]
fn block_line_clamp_ignores_wrapped_trailing_whitespace() {
    let system = test_system();
    let runs = [text_run(LINE_CLAMP_TEXT, "IBM Plex Sans")];
    let width = (300..800)
        .map(|width| px(width as f32))
        .find(|width| {
            let layout = system.layout_text(TextLayoutRequest {
                text: LINE_CLAMP_TEXT,
                font_size: px(14.),
                runs: &runs,
                options: TextLayoutOptions {
                    wrap_width: Some(*width),
                    text_align: TextAlign::Left,
                    ..Default::default()
                },
            });

            layout.visual_lines.len() == 2
                && layout.platform_layout.size().width <= *width + px(0.01)
                && layout
                    .visual_lines
                    .iter()
                    .any(|line| line.advance_width > *width + px(0.01))
        })
        .expect("sample text should have a two-line width with hanging whitespace");
    let mut cx = headless();
    let mut view = OverflowParagraph::new(gpui::TruncateFrom::End);
    view.text = LINE_CLAMP_TEXT;
    view.max_lines = Some(2);
    view.width = f32::from(width);
    let (_window, painted) = open_overflow_paragraph(&mut cx, view, 900., 1.0);

    let captured = painted.borrow();
    let (text, layout) = &captured[0];
    assert_eq!(text.as_ref(), LINE_CLAMP_TEXT);
    assert_paragraph_fits(text, layout, f32::from(width), 2);
}

#[test]
fn block_paragraph_remeasures_nowrap_overflow_and_restores_original_text() {
    let mut cx = headless();
    let mut view = OverflowParagraph::new(gpui::TruncateFrom::Middle);
    view.remeasure = true;
    let measurements = view.measurements.clone();
    let (window, painted) = open_overflow_paragraph(&mut cx, view, 2200., 1.0);

    {
        let measured = measurements.borrow();
        assert_eq!(measured.len(), 4);
        assert!(measured[0].0.len() < measured[1].0.len());
        assert_eq!(measured[2].0.as_ref(), OVERFLOW_TEXT);
        assert_eq!(measured[0].0, measured[3].0);

        for ((text, layout), width) in measured.iter().zip([80., 200., 2000., 80.]) {
            assert_paragraph_fits(text, layout, width, 1);
        }
    }

    let initial = painted.borrow()[0].0.clone();

    for width in [90., 2000., 180.] {
        update_view(&mut cx, window, |view| view.width = width);
        let captured = painted.borrow();
        assert_paragraph_fits(&captured[0].0, &captured[0].1, width, 1);

        if width == 2000. {
            assert_eq!(captured[0].0.as_ref(), OVERFLOW_TEXT);
        }
    }

    assert_eq!(painted.borrow()[0].0, initial);

    update_view(&mut cx, window, |view| {
        view.text = "hello gpui-ce";
        view.font = font(NOTO_SANS.family);
        view.width = 500.;
    });
    let normal = painted.borrow()[0].1.size.width;
    update_view(&mut cx, window, |view| {
        view.font.width = FontWidth::CONDENSED
    });
    let condensed = painted.borrow()[0].1.size.width;
    let available_width = f32::from((normal + condensed) / 2.0);
    assert!(condensed < normal);

    for width in [
        FontWidth::CONDENSED,
        FontWidth::NORMAL,
        FontWidth::CONDENSED,
    ] {
        update_view(&mut cx, window, |view| {
            view.font.width = width;
            view.width = available_width;
        });
        let captured = painted.borrow();
        let (text, layout) = &captured[0];
        assert_paragraph_fits(text, layout, available_width, 1);
        assert_eq!(
            text.as_ref() == "hello gpui-ce",
            width == FontWidth::CONDENSED
        );
    }
}

#[test]
fn block_ellipsis_preserves_span_metrics_and_only_places_display_ranges() {
    let mut cx = headless();
    let mut view = OverflowParagraph::new(gpui::TruncateFrom::Middle);
    view.styled = true;
    view.width = 260.;
    let (window, painted) = open_overflow_paragraph(&mut cx, view, 360., SCALE_FACTOR);

    for width in [260., 100., 260.] {
        update_view(&mut cx, window, |view| view.width = width);

        let captured = painted.borrow();
        let (text, layout) = &captured[0];
        assert_paragraph_fits(text, layout, width, 1);
        assert!(text.contains('…') && text.ends_with("end"));
        let spans = quads(&mut cx, window.into(), first_group_color());
        assert_eq!(spans.len(), 1);
        assert_close(
            spans[0].size.width,
            px((f32::from(layout.size.width) * SCALE_FACTOR).round() / SCALE_FACTOR),
            "outer span covers the displayed prefix and suffix",
        );
        assert_close(
            spans[0].size.height,
            px((f32::from(layout.size.height) * SCALE_FACTOR).round() / SCALE_FACTOR),
            "outer span uses the displayed line height",
        );

        if width == 100. {
            assert!(!text.contains('W'));
            assert!(quads(&mut cx, window.into(), second_group_color()).is_empty());
            assert!(
                cx.glyph_bounds(window.into(), second_group_color())
                    .unwrap()
                    .is_empty()
            );
            assert!(layout.size.height < px(46.));
        } else {
            assert!(layout.size.height >= px(46.));
            assert!(
                layout
                    .layout
                    .paint_fragments
                    .iter()
                    .any(|fragment| fragment.font_size == px(30.))
            );
            assert!(
                layout
                    .layout
                    .paint_fragments
                    .iter()
                    .any(|fragment| fragment.font_size == px(14.))
            );
            let backgrounds = quads(&mut cx, window.into(), second_group_color());
            assert!(!backgrounds.is_empty());

            for glyph in cx
                .glyph_bounds(window.into(), second_group_color())
                .unwrap()
            {
                assert!(
                    backgrounds
                        .iter()
                        .any(|bounds| bounds.contains(&logical_bounds(glyph).center()))
                );
            }
        }
    }
}

fn quads(
    cx: &mut HeadlessAppContext,
    window: gpui::AnyWindowHandle,
    color: Hsla,
) -> Vec<Bounds<Pixels>> {
    let mut bounds: Vec<_> = cx
        .solid_quad_bounds(window, color)
        .unwrap()
        .into_iter()
        .map(logical_bounds)
        .collect();
    bounds.sort_by(|left, right| {
        left.origin
            .y
            .partial_cmp(&right.origin.y)
            .unwrap()
            .then_with(|| left.origin.x.partial_cmp(&right.origin.x).unwrap())
    });

    bounds
}

struct NestedWrapping {
    width: f32,
    font_width: FontWidth,
    flat: bool,
}

const PREFIX: &str = "Before inter";
const SPAN: &str = "national café e\u{301} words ";
const INNER: &str = "can wrap across lines";
const SUFFIX: &str = " after.";

impl Render for NestedWrapping {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut paragraph = div()
            .block()
            .w(px(self.width))
            .font_family(NOTO_SANS.family)
            .font_width(self.font_width)
            .text_size(px(17.))
            .line_height(px(23.))
            .debug_selector(|| "paragraph".into());

        if self.flat {
            let text = format!("{PREFIX}{SPAN}{INNER}{SUFFIX}");
            paragraph = paragraph.child(StyledText::new(text).with_highlights([
                (
                    PREFIX.len()..PREFIX.len() + SPAN.len(),
                    HighlightStyle {
                        color: Some(first_group_color()),
                        background_color: Some(first_group_color()),
                        ..Default::default()
                    },
                ),
                (
                    PREFIX.len() + SPAN.len()..PREFIX.len() + SPAN.len() + INNER.len(),
                    HighlightStyle {
                        color: Some(second_group_color()),
                        background_color: Some(second_group_color()),
                        font_weight: Some(GpuiFontWeight::SEMIBOLD),
                        font_width: Some(FontWidth::NORMAL),
                        ..Default::default()
                    },
                ),
            ]));
        } else {
            paragraph = paragraph
                .child(PREFIX)
                .child(
                    div()
                        .inline()
                        .text_color(first_group_color())
                        .text_bg(first_group_color())
                        .child(SPAN)
                        .child(
                            div()
                                .inline()
                                .font_weight(GpuiFontWeight::SEMIBOLD)
                                .font_width(FontWidth::NORMAL)
                                .text_color(second_group_color())
                                .text_bg(second_group_color())
                                .child(INNER),
                        ),
                )
                .child(SUFFIX);
        }

        div().size_full().items_start().child(paragraph)
    }
}

#[test]
fn nested_spans_wrap_like_flat_styled_text_without_boundary_breaks() {
    let mut cx = headless();
    let nested = cx
        .open_window(size(px(360.), px(300.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| NestedWrapping {
                width: 130.,
                font_width: FontWidth::NORMAL,
                flat: false,
            })
        })
        .unwrap();
    let flat = cx
        .open_window(size(px(360.), px(300.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| NestedWrapping {
                width: 130.,
                font_width: FontWidth::NORMAL,
                flat: true,
            })
        })
        .unwrap();

    for (width, font_width) in [
        (130., FontWidth::NORMAL),
        (130., FontWidth::CONDENSED),
        (130., FontWidth::NORMAL),
        (179., FontWidth::NORMAL),
        (240., FontWidth::CONDENSED),
        (310., FontWidth::NORMAL),
    ] {
        for window in [nested, flat] {
            window
                .update(&mut cx, |view, _, cx| {
                    view.width = width;
                    view.font_width = font_width;
                    cx.notify();
                })
                .unwrap();
        }

        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds(nested.into(), "paragraph").unwrap(),
            cx.debug_bounds(flat.into(), "paragraph").unwrap()
        );

        for color in [first_group_color(), second_group_color()] {
            assert_eq!(
                quads(&mut cx, nested.into(), color),
                quads(&mut cx, flat.into(), color),
                "width {width}"
            );
        }
    }
}

#[derive(Clone, Copy)]
enum ParentContext {
    Block,
    Default,
    Flex,
    Grid,
}

struct ContextView {
    context: ParentContext,
}

impl Render for ContextView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut parent = div()
            .w(px(220.))
            .debug_selector(|| "parent".into())
            .text_size(px(16.))
            .line_height(px(24.));

        parent = match self.context {
            ParentContext::Block => parent.block(),
            ParentContext::Default => parent,
            ParentContext::Flex => parent.flex(),
            ParentContext::Grid => parent.grid().grid_cols(2),
        };

        div().size_full().items_start().child(
            parent
                .child(
                    div()
                        .inline()
                        .w(px(80.))
                        .h(px(31.))
                        .debug_selector(|| "context-inline".into())
                        .child("one two three four five six seven"),
                )
                .child(
                    div()
                        .inline_flex()
                        .w(px(20.))
                        .h(px(14.))
                        .debug_selector(|| "context-badge".into()),
                ),
        )
    }
}

#[test]
fn inline_sizing_depends_on_parent_context() {
    let mut cx = headless();

    for parent_context in [
        ParentContext::Block,
        ParentContext::Default,
        ParentContext::Flex,
        ParentContext::Grid,
    ] {
        let window = cx
            .open_window(size(px(300.), px(240.)), |window, cx| {
                window.set_scale_factor(SCALE_FACTOR);
                cx.new(|_| ContextView {
                    context: parent_context,
                })
            })
            .unwrap();
        cx.run_until_parked();

        let span = cx
            .debug_bounds(window.into(), "context-inline")
            .unwrap()
            .unwrap();
        let badge = cx
            .debug_bounds(window.into(), "context-badge")
            .unwrap()
            .unwrap();

        match parent_context {
            ParentContext::Block | ParentContext::Default => {
                assert!(span.size.width > px(80.));
                assert!(span.size.height >= px(48.));
                assert!(badge.origin.y > span.origin.y);
            }

            ParentContext::Flex | ParentContext::Grid => {
                assert_close(span.size.width, px(80.), "independent inline width");
                assert!((span.size.height - px(31.)).abs() < px(1.));
                assert!(badge.origin.x >= span.right());
            }
        }
    }
}

struct MixedFlow;

impl Render for MixedFlow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().items_start().child(
            div()
                .block()
                .w(px(220.))
                .text_size(px(16.))
                .line_height(px(24.))
                .debug_selector(|| "mixed".into())
                .child("before ")
                .child(
                    div()
                        .inline()
                        .bg(first_group_color())
                        .debug_selector(|| "split-span".into())
                        .child("first")
                        .child(
                            div()
                                .block()
                                .h(px(30.))
                                .debug_selector(|| "inner-block".into()),
                        )
                        .child("second"),
                )
                .child(" after")
                .child(div().hidden().h(px(300.)).child("hidden"))
                .child(div().inline().child(""))
                .child(
                    div()
                        .absolute()
                        .top(px(3.))
                        .left(px(4.))
                        .size(px(7.))
                        .debug_selector(|| "absolute".into()),
                )
                .child(
                    div()
                        .flex()
                        .h(px(12.))
                        .debug_selector(|| "following-block".into()),
                ),
        )
    }
}

#[test]
fn blocks_split_nested_spans_and_hidden_absolute_empty_children_add_no_rows() {
    let mut cx = headless();

    let window = cx
        .open_window(size(px(300.), px(240.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| MixedFlow)
        })
        .unwrap();
    cx.run_until_parked();

    let fragments = quads(&mut cx, window.into(), first_group_color());
    assert_eq!(fragments.len(), 2);

    let block = cx
        .debug_bounds(window.into(), "inner-block")
        .unwrap()
        .unwrap();
    let following = cx
        .debug_bounds(window.into(), "following-block")
        .unwrap()
        .unwrap();

    assert_close(
        block.origin.y,
        fragments[0].bottom(),
        "block follows first run",
    );
    assert_close(
        fragments[1].origin.y,
        block.bottom(),
        "span resumes after block",
    );
    assert_close(following.origin.y, fragments[1].bottom(), "no phantom rows");

    let paragraph = cx.debug_bounds(window.into(), "mixed").unwrap().unwrap();
    assert_close(
        paragraph.bottom(),
        following.bottom(),
        "wrapped height reaches parent",
    );
}

const ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12"><rect width="12" height="12" fill="#ff0000"/></svg>"##;

struct AtomicFlow {
    width: f32,
}

impl Render for AtomicFlow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().items_start().child(
            div()
                .block()
                .w(px(self.width))
                .text_size(px(16.))
                .line_height(px(24.))
                .child("Before these words ")
                .child(
                    div()
                        .inline()
                        .bg(first_group_color())
                        .debug_selector(|| "box-span".into())
                        .child(
                            div()
                                .inline_flex()
                                .items_center()
                                .gap(px(5.))
                                .w(px(75.))
                                .h(px(30.))
                                .align_middle()
                                .debug_selector(|| "badge".into())
                                .child(div().size(px(10.)).debug_selector(|| "badge-a".into()))
                                .child(div().size(px(12.)).debug_selector(|| "badge-b".into())),
                        ),
                )
                .child(" after.")
                .child(
                    div()
                        .inline()
                        .debug_selector(|| "icon-span".into())
                        .child(
                            gpui::img(Arc::new(gpui::Image::from_bytes(
                                gpui::ImageFormat::Svg,
                                ICON.to_vec(),
                            )))
                            .inline()
                            .size(px(12.))
                            .debug_selector(|| "image".into()),
                        )
                        .child(
                            gpui::svg()
                                .data(ICON)
                                .inline()
                                .size(px(14.))
                                .debug_selector(|| "svg".into()),
                        ),
                ),
        )
    }
}

#[test]
fn inline_flex_and_box_only_spans_move_as_atomic_content() {
    let mut cx = headless();

    let window = cx
        .open_window(size(px(340.), px(240.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| AtomicFlow { width: 150. })
        })
        .unwrap();
    let mut previous = None;

    for width in [150., 290.] {
        update_view(&mut cx, window, |view| view.width = width);

        let badge = cx.debug_bounds(window.into(), "badge").unwrap().unwrap();
        let span = cx.debug_bounds(window.into(), "box-span").unwrap().unwrap();

        let left = cx.debug_bounds(window.into(), "badge-a").unwrap().unwrap();
        let right = cx.debug_bounds(window.into(), "badge-b").unwrap().unwrap();

        let icon_span = cx
            .debug_bounds(window.into(), "icon-span")
            .unwrap()
            .unwrap();
        let image = cx.debug_bounds(window.into(), "image").unwrap().unwrap();
        let svg = cx.debug_bounds(window.into(), "svg").unwrap().unwrap();

        assert_bounds_close(icon_span, image.union(&svg), "box-only span bounds");
        assert_close(image.size.width, px(12.), "inline image stays atomic");
        assert_close(svg.size.width, px(14.), "inline SVG stays atomic");
        assert_bounds_close(span, badge, "badge span bounds");

        let backgrounds = quads(&mut cx, window.into(), first_group_color());
        assert_eq!(backgrounds.len(), 1);
        assert_bounds_close(backgrounds[0], badge, "badge span background");

        assert!((right.origin.x - left.right() - px(5.)).abs() < px(1.));
        assert!((left.center().y - badge.center().y).abs() < px(1.));

        if let Some((old_badge, offset)) = previous {
            assert!(badge.origin.y < old_badge);
            assert_eq!(left.origin - badge.origin, offset);
        }

        previous = Some((badge.origin.y, left.origin - badge.origin));
    }
}

#[derive(gpui::IntoElement)]
struct RenderedSpan {
    element: gpui::AnyElement,
    counts: Rc<[Cell<usize>; 4]>,
}

impl gpui::RenderOnce for RenderedSpan {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        self.counts[0].set(self.counts[0].get() + 1);

        LifecycleProbe {
            element: self.element,
            counts: self.counts,
        }
    }
}

struct LifecycleProbe {
    element: gpui::AnyElement,
    counts: Rc<[Cell<usize>; 4]>,
}

impl IntoElement for LifecycleProbe {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for LifecycleProbe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, ()) {
        self.counts[1].set(self.counts[1].get() + 1);
        (self.element.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        self.counts[2].set(self.counts[2].get() + 1);
        self.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        self.counts[3].set(self.counts[3].get() + 1);
        self.element.paint(window, cx);
    }
}

struct InteractiveFlow {
    width: f32,
    large: bool,
    atomic: bool,
    clicks: Rc<Cell<usize>>,
    hovered: Rc<Cell<bool>>,
    counts: Rc<[Cell<usize>; 4]>,
    callback: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    focus: gpui::FocusHandle,
    scroll: gpui::ScrollHandle,
}

impl Render for InteractiveFlow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let hovered = self.hovered.clone();
        let clicks = self.clicks.clone();
        let focus = self.focus.clone();

        let callback = self.callback.clone();

        let span = div()
            .id("interactive-span")
            .inline()
            .w(px(900.))
            .h(px(900.))
            .when(self.large, |span| {
                span.text_size(px(24.))
                    .line_height(px(34.))
                    .text_color(second_group_color())
            })
            .when(self.atomic, |span| {
                span.inline_flex().w(px(100.)).h(px(110.))
            })
            .on_hover(move |value, _, _| hovered.set(*value))
            .tooltip(|_, cx| cx.new(|_| InlineTooltip).into())
            .tooltip_show_delay(std::time::Duration::from_millis(10))
            .track_focus(&self.focus)
            .bg(first_group_color())
            .debug_selector(|| "interactive-span".into())
            .on_click(move |_, window, cx| {
                clicks.set(clicks.get() + 1);
                focus.focus(window, cx);
            })
            .child(gpui::text!(
                "these words can wrap across several lines with a short ending"
            ));

        div().size_full().items_start().child(
            div()
                .id("scroller")
                .block()
                .w(px(self.width))
                .h(px(150.))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(
                    div()
                        .block()
                        .w_full()
                        .text_size(px(17.))
                        .line_height(px(24.))
                        .child("Before ")
                        .child(RenderedSpan {
                            element: span.into_any_element(),
                            counts: self.counts.clone(),
                        })
                        .child(" after.")
                        .with_dynamic_prepaint_order(|_, _| [2, 1, 0].into_iter().collect())
                        .on_children_prepainted(move |bounds, _, _| {
                            *callback.borrow_mut() = bounds
                        }),
                )
                .child(div().h(px(160.))),
        )
    }
}

fn click(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle, position: Point<Pixels>) {
    cx.update_window(window, |_, window, cx| {
        window.simulate_mouse_move(position, cx);
        window.dispatch_event(
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                position,
                button: gpui::MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position,
                button: gpui::MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

#[test]
fn fragment_interaction_reflows_scrolls_and_preserves_wrapped_element_lifecycle() {
    let mut cx = headless();
    let clicks = Rc::new(Cell::new(0));
    let hovered = Rc::new(Cell::new(false));

    let counts = Rc::new(std::array::from_fn(|_| Cell::new(0)));
    let callback = Rc::new(RefCell::new(Vec::new()));

    let scroll = gpui::ScrollHandle::new();
    let focus = cx.update(|cx| cx.focus_handle());

    let window = cx
        .open_window(size(px(350.), px(300.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| InteractiveFlow {
                width: 190.,
                large: false,
                atomic: false,
                clicks: clicks.clone(),
                hovered: hovered.clone(),
                counts: counts.clone(),
                callback: callback.clone(),
                focus: focus.clone(),
                scroll: scroll.clone(),
            })
        })
        .unwrap();
    cx.run_until_parked();

    let regions = quads(&mut cx, window.into(), first_group_color());
    assert!(regions.len() >= 3);

    let union = regions
        .iter()
        .copied()
        .reduce(|left, right| left.union(&right))
        .unwrap();
    assert!(union.size.width < px(900.) && union.size.height < px(900.));
    assert_eq!(callback.borrow().len(), 3);
    assert_eq!(callback.borrow()[1], union);

    click(&mut cx, window.into(), regions[0].center());
    click(&mut cx, window.into(), regions.last().unwrap().center());
    assert_eq!(clicks.get(), 2);
    assert!(hovered.get());
    assert!(
        cx.update_window(window.into(), |_, window, _| focus.is_focused(window))
            .unwrap()
    );

    let gap = gpui::point(regions[0].origin.x / 2., regions[0].center().y);
    assert!(union.contains(&gap) && !regions.iter().any(|right| right.contains(&gap)));

    click(&mut cx, window.into(), gap);
    assert_eq!(
        clicks.get(),
        2,
        "the union's empty gap must not receive clicks"
    );

    assert!(!hovered.get());

    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(regions[0].center(), cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.advance_clock(std::time::Duration::from_millis(20));
    cx.run_until_parked();
    assert!(
        cx.debug_bounds(window.into(), "inline-tooltip")
            .unwrap()
            .is_some()
    );

    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(gap, cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    assert!(
        cx.debug_bounds(window.into(), "inline-tooltip")
            .unwrap()
            .is_none()
    );

    window
        .update(&mut cx, |view, _, cx| {
            view.width = 250.;
            view.large = true;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();

    let resized = quads(&mut cx, window.into(), first_group_color());
    assert_ne!(resized, regions);
    assert!(resized.iter().any(|right| right.size.height >= px(34.)));

    click(&mut cx, window.into(), resized[1].center());
    assert_eq!(clicks.get(), 3);

    scroll.set_offset(gpui::point(px(0.), px(-24.)));
    window.update(&mut cx, |_, _, cx| cx.notify()).unwrap();
    cx.run_until_parked();

    let scrolled = quads(&mut cx, window.into(), first_group_color());
    assert_close(
        scrolled[1].origin.y,
        resized[1].origin.y - px(24.),
        "scroll updates fragment origin",
    );

    click(&mut cx, window.into(), scrolled[1].center());
    assert_eq!(clicks.get(), 4);

    scroll.set_offset(Point::default());
    window
        .update(&mut cx, |view, _, cx| {
            view.atomic = true;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();

    let atomic = cx
        .debug_bounds(window.into(), "interactive-span")
        .unwrap()
        .unwrap();
    assert_close(
        atomic.size.width,
        px(100.),
        "display change restores atomic width",
    );
    assert_close(
        atomic.size.height,
        px(110.),
        "display change restores atomic height",
    );

    let counts: Vec<_> = counts.iter().map(Cell::get).collect();
    assert!(counts[0] > 1);
    assert!(
        counts.iter().all(|count| *count == counts[0]),
        "each rendered wrapper runs each lifecycle once: {counts:?}"
    );
}

fn assert_bounds_close(actual: Bounds<Pixels>, expected: Bounds<Pixels>, context: &str) {
    assert_point_close(actual.origin, expected.origin, context);
    assert_close(actual.size.width, expected.size.width, context);
    assert_close(actual.size.height, expected.size.height, context);
}

struct InlineTooltip;

impl Render for InlineTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size(px(20.))
            .debug_selector(|| "inline-tooltip".into())
    }
}

struct MixedTypography;

impl Render for MixedTypography {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().items_start().child(
            div()
                .block()
                .w(px(170.))
                .text_size(px(17.))
                .line_height(px(24.))
                .child("Before ")
                .child(
                    div()
                        .inline()
                        .text_size(px(30.))
                        .line_height(px(46.))
                        .text_color(first_group_color())
                        .text_bg(first_group_color())
                        .underline()
                        .child("Tall letters wrap ")
                        .child(
                            div()
                                .inline()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .text_color(second_group_color())
                                .text_bg(second_group_color())
                                .child("small tail"),
                        ),
                )
                .child(" after."),
        )
    }
}

#[test]
fn mixed_font_sizes_colors_and_underlines_paint_in_their_wrapped_rows() {
    let mut cx = headless();

    let window = cx
        .open_window(size(px(320.), px(300.)), |window, cx| {
            window.set_scale_factor(SCALE_FACTOR);
            cx.new(|_| MixedTypography)
        })
        .unwrap();
    cx.run_until_parked();

    let mut glyph_heights = Vec::new();

    for color in [first_group_color(), second_group_color()] {
        let backgrounds = quads(&mut cx, window.into(), color);
        assert!(!backgrounds.is_empty());

        let underlines: Vec<_> = cx
            .underline_bounds(window.into(), color)
            .unwrap()
            .into_iter()
            .map(logical_bounds)
            .collect();
        assert_eq!(underlines.len(), backgrounds.len());

        for underline in underlines {
            assert!(
                backgrounds
                    .iter()
                    .any(
                        |background| (underline.origin.x - background.origin.x).abs() < px(1.)
                            && underline.origin.y >= background.origin.y
                            && underline.bottom() <= background.bottom() + px(1.)
                    ),
                "underline {underline:?} outside {backgrounds:?}"
            );
        }

        let glyphs: Vec<_> = cx
            .glyph_bounds(window.into(), color)
            .unwrap()
            .into_iter()
            .map(logical_bounds)
            .collect();
        assert!(!glyphs.is_empty());

        for glyph in &glyphs {
            assert!(
                backgrounds
                    .iter()
                    .any(|background| background.contains(&glyph.center())),
                "glyph {glyph:?} outside {backgrounds:?}"
            );
        }

        glyph_heights.push(
            glyphs
                .iter()
                .map(|bounds| bounds.size.height)
                .fold(px(0.), Pixels::max),
        );
    }

    assert!(quads(&mut cx, window.into(), first_group_color()).len() >= 2);
    assert!(
        glyph_heights[0] > glyph_heights[1] * 1.5,
        "glyph painting must use each shaped run's font size"
    );
}
