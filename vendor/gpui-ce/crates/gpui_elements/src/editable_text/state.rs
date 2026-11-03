use crate::editable_text::{
    StringStorage, TextBoundary, UnicodeTextStorage, actions::EditableTextActionHandler,
    caret::CaretNotify, history::EditableTextHistory, layout::EditableTextLayoutResult,
};
use gpui::{
    App, Bounds, CaretAffinity, CaretPosition, CaretSelection, CaretSelectionMovement,
    ClipboardItem, Context, ElementId, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, NavigationDirection, Pixels, Point, ShapedText, Subscription,
    TextDirection as Direction, TextMovement, TextRangeExt, TextSelectionKind, UTF16Selection,
    Window, point, utf16_to_utf8_offset,
};
use std::{borrow::Cow, ops::Range};

const CARET_PIXELS_EPSILON: Pixels = gpui::px(4.);

#[derive(Clone, Copy)]
enum SelectionGroup {
    Word,
    Line,
    Document,
}

impl SelectionGroup {
    fn layout_kind(self) -> Option<TextSelectionKind> {
        match self {
            Self::Word => Some(TextSelectionKind::Word),
            Self::Line => Some(TextSelectionKind::HardLine),
            Self::Document => None,
        }
    }
}

#[derive(Debug)]
struct OldDocumentVersion;

#[derive(Clone, Copy)]
struct SelectionDragVisualCaret {
    /// The selection state for which `position` is the displayed caret.
    selection: CaretSelection,
    /// The caret position directly under the pointer.
    position: CaretPosition,
}

/// Internal state for an editable text element.
pub struct EditableTextState {
    /// Backing text storage, usually `StringStorage`; custom storage can support long documents.
    storage: Box<dyn UnicodeTextStorage>,

    /// This input's affinity-aware selection and horizontal coordinate retained during vertical
    /// navigation. The caret is the cursor, and the anchor is the other selection endpoint.
    /// The coordinate is measured from the layout's left edge and is reset by operations
    /// other than consecutive vertical movements.
    selection_movement: CaretSelectionMovement,

    /// UTF-8 byte range in `storage` being composed during IME input.
    marked_range: Option<Range<usize>>,

    /// True while the user holds the mouse button to select text, including while dragging.
    /// Cleared on mouse-up or when this input loses focus.
    is_selecting: bool,
    /// Caret captured at single-click drag start. Reused by
    /// `adjust_drag_endpoint_at_visual_line_edge` because edge handling can shift
    /// `selection_movement.result.anchor`.
    selection_drag_anchor: Option<CaretPosition>,
    /// The pointer-aligned caret for the current drag selection.
    selection_drag_visual_caret: Option<SelectionDragVisualCaret>,
    /// Last click's position relative to this element, used to match nearby clicks.
    last_click_position: Option<Point<Pixels>>,
    /// Count of consecutive nearby clicks, used to choose single, double, or triple-click behavior.
    click_count: usize,

    focus_handle: FocusHandle,
    blur_subscription: Option<Subscription>,
    history: Option<EditableTextHistory>,
    accessibility_text_metrics: AccessibilityTextMetrics,

    pub(super) layout_data: EditableTextLayoutResult,
}

impl EventEmitter<CaretNotify> for EditableTextState {}

/// Event emitted when an `EditableTextState` is changed.
///
/// This is not suitable for input sanitation (which should occur before the mutation).
pub struct TextChanged;
impl EventEmitter<TextChanged> for EditableTextState {}

impl Focusable for EditableTextState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl AsRef<str> for EditableTextState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl EditableTextState {
    /// Uses a pre-existing state attached to the element at `key`, as long as the element has existed over consecutive frames.
    /// If the state does not yet exist, a new one is created using the default [`UnicodeTextStorage`] medium.
    pub fn use_keyed(key: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::use_keyed_init(key, window, cx, |_, _| StringStorage::default())
    }

    /// Uses a pre-existing state attached to the element at `key`, as long as the element has existed over consecutive frames.
    /// If the state does not yet exist, a new one is created calling `init` to create a [`UnicodeTextStorage`] medium.
    ///
    /// ```
    /// # use gpui::{RenderOnce, Window, App, IntoElement, ElementId};
    /// # use gpui_ce_elements::editable_text::{EditableTextState, StringStorage, editable_text};
    /// pub struct Form;
    /// impl RenderOnce for Form {
    ///     fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    ///         let field_a_id = ElementId::from("field_a");
    ///         let field_a = EditableTextState::use_keyed_init(field_a_id.clone(), window, cx,
    ///             |_window, _cx| StringStorage::from("this is some default editable text content"));
    ///         editable_text(field_a_id).state(field_a.downgrade())
    ///     }
    /// }
    /// ```
    pub fn use_keyed_init<F, StorageType>(
        key: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
        init: F,
    ) -> Entity<Self>
    where
        F: 'static + FnOnce(&mut Window, &mut Context<'_, EditableTextState>) -> StorageType,
        StorageType: 'static + UnicodeTextStorage,
    {
        window.use_keyed_state(key, cx, |window, cx| Self::new(init(window, cx), cx))
    }

    /// Creates a new EditableText state with a given storage medium.
    ///
    /// Does not intrinsicly handle the state being attached to an element
    /// over multiple frames (e.g. via [`RenderOnce`]). Use [`use_keyed`] or [`use_keyed_init`] for that.
    ///
    /// Expected to be called via [`AppContext::new`] such as:
    /// ```
    /// # use gpui::{AppContext, Window, App, Entity};
    /// # use gpui_ce_elements::editable_text::{StringStorage, EditableTextState};
    /// # fn new(_window: &mut Window, cx: &mut App) -> Entity<EditableTextState> {
    /// cx.new(|cx| EditableTextState::new(StringStorage::default(), cx))
    /// # }
    /// ```
    pub fn new(storage: impl UnicodeTextStorage + 'static, cx: &mut Context<Self>) -> Self {
        let accessibility_text_metrics = AccessibilityTextMetrics::new(storage.content_utf8());

        Self {
            storage: Box::new(storage),

            selection_movement: CaretSelectionMovement::default(),
            marked_range: None,

            is_selecting: false,
            selection_drag_anchor: None,
            selection_drag_visual_caret: None,
            last_click_position: None,
            click_count: 0,

            focus_handle: cx.focus_handle(),
            blur_subscription: None,
            // TODO: what is the best way to give users access to configure this via element
            history: Some(EditableTextHistory::default()),
            accessibility_text_metrics,

            layout_data: EditableTextLayoutResult::default(),
        }
    }

    /// Returns the current contents of [`storage`] as a string slice.
    pub fn as_str(&self) -> &str {
        self.storage.content_utf8()
    }

    pub fn version(&self) -> u16 {
        self.storage.version()
    }

    /// Replaces the contents of the stored text with the provided string slice.
    pub fn emplace(&mut self, content: &str, cx: &mut Context<Self>) {
        let len = self.storage.content_utf8().len();
        self.replace_text(0..len, content);
        self.emit_text_changed(cx);
        cx.notify();
    }

    /// Returns the current selection as a canonical logical-order UTF-8 byte range.
    pub(super) fn selected_byte_range(&self) -> Range<usize> {
        self.selection_movement.result.byte_range()
    }

    pub(super) fn caret_selection(&self) -> CaretSelection {
        self.selection_movement.result
    }

    fn set_selection(&mut self, selection: impl Into<CaretSelection>) {
        self.selection_movement = CaretSelectionMovement {
            result: selection.into(),
            vertical_navigation_x: None,
        };
    }

    pub(super) fn observe_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.blur_subscription.is_some() {
            return;
        }

        let focus_handle = self.focus_handle.clone();
        let subscription = cx.on_blur(&focus_handle, window, |_state, window, cx| {
            cx.defer_in(window, |state, _window, cx| {
                state.clear_selection(cx);
            });
        });
        self.blur_subscription = Some(subscription);
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.set_selection(self.caret_selection().caret);
        self.is_selecting = false;
        self.selection_drag_anchor = None;
        self.selection_drag_visual_caret = None;
        self.last_click_position = None;
        self.click_count = 0;

        cx.notify();
    }

    pub(super) fn visible_caret(&self) -> CaretPosition {
        match self.selection_drag_visual_caret {
            Some(drag_caret) if drag_caret.selection == self.selection_movement.result => {
                drag_caret.position
            }
            _ => self.caret_selection().caret,
        }
    }

    /// Returns the IME marked range for character operations.
    pub(super) fn marked_range(&self) -> Option<Range<usize>> {
        self.marked_range.clone()
    }

    pub(super) fn accessibility_text_metrics(&self) -> &AccessibilityTextMetrics {
        &self.accessibility_text_metrics
    }
}

pub(super) struct AccessibilityTextMetrics {
    pub(super) character_lengths: Vec<u8>,
    byte_offsets: Vec<usize>,
}

impl AccessibilityTextMetrics {
    fn new(text: &str) -> Self {
        let mut metrics = Self {
            character_lengths: Vec::new(),
            byte_offsets: Vec::new(),
        };
        metrics.refresh(text);

        metrics
    }

    fn refresh(&mut self, text: &str) {
        self.character_lengths.clear();
        self.byte_offsets.clear();

        for (offset, character) in text.char_indices() {
            self.character_lengths.push(character.len_utf8() as u8);
            self.byte_offsets.push(offset);
        }

        self.byte_offsets.push(text.len());
    }

    pub(super) fn character_indices_for_selection(
        &self,
        selection: CaretSelection,
    ) -> (usize, usize) {
        let byte_to_character = |offset: usize| {
            self.byte_offsets
                .partition_point(|byte_offset| *byte_offset <= offset)
                .saturating_sub(1)
        };

        (
            byte_to_character(selection.anchor.index),
            byte_to_character(selection.caret.index),
        )
    }
}

impl EditableTextState {
    fn replace_storage_range(&mut self, range: Range<usize>, text: &str) {
        self.storage.replace_range(range, text);
        self.accessibility_text_metrics
            .refresh(self.storage.content_utf8());
    }

    /// Validates/sanitizes incoming text according to the rules of the field.
    fn validate_incoming_text<'text>(
        &self,
        _range: &Range<usize>,
        text_to_insert: &'text str,
    ) -> Cow<'text, str> {
        // TODO: Apply text sanitization, ideally using externally-sourced implementations.
        // example optional/opt-in sanitations include:
        // - single-line fields should prune /n & /r
        // - maximum utf8 length
        // - numbers only
        // should also consider validation support, for features such as:
        // - total syntax evaluation (e.g. passwords)
        // - conforms to regex or math (e.g. ssn, phone number, email, etc)
        let mut text_to_insert = Cow::Borrowed(text_to_insert);

        if !self.layout_data.supports_multiline {
            text_to_insert = Cow::Owned(text_to_insert.replace("\n", "").replace("\r", ""));
        }

        /* A sample implementation of max-length sampled from gpuikit
        // Decide the effective new text up front (honouring `max_length`).
        // This avoids the "apply, then truncate" path which would leave the caret past the end.
        let max_length = None::<usize>;
        if let Some(cap) = max_length {
            let existing_len = self.as_str().len() - (range.end - range.start);
            let room = cap.saturating_sub(existing_len);
            text_to_insert = &text_to_insert[..text_to_insert.len().min(room)];
        }
        */

        // for now, this function is no-op
        text_to_insert
    }

    /// Internal method to record historical changes and perform text replacement in storage.
    /// Selection is moved to the end of the inserted text and ime marked range is cleared.
    fn replace_text(&mut self, range: Range<usize>, text_to_insert: &str) {
        let affinity = CaretAffinity::from(text_to_insert);

        self.replace_text_with_affinity(range, text_to_insert, affinity);
    }

    fn replace_text_with_affinity(
        &mut self,
        range: Range<usize>,
        text_to_insert: &str,
        affinity: CaretAffinity,
    ) {
        let end_pos = range.start + text_to_insert.len();
        self.record_history(range.clone(), text_to_insert.len());
        self.replace_storage_range(range, text_to_insert);

        self.set_selection(CaretPosition::from((end_pos, affinity)));
        self.marked_range = None;
    }

    fn emit_text_changed(&self, cx: &mut Context<Self>) {
        cx.emit(TextChanged);
    }
}

// Screen space (text layout engine output) & String space transformers
impl EditableTextState {
    fn current_document(&self) -> Result<&ShapedText, OldDocumentVersion> {
        if self.layout_data.state.last_seen_storage_version != self.storage.version() {
            return Err(OldDocumentVersion);
        }

        Ok(self
            .layout_data
            .document
            .as_deref()
            .expect("editable text layout invariant violated: current version has no document"))
    }

    fn point_for_caret(&self, caret: CaretPosition) -> Option<Point<Pixels>> {
        self.current_document()
            .ok()?
            .visual_position_for_caret(caret, self.layout_data.line_height)
            .map(|point| point + self.layout_data.document_offset)
    }

    /// Returns the storage range a deletion command should remove using the current text layout.
    /// Returns `None` when the current layout cannot provide the range.
    fn deletion_range_from_layout(
        &self,
        direction: NavigationDirection,
        boundary: TextBoundary,
    ) -> Option<Range<usize>> {
        let document = self.current_document().ok()?;
        let caret = self.caret_selection().caret;

        let movement = match (direction, boundary) {
            (NavigationDirection::Back, TextBoundary::Cluster) => {
                return document
                    .logical_cluster_before(caret)
                    .filter(|range| !range.is_empty() && self.as_str().contains_range(range));
            }
            (NavigationDirection::Forward, TextBoundary::Cluster) => {
                return document
                    .logical_cluster_after(caret)
                    .filter(|range| !range.is_empty() && self.as_str().contains_range(range));
            }
            (NavigationDirection::Back, TextBoundary::Word) => {
                Direction::Left.with_boundary(TextBoundary::Word)
            }
            (NavigationDirection::Forward, TextBoundary::Word) => {
                Direction::Right.with_boundary(TextBoundary::Word)
            }
            (NavigationDirection::Back, TextBoundary::HardLine) => {
                Direction::Start.with_boundary(TextBoundary::HardLine)
            }
            (NavigationDirection::Forward, TextBoundary::HardLine) => {
                Direction::End.with_boundary(TextBoundary::HardLine)
            }
            (_, TextBoundary::VisualLine) => return None,
            (_, TextBoundary::Document) => return None,
        };

        let target = document.caret_movement(caret, movement, None).result.index;

        Some(target.min(caret.index)..target.max(caret.index))
    }

    /// Returns the utf-8 character position of the start of the line that contains the provided pixel-point.
    fn caret_for_pixel_point(&self, point: Point<Pixels>, line_height: Pixels) -> CaretPosition {
        let storage_len_utf8 = self.as_str().len();
        if storage_len_utf8 == 0 {
            return CaretPosition::default();
        }

        let Ok(document) = self.current_document() else {
            return CaretPosition::attached_to_previous_cluster(storage_len_utf8);
        };

        document
            .closest_caret_for_pixel_point(point, line_height)
            .unwrap_or_else(|closest| closest)
    }

    fn index_for_pixel_point(&self, point: Point<Pixels>, line_height: Pixels) -> usize {
        self.caret_for_pixel_point(point, line_height).index
    }

    /// Returns the caret for `endpoint`, using `opposite_endpoint` as the selection's other end.
    /// Across visual lines, it maps an upper line's end to its start or a lower line's start to its
    /// end; otherwise, it returns `endpoint` unchanged.
    fn adjust_drag_endpoint_at_visual_line_edge(
        &self,
        endpoint: CaretPosition,
        opposite_endpoint: CaretPosition,
    ) -> CaretPosition {
        let Ok(document) = self.current_document() else {
            return endpoint;
        };

        let line_height = self.layout_data.line_height;
        let Some(endpoint_visual_position) =
            document.visual_position_for_caret(endpoint, line_height)
        else {
            return endpoint;
        };
        let Some(opposite_visual_position) =
            document.visual_position_for_caret(opposite_endpoint, line_height)
        else {
            return endpoint;
        };

        let (endpoint_edge, target_edge) =
            if opposite_visual_position.y > endpoint_visual_position.y {
                (
                    Direction::End.with_boundary(TextBoundary::VisualLine),
                    Direction::Start.with_boundary(TextBoundary::VisualLine),
                )
            } else if opposite_visual_position.y < endpoint_visual_position.y {
                (
                    Direction::Start.with_boundary(TextBoundary::VisualLine),
                    Direction::End.with_boundary(TextBoundary::VisualLine),
                )
            } else {
                return endpoint;
            };

        if document
            .caret_movement(endpoint, endpoint_edge, None)
            .result
            .index
            != endpoint.index
        {
            return endpoint;
        }

        document.caret_movement(endpoint, target_edge, None).result
    }

    fn find_point_for_caret(&self, caret: CaretPosition) -> Point<Pixels> {
        self.point_for_caret(caret).unwrap_or_default()
    }

    fn line_range_for_cut(&self) -> Range<usize> {
        let caret = self.caret_selection().caret;
        let mut range = match self.current_document() {
            Ok(document) => {
                let [start, end] = [Direction::Start, Direction::End].map(|direction| {
                    let movement = direction.with_boundary(TextBoundary::HardLine);

                    document.caret_movement(caret, movement, None).result.index
                });

                start.min(end)..start.max(end)
            }
            Err(OldDocumentVersion) => {
                let [start, end] =
                    [NavigationDirection::Back, NavigationDirection::Forward].map(|direction| {
                        self.storage.offset_from_caret(
                            caret.index,
                            direction,
                            TextBoundary::HardLine,
                        )
                    });

                start..end
            }
        };

        let adjustment = if range.end < self.as_str().len() {
            Some((&mut range.end, NavigationDirection::Forward))
        } else if range.start > 0 {
            Some((&mut range.start, NavigationDirection::Back))
        } else {
            None
        };

        if let Some((endpoint, direction)) = adjustment {
            *endpoint = self
                .storage
                .offset_from_caret(*endpoint, direction, TextBoundary::Cluster);
        }

        range
    }
}

// Internal user action / logical processors
impl EditableTextState {
    fn scroll_to_caret(&mut self) {
        if self.layout_data.scroll_bounds.is_empty() {
            return;
        }
        let Some(content_size) = self.layout_data.state.size else {
            return;
        };

        // point will be relative to content_size, and may or may not be within the current scroll_bounds
        let point = self.find_point_for_caret(self.visible_caret());

        // this scroll_offset diverges from the rest of gpui, as it is stored in the
        // positive real number space (interactivity stores it in the negatives)
        let mut scroll_offset = Cow::Borrowed(&self.layout_data.scroll_bounds.origin);

        if self.layout_data.scroll_bounds.contains(&point) {
            return;
        }

        // No existing "shift bounds origin so <point> is contained", but that is effectively what this does
        if point.x < self.layout_data.scroll_bounds.left() {
            scroll_offset.to_mut().x = point.x;
        }
        if point.y < self.layout_data.scroll_bounds.top() {
            scroll_offset.to_mut().y = point.y;
        }
        let right = self.layout_data.scroll_bounds.right();
        if point.x > right {
            scroll_offset.to_mut().x += point.x - right;
        }
        let bottom = self.layout_data.scroll_bounds.bottom();
        let point_bottom = point.y + self.layout_data.line_height;
        if point_bottom > bottom {
            let delta = point_bottom - bottom;
            scroll_offset.to_mut().y += delta;
        }

        if let Cow::Owned(mut offset) = scroll_offset {
            offset.x = offset.x.clamp(Pixels::ZERO, content_size.width);
            offset.y = offset.y.clamp(Pixels::ZERO, content_size.height);
            self.layout_data.next_scroll_offset = Some(offset);
        }
    }

    /// Moves the caret to the provided position.
    ///
    /// Will cause the current scroll position/offset to update on the next frame,
    /// if the line the carent is on is out of view.
    pub fn move_to(&mut self, caret_pos: usize, cx: &mut Context<Self>) {
        self.move_to_caret(self.caret_for_index(caret_pos), cx);
    }

    fn move_to_caret(&mut self, caret: CaretPosition, cx: &mut Context<Self>) {
        self.apply_selection_movement(self.selection_movement.move_or_select_to(caret, false), cx);
    }

    fn caret_for_index(&self, caret_pos: usize) -> CaretPosition {
        let len = self.storage.content_utf8().len();
        let index = caret_pos.min(len);

        if index > 0 && index == len {
            CaretPosition::attached_to_previous_cluster(index)
        } else {
            CaretPosition::attached_to_next_cluster(index)
        }
    }

    /// Changes the current selection to extend to the provided position.
    ///
    /// Will cause the current scroll position/offset to update on the next frame,
    /// if the line the carent is on is out of view.
    pub fn select_to(&mut self, caret_pos: usize, cx: &mut Context<Self>) {
        self.select_to_caret(self.caret_for_index(caret_pos), cx);
    }

    fn select_to_caret(&mut self, caret: CaretPosition, cx: &mut Context<Self>) {
        self.apply_selection_movement(self.selection_movement.move_or_select_to(caret, true), cx);
    }

    fn apply_selection_movement(
        &mut self,
        mut moved: CaretSelectionMovement,
        cx: &mut Context<Self>,
    ) {
        let storage_len = self.storage.content_utf8().len();
        moved.result.caret.index = moved.result.caret.index.min(storage_len);
        moved.result.anchor.index = moved.result.anchor.index.min(storage_len);
        self.selection_movement = moved;

        cx.emit(CaretNotify::PauseBlinking);
        self.scroll_to_caret();
        cx.notify();
    }

    /// Removes a chunk of text at the cursor/selection.
    /// No-op if the element is currently not accepting input.
    ///
    /// If there is a selection of multiple characters, the slice of text represented
    /// by range is replaced with an empty string.
    /// If there is no selection, `direction` and `boundary` are used to determine the slice of text to remove.
    ///
    /// [`NavigationDirection::Back`] represents scanning earlier in the text string from the caret.
    ///
    /// [`NavigationDirection::Forward`] represents scanning later in the text string from the caret.
    ///
    /// [`TextBoundary`] describes how far to jump from the caret.
    pub fn delete_linear(
        &mut self,
        direction: NavigationDirection,
        boundary: TextBoundary,
        cx: &mut Context<Self>,
    ) {
        if !self.layout_data.accepts_input {
            return;
        }

        let range = self.selected_byte_range();
        let had_selection = !range.is_empty();
        let range = if had_selection {
            range
        } else {
            self.deletion_range_from_layout(direction, boundary)
                .unwrap_or_else(|| {
                    self.storage.range_from_caret(
                        self.caret_selection().caret.index,
                        direction,
                        boundary,
                    )
                })
        };

        let storage_len_utf8 = self.storage.content_utf8().len();
        let start = range.start.min(storage_len_utf8);
        let end = range.end.max(start).min(storage_len_utf8);

        if !had_selection
            && start < end
            && ((direction == NavigationDirection::Back && start > 0)
                || (direction == NavigationDirection::Forward && end == storage_len_utf8))
        {
            self.replace_text_with_affinity(start..end, "", CaretAffinity::Upstream);
        } else {
            self.replace_text(start..end, "");
        }

        self.emit_text_changed(cx);
        cx.notify();
    }

    /// Moves the caret somewhere relative to its current location, according to `direction` and `boundary`.
    ///
    /// If there is currently a selection, the cursor will jump to the start/end of that selection based on `direction`.
    ///
    /// [`NavigationDirection::Back`] represents scanning earlier in the text string from the current caret.
    ///
    /// [`NavigationDirection::Forward`] represents scanning later in the text string from the current caret.
    ///
    /// [`TextBoundary`] describes how far to jump from the current caret
    pub fn nav_linear(
        &mut self,
        direction: NavigationDirection,
        boundary: TextBoundary,
        cx: &mut Context<Self>,
    ) {
        self.move_linear(direction, boundary, false, cx);
    }

    fn nav_semantic(&mut self, movement: TextMovement, cx: &mut Context<Self>) {
        self.move_semantic(movement, false, cx);
    }

    fn select_semantic(&mut self, movement: TextMovement, cx: &mut Context<Self>) {
        self.move_semantic(movement, true, cx);
    }

    /// Calculates destinations for both navigation and selection extension.
    fn move_semantic(&mut self, movement: TextMovement, extend: bool, cx: &mut Context<Self>) {
        let document_endpoint = match (movement.direction, movement.boundary) {
            (Direction::Start, TextBoundary::Document) => Some(0),
            (Direction::End, TextBoundary::Document) => Some(self.storage.content_utf8().len()),
            _ => None,
        };

        if let Some(index) = document_endpoint {
            // Document commands use storage bounds regardless of layout freshness.
            let caret = self.caret_for_index(index);
            self.apply_selection_movement(
                self.selection_movement.move_or_select_to(caret, extend),
                cx,
            );

            return;
        }

        if let Ok(document) = self.current_document() {
            let moved = document.selection_movement(
                self.selection_movement.result,
                movement,
                extend,
                self.selection_movement.vertical_navigation_x,
                self.layout_data.line_height,
            );
            self.apply_selection_movement(moved, cx);

            return;
        }

        let direction = match movement.direction {
            Direction::Left | Direction::Up | Direction::Start => NavigationDirection::Back,
            Direction::Right | Direction::Down | Direction::End => NavigationDirection::Forward,
        };
        let boundary = movement.boundary;
        let horizontal = matches!(movement.direction, Direction::Left | Direction::Right)
            && matches!(boundary, TextBoundary::Cluster | TextBoundary::Word);
        let collapse_selection =
            !extend && !self.selection_movement.result.is_empty() && horizontal;
        let index = if collapse_selection {
            let selected_range = self.selection_movement.result.byte_range();

            match direction {
                NavigationDirection::Back => selected_range.start,
                NavigationDirection::Forward => selected_range.end,
            }
        } else {
            self.storage
                .offset_from_caret(self.caret_selection().caret.index, direction, boundary)
        };
        let caret = CaretPosition::attached_to_next_cluster(index);

        self.apply_selection_movement(self.selection_movement.move_or_select_to(caret, extend), cx);
    }

    fn move_linear(
        &mut self,
        direction: NavigationDirection,
        boundary: TextBoundary,
        extend: bool,
        cx: &mut Context<Self>,
    ) {
        if boundary == TextBoundary::Document {
            let direction = match direction {
                NavigationDirection::Back => Direction::Start,
                NavigationDirection::Forward => Direction::End,
            };
            self.move_semantic(direction.with_boundary(boundary), extend, cx);

            return;
        }

        let index = if !extend && !self.selection_movement.result.is_empty() {
            let selected_range = self.selection_movement.result.byte_range();

            match direction {
                NavigationDirection::Back => selected_range.start,
                NavigationDirection::Forward => selected_range.end,
            }
        } else {
            self.storage
                .offset_from_caret(self.caret_selection().caret.index, direction, boundary)
        };
        let caret = self.caret_for_index(index);

        self.apply_selection_movement(self.selection_movement.move_or_select_to(caret, extend), cx);
    }

    /// Sets the current selection to be the entire text in the storage medium
    pub fn select_document(&mut self, cx: &mut Context<Self>) {
        self.set_selection(self.storage_selection_at(0, SelectionGroup::Document));
        cx.notify();
    }

    /// Extends the current selection to include some amount of textrelative the current
    /// location of the caret, according to `direction` and `boundary`.
    ///
    /// [`NavigationDirection::Back`] represents scanning earlier in the text string from the current caret.
    ///
    /// [`NavigationDirection::Forward`] represents scanning later in the text string from the current caret.
    ///
    /// [`TextBoundary`] describes how far to jump from the current caret
    pub fn select_linear(
        &mut self,
        direction: NavigationDirection,
        boundary: TextBoundary,
        cx: &mut Context<Self>,
    ) {
        self.move_linear(direction, boundary, true, cx);
    }

    /// Updates the mouse-click tracker so we can detect when a mouse click results in different actions.
    fn apply_click(&mut self, click_count: usize, text_position: Point<Pixels>) {
        let should_continue_click =
            click_count > 1 && self.is_position_nearly_at_previous_click(text_position);
        self.click_count = if should_continue_click {
            click_count
        } else {
            1
        };
        self.last_click_position = Some(text_position);
    }

    fn is_position_nearly_at_previous_click(&self, point: Point<Pixels>) -> bool {
        match self.last_click_position {
            None => false,
            Some(previous_pos) => point.is_nearly_eq(&previous_pos, CARET_PIXELS_EPSILON),
        }
    }

    fn storage_selection_at(&self, caret_pos: usize, group: SelectionGroup) -> Range<usize> {
        use NavigationDirection::*;
        use TextBoundary::*;

        match group {
            SelectionGroup::Word => self.storage.word_range_at(caret_pos),
            SelectionGroup::Line => {
                let line_start = self.storage.offset_from_caret(caret_pos, Back, HardLine);
                let line_end = self.storage.offset_from_caret(caret_pos, Forward, HardLine);
                let line_end_with_newline = if line_end < self.storage.content_utf8().len() {
                    self.storage.offset_from_caret(line_end, Forward, Cluster)
                } else {
                    line_end
                };

                line_start..line_end_with_newline
            }
            SelectionGroup::Document => 0..self.storage.content_utf8().len(),
        }
    }

    fn select_group_at(
        &mut self,
        point: Point<Pixels>,
        line_height: Pixels,
        group: SelectionGroup,
        cx: &mut Context<Self>,
    ) {
        let selection = match group.layout_kind() {
            Some(kind) => match self.current_document() {
                Ok(document) => document.selection_from_pixel_point(point, line_height, kind),
                Err(OldDocumentVersion) => {
                    let caret_pos = self.caret_for_pixel_point(point, line_height).index;

                    self.storage_selection_at(caret_pos, group)
                }
            },
            None => self.storage_selection_at(0, group),
        };
        self.set_selection(selection);
        cx.notify();
    }
}

// History management
impl EditableTextState {
    /// Returns the history log of the element, which is the data that supports undo/redo operations.
    pub fn history(&self) -> Option<&EditableTextHistory> {
        self.history.as_ref()
    }

    fn record_history(&mut self, range: Range<usize>, new_text_len: usize) {
        // Don't record during IME composition
        if self.marked_range.is_some() {
            return;
        }

        let Some(history) = &mut self.history else {
            return;
        };

        // Capture the text that will be replaced
        let old_text = &self.storage.content_utf8()[range.clone()];
        history.record(
            range,
            old_text,
            new_text_len,
            self.selection_movement.result,
        );
    }

    fn apply_from_history(&mut self, src: HistoryKind, dst: HistoryKind, cx: &mut Context<Self>) {
        let Some(entry) = self.history.as_mut().and_then(|history| history.take(src)) else {
            return;
        };

        let range = entry.char_range(self.storage.content_utf8().len());
        // Snapshot the sub-slice that is being replaced
        let removed_text = self.storage.content_utf8()[range.clone()].to_string();

        // Replace the slice with the history value
        self.replace_storage_range(range, &entry.old_text);

        // Push the entry onto the redo stack so the undo can be undone
        let selection = entry.selected_range;
        self.history
            .as_mut()
            .expect("history was available when the entry was taken")
            .push(dst, entry.as_inverted(removed_text));
        self.set_selection(selection);

        self.scroll_to_caret();
        cx.notify();
    }
}

impl EditableTextState {
    fn ime_resolve_range(&self, range_utf16: Option<Range<usize>>) -> Range<usize> {
        // Use a series of fallbacks to pick the range to operate on.
        // Fallback order: IME provided range, active IME marked range, selection
        let range = range_utf16.map(|range_utf16| self.storage.utf_range_16to8(&range_utf16));
        let range = range.or_else(|| self.marked_range.clone());
        let range = range.unwrap_or_else(|| self.selected_byte_range());

        let storage_len_utf8 = self.as_str().len();
        range.start.min(storage_len_utf8)..range.end.min(storage_len_utf8)
    }

    fn ime_mark_text_in_range(&mut self, range: &Range<usize>, text_len: usize) {
        self.marked_range = match text_len {
            0 => None,
            _ => Some(range.start..range.start + text_len),
        };
    }

    fn ime_mark_selected_range(
        &mut self,
        range_overwritten: &Range<usize>,
        new_selected_range_utf16: &Option<Range<usize>>,
        inserted_text: &str,
    ) {
        let selection: CaretSelection = {
            let new_range = new_selected_range_utf16.as_ref();
            let new_range = new_range.map(|range_utf16| {
                utf16_to_utf8_offset(inserted_text, range_utf16.start) + range_overwritten.start
                    ..utf16_to_utf8_offset(inserted_text, range_utf16.end) + range_overwritten.start
            });
            let new_range = new_range.unwrap_or_else(|| {
                range_overwritten.start + inserted_text.len()
                    ..range_overwritten.start + inserted_text.len()
            });

            new_range.into()
        };
        self.set_selection(selection);
    }
}

// IME handler
impl EntityInputHandler for EditableTextState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.storage.utf_range_16to8(&range_utf16);
        let storage_len_utf8 = self.storage.content_utf8().len();
        let clamped_range = range.start.min(storage_len_utf8)..range.end.min(storage_len_utf8);
        adjusted_range.replace(self.storage.utf_range_8to16(&clamped_range));
        Some(self.storage.content_utf8()[clamped_range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let selection_range = self.selected_byte_range();

        Some(UTF16Selection {
            range: self.storage.utf_range_8to16(&selection_range),
            reversed: self.selection_movement.result.endpoint_ordering()
                == std::cmp::Ordering::Greater,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.storage.utf_range_8to16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text_to_insert: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range_utf8 = self.ime_resolve_range(range_utf16);
        let text_to_insert = self.validate_incoming_text(&range_utf8, text_to_insert);
        self.replace_text(range_utf8, text_to_insert.as_ref());
        cx.emit(CaretNotify::PauseBlinking);
        self.emit_text_changed(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text_to_insert: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = self.ime_resolve_range(range_utf16);
        let text_to_insert = self.validate_incoming_text(&range, text_to_insert);
        self.replace_text(range.clone(), text_to_insert.as_ref());

        self.ime_mark_text_in_range(&range, text_to_insert.len());
        self.ime_mark_selected_range(&range, &new_selected_range_utf16, text_to_insert.as_ref());

        self.emit_text_changed(cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.storage.utf_range_16to8(&range_utf16);
        let line_height = window.line_height();

        let document = self.current_document().ok()?;
        let document_origin = bounds.origin + self.layout_data.document_offset
            - self.layout_data.scroll_bounds.origin;
        let start = range.start.min(document.text.len());
        let end = range.end.min(document.text.len());

        if start == end {
            let caret = CaretPosition::attached_to_next_cluster(start);
            let position = document.visual_position_for_caret(caret, line_height)?;
            return Some(Bounds::from_corners(
                document_origin + position,
                document_origin + position + point(CARET_PIXELS_EPSILON, line_height),
            ));
        }

        let selection = document
            .selection_bounds(start..end, line_height)
            .into_iter()
            .next()?;
        Some(Bounds::new(
            document_origin + selection.origin,
            selection.size,
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let point =
            point + self.layout_data.scroll_bounds.origin - self.layout_data.document_offset;
        let index = self.index_for_pixel_point(point, window.line_height());
        Some(self.storage.utf_offset_8to16(index))
    }
}

// Input Action handler
use super::{actions::*, history::HistoryKind};
impl<'app> EditableTextActionHandler<Context<'app, Self>> for EditableTextState {
    fn escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<'app, Self>) {
        self.set_selection(0);
        cx.notify();

        window.blur(cx);
    }

    fn insert_enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<'app, Self>) {
        if !self.layout_data.supports_multiline {
            return;
        }
        if !self.layout_data.accepts_input {
            return;
        }
        self.replace_text_in_range(None, "\n", window, cx);
    }

    fn insert_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<'app, Self>) {
        if !self.layout_data.accepts_input {
            return;
        }
        self.replace_text_in_range(None, "\t", window, cx);
    }

    fn delete_left(&mut self, _: &DeleteLeft, _: &mut Window, cx: &mut Context<'app, Self>) {
        self.delete_linear(NavigationDirection::Back, TextBoundary::Cluster, cx);
    }

    fn delete_right(&mut self, _: &DeleteRight, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.delete_linear(NavigationDirection::Forward, TextBoundary::Cluster, cx);
    }

    fn delete_word_left(
        &mut self,
        _: &DeleteWordLeft,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.delete_linear(NavigationDirection::Back, TextBoundary::Word, cx);
    }

    fn delete_word_right(
        &mut self,
        _: &DeleteWordRight,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.delete_linear(NavigationDirection::Forward, TextBoundary::Word, cx);
    }

    fn delete_to_line_start(
        &mut self,
        _: &DeleteToLineStart,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.delete_linear(NavigationDirection::Back, TextBoundary::HardLine, cx);
    }

    fn delete_to_line_end(
        &mut self,
        _: &DeleteToLineEnd,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.delete_linear(NavigationDirection::Forward, TextBoundary::HardLine, cx);
    }

    fn nav_left(&mut self, _: &NavLeft, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::Left.with_boundary(TextBoundary::Cluster), cx);
    }

    fn nav_right(&mut self, _: &NavRight, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::Right.with_boundary(TextBoundary::Cluster), cx);
    }

    fn nav_up(&mut self, _: &NavUp, _window: &mut Window, cx: &mut Context<'app, Self>) {
        let direction = if self.layout_data.supports_multiline {
            Direction::Up
        } else {
            Direction::Start
        };

        self.nav_semantic(direction.with_boundary(TextBoundary::VisualLine), cx);
    }

    fn nav_down(&mut self, _: &NavDown, _window: &mut Window, cx: &mut Context<'app, Self>) {
        let direction = if self.layout_data.supports_multiline {
            Direction::Down
        } else {
            Direction::End
        };

        self.nav_semantic(direction.with_boundary(TextBoundary::VisualLine), cx);
    }

    fn nav_line_start(&mut self, _: &NavLineStart, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::Start.with_boundary(TextBoundary::HardLine), cx);
    }

    fn nav_line_end(&mut self, _: &NavLineEnd, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::End.with_boundary(TextBoundary::HardLine), cx);
    }

    fn nav_start(&mut self, _: &NavDocumentStart, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::Start.with_boundary(TextBoundary::Document), cx);
    }

    fn nav_end(&mut self, _: &NavDocumentEnd, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::End.with_boundary(TextBoundary::Document), cx);
    }

    fn nav_left_word(&mut self, _: &NavWordLeft, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::Left.with_boundary(TextBoundary::Word), cx);
    }

    fn nav_right_word(&mut self, _: &NavWordRight, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.nav_semantic(Direction::Right.with_boundary(TextBoundary::Word), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.select_document(cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.select_semantic(Direction::Left.with_boundary(TextBoundary::Cluster), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.select_semantic(Direction::Right.with_boundary(TextBoundary::Cluster), cx);
    }

    fn select_up(&mut self, _: &SelectUp, _window: &mut Window, cx: &mut Context<'app, Self>) {
        let movement = if self.layout_data.supports_multiline {
            Direction::Up.with_boundary(TextBoundary::VisualLine)
        } else {
            Direction::Start.with_boundary(TextBoundary::Document)
        };

        self.select_semantic(movement, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _window: &mut Window, cx: &mut Context<'app, Self>) {
        let movement = if self.layout_data.supports_multiline {
            Direction::Down.with_boundary(TextBoundary::VisualLine)
        } else {
            Direction::End.with_boundary(TextBoundary::Document)
        };

        self.select_semantic(movement, cx);
    }

    fn select_start(
        &mut self,
        _: &SelectDocumentStart,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.select_semantic(Direction::Start.with_boundary(TextBoundary::Document), cx);
    }

    fn select_end(&mut self, _: &SelectDocumentEnd, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.select_semantic(Direction::End.with_boundary(TextBoundary::Document), cx);
    }

    fn select_left_word(
        &mut self,
        _: &SelectWordLeft,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.select_semantic(Direction::Left.with_boundary(TextBoundary::Word), cx);
    }

    fn select_right_word(
        &mut self,
        _: &SelectWordRight,
        _w: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        self.select_semantic(Direction::Right.with_boundary(TextBoundary::Word), cx);
    }

    fn cut(&mut self, _: &Cut, _w: &mut Window, cx: &mut Context<'app, Self>) {
        if !self.layout_data.accepts_input {
            return;
        }

        let range = self.selected_byte_range();
        let range_to_cut = if range.is_empty() {
            self.line_range_for_cut()
        } else {
            range
        };

        let slice = &self.storage.content_utf8()[range_to_cut.clone()];
        cx.write_to_clipboard(ClipboardItem::new_string(slice.to_string()));
        self.replace_text(range_to_cut, "");

        self.emit_text_changed(cx);
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, _w: &mut Window, cx: &mut Context<'app, Self>) {
        let range = self.selected_byte_range();

        if range.is_empty() {
            return;
        }

        let slice = &self.storage.content_utf8()[range];
        cx.write_to_clipboard(ClipboardItem::new_string(slice.to_string()));
    }

    fn paste(&mut self, _: &Paste, _w: &mut Window, cx: &mut Context<'app, Self>) {
        if !self.layout_data.accepts_input {
            return;
        }

        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };

        let range = self.ime_resolve_range(None);
        let text_to_insert = self.validate_incoming_text(&range, &text);
        self.replace_text(range, text_to_insert.as_ref());
        self.emit_text_changed(cx);
        cx.notify();
    }

    fn undo(&mut self, _: &Undo, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.apply_from_history(HistoryKind::Undo, HistoryKind::Redo, cx);
    }

    fn redo(&mut self, _: &Redo, _w: &mut Window, cx: &mut Context<'app, Self>) {
        self.apply_from_history(HistoryKind::Redo, HistoryKind::Undo, cx);
    }

    fn on_mouse_down(
        &mut self,
        event: &gpui::MouseDownEvent,
        text_position: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        const DOUBLE_CLICK: usize = 2;
        const TRIPLE_CLICK: usize = 3;

        let line_height = self.layout_data.line_height;
        let caret = self.caret_for_pixel_point(text_position, line_height);

        self.is_selecting = true;
        self.selection_drag_visual_caret = None;
        self.apply_click(event.click_count, text_position);

        match self.click_count {
            DOUBLE_CLICK => {
                self.select_group_at(text_position, line_height, SelectionGroup::Word, cx)
            }
            TRIPLE_CLICK => {
                self.select_group_at(text_position, line_height, SelectionGroup::Line, cx)
            }
            _ if event.modifiers.shift => self.select_to_caret(caret, cx),
            _ => self.move_to_caret(caret, cx),
        }

        self.selection_drag_anchor = (self.click_count == 1 && !event.modifiers.shift)
            .then_some(self.selection_movement.result.anchor);
    }

    fn on_mouse_up(
        &mut self,
        _event: &gpui::MouseUpEvent,
        _w: &mut Window,
        _cx: &mut Context<'app, Self>,
    ) {
        self.is_selecting = false;
        self.selection_drag_anchor = None;
    }

    fn on_mouse_move(
        &mut self,
        _event: &gpui::MouseMoveEvent,
        text_position: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<'app, Self>,
    ) {
        if self.is_selecting && self.click_count == 1 {
            let pointer_caret =
                self.caret_for_pixel_point(text_position, self.layout_data.line_height);
            let mut selection_caret = pointer_caret;

            if let Some(anchor) = self.selection_drag_anchor {
                self.selection_movement.result.anchor =
                    self.adjust_drag_endpoint_at_visual_line_edge(anchor, selection_caret);
                selection_caret =
                    self.adjust_drag_endpoint_at_visual_line_edge(selection_caret, anchor);
            }

            self.selection_drag_visual_caret = Some(SelectionDragVisualCaret {
                selection: self.selection_movement.result.with_caret(selection_caret),
                position: pointer_caret,
            });
            self.select_to_caret(selection_caret, cx);
        }
    }
}

/// Backlog:
/// - tests for document layout and wrap boundaries;
///     permutations of: single and multiline fields, wrap vs no-wrap, overflow scroll vs no scroll
#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

    use super::*;
    use crate::editable_text::StringStorage;
    use gpui::{
        AppContext, Entity, IntoElement, PlatformTextSystem, Render, TestAppContext, TextRun,
        TextSystem, WindowHandle, WindowTextSystem, div, font, px,
    };
    use gpui_parley::{ParleyTextSystem, SystemFonts};

    type TestMovementAction =
        fn(&mut EditableTextState, &mut Window, &mut Context<'_, EditableTextState>);

    struct TestView {
        input: Entity<EditableTextState>,
    }

    struct WrappingStorage {
        text: String,
        version: u16,
    }

    impl UnicodeTextStorage for WrappingStorage {
        fn version(&self) -> u16 {
            self.version
        }

        fn content_utf8(&self) -> &str {
            &self.text
        }

        fn len_utf16(&self) -> usize {
            self.text.encode_utf16().count()
        }

        fn replace_range(&mut self, range: Range<usize>, text: &str) {
            self.text.replace_range(range, text);
            self.version = self.version.wrapping_add(1);
        }
    }

    impl Render for TestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn default_state(content: &str, cx: &mut Context<EditableTextState>) -> EditableTextState {
        let mut state = EditableTextState::new(StringStorage::from(content), cx);
        state.layout_data.state.last_seen_storage_version = state.version().wrapping_sub(1);
        state
    }

    #[test]
    fn accessibility_selection_uses_character_indices_and_preserves_direction() {
        let text = "A😀日本B";
        let metrics = AccessibilityTextMetrics::new(text);
        let start = 1;
        let end = "A😀日本".len();
        let forward = CaretSelection {
            anchor: CaretPosition::attached_to_next_cluster(start),
            caret: CaretPosition::attached_to_next_cluster(end),
        };
        let reversed = CaretSelection {
            anchor: forward.caret,
            caret: forward.anchor,
        };

        assert_eq!(metrics.character_indices_for_selection(forward), (1, 4));
        assert_eq!(metrics.character_indices_for_selection(reversed), (4, 1));
        assert_eq!(
            metrics.character_indices_for_selection(CaretSelection::from(5)),
            (2, 2)
        );
    }

    #[gpui::test]
    fn accessibility_metrics_start_current_and_refresh_after_multibyte_edits(
        cx: &mut TestAppContext,
    ) {
        let view = create_test_input(cx, "A😀B", 5);
        view.update(cx, |view, _window, cx| {
            view.input.update(cx, |input, _cx| {
                let metrics = input.accessibility_text_metrics();
                let repeated = input.accessibility_text_metrics();

                assert!(std::ptr::eq(metrics, repeated));
                assert_eq!(metrics.character_lengths, [1, 4, 1]);
                assert_eq!(metrics.byte_offsets, [0, 1, 5, 6]);
                assert_eq!(
                    repeated.character_indices_for_selection(input.caret_selection()),
                    (2, 2)
                );

                input.replace_text(1..5, "日本");
                assert_eq!(input.as_str(), "A日本B");

                let metrics = input.accessibility_text_metrics();
                assert_eq!(metrics.character_lengths, [1, 3, 3, 1]);
                assert_eq!(metrics.byte_offsets, [0, 1, 4, 7, 8]);
                assert_eq!(
                    metrics.character_indices_for_selection(input.caret_selection()),
                    (3, 3)
                );

                input.replace_text(0..input.as_str().len(), "");
                let metrics = input.accessibility_text_metrics();
                assert!(metrics.character_lengths.is_empty());
                assert_eq!(metrics.byte_offsets, [0]);
            });
        })
        .unwrap();
    }

    #[gpui::test]
    fn accessibility_metrics_refresh_when_storage_version_wraps(cx: &mut TestAppContext) {
        let view = cx.add_window(|_window, cx| {
            let input = cx.new(|cx| {
                EditableTextState::new(
                    WrappingStorage {
                        text: "😀".into(),
                        version: u16::MAX,
                    },
                    cx,
                )
            });

            TestView { input }
        });

        view.update(cx, |view, _window, cx| {
            view.input.update(cx, |input, _cx| {
                assert_eq!(input.version(), u16::MAX);
                assert_eq!(input.accessibility_text_metrics().byte_offsets, [0, 4]);

                input.replace_text(0..4, "日本");
                assert_eq!(input.version(), 0);
                assert_eq!(input.accessibility_text_metrics().byte_offsets, [0, 3, 6]);
            });
        })
        .unwrap();
    }

    fn create_test_input(
        cx: &mut TestAppContext,
        content: &str,
        range: impl Into<CaretSelection>,
    ) -> WindowHandle<TestView> {
        cx.add_window(|_window, cx| {
            let input = cx.new(|cx| {
                let mut input = default_state(content, cx);
                input.set_selection(range);
                input.layout_data.accepts_input = true;
                input
            });
            TestView { input }
        })
    }

    fn movement_document(text: &str, wrap_width: Option<Pixels>) -> Arc<ShapedText> {
        let backend = Arc::new(
            ParleyTextSystem::new_with_system_font(SystemFonts::Skip, "IBM Plex Sans")
                .with_fallback_families(["IBM Plex Sans", "Noto Color Emoji"]),
        );
        backend
            .add_fonts(vec![
                Cow::Borrowed(*gpui_fonts::IBM_PLEX),
                Cow::Borrowed(*gpui_fonts::NOTO_COLOR_EMOJI),
            ])
            .unwrap();
        let text_system = WindowTextSystem::new(Arc::new(TextSystem::new(backend)));
        let run = TextRun {
            len: text.len(),
            font: font("IBM Plex Sans"),
            ..Default::default()
        };

        Arc::new(
            text_system
                .shape_text(text, px(18.), &[run], wrap_width, None)
                .unwrap(),
        )
    }

    fn update_movement_input(
        view: WindowHandle<TestView>,
        cx: &mut TestAppContext,
        update: impl FnOnce(&mut EditableTextState, &mut Window, &mut Context<EditableTextState>),
    ) {
        view.update(cx, |view, window, cx| {
            view.input.update(cx, |input, cx| update(input, window, cx));
        })
        .unwrap();
    }

    // Disable grouping for predictable test behavior
    fn without_history_grouping(state: &mut EditableTextState) {
        state
            .history
            .get_or_insert_default()
            .set_grouping_interval(Duration::from_secs(0));
    }

    fn is_history_kind_available(state: &EditableTextState, kind: HistoryKind) -> bool {
        state
            .history()
            .map(|history| history.has_next(kind))
            .unwrap_or_default()
    }

    #[gpui::test]
    fn test_left_at_start_of_content(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left(&NavLeft, window, cx);
            assert_eq!(input.caret_selection(), 0.into());
        });
    }

    #[gpui::test]
    fn test_left_moves_by_grapheme(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 3);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left(&NavLeft, window, cx);
            assert_eq!(input.caret_selection(), 2.into());
        });
    }

    #[gpui::test]
    fn test_left_collapses_selection_to_start(cx: &mut TestAppContext) {
        let view = create_test_input(
            cx,
            "hello",
            CaretSelection {
                anchor: CaretPosition::attached_to_next_cluster(4),
                caret: CaretPosition::attached_to_next_cluster(1),
            },
        );
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left(&NavLeft, window, cx);
            assert_eq!(input.caret_selection(), 1.into());
        });
    }

    #[gpui::test]
    fn test_left_stops_at_end_of_line(cx: &mut TestAppContext) {
        // "ab\ncd" - cursor at position 3 (start of "cd", after newline)
        // Pressing left should move to position 2 (end of "ab", before newline)
        let view = create_test_input(cx, "ab\ncd", 3);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left(&NavLeft, window, cx);
            assert_eq!(input.caret_selection(), 2.into()); // cursor at end of line 1
        });
    }

    #[gpui::test]
    fn test_right_at_end_of_content(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 5);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right(&NavRight, window, cx);
            assert_eq!(input.caret_selection(), 5.into());
        });
    }

    #[gpui::test]
    fn test_right_moves_by_grapheme(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 2);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right(&NavRight, window, cx);
            assert_eq!(input.caret_selection(), 3.into());
        });
    }

    #[gpui::test]
    fn test_right_collapses_selection_to_end(cx: &mut TestAppContext) {
        let view = create_test_input(
            cx,
            "hello",
            CaretSelection {
                anchor: CaretPosition::attached_to_next_cluster(4),
                caret: CaretPosition::attached_to_next_cluster(1),
            },
        );
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right(&NavRight, window, cx);
            assert_eq!(input.caret_selection(), 4.into());
        });
    }

    #[gpui::test]
    fn test_right_stops_at_end_of_line(cx: &mut TestAppContext) {
        // "ab\ncd" - cursor at position 1 (after 'a')
        // Pressing right should move to position 2 (end of "ab", before newline)
        let view = create_test_input(cx, "ab\ncd", 1);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right(&NavRight, window, cx);
            assert_eq!(input.caret_selection(), 2.into()); // cursor at end of line 1
        });
    }

    #[gpui::test]
    fn test_right_crosses_newline(cx: &mut TestAppContext) {
        // "ab\ncd" - cursor at position 2 (end of "ab", before newline)
        // Pressing right should move to position 3 (after newline, start of "cd")
        let view = create_test_input(cx, "ab\ncd", 2);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right(&NavRight, window, cx);
            assert_eq!(input.caret_selection(), 3.into()); // cursor at start of line 2
        });
    }

    #[gpui::test]
    fn test_left_crosses_newline(cx: &mut TestAppContext) {
        // "ab\ncd" - cursor at position 2 (end of "ab", before newline)
        // Pressing left should move to position 1 (after 'a')
        let view = create_test_input(cx, "ab\ncd", 2);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left(&NavLeft, window, cx);
            assert_eq!(input.caret_selection(), 1.into());
        });
    }

    #[gpui::test]
    fn test_home_moves_to_line_start(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "first\nsecond", 9);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_line_start(&NavLineStart, window, cx);
            assert_eq!(input.caret_selection(), 6.into());
        });
    }

    #[gpui::test]
    fn test_end_moves_to_line_end(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "first\nsecond", 8);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_line_end(&NavLineEnd, window, cx);
            assert_eq!(input.caret_selection(), 12.into());
        });
    }

    #[gpui::test]
    fn test_move_to_beginning(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "first\nsecond\nthird", 9);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_start(&NavDocumentStart, window, cx);
            assert_eq!(input.caret_selection(), 0.into());
        });
    }

    #[gpui::test]
    fn test_move_to_end(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "first\nsecond\nthird", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_end(&NavDocumentEnd, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection::from(CaretPosition::attached_to_previous_cluster(18))
            );
        });
    }

    #[gpui::test]
    fn test_word_left_at_start(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left_word(&NavWordLeft, window, cx);
            assert_eq!(input.caret_selection(), 0.into());
        });
    }

    #[gpui::test]
    fn test_word_left_stops_at_boundary(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world test", 11);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left_word(&NavWordLeft, window, cx);
            assert_eq!(input.caret_selection(), 6.into());
        });
    }

    #[gpui::test]
    fn test_word_right_at_end(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world", 11);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right_word(&NavWordRight, window, cx);
            assert_eq!(input.caret_selection(), 11.into());
        });
    }

    #[gpui::test]
    fn test_word_right_stops_at_boundary(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world test", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_right_word(&NavWordRight, window, cx);
            assert_eq!(input.caret_selection(), 5.into());
        });
    }

    #[gpui::test]
    fn test_select_left_extends_selection(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 3);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_left(&SelectLeft, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(3),
                    caret: CaretPosition::attached_to_next_cluster(2),
                }
            );
        });
    }

    #[gpui::test]
    fn test_select_right_extends_selection(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 2..2);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_right(&SelectRight, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(2),
                    caret: CaretPosition::attached_to_next_cluster(3),
                }
            );
        });
    }

    #[gpui::test]
    fn test_select_all(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello\nworld", 3);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_all(&SelectAll, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(11),
                    caret: CaretPosition::attached_to_next_cluster(0),
                }
            );
        });
    }

    #[gpui::test]
    fn test_select_to_beginning(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world", 6);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_start(&SelectDocumentStart, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(6),
                    caret: CaretPosition::attached_to_next_cluster(0),
                }
            );
        });
    }

    #[gpui::test]
    fn test_select_to_end(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world", 6);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_end(&SelectDocumentEnd, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(6),
                    caret: CaretPosition::attached_to_previous_cluster(11),
                }
            );
        });
    }

    #[derive(Clone, Copy, Debug)]
    enum TestDeletion {
        PreviousCluster,
        NextCluster,
        PreviousWord,
        NextWord,
        LineStart,
        LineEnd,
    }

    #[gpui::test]
    fn deletion_commands_preserve_results_carets_and_history(cx: &mut TestAppContext) {
        let selected_hello = CaretSelection {
            anchor: CaretPosition::attached_to_next_cluster(5),
            caret: CaretPosition::attached_to_next_cluster(0),
        };
        let selected_world = CaretSelection {
            anchor: CaretPosition::attached_to_next_cluster(11),
            caret: CaretPosition::attached_to_next_cluster(6),
        };

        for (command, text, selection, remaining, expected_caret) in [
            (
                TestDeletion::PreviousCluster,
                "hello world",
                selected_world,
                "hello ",
                Some(CaretPosition::attached_to_next_cluster(6)),
            ),
            (
                TestDeletion::PreviousCluster,
                "hello",
                5.into(),
                "hell",
                Some(CaretPosition::attached_to_previous_cluster(4)),
            ),
            (
                TestDeletion::PreviousCluster,
                "hello",
                1.into(),
                "ello",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::PreviousCluster,
                "hello",
                0.into(),
                "hello",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::PreviousCluster,
                "Hi 👋",
                7.into(),
                "Hi ",
                Some(CaretPosition::attached_to_previous_cluster(3)),
            ),
            (
                TestDeletion::PreviousCluster,
                "a😀b",
                5.into(),
                "ab",
                Some(CaretPosition::attached_to_previous_cluster(1)),
            ),
            (
                TestDeletion::PreviousCluster,
                "a👨‍👩‍👧b",
                (19..19).into(),
                "ab",
                Some(CaretPosition::attached_to_previous_cluster(1)),
            ),
            (
                TestDeletion::NextCluster,
                "hello world",
                selected_hello,
                " world",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::NextCluster,
                "hello",
                0.into(),
                "ello",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::NextCluster,
                "hello",
                4.into(),
                "hell",
                Some(CaretPosition::attached_to_previous_cluster(4)),
            ),
            (
                TestDeletion::NextCluster,
                "hello",
                5.into(),
                "hello",
                Some(CaretPosition::attached_to_next_cluster(5)),
            ),
            (
                TestDeletion::NextCluster,
                "a😀b",
                1.into(),
                "ab",
                Some(CaretPosition::attached_to_next_cluster(1)),
            ),
            (
                TestDeletion::PreviousWord,
                "hello world",
                5.into(),
                " world",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::PreviousWord,
                "hello world",
                selected_hello,
                " world",
                None,
            ),
            (
                TestDeletion::PreviousWord,
                "hello world",
                0.into(),
                "hello world",
                None,
            ),
            (
                TestDeletion::NextWord,
                "hello world",
                0.into(),
                " world",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::NextWord,
                "hello world",
                6.into(),
                "hello ",
                None,
            ),
            (
                TestDeletion::NextWord,
                "hello world",
                selected_hello,
                " world",
                None,
            ),
            (
                TestDeletion::NextWord,
                "hello world",
                11.into(),
                "hello world",
                None,
            ),
            (
                TestDeletion::LineStart,
                "hello world",
                5.into(),
                " world",
                Some(CaretPosition::attached_to_next_cluster(0)),
            ),
            (
                TestDeletion::LineStart,
                "line1\nline2\nline3",
                8.into(),
                "line1\nne2\nline3",
                None,
            ),
            (
                TestDeletion::LineStart,
                "hello world",
                0.into(),
                "hello world",
                None,
            ),
            (
                TestDeletion::LineEnd,
                "hello world",
                5.into(),
                "hello",
                Some(CaretPosition::attached_to_previous_cluster(5)),
            ),
            (
                TestDeletion::LineEnd,
                "line1\nline2\nline3",
                8.into(),
                "line1\nli\nline3",
                None,
            ),
            (
                TestDeletion::LineEnd,
                "hello world",
                11.into(),
                "hello world",
                None,
            ),
        ] {
            let view = create_test_input(cx, text, selection);

            update_movement_input(view, cx, |input, window, cx| {
                without_history_grouping(input);

                match command {
                    TestDeletion::PreviousCluster => input.delete_left(&DeleteLeft, window, cx),
                    TestDeletion::NextCluster => input.delete_right(&DeleteRight, window, cx),
                    TestDeletion::PreviousWord => {
                        input.delete_word_left(&DeleteWordLeft, window, cx)
                    }
                    TestDeletion::NextWord => input.delete_word_right(&DeleteWordRight, window, cx),
                    TestDeletion::LineStart => {
                        input.delete_to_line_start(&DeleteToLineStart, window, cx)
                    }
                    TestDeletion::LineEnd => input.delete_to_line_end(&DeleteToLineEnd, window, cx),
                }

                assert_eq!(
                    input.as_str(),
                    remaining,
                    "{command:?}: {text:?} at {selection:?}"
                );

                if let Some(caret) = expected_caret {
                    assert_eq!(
                        input.caret_selection(),
                        caret.into(),
                        "{command:?}: {text:?}"
                    );
                }

                assert_history_round_trip(input, text, selection, window, cx);
            });
        }
    }

    fn assert_history_round_trip(
        input: &mut EditableTextState,
        original: &str,
        selection: CaretSelection,
        window: &mut Window,
        cx: &mut Context<EditableTextState>,
    ) {
        let edited = input.as_str().to_owned();
        input.undo(&Undo, window, cx);
        assert_eq!(input.as_str(), original);
        assert_eq!(input.caret_selection(), selection);

        input.redo(&Redo, window, cx);
        assert_eq!(input.as_str(), edited);
    }

    #[gpui::test]
    fn enter_preserves_insertion_selection_and_history(cx: &mut TestAppContext) {
        for (selection, expected) in [
            (CaretSelection::from(5), "hello\n world"),
            (
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(6),
                    caret: CaretPosition::attached_to_next_cluster(5),
                },
                "hello\nworld",
            ),
        ] {
            let view = create_test_input(cx, "hello world", selection);
            update_movement_input(view, cx, |input, window, cx| {
                without_history_grouping(input);
                input.layout_data.supports_multiline = true;

                input.insert_enter(&Enter, window, cx);
                assert_eq!(input.as_str(), expected);
                assert_eq!(input.caret_selection(), 6.into());
                assert_eq!(
                    input.caret_selection().caret.affinity,
                    CaretAffinity::Downstream
                );
                assert_history_round_trip(input, "hello world", selection, window, cx);
            });
        }
    }

    #[gpui::test]
    fn test_copy_with_selection(cx: &mut TestAppContext) {
        let view = create_test_input(
            cx,
            "hello world",
            CaretSelection {
                anchor: CaretPosition::attached_to_next_cluster(11),
                caret: CaretPosition::attached_to_next_cluster(6),
            },
        );
        update_movement_input(view, cx, |input, window, cx| {
            input.copy(&Copy, window, cx);
        });

        let clipboard = cx.read_from_clipboard();
        assert!(clipboard.is_some());
        assert_eq!(clipboard.unwrap().text().as_deref(), Some("world"));
    }

    #[gpui::test]
    fn cut_preserves_selected_text_clipboard_and_history(cx: &mut TestAppContext) {
        let view = create_test_input(
            cx,
            "hello world",
            CaretSelection {
                anchor: CaretPosition::attached_to_next_cluster(5),
                caret: CaretPosition::attached_to_next_cluster(0),
            },
        );
        update_movement_input(view, cx, |input, window, cx| {
            without_history_grouping(input);
            let selection = input.caret_selection();

            input.cut(&Cut, window, cx);
            assert_eq!(input.as_str(), " world");
            assert_eq!(input.caret_selection(), 0.into());
            assert_history_round_trip(input, "hello world", selection, window, cx);
        });

        let clipboard = cx.read_from_clipboard();
        assert_eq!(clipboard.unwrap().text().as_deref(), Some("hello"));
    }

    #[gpui::test]
    fn paste_preserves_inserted_text_carets_and_history(cx: &mut TestAppContext) {
        for (original, caret, inserted, expected, expected_caret) in [
            ("hello world", 5, " there", "hello there world", 11),
            ("hello", 5, " world", "hello world", 11),
        ] {
            let view = create_test_input(cx, original, caret);
            cx.write_to_clipboard(ClipboardItem::new_string(inserted.to_string()));

            update_movement_input(view, cx, |input, window, cx| {
                without_history_grouping(input);
                let selection = input.caret_selection();

                EditableTextActionHandler::paste(input, &Paste, window, cx);
                assert_eq!(input.as_str(), expected);
                assert_eq!(
                    input.caret_selection(),
                    CaretPosition::attached_to_previous_cluster(expected_caret).into()
                );
                assert_history_round_trip(input, original, selection, window, cx);
            });
        }
    }

    #[gpui::test]
    fn fallback_navigation_preserves_grapheme_boundaries_and_caret_affinity(
        cx: &mut TestAppContext,
    ) {
        for (name, text, offsets) in [
            ("multibyte", "café", &[0, 1, 2, 3, 5][..]),
            ("waving emoji", "a👋b", &[0, 1, 5, 6][..]),
            ("simple emoji", "a😀b", &[0, 1, 5, 6][..]),
            ("skin tone", "a👋🏽b", &[0, 1, 9, 10][..]),
            ("ZWJ family", "x👨‍👩‍👧y", &[0, 1, 19, 20][..]),
            ("flag", "x🇯🇵y", &[0, 1, 9, 10][..]),
            ("combining mark", "ae\u{0301}b", &[0, 1, 4, 5][..]),
            (
                "multiple combining marks",
                "xe\u{0308}\u{0301}y",
                &[0, 1, 6, 7][..],
            ),
            ("CJK", "a你好b", &[0, 1, 4, 7, 8][..]),
            ("mixed scripts", "Hi你😀", &[0, 1, 2, 5, 9][..]),
            ("variation selector", "a☺\u{FE0F}b", &[0, 1, 7, 8][..]),
            ("keycap", "x1\u{FE0F}\u{20E3}y", &[0, 1, 8, 9][..]),
        ] {
            assert_eq!(offsets.last(), Some(&text.len()), "{name}");
            let view = create_test_input(cx, text, 0);

            update_movement_input(view, cx, |input, window, cx| {
                for &expected in &offsets[1..] {
                    input.nav_right(&NavRight, window, cx);
                    assert_eq!(input.caret_selection(), expected.into(), "{name}");
                }

                for &expected in offsets[..offsets.len() - 1].iter().rev() {
                    input.nav_left(&NavLeft, window, cx);
                    assert_eq!(input.caret_selection(), expected.into(), "{name}");
                }
            });
        }
    }

    #[gpui::test]
    fn test_selection_with_multibyte_characters(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "日本語", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_right(&SelectRight, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(0),
                    caret: CaretPosition::attached_to_next_cluster(3),
                }
            );
            input.select_right(&SelectRight, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(0),
                    caret: CaretPosition::attached_to_next_cluster(6),
                }
            );
            input.select_right(&SelectRight, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(0),
                    caret: CaretPosition::attached_to_next_cluster(9),
                }
            );
        });
    }

    #[gpui::test]
    fn test_find_line_start_and_end(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "first\nsecond\nthird", 0);
        update_movement_input(view, cx, |input, _window, _cx| {
            use NavigationDirection::*;
            use TextBoundary::*;
            let storage = &input.storage;

            assert_eq!(storage.offset_from_caret(0, Back, HardLine), 0);
            assert_eq!(storage.offset_from_caret(3, Back, HardLine), 0);
            assert_eq!(storage.offset_from_caret(6, Back, HardLine), 6);
            assert_eq!(storage.offset_from_caret(13, Back, HardLine), 13);

            assert_eq!(storage.offset_from_caret(0, Forward, HardLine), 5);
            assert_eq!(storage.offset_from_caret(6, Forward, HardLine), 12);
            assert_eq!(storage.offset_from_caret(13, Forward, HardLine), 18);
        });
    }

    #[gpui::test]
    fn test_operations_on_empty_content(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_left(&NavLeft, window, cx);
            assert_eq!(input.caret_selection(), 0.into());

            input.nav_right(&NavRight, window, cx);
            assert_eq!(input.caret_selection(), 0.into());

            input.delete_left(&DeleteLeft, window, cx);
            assert_eq!(input.as_str(), "");

            input.delete_right(&DeleteRight, window, cx);
            assert_eq!(input.as_str(), "");

            input.select_all(&SelectAll, window, cx);
            assert_eq!(input.caret_selection(), 0.into());
        });
    }

    #[gpui::test]
    fn test_set_content_resets_selection(cx: &mut TestAppContext) {
        let view = create_test_input(
            cx,
            "hello world",
            CaretSelection {
                anchor: CaretPosition::attached_to_next_cluster(8),
                caret: CaretPosition::attached_to_next_cluster(3),
            },
        );
        update_movement_input(view, cx, |input, window, cx| {
            input.marked_range = Some(5..7);
            input.replace_text_in_range(Some(0..11), "new content", window, cx);
            assert_eq!(input.as_str(), "new content");
            assert_eq!(
                input.caret_selection(),
                CaretPosition::attached_to_previous_cluster(11).into()
            );
            assert_eq!(input.marked_range, None);
        });
    }

    #[gpui::test]
    fn test_cursor_clamped_to_content_length(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 100);
        update_movement_input(view, cx, |input, _window, cx| {
            input.move_to(1000, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection::from(CaretPosition::attached_to_previous_cluster(5))
            );

            input.set_selection(0);
            input.select_to(1000, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(0),
                    caret: CaretPosition::attached_to_previous_cluster(5),
                }
            );
        });
    }

    #[gpui::test]
    fn test_previous_boundary_at_start(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 0);
        update_movement_input(view, cx, |input, _window, _cx| {
            use NavigationDirection::*;
            use TextBoundary::*;
            assert_eq!(input.storage.offset_from_caret(0, Back, Cluster), 0);
        });
    }

    #[gpui::test]
    fn test_next_boundary_at_end(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 0);
        update_movement_input(view, cx, |input, _window, _cx| {
            use NavigationDirection::*;
            use TextBoundary::*;
            let storage = &input.storage;
            assert_eq!(storage.offset_from_caret(5, Forward, Cluster), 5);
            assert_eq!(storage.offset_from_caret(100, Forward, Cluster), 5);
        });
    }

    #[gpui::test]
    fn test_word_range_at_boundary(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello world", 0);
        update_movement_input(view, cx, |input, _window, _cx| {
            let range = input.storage.word_range_at(5);
            assert_eq!(range.start, 0);
            assert_eq!(range.end, 5);

            let range = input.storage.word_range_at(8);
            assert_eq!(range.start, 6);
            assert_eq!(range.end, 11);
        });
    }

    #[gpui::test]
    fn test_select_group_at_falls_back_without_a_current_layout(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "first line\nlast", 0);
        update_movement_input(view, cx, |input, _window, cx| {
            input.layout_data.state.last_seen_storage_version = input.version();
            input.emplace("first line\nlast word", cx);
            assert!(matches!(input.current_document(), Err(OldDocumentVersion)));

            let position = point(gpui::px(0.), gpui::px(0.));
            let line_height = gpui::px(16.);

            input.select_group_at(position, line_height, SelectionGroup::Word, cx);
            assert_eq!(input.selected_byte_range(), 16..20);

            input.select_group_at(position, line_height, SelectionGroup::Line, cx);
            assert_eq!(input.selected_byte_range(), 11..20);

            input.select_group_at(position, line_height, SelectionGroup::Document, cx);
            assert_eq!(input.selected_byte_range(), 0..20);
        });
    }

    #[gpui::test]
    fn test_select_emoji_with_shift(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "a😀b", 1); // cursor before emoji
        update_movement_input(view, cx, |input, window, cx| {
            input.select_right(&SelectRight, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(1),
                    caret: CaretPosition::attached_to_next_cluster(5),
                }
            ); // selected the entire emoji
        });
    }

    // Single-line input tests

    fn create_single_line_input(
        cx: &mut TestAppContext,
        content: &str,
        selected_range: impl Into<CaretSelection>,
    ) -> WindowHandle<TestView> {
        cx.add_window(|_window, cx| {
            let input = cx.new(|cx| {
                let mut input = default_state(content, cx);
                input.set_selection(selected_range);
                input
            });
            TestView { input }
        })
    }

    #[gpui::test]
    fn test_single_line_enter_does_nothing(cx: &mut TestAppContext) {
        let view = create_single_line_input(cx, "hello", 5);
        update_movement_input(view, cx, |input, window, cx| {
            input.insert_enter(&Enter, window, cx);
            assert_eq!(input.as_str(), "hello");
            assert_eq!(input.caret_selection(), 5.into());
        });
    }

    #[gpui::test]
    fn test_single_line_up_moves_to_start(cx: &mut TestAppContext) {
        let view = create_single_line_input(cx, "hello world", 5);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_up(&NavUp, window, cx);
            assert_eq!(input.caret_selection(), 0.into());
        });
    }

    #[gpui::test]
    fn test_single_line_down_moves_to_end(cx: &mut TestAppContext) {
        let view = create_single_line_input(cx, "hello world", 5);
        update_movement_input(view, cx, |input, window, cx| {
            input.nav_down(&NavDown, window, cx);
            assert_eq!(input.caret_selection(), 11.into()); // "hello world".len() == 11
        });
    }

    #[gpui::test]
    fn test_single_line_select_up_selects_to_start(cx: &mut TestAppContext) {
        let view = create_single_line_input(cx, "hello world", 5);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_up(&SelectUp, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(5),
                    caret: CaretPosition::attached_to_next_cluster(0),
                }
            );
        });
    }

    #[gpui::test]
    fn test_single_line_select_down_selects_to_end(cx: &mut TestAppContext) {
        let view = create_single_line_input(cx, "hello world", 5);
        update_movement_input(view, cx, |input, window, cx| {
            input.select_down(&SelectDown, window, cx);
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(5),
                    caret: CaretPosition::attached_to_previous_cluster(11),
                }
            ); // "hello world".len() == 11
        });
    }

    #[gpui::test]
    fn history_lifecycle_preserves_content_availability_and_accessibility_metrics(
        cx: &mut TestAppContext,
    ) {
        let view = create_test_input(cx, "hello", 0);
        update_movement_input(view, cx, |input, window, cx| {
            for kind in [HistoryKind::Undo, HistoryKind::Redo] {
                assert!(!is_history_kind_available(input, kind));
            }

            input.undo(&Undo, window, cx);
            assert_eq!(input.as_str(), "hello");
            input.redo(&Redo, window, cx);
            assert_eq!(input.as_str(), "hello");
        });

        for (name, original, inserted, edited, lengths, offsets) in [
            (
                "ASCII content",
                "hello",
                " world",
                "hello world",
                &[1; 11][..],
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11][..],
            ),
            (
                "availability",
                "hello",
                "!",
                "hello!",
                &[1; 6][..],
                &[0, 1, 2, 3, 4, 5, 6][..],
            ),
            (
                "Unicode metrics",
                "A😀B",
                "日本",
                "A😀B日本",
                &[1, 4, 1, 3, 3][..],
                &[0, 1, 5, 6, 9, 12][..],
            ),
        ] {
            let view = create_test_input(cx, original, original.len());
            update_movement_input(view, cx, |input, window, cx| {
                without_history_grouping(input);
                assert!(
                    !is_history_kind_available(input, HistoryKind::Undo),
                    "{name}"
                );
                assert!(
                    !is_history_kind_available(input, HistoryKind::Redo),
                    "{name}"
                );

                input.replace_text_in_range(None, inserted, window, cx);
                assert_eq!(input.as_str(), edited, "{name}");
                assert_eq!(
                    input.accessibility_text_metrics().character_lengths,
                    lengths,
                    "{name}"
                );
                assert!(
                    is_history_kind_available(input, HistoryKind::Undo),
                    "{name}"
                );
                assert!(
                    !is_history_kind_available(input, HistoryKind::Redo),
                    "{name}"
                );

                input.undo(&Undo, window, cx);
                assert_eq!(input.as_str(), original, "{name}");
                assert!(
                    !is_history_kind_available(input, HistoryKind::Undo),
                    "{name}"
                );
                assert!(
                    is_history_kind_available(input, HistoryKind::Redo),
                    "{name}"
                );

                if name == "Unicode metrics" {
                    assert_eq!(
                        input.accessibility_text_metrics().byte_offsets,
                        [0, 1, 5, 6]
                    );
                }

                input.redo(&Redo, window, cx);
                assert_eq!(input.as_str(), edited, "{name}");
                assert_eq!(
                    input.accessibility_text_metrics().byte_offsets,
                    offsets,
                    "{name}"
                );
                assert!(
                    is_history_kind_available(input, HistoryKind::Undo),
                    "{name}"
                );
                assert!(
                    !is_history_kind_available(input, HistoryKind::Redo),
                    "{name}"
                );
            });
        }
    }

    #[gpui::test]
    fn test_undo_restores_selection(cx: &mut TestAppContext) {
        let view = create_test_input(
            cx,
            "hello world",
            CaretSelection {
                anchor: CaretPosition::attached_to_next_cluster(5),
                caret: CaretPosition::attached_to_next_cluster(0),
            },
        );
        update_movement_input(view, cx, |input, window, cx| {
            without_history_grouping(input);
            let mut selection = input.caret_selection();
            selection.caret.affinity = CaretAffinity::Upstream;
            selection.anchor.affinity = CaretAffinity::Downstream;
            input.set_selection(selection);

            // Delete selection
            input.replace_text_in_range(None, "", window, cx);
            assert_eq!(input.as_str(), " world");
            assert_eq!(input.caret_selection(), 0.into());

            // Undo should restore content and selection
            input.undo(&Undo, window, cx);
            assert_eq!(input.as_str(), "hello world");
            assert_eq!(
                input.caret_selection(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_next_cluster(5),
                    caret: CaretPosition::attached_to_previous_cluster(0),
                }
            );
        });
    }

    #[gpui::test]
    fn test_multiple_undo_redo(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "", 0);
        update_movement_input(view, cx, |input, window, cx| {
            without_history_grouping(input);

            input.replace_text_in_range(None, "a", window, cx);
            input.replace_text_in_range(None, "b", window, cx);
            input.replace_text_in_range(None, "c", window, cx);
            assert_eq!(input.as_str(), "abc");

            input.undo(&Undo, window, cx);
            assert_eq!(input.as_str(), "ab");

            input.undo(&Undo, window, cx);
            assert_eq!(input.as_str(), "a");

            input.undo(&Undo, window, cx);
            assert_eq!(input.as_str(), "");

            input.redo(&Redo, window, cx);
            assert_eq!(input.as_str(), "a");

            input.redo(&Redo, window, cx);
            assert_eq!(input.as_str(), "ab");

            input.redo(&Redo, window, cx);
            assert_eq!(input.as_str(), "abc");
        });
    }

    #[gpui::test]
    fn test_new_edit_clears_redo_stack(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "hello", 5);
        update_movement_input(view, cx, |input, window, cx| {
            without_history_grouping(input);

            input.replace_text_in_range(None, " world", window, cx);
            assert_eq!(input.as_str(), "hello world");

            input.undo(&Undo, window, cx);
            assert_eq!(input.as_str(), "hello");
            assert!(is_history_kind_available(input, HistoryKind::Redo));

            // New edit should clear redo stack
            input.replace_text_in_range(None, "!", window, cx);
            assert_eq!(input.as_str(), "hello!");
            assert!(!is_history_kind_available(input, HistoryKind::Redo));
        });
    }

    #[gpui::test]
    fn cut_without_selection_removes_complete_hard_line(cx: &mut TestAppContext) {
        for (name, text, caret, remaining, expected_clipboard, current_layout) in [
            (
                "middle",
                "line1\nline2\nline3",
                8,
                "line1\nline3",
                "line2\n",
                false,
            ),
            (
                "first CRLF",
                "line1\r\nline2",
                2,
                "line2",
                "line1\r\n",
                false,
            ),
            ("last", "line1\nline2", 8, "line1", "\nline2", false),
            ("empty", "line1\n\nline3", 6, "line1\nline3", "\n", false),
            ("only", "hello", 2, "", "hello", false),
            ("empty document", "", 0, "", "", false),
            ("empty final line", "line1\n", 6, "line1", "\n", false),
            (
                "multibyte separator fallback",
                "line1\u{2028}line2",
                2,
                "",
                "line1\u{2028}line2",
                false,
            ),
            ("only current layout", "hello", 2, "", "hello", true),
            ("empty current layout", "", 0, "", "", true),
        ] {
            let view = create_test_input(cx, text, caret);
            view.update(cx, |view, window, cx| {
                let document = current_layout.then(|| {
                    window
                        .text_system()
                        .shape_text(
                            text,
                            gpui::px(16.),
                            &[window.text_style().to_run(text.len())],
                            None,
                            None,
                        )
                        .unwrap()
                });

                view.input.update(cx, |input, cx| {
                    if let Some(document) = document {
                        input.layout_data.document = Some(document.into());
                        input.layout_data.state.last_seen_storage_version = input.version();
                    }

                    assert_eq!(input.current_document().is_ok(), current_layout, "{name}");
                    without_history_grouping(input);

                    input.cut(&Cut, window, cx);
                    assert_eq!(input.as_str(), remaining, "{name}");

                    input.undo(&Undo, window, cx);
                    assert_eq!(input.as_str(), text, "{name}");
                    assert_eq!(input.caret_selection(), caret.into(), "{name}");
                });
            })
            .unwrap();

            let clipboard = cx.read_from_clipboard().and_then(|item| item.text());
            assert_eq!(
                clipboard.as_deref().unwrap_or_default(),
                expected_clipboard,
                "{name}"
            );
        }
    }

    #[gpui::test]
    fn ime_composition_uses_relative_utf16_ranges_around_surrogate_pairs(cx: &mut TestAppContext) {
        let view = create_test_input(cx, "A😀B", 0);
        update_movement_input(view, cx, |input, window, cx| {
            input.replace_and_mark_text_in_range(Some(1..3), "にほん", Some(1..2), window, cx);
            assert_eq!(input.as_str(), "AにほんB");
            assert_eq!(
                input.accessibility_text_metrics().character_lengths,
                [1, 3, 3, 3, 1]
            );
            assert_eq!(
                input.marked_text_range(window, cx),
                Some(1..4),
                "marked ranges exposed to the platform use document UTF-16 offsets"
            );
            assert_eq!(
                input.selected_text_range(false, window, cx).unwrap().range,
                2..3,
                "composition selections are relative to the inserted text"
            );

            input.replace_and_mark_text_in_range(None, "日本", None, window, cx);
            assert_eq!(input.as_str(), "A日本B");
            assert_eq!(
                input.accessibility_text_metrics().byte_offsets,
                [0, 1, 4, 7, 8]
            );
            assert_eq!(input.marked_text_range(window, cx), Some(1..3));
            assert_eq!(
                input.selected_text_range(false, window, cx).unwrap().range,
                3..3
            );

            input.unmark_text(window, cx);
            assert_eq!(input.marked_text_range(window, cx), None);
            assert_eq!(
                input.selected_text_range(false, window, cx).unwrap().range,
                3..3
            );
        });
    }

    #[gpui::test]
    fn document_movements_reach_endpoints_with_current_or_stale_layout(cx: &mut TestAppContext) {
        let actions: [(&str, bool, bool, TestMovementAction); 10] = [
            ("nav start", false, false, |input, window, cx| {
                input.nav_start(&NavDocumentStart, window, cx);
            }),
            ("nav end", true, false, |input, window, cx| {
                input.nav_end(&NavDocumentEnd, window, cx);
            }),
            ("select start", false, true, |input, window, cx| {
                input.select_start(&SelectDocumentStart, window, cx);
            }),
            ("select end", true, true, |input, window, cx| {
                input.select_end(&SelectDocumentEnd, window, cx);
            }),
            ("single-line select up", false, true, |input, window, cx| {
                input.select_up(&SelectUp, window, cx);
            }),
            (
                "single-line select down",
                true,
                true,
                |input, window, cx| {
                    input.select_down(&SelectDown, window, cx);
                },
            ),
            ("logical nav start", false, false, |input, _window, cx| {
                input.nav_linear(NavigationDirection::Back, TextBoundary::Document, cx);
            }),
            ("logical nav end", true, false, |input, _window, cx| {
                input.nav_linear(NavigationDirection::Forward, TextBoundary::Document, cx);
            }),
            ("logical select start", false, true, |input, _window, cx| {
                input.select_linear(NavigationDirection::Back, TextBoundary::Document, cx);
            }),
            ("logical select end", true, true, |input, _window, cx| {
                input.select_linear(NavigationDirection::Forward, TextBoundary::Document, cx);
            }),
        ];

        for (text, current_layout) in [("a😀bc", true), ("a😀bc", false), ("", true), ("", false)]
        {
            let document = movement_document(text, None);
            let view = create_test_input(cx, text, 0);
            let start = 1.min(text.len());
            let end = "a😀".len().min(text.len());
            let initial_selections = [
                CaretPosition::attached_to_previous_cluster(start).into(),
                CaretSelection {
                    anchor: CaretPosition::attached_to_previous_cluster(start),
                    caret: CaretPosition::attached_to_next_cluster(end),
                },
                CaretSelection {
                    anchor: CaretPosition::attached_to_previous_cluster(end),
                    caret: CaretPosition::attached_to_next_cluster(start),
                },
            ];

            for initial in initial_selections {
                for (name, to_end, extend, action) in actions {
                    update_movement_input(view, cx, |input, window, cx| {
                        input.layout_data.document = Some(document.clone());
                        input.layout_data.line_height = px(24.);
                        input.layout_data.state.last_seen_storage_version = if current_layout {
                            input.version()
                        } else {
                            input.version().wrapping_sub(1)
                        };
                        input.selection_movement = CaretSelectionMovement {
                            result: initial,
                            vertical_navigation_x: Some(px(37.)),
                        };
                        assert_eq!(input.current_document().is_ok(), current_layout);

                        action(input, window, cx);

                        let index = if to_end { text.len() } else { 0 };
                        let caret = input.caret_for_index(index);
                        let expected = if extend {
                            initial.with_caret(caret)
                        } else {
                            caret.into()
                        };
                        assert_eq!(
                            input.caret_selection(),
                            expected,
                            "{name}, {text:?}, current={current_layout}, initial={initial:?}"
                        );
                        assert_eq!(
                            input.selection_movement.vertical_navigation_x, None,
                            "{name}"
                        );
                    });
                }
            }
        }
    }

    #[gpui::test]
    fn current_layout_preserves_visual_destinations_and_selection_affinity(
        cx: &mut TestAppContext,
    ) {
        let actions: [(
            Direction,
            TextBoundary,
            TestMovementAction,
            TestMovementAction,
        ); 4] = [
            (
                Direction::Left,
                TextBoundary::Cluster,
                |input, window, cx| input.nav_left(&NavLeft, window, cx),
                |input, window, cx| input.select_left(&SelectLeft, window, cx),
            ),
            (
                Direction::Right,
                TextBoundary::Cluster,
                |input, window, cx| input.nav_right(&NavRight, window, cx),
                |input, window, cx| input.select_right(&SelectRight, window, cx),
            ),
            (
                Direction::Left,
                TextBoundary::Word,
                |input, window, cx| input.nav_left_word(&NavWordLeft, window, cx),
                |input, window, cx| input.select_left_word(&SelectWordLeft, window, cx),
            ),
            (
                Direction::Right,
                TextBoundary::Word,
                |input, window, cx| input.nav_right_word(&NavWordRight, window, cx),
                |input, window, cx| input.select_right_word(&SelectWordRight, window, cx),
            ),
        ];

        for (text, index) in [
            ("אבג דהו", 2),
            ("abc אבג xyz", 4),
            ("a👩🏽‍💻b", 1),
            ("ab\ncd", 2),
        ] {
            let document = movement_document(text, None);
            let view = create_test_input(cx, text, 0);
            let initial = CaretSelection {
                anchor: CaretPosition::attached_to_previous_cluster(text.len()),
                caret: CaretPosition::attached_to_next_cluster(index),
            };

            for (direction, boundary, nav, select) in actions {
                for (extend, action) in [(false, nav), (true, select)] {
                    update_movement_input(view, cx, |input, window, cx| {
                        input.layout_data.document = Some(document.clone());
                        input.layout_data.line_height = px(24.);
                        input.layout_data.state.last_seen_storage_version = input.version();
                        input.set_selection(initial);
                        assert!(input.current_document().is_ok());
                        let expected = document.selection_movement(
                            initial,
                            direction.with_boundary(boundary),
                            extend,
                            None,
                            px(24.),
                        );

                        action(input, window, cx);

                        assert_eq!(
                            input.selection_movement, expected,
                            "{text:?}, {direction:?}, {boundary:?}, extend={extend}"
                        );
                        assert!(text.is_char_boundary(input.caret_selection().caret.index));

                        if extend {
                            assert_eq!(input.caret_selection().anchor, initial.anchor);
                        } else {
                            assert!(input.caret_selection().is_empty());
                        }
                    });
                }
            }

            update_movement_input(view, cx, |input, window, cx| {
                input.set_selection(CaretPosition::attached_to_next_cluster(index));
                let expected = document
                    .caret_movement(
                        input.caret_selection().caret,
                        Direction::Right.with_boundary(TextBoundary::Cluster),
                        None,
                    )
                    .result;

                input.nav_right(&NavRight, window, cx);

                assert_eq!(input.caret_selection(), expected.into());
                assert_ne!(input.caret_selection().caret.index, index);

                if text == "אבג דהו" {
                    assert_eq!(
                        input.caret_selection().caret.index,
                        0,
                        "visual right traverses RTL toward earlier storage bytes"
                    );
                }

                if text == "a👩🏽‍💻b" {
                    assert_eq!(input.caret_selection().caret.index, "a👩🏽‍💻".len());
                }
            });
        }
    }

    #[gpui::test]
    fn vertical_movement_retains_horizontal_target_until_nonvertical_action(
        cx: &mut TestAppContext,
    ) {
        for wrap_width in [None, Some(px(65.))] {
            let text = "abcd efgh ijkl\nm\nabcd efgh ijkl";
            let document = movement_document(text, wrap_width);
            assert!(document.visual_lines().len() >= 3);
            let view = create_test_input(cx, text, 3);
            update_movement_input(view, cx, |input, window, cx| {
                input.layout_data.document = Some(document.clone());
                input.layout_data.line_height = px(24.);
                input.layout_data.supports_multiline = true;
                input.layout_data.state.last_seen_storage_version = input.version();
                assert!(input.current_document().is_ok());
                let anchor = input.caret_selection().caret;

                input.select_down(&SelectDown, window, cx);

                let retained_x = input.selection_movement.vertical_navigation_x;
                assert!(retained_x.is_some());
                assert_eq!(input.caret_selection().anchor, anchor);
                let first_caret = input.caret_selection().caret;

                input.select_down(&SelectDown, window, cx);

                assert_eq!(input.selection_movement.vertical_navigation_x, retained_x);
                assert_ne!(input.caret_selection().caret, first_caret);
                assert_eq!(input.caret_selection().anchor, anchor);

                input.nav_up(&NavUp, window, cx);

                assert_eq!(input.selection_movement.vertical_navigation_x, retained_x);
                assert!(input.caret_selection().is_empty());

                input.nav_right(&NavRight, window, cx);

                assert_eq!(input.selection_movement.vertical_navigation_x, None);

                input.layout_data.state.last_seen_storage_version = input.version().wrapping_sub(1);
                input.selection_movement.vertical_navigation_x = retained_x;
                input.nav_down(&NavDown, window, cx);

                assert_eq!(input.selection_movement.vertical_navigation_x, None);
            });
        }
    }

    #[gpui::test]
    fn logical_and_absolute_movements_preserve_storage_order_and_apply_once(
        cx: &mut TestAppContext,
    ) {
        let text = "אבג";
        let document = movement_document(text, None);
        let view = create_test_input(cx, text, 2);
        let entity = view
            .update(cx, |view, _window, _cx| view.input.clone())
            .unwrap();
        let blink_events = Rc::new(Cell::new(0));
        let notifications = Rc::new(Cell::new(0));
        let subscriptions = cx.update(|cx| {
            let events = blink_events.clone();
            let subscription = cx.subscribe(&entity, move |_entity, _event: &CaretNotify, _cx| {
                events.set(events.get() + 1);
            });
            let changes = notifications.clone();
            let observer = cx.observe(&entity, move |_entity, _cx| {
                changes.set(changes.get() + 1);
            });

            (subscription, observer)
        });

        for current_layout in [true, false] {
            for (direction, expected_index) in [
                (NavigationDirection::Back, 0),
                (NavigationDirection::Forward, 4),
            ] {
                let previous_events = blink_events.get();
                let previous_notifications = notifications.get();
                update_movement_input(view, cx, |input, _window, cx| {
                    input.layout_data.document = Some(document.clone());
                    input.layout_data.state.last_seen_storage_version = if current_layout {
                        input.version()
                    } else {
                        input.version().wrapping_sub(1)
                    };
                    input.set_selection(2);
                    input.nav_linear(direction, TextBoundary::Cluster, cx);

                    assert_eq!(input.caret_selection(), expected_index.into());
                });
                assert_eq!(blink_events.get(), previous_events + 1);
                assert_eq!(notifications.get(), previous_notifications + 1);
            }

            update_movement_input(view, cx, |input, _window, cx| {
                let initial = CaretSelection {
                    anchor: CaretPosition::attached_to_previous_cluster(0),
                    caret: CaretPosition::attached_to_next_cluster(2),
                };
                input.set_selection(initial);
                input.select_linear(NavigationDirection::Forward, TextBoundary::Cluster, cx);

                assert_eq!(
                    input.caret_selection(),
                    initial.with_caret(CaretPosition::attached_to_next_cluster(4))
                );

                for (caret, anchor) in [(0, 4), (4, 0)] {
                    let initial = CaretSelection {
                        caret: CaretPosition::attached_to_next_cluster(caret),
                        anchor: CaretPosition::attached_to_previous_cluster(anchor),
                    };
                    input.set_selection(initial);
                    input.nav_linear(NavigationDirection::Back, TextBoundary::Word, cx);

                    assert_eq!(input.caret_selection(), 0.into());

                    input.set_selection(initial);
                    input.nav_linear(NavigationDirection::Forward, TextBoundary::Word, cx);

                    assert_eq!(input.caret_selection(), 4.into());
                }

                input.selection_movement.vertical_navigation_x = Some(px(11.));
                let caret = CaretPosition::attached_to_previous_cluster(100);
                input.move_to_caret(caret, cx);

                assert_eq!(
                    input.caret_selection(),
                    CaretPosition::attached_to_previous_cluster(text.len()).into()
                );
                assert_eq!(input.selection_movement.vertical_navigation_x, None);

                input.selection_movement.vertical_navigation_x = Some(px(11.));
                input.select_to_caret(CaretPosition::attached_to_next_cluster(0), cx);

                assert_eq!(
                    input.caret_selection().anchor,
                    CaretPosition::attached_to_previous_cluster(text.len())
                );
                assert_eq!(
                    input.caret_selection().caret,
                    CaretPosition::attached_to_next_cluster(0)
                );
                assert_eq!(input.selection_movement.vertical_navigation_x, None);
            });
        }

        let actions: [TestMovementAction; 4] = [
            |input, window, cx| input.nav_start(&NavDocumentStart, window, cx),
            |input, window, cx| input.nav_left(&NavLeft, window, cx),
            |input, _window, cx| {
                input.move_to_caret(CaretPosition::attached_to_previous_cluster(4), cx)
            },
            |input, _window, cx| {
                input.select_to_caret(CaretPosition::attached_to_previous_cluster(4), cx)
            },
        ];

        for current_layout in [true, false] {
            for action in actions {
                let previous_events = blink_events.get();
                let previous_notifications = notifications.get();
                update_movement_input(view, cx, |input, window, cx| {
                    input.layout_data.document = Some(document.clone());
                    input.layout_data.state.last_seen_storage_version = if current_layout {
                        input.version()
                    } else {
                        input.version().wrapping_sub(1)
                    };
                    input.set_selection(2);
                    action(input, window, cx);
                });
                assert_eq!(blink_events.get(), previous_events + 1);
                assert_eq!(notifications.get(), previous_notifications + 1);
            }
        }

        drop(subscriptions);
    }
}
