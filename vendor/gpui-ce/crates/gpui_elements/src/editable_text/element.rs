use crate::editable_text::{
    BLINK_INTERVAL_500MS, Caret, EditableTextState,
    actions::{DEFAULT_INPUT_CONTEXT, EditableTextActionElement, EditableTextActionHandler},
    layout::{EditableTextLayoutResult, EditableTextLayoutState},
};
use gpui::{
    A11ySubtreeBuilder, App, Bounds, CaretPosition, CursorStyle, DefiniteLength, DispatchPhase,
    Display, Element, ElementId, ElementInputHandler, Entity, FocusHandle, Focusable, Hitbox,
    HitboxBehavior, Hsla, InteractiveElement, Interactivity, IntoElement, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, ParagraphDirection, Pixels, Point,
    ShapedText, SharedString, Size, StatefulInteractiveElement, Style, StyleRefinement, Styled,
    TextAlign, TextLayout, TextLayoutOptions, UnicodeBidi, WeakEntity, Window, accesskit, point,
    px, relative, size,
};
use palette::IntoColor;
use smallvec::SmallVec;
use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc, time::Duration};

/// Creates a text input element.
/// See [`EditableTextElement`] for usage.
///
/// By default it is multiline, and therefore this is semantically equivalent to [`text_area`].
#[track_caller]
pub fn editable_text(id: impl Into<ElementId>) -> EditableTextElement {
    let mut this = EditableTextElement {
        interactivity: Interactivity::default(),
        state_entity: Rc::new(RefCell::new(WeakEntity::new_invalid())),
        supports_multiline: true,
        placeholder: None,
        accepts_input: true,
        colors: EditableTextColors::default(),
        caret_blink_interval: None,
        caret_width: px(2.),
        caret_height: relative(1.).into(),
    };
    this.interactivity.element_id = Some(id.into());

    this = this.key_context(DEFAULT_INPUT_CONTEXT);
    this.register_actions();

    this
}

/// Creates a singleline text input element.
/// See [`EditableTextElement`] for usage.
#[track_caller]
pub fn text_input(id: impl Into<ElementId>) -> EditableTextElement {
    editable_text(id).multiline(false)
}

/// Creates a multiline text input element.
/// See [`EditableTextElement`] for usage.
#[track_caller]
pub fn text_area(id: impl Into<ElementId>) -> EditableTextElement {
    editable_text(id).multiline(true)
}

/// An input field which users can type text into.
#[derive(derive_setters::Setters)]
#[setters(generate = false)]
pub struct EditableTextElement {
    interactivity: Interactivity,
    // Populated on first render with an entity stored/attached to the view.
    // This reference is shared with the action handlers, which are processed between renders
    // and therefore cannot otherwise access state attached to the view.
    state_entity: Rc<RefCell<WeakEntity<EditableTextState>>>,
    /// Configures whether the field supports multiple lines of text.
    /// Disabling this prevents actions like `enter` and navigating up and down.
    ///
    /// Inputs containing newlines, such as pasted text, are not automatically sanitized.
    /// This is a limitation of the current state of implementation and requires further iteration.
    #[setters(generate, rename = "multiline")]
    supports_multiline: bool,
    /// Assigns the text that should be displayed when storage of the element is empty.
    #[setters(generate, strip_option, into)]
    placeholder: Option<SharedString>,
    /// Configures whether the element is enabled and can accept input.
    #[setters(generate)]
    accepts_input: bool,
    colors: EditableTextColors,
    /// Sets the blinking interval of the caret.
    #[setters(generate, strip_option)]
    caret_blink_interval: Option<Duration>,
    /// Sets the width of the caret / text-cursor.
    #[setters(generate, rename = "caret_w", into)]
    caret_width: Pixels,
    /// Sets the height of the caret / text-cursor.
    ///
    /// Relative lengths are resolved against the current line height. The default is
    /// `relative(1.)`, which makes the caret as tall as the line.
    #[setters(generate, rename = "caret_h", into)]
    caret_height: DefiniteLength,
}

/// EditableText styling that goes beyond what Style/StyleRefinement supports
struct EditableTextColors {
    /// Color of the placeholder text when the storage is empty.
    /// Could be reconceived as a refinement of text_color when the field is empty
    placeholder: Hsla,
    /// Color of the selection box.
    /// Could be driven by platform-provided styling?
    selection: Hsla,
    /// Color of the caret / text cursor
    caret: Hsla,
    /// Color of IME marked underlines
    ime_underline: Hsla,
}
impl Default for EditableTextColors {
    fn default() -> Self {
        use palette::RgbHue;
        const WHITE_50PC: Hsla = Hsla::new_const(RgbHue::new(0.), 0., 1., 0.5);
        const WHITE_70PC: Hsla = Hsla::new_const(RgbHue::new(0.), 0., 1., 0.7);
        // approx rgb(38 79 120) or oklch(41.9% 0.0829 250.4)
        const LIGHT_NAVY_BLUE_50PC: Hsla = Hsla::new_const(RgbHue::new(210.), 0.519, 0.31, 0.5);
        Self {
            placeholder: WHITE_50PC,
            selection: LIGHT_NAVY_BLUE_50PC,
            caret: gpui::white(),
            ime_underline: WHITE_70PC,
        }
    }
}

impl EditableTextElement {
    /// Assigns the underlying state of this element, which should persist across multiple frames.
    /// The user should either create the entity once or utilize `Window::use_keyed_state`
    /// to create an entity intrinsicly linked to the element.
    /// If no state is configured, one will be linked to this element on first render via `Window::use_keyed_state`.
    pub fn state(self, state: WeakEntity<EditableTextState>) -> Self {
        *self.state_entity.borrow_mut() = state;
        self
    }

    /// Sets the blinking interval of the caret to 500ms
    pub fn caret_blink_interval_500ms(self) -> Self {
        self.caret_blink_interval(BLINK_INTERVAL_500MS)
    }

    /// Sets the color of the placeholder text which is rendered when the element's stored text is empty.
    ///
    /// Cannot be refined via [`StyleRefinement`](gpui::StyleRefinement) due to limitations in the fields of [`Style`](gpui::Style).
    pub fn placeholder_color(mut self, color: impl IntoColor<Hsla>) -> Self {
        self.colors.placeholder = color.into_color();
        self
    }

    /// Sets the color of the box highlighting selected text.
    ///
    /// Cannot be refined via [`StyleRefinement`](gpui::StyleRefinement) due to limitations in the fields of [`Style`](gpui::Style).
    pub fn selection_color(mut self, color: Hsla) -> Self {
        self.colors.selection = color;
        self
    }

    /// Sets the color of the caret / text-cursor.
    ///
    /// Cannot be refined via [`StyleRefinement`](gpui::StyleRefinement) due to limitations in the fields of [`Style`](gpui::Style).
    pub fn caret_color(mut self, color: Hsla) -> Self {
        self.colors.caret = color;
        self
    }

    /// Sets the color of the underlines rendered underneath text being editted/marked by InputMethodEditors
    /// (for writing Chinese, Japanese, and Korean utf-16).
    ///
    /// Cannot be refined via [`StyleRefinement`](gpui::StyleRefinement) due to limitations in the fields of [`Style`](gpui::Style).
    pub fn marked_color(mut self, color: Hsla) -> Self {
        self.colors.ime_underline = color;
        self
    }
}

impl InteractiveElement for EditableTextElement {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

// forced implementation since the API for the element doesnt use Stateful<Element>
impl StatefulInteractiveElement for EditableTextElement {}

impl Styled for EditableTextElement {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl IntoElement for EditableTextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl EditableTextActionElement<EditableTextState> for EditableTextElement {
    fn state_entity_rc(&self) -> &Rc<RefCell<WeakEntity<EditableTextState>>> {
        &self.state_entity
    }
}

struct PrelayoutState {
    state: Entity<EditableTextState>,
    prev_layout_state: EditableTextLayoutState,
    storage_version: u16,
    show_placeholder: bool,
    text: Option<SharedString>,
    placeholder_color: Hsla,
    supports_multiline: bool,
    accepts_input: bool,
}

#[doc(hidden)]
pub struct LayoutState {
    layout_id: LayoutId,
    state: Entity<EditableTextState>,
    caret: Entity<Caret>,
}

struct InteractivityPrepaint {
    hitbox: Option<Hitbox>,
    scroll_offset: Point<Pixels>,
    document_offset: Point<Pixels>,
    inner_bounds: Bounds<Pixels>,
    caret_visible: bool,
}

impl InteractivityPrepaint {
    fn document_origin(&self) -> Point<Pixels> {
        self.inner_bounds.origin + self.scroll_offset + self.document_offset
    }
}

struct AccessibilityPrepaint {
    text_run: accesskit::Node,
    anchor: usize,
    focus: usize,
}

/// Internal type containing prepaint information used to paint the element
#[doc(hidden)]
pub struct PrepaintState {
    bounds: Bounds<Pixels>,
    interactivity: InteractivityPrepaint,
    focus_handle: FocusHandle,
    elements: PrepaintElements,
    accessibility: Option<AccessibilityPrepaint>,
}

impl Element for EditableTextElement {
    type RequestLayoutState = LayoutState;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.interactivity.element_id.clone()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.interactivity.source_location()
    }

    fn a11y_role(&self) -> Option<accesskit::Role> {
        Some(if self.supports_multiline {
            accesskit::Role::MultilineTextInput
        } else {
            accesskit::Role::TextInput
        })
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        if !self.accepts_input {
            node.set_read_only();
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        let accessibility = prepaint
            .accessibility
            .take()
            .expect("accessibility data was prepared while building the tree");
        let text_run_id = builder.synthetic_node_id("text");
        builder.push_child(text_run_id, accessibility.text_run);
        builder
            .parent_node()
            .set_text_selection(accesskit::TextSelection {
                anchor: accesskit::TextPosition {
                    node: text_run_id,
                    character_index: accessibility.anchor,
                },
                focus: accesskit::TextPosition {
                    node: text_run_id,
                    character_index: accessibility.focus,
                },
            });
    }

    fn request_layout(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let entity = self.find_or_create_state(window, cx);
        entity.update(cx, |state, cx| state.observe_blur(window, cx));
        let caret = self.find_or_create_caret(&entity, window, cx);

        if let Some(duration) = self.caret_blink_interval.take()
            && caret.read(cx).blink_interval() != duration
        {
            caret.update(cx, |caret, _cx| caret.set_blink_interval(duration));
        }

        // Read new state information from the underlying entity.
        // Block-wrapped so that the state being read is dropped before continuing.
        let (prelayout, next_scroll_offset, direction_text) = {
            let state = entity.read(cx);
            let show_placeholder = state.as_str().is_empty();
            let direction_text = SharedString::from(state.as_str());
            let text = match show_placeholder {
                false => Some(direction_text.clone()),
                true => self.placeholder.clone(),
            };

            let prelayout = PrelayoutState {
                state: entity.clone(),
                prev_layout_state: state.layout_data.state,
                show_placeholder,
                storage_version: state.version(),
                text,
                placeholder_color: self.colors.placeholder,
                supports_multiline: self.supports_multiline,
                accepts_input: self.accepts_input,
            };
            (
                prelayout,
                state.layout_data.next_scroll_offset,
                direction_text,
            )
        };

        // Update the scroll offset of the element when the user's caret goes out of scope.
        if let Some(scroll_offset) = next_scroll_offset {
            self.interactivity
                .set_scroll_offset(global_id, window, -scroll_offset);

            // Clear scroll_layout here in the very likely event that we wont need to
            // recompute layout, in which case the layout result isnt rebuilt during `perform_text_layout`.
            entity.update(cx, |state, _cx| {
                state.layout_data.next_scroll_offset = None;
            });
        }

        let layout_id = self.interactivity.request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            |style, window, cx| {
                window.with_text_style(style.text_style().cloned(), move |window| {
                    let text_layout_id =
                        prelayout.perform_text_layout(style.effective_unicode_bidi(), window);
                    window.request_layout(style.clone(), Some(text_layout_id), cx)
                })
            },
        );
        window.set_layout_direction_text(layout_id, direction_text);

        (
            layout_id,
            LayoutState {
                layout_id,
                state: entity,
                caret,
            },
        )
    }

    fn prepaint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let bounds = window.parent_relative_layout_bounds(request_layout.layout_id);

        // should reflect the text content layout size of the stored text,
        // so that scrolling can take it into account during prepaint.
        let (content_size, focus_handle) = {
            let state = request_layout.state.read(cx);
            let content_size = state.layout_data.state.size.unwrap_or_else(|| bounds.size);
            let focus_handle = state.focus_handle(cx);
            (content_size, focus_handle)
        };

        let is_focused = focus_handle.is_focused(window);
        let caret_visible = request_layout
            .caret
            .update(cx, |caret, cx| caret.update_focus(is_focused, cx));
        window.set_focus_handle(&focus_handle, cx);

        let prepaint = self.interactivity.prepaint(
            global_id,
            inspector_id,
            bounds,
            content_size,
            window,
            cx,
            |style, scroll_offset, hitbox, window, cx| {
                let hitbox =
                    hitbox.or_else(|| Some(window.insert_hitbox(bounds, HitboxBehavior::Normal)));
                let inner_bounds = {
                    let padding = style
                        .padding
                        .to_pixels(bounds.size.into(), window.rem_size());

                    let mut bounds = bounds;
                    bounds.origin += point(padding.left, padding.top);
                    bounds.size.width -= padding.left + padding.right;
                    bounds.size.height -= padding.top + padding.bottom;
                    bounds
                };
                request_layout.state.update(cx, |state, _cx| {
                    // while gpui tracks scroll_offset with negative values,
                    // this is converted into positive for usage with bounds
                    state.layout_data.scroll_bounds =
                        Bounds::new(-scroll_offset, inner_bounds.size);
                });
                let document_offset = request_layout.state.read(cx).layout_data.document_offset;
                InteractivityPrepaint {
                    hitbox,
                    scroll_offset,
                    document_offset,
                    inner_bounds,
                    caret_visible,
                }
            },
        );

        let state = request_layout.state.read(cx);
        let accessibility = window.is_a11y_active().then(|| {
            let metrics = state.accessibility_text_metrics();
            let (anchor, focus) = metrics.character_indices_for_selection(state.caret_selection());
            let mut text_run = accesskit::Node::new(accesskit::Role::TextRun);
            text_run.set_value(state.as_str());
            text_run.set_character_lengths(metrics.character_lengths.clone());

            AccessibilityPrepaint {
                text_run,
                anchor,
                focus,
            }
        });
        let elements = PrepaintElements::build_elements(
            state,
            &prepaint,
            &self.colors,
            self.caret_width,
            self.caret_height,
            window,
        );

        PrepaintState {
            bounds,
            interactivity: prepaint,
            focus_handle,
            elements,
            accessibility,
        }
    }

    fn paint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let bounds = prepaint.bounds;

        if let Some(hitbox) = &prepaint.interactivity.hitbox {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

        let accepts_input = self.accepts_input;
        let hitbox = prepaint.interactivity.hitbox.clone();
        let line_height = request_layout.state.read(cx).layout_data.line_height;
        let perform_paint = |style: &Style, window: &mut Window, cx: &mut App| {
            if style.display == Display::None {
                return;
            }

            // Register event listeners to the window for the next frame
            if accepts_input {
                Self::process_frame_events(prepaint, bounds, &request_layout.state, window, cx);
            }

            // Actually draw the elements we constructed during prepaint
            if let Some(document) = prepaint.elements.document.take() {
                let _ = document.paint(
                    prepaint.interactivity.document_origin(),
                    line_height,
                    TextAlign::Left,
                    Some(bounds),
                    window,
                    cx,
                );
            }

            for quad in prepaint.elements.ime_marked.drain(..) {
                window.paint_quad(quad);
            }

            for quad in prepaint.elements.selection.drain(..) {
                window.paint_quad(quad);
            }

            if let Some(quad) = prepaint.elements.caret.take() {
                window.paint_quad(quad);
            }
        };

        self.interactivity.paint(
            global_id,
            inspector_id,
            bounds,
            hitbox.as_ref(),
            window,
            cx,
            perform_paint,
        );
    }
}

impl EditableTextElement {
    fn find_or_create_state(&self, window: &mut Window, cx: &mut App) -> Entity<EditableTextState> {
        if let Some(entity) = self.state_entity.borrow().upgrade() {
            return entity;
        }
        let Some(element_id) = self.interactivity.element_id.clone() else {
            unimplemented!("all input elements must be assigned an id")
        };

        let state = EditableTextState::use_keyed(element_id, window, cx);
        // store a reference to the entity owned by the element for access in action handlers
        *self.state_entity_rc().borrow_mut() = state.downgrade();
        state
    }

    fn find_or_create_caret(
        &self,
        state: &Entity<EditableTextState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Caret> {
        let Some(element_id) = self.interactivity.element_id.clone() else {
            unimplemented!("all input elements must be assigned an id")
        };

        window.use_keyed_state(element_id, cx, |_window, cx| {
            let mut caret = Caret::default();
            caret.subscribe_to(state, cx);
            caret
        })
    }

    fn process_frame_events(
        prepaint: &PrepaintState,
        bounds: Bounds<Pixels>,
        entity: &Entity<EditableTextState>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let inner_bounds = prepaint.interactivity.inner_bounds;
        let document_origin = prepaint.interactivity.document_origin();

        let ime_handler = ElementInputHandler::new(inner_bounds, entity.clone());
        window.handle_input(&prepaint.focus_handle, ime_handler, cx);

        window.on_mouse_event({
            let focus_handle = prepaint.focus_handle.clone();
            let state = entity.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                if !bounds.contains(&event.position) {
                    return;
                }
                if event.button != MouseButton::Left {
                    return;
                }

                cx.stop_propagation();
                window.focus(&focus_handle, cx);

                let text_position = event.position - document_origin;
                state.update(cx, |state, cx| {
                    state.on_mouse_down(event, text_position, window, cx);
                });
            }
        });
        window.on_mouse_event({
            let state = entity.clone();
            move |event: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                if event.button != MouseButton::Left {
                    return;
                }

                state.update(cx, |state, cx| {
                    state.on_mouse_up(event, window, cx);
                });
            }
        });
        window.on_mouse_event({
            let state = entity.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }

                let text_position = event.position - document_origin;
                state.update(cx, |state, cx| {
                    state.on_mouse_move(event, text_position, window, cx);
                });
            }
        });
    }
}

impl PrelayoutState {
    fn perform_text_layout(self, unicode_bidi: UnicodeBidi, window: &mut Window) -> LayoutId {
        // NOTE: Loosely mirrors TextLayout::layout
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.pixel_snap(
            text_style
                .line_height
                .to_pixels(font_size.into(), window.rem_size()),
        );

        let color = match self.show_placeholder {
            false => text_style.color,
            true => self.placeholder_color,
        };

        let text = self.text.unwrap_or_default();

        window.request_measured_layout(
            Default::default(),
            // This is invoked sometime in the near future (before prepaint but not immediately),
            // so we avoid doing any pre-emptive work until the layout engine is ready.
            move |known_dimensions, available_space, window, cx| {
                let runs = vec![gpui::TextRun {
                    len: text.len(),
                    font: text_style.font(),
                    color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                    letter_spacing: None,
                }];

                let wrap_width = TextLayout::evaluate_wrap_width(
                    &text_style.white_space,
                    known_dimensions,
                    available_space,
                );

                let truncation =
                    TextLayout::evaluate_overflow(&text_style, known_dimensions, available_space);
                let alignment_width =
                    TextLayout::evaluate_alignment_width(known_dimensions, available_space);
                let element_direction = window.resolved_direction();
                let direction = if unicode_bidi == UnicodeBidi::Plaintext {
                    ParagraphDirection::Auto
                } else {
                    element_direction.into()
                };
                let options = TextLayoutOptions {
                    wrap_width,
                    line_clamp: text_style.line_clamp,
                    alignment_width,
                    text_align: text_style.text_align,
                    direction,
                    unicode_bidi,
                };

                if let Some(size) = self.prev_layout_state.size
                    && truncation.width.is_none()
                    && self.storage_version == self.prev_layout_state.last_seen_storage_version
                    && self.prev_layout_state.options == options
                {
                    return size;
                }

                let (text, runs) = TextLayout::apply_truncation(
                    text.clone(),
                    &text_style,
                    font_size,
                    line_height,
                    wrap_width,
                    &truncation,
                    &runs,
                    options.direction,
                    options.unicode_bidi,
                    window,
                    cx,
                );
                let document = window
                    .text_system()
                    .shape_text_with_options(text, font_size, &runs, options)
                    .ok()
                    .map(Arc::new);
                let size = document
                    .as_ref()
                    .map_or_else(Size::default, |document| document.size(line_height));
                let document_offset = if element_direction.is_rtl() {
                    document.as_deref().map_or_else(Point::default, |document| {
                        editable_document_offset(document, line_height)
                    })
                } else {
                    Point::default()
                };
                let initial_scroll = self
                    .prev_layout_state
                    .size
                    .is_none()
                    .then_some(document_offset)
                    .filter(|offset| offset.x > Pixels::ZERO);
                let refresh_for_initial_scroll = initial_scroll.is_some();

                let layout_data = EditableTextLayoutResult {
                    supports_multiline: self.supports_multiline,
                    accepts_input: self.accepts_input,
                    // updated during prepaint
                    scroll_bounds: Bounds::default(),
                    state: EditableTextLayoutState {
                        size: Some(size),
                        last_seen_storage_version: self.storage_version,
                        options,
                    },
                    document,
                    line_height,
                    document_offset,
                    next_scroll_offset: initial_scroll,
                };

                // Update the state for use in prepaint, paint, and action handlers.
                // request_measured_layout caches this scope for processing later
                // between layout and prepaint, so we cant just copy/move these values to the outer scope.
                self.state.update(cx, move |state, cx| {
                    state.layout_data = layout_data;
                    if state.layout_data.next_scroll_offset.is_some() {
                        cx.notify();
                    }
                });
                if refresh_for_initial_scroll {
                    let state = self.state.clone();
                    cx.defer(move |cx| {
                        state.update(cx, |_state, cx| cx.notify());
                    });
                }

                size
            },
        )
    }
}

#[derive(Default)]
struct PrepaintElements {
    document: Option<Arc<ShapedText>>,
    selection: SmallVec<[PaintQuad; 20]>,
    ime_marked: SmallVec<[PaintQuad; 2]>,
    caret: Option<PaintQuad>,
}

impl PrepaintElements {
    fn build_elements(
        state: &EditableTextState,
        prepaint: &InteractivityPrepaint,
        colors: &EditableTextColors,
        caret_width: Pixels,
        caret_height: DefiniteLength,
        window: &mut Window,
    ) -> PrepaintElements {
        let mut elements = PrepaintElements::default();
        let Some(document) = &state.layout_data.document else {
            return elements;
        };

        let line_height = state.layout_data.line_height;
        let document_top = prepaint.scroll_offset.y;
        let document_bottom = document_top + line_height * document.line_count() as f32;

        if document_bottom < Pixels::ZERO || document_top > prepaint.inner_bounds.size.height {
            return elements;
        }

        let document_origin = prepaint.document_origin();
        let quads = |range: Range<usize>, color, offset_y| {
            let start = range.start.min(document.text.len());
            let end = range.end.min(document.text.len());

            document
                .selection_bounds(start..end, line_height)
                .into_iter()
                .map(move |bounds| {
                    PaintQuad::new(Bounds::from_corners(
                        document_origin + bounds.origin + point(Pixels::ZERO, offset_y),
                        document_origin + bounds.bottom_right(),
                    ))
                    .background(color)
                })
        };

        elements.document = Some(document.clone());

        elements.selection.extend(quads(
            state.selected_byte_range(),
            colors.selection,
            Pixels::ZERO,
        ));

        if let Some(range) = state.marked_range() {
            elements
                .ime_marked
                .extend(quads(range, colors.ime_underline, line_height - px(2.)));
        }

        if prepaint.caret_visible {
            let caret_point = document_origin
                + document
                    .visual_position_for_caret(state.visible_caret(), line_height)
                    .unwrap_or_default();
            let caret_height = caret_height.to_pixels(line_height.into(), window.rem_size());
            let vertical_offset = (line_height - caret_height) / 2.;
            elements.caret = Some(
                PaintQuad::new(Bounds::new(
                    caret_point + point(Pixels::ZERO, vertical_offset),
                    size(caret_width, caret_height),
                ))
                .background(colors.caret),
            );
        }

        elements
    }
}

fn editable_document_offset(document: &ShapedText, line_height: Pixels) -> Point<Pixels> {
    let mut left = Pixels::ZERO;

    for caret in [
        CaretPosition::attached_to_next_cluster(0),
        CaretPosition::attached_to_previous_cluster(document.text.len()),
    ] {
        if let Some(visual_position) = document.visual_position_for_caret(caret, line_height) {
            left = left.min(visual_position.x);
        }
    }

    for bounds in document.selection_bounds(0..document.text.len(), line_height) {
        left = left.min(bounds.left());
    }

    point(-left, Pixels::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editable_text::{StringStorage, actions::default_bindings};
    use gpui::{
        AppContext as _, CaretPosition, CaretSelection, Context, EntityInputHandler,
        HeadlessAppContext, KeyDownEvent, KeyUpEvent, Keystroke, LayoutDirection, Modifiers,
        ModifiersChangedEvent, NavigationDirection, PlatformInput, PlatformTextSystem, Render,
        ScaledPixels, TestTextSystem, WindowHandle, div, hsla, prelude::*, rgba,
    };
    use gpui_parley::{ParleyTextSystem, SystemFonts};
    use std::{borrow::Cow, collections::HashSet, sync::Arc};

    const CONTAINER_COLOR: Hsla = hsla(0.72, 0.45, 0.32, 1.0);
    const INPUT_COLOR: Hsla = hsla(0.08, 0.55, 0.28, 1.0);
    const SELECTION_COLOR: Hsla = hsla(0.37, 0.65, 0.42, 1.0);
    const CARET_COLOR: Hsla = hsla(0.95, 0.8, 0.6, 1.0);
    const TEXT_COLOR: Hsla = hsla(0.0, 0.0, 0.1, 1.0);
    const MARKED_COLOR: Hsla = hsla(0.55, 0.8, 0.6, 1.0);

    struct BidiInputView {
        input: Entity<EditableTextState>,
        padding: Pixels,
        width: Pixels,
        wrap: bool,
        direction: LayoutDirection,
        unicode_bidi: Option<UnicodeBidi>,
        placeholder: Option<&'static str>,
    }

    impl Render for BidiInputView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().p(px(24.)).child(
                editable_text("bidi-input")
                    .state(self.input.downgrade())
                    .p(self.padding)
                    .w(self.width)
                    .h(px(120.))
                    .border_1()
                    .rounded_lg()
                    .bg(INPUT_COLOR)
                    .text_color(TEXT_COLOR)
                    .selection_color(SELECTION_COLOR)
                    .caret_color(CARET_COLOR)
                    .marked_color(MARKED_COLOR)
                    .text_size(px(20.))
                    .line_height(px(if self.wrap { 27.5 } else { 28. }))
                    .direction(self.direction)
                    .text_start()
                    .when_some(self.unicode_bidi, |input, unicode_bidi| {
                        input.unicode_bidi(unicode_bidi)
                    })
                    .when_some(self.placeholder, |input, placeholder| {
                        input.placeholder(placeholder)
                    })
                    .when(self.wrap, |input| {
                        input.flex_col().whitespace_normal().overflow_y_scroll()
                    })
                    .when(!self.wrap, |input| {
                        input.whitespace_nowrap().overflow_x_scroll()
                    }),
            )
        }
    }

    struct BidiInputFixture {
        input: Entity<EditableTextState>,
        window: WindowHandle<BidiInputView>,
        padding: Pixels,
        scale: f32,
        cx: HeadlessAppContext,
    }

    impl BidiInputFixture {
        fn new(text: &str, padding: f32, width: f32, wrap: bool, scale: f32) -> Self {
            Self::new_with_direction(text, padding, width, wrap, scale, LayoutDirection::Auto)
        }

        fn new_with_direction(
            text: &str,
            padding: f32,
            width: f32,
            wrap: bool,
            scale: f32,
            direction: LayoutDirection,
        ) -> Self {
            Self::new_with_configuration(text, padding, width, wrap, scale, direction, None, None)
        }

        fn new_with_configuration(
            text: &str,
            padding: f32,
            width: f32,
            wrap: bool,
            scale: f32,
            direction: LayoutDirection,
            unicode_bidi: Option<UnicodeBidi>,
            placeholder: Option<&'static str>,
        ) -> Self {
            let system = ParleyTextSystem::new_with_system_font(SystemFonts::Skip, "IBM Plex Sans")
                .with_fallback_families(["IBM Plex Sans", "Noto Sans Hebrew", "Noto Sans Arabic"]);
            system
                .add_fonts(vec![
                    Cow::Borrowed(*gpui_fonts::IBM_PLEX),
                    Cow::Borrowed(*gpui_fonts::NOTO_HEBREW),
                    Cow::Borrowed(*gpui_fonts::NOTO_ARABIC),
                ])
                .unwrap();

            let mut cx = HeadlessAppContext::new(Arc::new(system));
            let input =
                cx.update(|cx| cx.new(|cx| EditableTextState::new(StringStorage::from(text), cx)));

            let window = cx
                .open_window(size(px(500.), px(240.)), |window, cx| {
                    window.set_scale_factor(scale);

                    cx.new(|_cx| BidiInputView {
                        input: input.clone(),
                        padding: px(padding),
                        width: px(width),
                        wrap,
                        direction,
                        unicode_bidi,
                        placeholder,
                    })
                })
                .unwrap();
            cx.run_until_parked();

            Self {
                input,
                window,
                padding: px(padding),
                scale,
                cx,
            }
        }

        fn origin(&mut self) -> Point<Pixels> {
            let bounds = only_quad(&mut self.cx, self.window.into(), INPUT_COLOR)
                .map(|value| px(value.as_f32() / self.scale));
            let (scroll, document_offset) = self.cx.update(|cx| {
                let layout = &self.input.read(cx).layout_data;
                (layout.scroll_bounds.origin, layout.document_offset)
            });

            bounds.origin + point(self.padding, self.padding) - scroll + document_offset
        }

        fn document(&mut self) -> Arc<ShapedText> {
            self.cx
                .update(|cx| self.input.read(cx).layout_data.document.clone().unwrap())
        }

        fn line_height(&mut self) -> Pixels {
            self.cx
                .update(|cx| self.input.read(cx).layout_data.line_height)
        }

        fn scroll(&mut self, offset: Point<Pixels>) {
            self.cx.update(|cx| {
                self.input.update(cx, |state, cx| {
                    state.layout_data.next_scroll_offset = Some(offset);
                    cx.notify();
                })
            });
            self.cx.run_until_parked();
            self.cx.update(|cx| {
                assert_eq!(self.input.read(cx).layout_data.scroll_bounds.origin, offset);
            });
        }

        fn point_for_caret(&mut self, caret: CaretPosition) -> Point<Pixels> {
            let line_height = self.line_height();
            let local = self
                .document()
                .visual_position_for_caret(caret, line_height)
                .unwrap();

            self.origin() + local + point(px(0.), line_height / 2.)
        }

        fn point(&mut self, idx: usize) -> Point<Pixels> {
            self.point_for_caret(CaretPosition::attached_to_next_cluster(idx))
        }

        fn update_input(
            &mut self,
            update: impl FnOnce(&mut EditableTextState, &mut Context<EditableTextState>),
        ) {
            self.cx
                .update_window(self.window.into(), |_view, window, cx| {
                    let focus_handle = self.input.read(cx).focus_handle(cx);
                    window.focus(&focus_handle, cx);
                    self.input.update(cx, update);
                })
                .unwrap();
            self.cx.run_until_parked();
        }

        fn dispatch(&mut self, event: PlatformInput) {
            self.cx
                .update_window(self.window.into(), |_view, window, cx| {
                    window.dispatch_event(event, cx);
                })
                .unwrap();
            self.cx.run_until_parked();
        }

        fn key_down(&mut self, keystroke: &str) {
            let keystroke = Keystroke::parse(keystroke).unwrap();
            self.dispatch(PlatformInput::KeyDown(KeyDownEvent {
                keystroke,
                is_held: false,
                prefer_character_input: false,
            }));
        }

        fn key_up(&mut self, keystroke: &str) {
            let keystroke = Keystroke::parse(keystroke).unwrap();
            self.dispatch(PlatformInput::KeyUp(KeyUpEvent { keystroke }));
        }

        fn change_modifiers(&mut self, modifiers: Modifiers) {
            self.dispatch(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers,
                ..Default::default()
            }));
        }

        fn down(&mut self, position: Point<Pixels>) {
            self.dispatch(PlatformInput::MouseDown(MouseDownEvent {
                position,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }));
        }

        fn drag(&mut self, idx: usize) {
            let position = self.point(idx);
            self.drag_to(position);
        }

        fn drag_to(&mut self, position: Point<Pixels>) {
            self.dispatch(PlatformInput::MouseMove(MouseMoveEvent {
                position,
                pressed_button: Some(MouseButton::Left),
                ..Default::default()
            }));
        }

        fn up(&mut self, idx: usize) {
            let position = self.point(idx);
            self.dispatch(PlatformInput::MouseUp(MouseUpEvent {
                position,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }));
        }

        fn assert_selection(&mut self, anchor: usize, caret_idx: usize) {
            let caret = self
                .cx
                .update(|cx| self.input.read(cx).caret_selection().caret);
            assert_eq!(caret.index, caret_idx);
            self.assert_caret_selection(anchor, caret);
        }

        fn assert_caret_selection(&mut self, anchor: usize, expected_caret: CaretPosition) {
            let caret_idx = expected_caret.index;
            let range = anchor.min(caret_idx)..anchor.max(caret_idx);
            let caret = self.cx.update(|cx| {
                let state = self.input.read(cx);
                assert_eq!(state.selected_byte_range(), range);
                assert_eq!(state.caret_selection().caret, expected_caret);

                state.caret_selection().caret
            });

            let origin = self.origin();
            let document = self.document();
            let line_height = self.line_height();
            let caret_visual_position = document
                .visual_position_for_caret(caret, line_height)
                .unwrap();
            self.assert_quads(
                CARET_COLOR,
                vec![Bounds::new(
                    origin + caret_visual_position,
                    size(px(2.), line_height),
                )],
            );

            let expected = document
                .selection_bounds(range, line_height)
                .into_iter()
                .map(|bounds| Bounds::new(origin + bounds.origin, bounds.size))
                .collect();
            self.assert_quads(SELECTION_COLOR, expected);
        }

        fn assert_quads(&mut self, color: Hsla, expected: Vec<Bounds<Pixels>>) {
            let actual = self
                .cx
                .solid_quad_bounds(self.window.into(), color)
                .unwrap();
            assert_eq!(actual.len(), expected.len(), "quad count for {color:?}");

            for (actual, expected) in actual.into_iter().zip(expected) {
                let actual = actual.map(|value| px(value.as_f32() / self.scale));
                assert!(
                    (actual.origin - expected.origin).magnitude() <= 1.,
                    "{actual:?} != {expected:?}"
                );
                assert!((actual.size.width - expected.size.width).abs() <= px(1.));
                assert!((actual.size.height - expected.size.height).abs() <= px(1.));
            }
        }
    }

    #[test]
    fn parley_input_clicks_and_drags_preserve_visual_carets() {
        for (text, anchor, targets) in [
            ("שלום עולם", 15, [13, 15, 17]),
            (
                "مرحبا بالعالم",
                "مرحبا بالعا".len(),
                [
                    "مرحبا بالع".len(),
                    "مرحبا بالعا".len(),
                    "مرحبا بالعال".len(),
                ],
            ),
            ("abc אבגד def", 8, [6, 8, 10]),
            ("hello world", 3, [4, 3, 2]),
        ] {
            for (padding, scale) in [(0., 1.), (8., 1.), (8., 1.5)] {
                let mut fixture = BidiInputFixture::new(text, padding, 320., false, scale);

                for offset in [-0.25, 0., 0.25] {
                    let position = fixture.point(anchor) + point(px(offset), px(0.));
                    fixture.down(position);
                    fixture.assert_selection(anchor, anchor);
                    fixture.up(anchor);
                }

                let position = fixture.point(anchor);
                fixture.down(position);

                for target in targets {
                    fixture.drag(target);
                    fixture.assert_selection(anchor, target);
                }

                fixture.up(targets[2]);
                fixture.drag(anchor);
                fixture.assert_selection(anchor, targets[2]);
            }
        }
    }

    #[test]
    fn shift_home_and_end_move_without_selecting() {
        for (text, direction, line_start, middle, line_end, outer_start, outer_end) in [
            (
                "first\nsecond\nthird",
                LayoutDirection::LeftToRight,
                6,
                9,
                12,
                2,
                15,
            ),
            (
                "first\nabc אבגד def\nthird",
                LayoutDirection::LeftToRight,
                6,
                14,
                22,
                2,
                25,
            ),
            (
                "אבגד\nהוזח\nטיכל",
                LayoutDirection::RightToLeft,
                9,
                13,
                17,
                4,
                22,
            ),
        ] {
            let mut fixture =
                BidiInputFixture::new_with_direction(text, 8.0, 320.0, false, 1.0, direction);
            fixture.cx.update(|cx| {
                cx.bind_keys(default_bindings().as_keybindings(Some(DEFAULT_INPUT_CONTEXT)));
            });
            fixture.cx.run_until_parked();
            fixture.update_input(|input, cx| input.move_to(middle, cx));

            fixture.change_modifiers(Modifiers::shift());
            fixture.key_down("shift-home");
            fixture.assert_selection(line_start, line_start);
            fixture.key_up("shift-home");

            fixture.key_down("shift-end");
            fixture.assert_selection(line_end, line_end);
            fixture.key_up("shift-end");

            fixture.update_input(|input, cx| input.move_to(middle, cx));
            fixture.key_down("shift-end");
            fixture.assert_selection(line_end, line_end);
            fixture.key_up("shift-end");

            fixture.key_down("shift-home");
            fixture.assert_selection(line_start, line_start);
            fixture.key_up("shift-home");

            fixture.update_input(|input, cx| {
                input.move_to(outer_start, cx);
                input.select_to(outer_end, cx);
            });
            fixture.key_down("shift-home");
            fixture.assert_selection(line_end + 1, line_end + 1);
            fixture.key_up("shift-home");

            fixture.key_down("shift-end");
            fixture.assert_selection(text.len(), text.len());
        }
    }

    #[test]
    fn parley_hebrew_click_uses_the_painted_gap() {
        let mut fixture = BidiInputFixture::new("שלום עולם", 8., 320., false, 1.5);
        let mut glyphs = fixture
            .cx
            .glyph_bounds(fixture.window.into(), TEXT_COLOR)
            .unwrap();
        glyphs.sort_by(|left, right| left.origin.x.partial_cmp(&right.origin.x).unwrap());
        assert_eq!(glyphs.len(), 8);

        let gap =
            px((glyphs[0].right().as_f32() + glyphs[1].left().as_f32()) / (2. * fixture.scale));
        let position = point(gap, fixture.point(15).y);
        fixture.down(position);
        fixture.assert_selection(15, 15);
        fixture.drag(13);
        fixture.assert_selection(15, 13);
    }

    #[test]
    fn explicit_rtl_aligns_editable_ltr_text_and_keeps_interaction_geometry() {
        let mut fixture = BidiInputFixture::new_with_direction(
            "English 123",
            8.0,
            320.0,
            false,
            1.0,
            LayoutDirection::RightToLeft,
        );
        let document = fixture.document();
        assert_eq!(
            document.visual_lines()[0].direction,
            gpui::ResolvedDirection::RightToLeft
        );
        assert!(document.selection_bounds(0..7, px(28.0))[0].origin.x > px(150.0));

        let position = fixture.point(3);
        fixture.down(position);
        fixture.assert_selection(3, 3);
        fixture.drag(7);
        fixture.assert_selection(3, 7);
        fixture.up(7);
    }

    #[test]
    fn directional_text_clicks_and_drags_preserve_endpoint_affinity() {
        for (text, anchor, direction) in [
            ("English 123", 3, LayoutDirection::RightToLeft),
            ("مرحبا", "مر".len(), LayoutDirection::LeftToRight),
            ("English 123", 3, LayoutDirection::LeftToRight),
            ("مرحبا", "مر".len(), LayoutDirection::RightToLeft),
        ] {
            let start = CaretPosition::attached_to_next_cluster(0);
            let end = CaretPosition::attached_to_previous_cluster(text.len());
            for (endpoint, opposite) in [(start, end), (end, start)] {
                let mut fixture =
                    BidiInputFixture::new_with_direction(text, 8.0, 320.0, false, 1.0, direction);
                let opposite_point = fixture.point_for_caret(opposite);
                let mut endpoint_point = fixture.point_for_caret(endpoint);
                endpoint_point.x += if endpoint_point.x < opposite_point.x {
                    px(-1.0)
                } else {
                    px(1.0)
                };

                fixture.down(endpoint_point);
                fixture.assert_caret_selection(endpoint.index, endpoint);

                for drag_anchor in [CaretPosition::attached_to_next_cluster(anchor), opposite] {
                    let mut fixture = BidiInputFixture::new_with_direction(
                        text, 8.0, 320.0, false, 1.0, direction,
                    );
                    let anchor_point = fixture.point_for_caret(drag_anchor);
                    fixture.down(anchor_point);
                    fixture.drag_to(endpoint_point);
                    fixture.assert_caret_selection(drag_anchor.index, endpoint);
                }
            }

            let mut fixture =
                BidiInputFixture::new_with_direction(text, 8.0, 320.0, false, 1.0, direction);
            fixture.update_input(|state, cx| {
                state.move_to(anchor, cx);
                state.select_linear(
                    NavigationDirection::Forward,
                    crate::editable_text::TextBoundary::Document,
                    cx,
                );
            });
            fixture.assert_caret_selection(
                anchor,
                CaretPosition::attached_to_previous_cluster(text.len()),
            );
        }
    }

    #[test]
    fn backspace_keeps_caret_at_text_end_when_direction_opposes_the_text() {
        for (text, remaining, direction) in [
            ("abc", "ab", LayoutDirection::RightToLeft),
            ("مرحبا", "مرحب", LayoutDirection::LeftToRight),
        ] {
            let mut fixture =
                BidiInputFixture::new_with_direction(text, 8.0, 320.0, false, 1.0, direction);

            fixture.update_input(|state, cx| {
                state.move_to(text.len(), cx);
                state.delete_linear(
                    NavigationDirection::Back,
                    crate::editable_text::TextBoundary::Cluster,
                    cx,
                );
            });

            let caret = fixture.cx.update(|cx| {
                let state = fixture.input.read(cx);
                assert_eq!(state.as_str(), remaining);
                state.caret_selection().caret
            });
            assert_eq!(
                caret,
                CaretPosition::attached_to_previous_cluster(remaining.len())
            );

            let document = fixture.document();
            let line_height = fixture.line_height();
            let downstream = document
                .visual_position_for_caret(
                    CaretPosition::attached_to_next_cluster(caret.index),
                    line_height,
                )
                .unwrap();
            let upstream = document
                .visual_position_for_caret(caret, line_height)
                .unwrap();
            assert_ne!(upstream.x, downstream.x);

            fixture.assert_selection(caret.index, caret.index);
        }
    }

    #[test]
    fn auto_editable_direction_ignores_placeholder_text() {
        let mut fixture = BidiInputFixture::new_with_configuration(
            "",
            8.0,
            320.0,
            false,
            1.0,
            LayoutDirection::Auto,
            None,
            Some("مرحبا"),
        );

        assert_eq!(
            fixture.document().visual_lines()[0].direction,
            gpui::ResolvedDirection::LeftToRight
        );
    }

    #[test]
    fn rtl_editable_overflow_starts_at_the_right_and_remains_reachable() {
        let mut fixture = BidiInputFixture::new_with_direction(
            "English text that is much wider than the input",
            8.0,
            120.0,
            false,
            1.0,
            LayoutDirection::RightToLeft,
        );
        let (scroll, document_offset, next_scroll, content_size) = fixture.cx.update(|cx| {
            let layout = &fixture.input.read(cx).layout_data;
            (
                layout.scroll_bounds.origin,
                layout.document_offset,
                layout.next_scroll_offset,
                layout.state.size,
            )
        });
        assert!(document_offset.x > Pixels::ZERO);
        assert!(
            scroll.x > Pixels::ZERO
                && scroll.x <= document_offset.x
                && document_offset.x - scroll.x <= px(2.01),
            "scroll={scroll:?}, offset={document_offset:?}, next={next_scroll:?}, content={content_size:?}"
        );

        fixture.scroll(Point::default());
        let position = fixture.point(3);
        fixture.down(position);
        fixture.assert_selection(3, 3);
        fixture.up(3);
    }

    #[test]
    fn plaintext_paragraph_direction_does_not_change_ltr_scrolling_direction() {
        let mut fixture = BidiInputFixture::new_with_configuration(
            "اسماء.شبكة/%20/test/",
            8.0,
            120.0,
            false,
            1.0,
            LayoutDirection::LeftToRight,
            Some(UnicodeBidi::Plaintext),
            None,
        );
        let document = fixture.document();
        let (scroll, document_offset) = fixture.cx.update(|cx| {
            let layout = &fixture.input.read(cx).layout_data;
            (layout.scroll_bounds.origin, layout.document_offset)
        });

        assert_eq!(
            document.visual_lines()[0].direction,
            gpui::ResolvedDirection::RightToLeft
        );
        assert_eq!(document_offset, Point::default());
        assert_eq!(scroll.x, Pixels::ZERO);
    }

    #[test]
    fn parley_scrolled_input_keeps_selection_and_marked_text_under_the_glyphs() {
        let text = "0123456789 שלום עולם 0123456789";
        let anchor = "0123456789 שלום עול".len();
        let focus = "0123456789 שלום עו".len();
        let mut fixture = BidiInputFixture::new(text, 8., 200., false, 1.5);
        fixture.scroll(point(px(45.), px(0.)));

        let position = fixture.point(anchor);
        fixture.down(position);
        fixture.drag(focus);
        fixture.assert_selection(anchor, focus);
        fixture.up(focus);

        let marked_start = text[..focus].encode_utf16().count();
        fixture
            .cx
            .update_window(fixture.window.into(), |_view, window, cx| {
                fixture.input.update(cx, |state, cx| {
                    state.replace_and_mark_text_in_range(
                        Some(marked_start..marked_start + 1),
                        "ל",
                        None,
                        window,
                        cx,
                    );
                });
            })
            .unwrap();
        fixture.cx.run_until_parked();
        fixture.scroll(point(px(45.), px(0.)));

        let origin = fixture.origin();
        let expected = fixture
            .document()
            .selection_bounds(focus..anchor, px(28.))
            .into_iter()
            .map(|bounds| {
                Bounds::new(
                    origin + bounds.origin + point(px(0.), px(26.)),
                    size(bounds.size.width, px(2.)),
                )
            })
            .collect();
        fixture.assert_quads(MARKED_COLOR, expected);
    }

    #[test]
    fn parley_wrapped_rtl_drag_preserves_the_anchor_across_rows() {
        let mut fixture = BidiInputFixture::new("שלום עולם שלום עולם", 8., 120., true, 1.5);
        assert!(fixture.document().line_count() > 1);

        let anchor = 2;
        let focus = "שלום עולם של".len();
        let start = fixture.point(anchor);
        assert!(fixture.point(focus).y > start.y);
        fixture.down(start);
        let position = fixture.point(focus) - point(px(0.), fixture.line_height() / 2. - px(0.05));
        fixture.dispatch(PlatformInput::MouseMove(MouseMoveEvent {
            position,
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        }));
        fixture.assert_selection(anchor, focus);
        fixture.drag(0);
        fixture.assert_selection(anchor, 0);
        fixture.up(0);
    }

    #[test]
    fn wrapped_opposite_direction_drag_reaches_the_document_end() {
        let text = "English text wraps here";
        let anchor = 3;
        let end = CaretPosition::attached_to_previous_cluster(text.len());
        let mut fixture = BidiInputFixture::new_with_direction(
            text,
            8.0,
            120.0,
            true,
            1.0,
            LayoutDirection::RightToLeft,
        );
        assert!(fixture.document().line_count() > 1);

        let anchor_point = fixture.point(anchor);
        let mut end_point = fixture.point_for_caret(end);
        assert!(end_point.y > anchor_point.y);
        end_point.x += px(1.0);

        fixture.down(anchor_point);
        fixture.drag_to(end_point);
        fixture.assert_caret_selection(anchor, end);
    }

    struct CenteredEditableTextView {
        extent: f32,
        input: Entity<EditableTextState>,
    }

    impl Render for CenteredEditableTextView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(320.0 + self.extent))
                .h(px(160.0 + self.extent))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(118.0))
                        .h(px(31.0))
                        .bg(CONTAINER_COLOR)
                        .child(
                            text_input("input")
                                .state(self.input.downgrade())
                                .bg(INPUT_COLOR)
                                .selection_color(SELECTION_COLOR)
                                .text_size(px(14.0)),
                        ),
                )
        }
    }

    fn only_quad(
        cx: &mut HeadlessAppContext,
        window: gpui::AnyWindowHandle,
        color: Hsla,
    ) -> Bounds<ScaledPixels> {
        let bounds = cx.solid_quad_bounds(window, color).unwrap();
        assert_eq!(bounds.len(), 1, "expected one rendered quad for {color:?}");
        bounds[0]
    }

    #[test]
    fn editable_text_keeps_its_device_pixel_offset_when_its_parent_moves() {
        for scale_factor in [1.0, 1.5] {
            let mut cx = HeadlessAppContext::new(Arc::new(TestTextSystem));
            let window = cx
                .open_window(size(px(420.0), px(260.0)), |window, cx| {
                    window.set_scale_factor(scale_factor);
                    let input = cx.new(|cx| {
                        let mut state = EditableTextState::new(StringStorage::from("x"), cx);
                        state.select_document(cx);
                        state
                    });

                    cx.new(|_| CenteredEditableTextView { extent: 0.0, input })
                })
                .unwrap();

            cx.run_until_parked();
            let any_window = window.into();
            let initial_container = only_quad(&mut cx, any_window, CONTAINER_COLOR);
            let initial_input = only_quad(&mut cx, any_window, INPUT_COLOR);
            let initial_selection = only_quad(&mut cx, any_window, SELECTION_COLOR);
            let expected_input_offset = initial_input.origin - initial_container.origin;
            let expected_selection_offset = initial_selection.origin - initial_input.origin;
            let mut container_origins = HashSet::from([(
                initial_container.origin.x.as_f32() as i32,
                initial_container.origin.y.as_f32() as i32,
            )]);

            for step in 1..=32 {
                window
                    .update(&mut cx, |view, _, cx| {
                        view.extent = step as f32;
                        cx.notify();
                    })
                    .unwrap();
                cx.run_until_parked();

                let container = only_quad(&mut cx, any_window, CONTAINER_COLOR);
                let input = only_quad(&mut cx, any_window, INPUT_COLOR);
                let selection = only_quad(&mut cx, any_window, SELECTION_COLOR);
                assert_eq!(
                    input.origin - container.origin,
                    expected_input_offset,
                    "editable control moved within its parent at scale {scale_factor}, step {step}"
                );
                assert_eq!(
                    selection.origin - input.origin,
                    expected_selection_offset,
                    "selected text moved within its control at scale {scale_factor}, step {step}"
                );
                container_origins.insert((
                    container.origin.x.as_f32() as i32,
                    container.origin.y.as_f32() as i32,
                ));
            }

            assert!(
                container_origins.len() > 8,
                "fixture did not cross enough device pixels at scale {scale_factor}"
            );
        }
    }

    #[test]
    fn editable_text_clears_selection_when_blurred() {
        let text = "selected text";
        let mut cx = HeadlessAppContext::new(Arc::new(TestTextSystem));
        let window = cx
            .open_window(size(px(420.0), px(260.0)), |_window, cx| {
                let input = cx.new(|cx| EditableTextState::new(StringStorage::from(text), cx));
                cx.new(|_| CenteredEditableTextView { extent: 0.0, input })
            })
            .unwrap();

        cx.run_until_parked();
        let input = window
            .update(&mut cx, |view, window, cx| {
                window.activate();
                let focus_handle = view.input.read(cx).focus_handle(cx);
                window.focus(&focus_handle, cx);
                view.input
                    .update(cx, |state, cx| state.select_to(text.len(), cx));

                view.input.clone()
            })
            .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(input.read(cx).selected_byte_range(), 0..text.len());
        });

        window
            .update(&mut cx, |_view, window, cx| window.blur(cx))
            .unwrap();
        cx.run_until_parked();

        cx.update(|cx| {
            assert_eq!(
                input.read(cx).caret_selection(),
                CaretSelection::from(CaretPosition::attached_to_previous_cluster(text.len()))
            );
        });
        assert!(
            cx.solid_quad_bounds(window.into(), SELECTION_COLOR)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn custom_placeholder_color_and_caret_size() {
        let color = rgba(0x33669980);
        let input = editable_text("i")
            .placeholder_color(color)
            .caret_w(px(3.))
            .caret_h(relative(0.5));
        assert_eq!(input.colors.placeholder, color.into_color());
        assert_eq!(input.caret_width, px(3.));
        assert_eq!(
            input.caret_height.to_pixels(px(20.).into(), px(16.)),
            px(10.)
        );
    }
}
