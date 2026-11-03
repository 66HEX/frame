use crate::{
    self as gpui, AbsoluteLength, AlignContent, AlignItems, AlignSelf, BorderStyle, CursorStyle,
    DefiniteLength, Display, Edges, Fill, Filter, FlexDirection, FlexWrap, Font, FontFeatures,
    FontStyle, FontWeight, FontWidth, GridPlacement, GridTemplate, GridTemplateMinSize,
    JustifyContent, LayoutDirection, Length, Pixels, SharedString, StrikethroughStyle,
    StyleRefinement, TextAlign, TextOverflow, TextStyleRefinement, TextTransform, UnderlineStyle,
    UnicodeBidi, VerticalAlign, WhiteSpace, px, relative, rems,
};
pub use gpui_macros::{
    border_style_methods, box_shadow_style_methods, cursor_style_methods, margin_style_methods,
    overflow_style_methods, padding_style_methods, position_style_methods,
    visibility_style_methods,
};
use palette::{Hsla, IntoColor};
use refineable::Refineable;
const ELLIPSIS: SharedString = SharedString::new_static("…");

/// A trait for elements that can be styled.
/// Use this to opt-in to a utility CSS-like styling API.
// gate on rust-analyzer so rust-analyzer never needs to expand this macro, it takes up to 10 seconds to expand due to inefficiencies in rust-analyzers proc-macro srv
#[cfg_attr(
    all(any(feature = "inspector", debug_assertions), not(rust_analyzer)),
    gpui_macros::derive_inspector_reflection
)]
pub trait Styled: Sized {
    /// Returns a reference to the style memory of this element.
    fn style(&mut self) -> &mut StyleRefinement;

    /// Applies the given refinement to this element's style.
    fn refine_style(mut self, refinement: &StyleRefinement) -> Self {
        self.style().refine(refinement);

        self
    }

    gpui_macros::style_helpers!();
    gpui_macros::visibility_style_methods!();
    gpui_macros::margin_style_methods!();
    gpui_macros::padding_style_methods!();
    gpui_macros::position_style_methods!();
    gpui_macros::overflow_style_methods!();
    gpui_macros::cursor_style_methods!();
    gpui_macros::border_style_methods!();
    gpui_macros::box_shadow_style_methods!();

    /// Sets the inline direction for this element and its logical descendants.
    fn direction(mut self, direction: LayoutDirection) -> Self {
        self.style().direction = Some(direction);

        self
    }

    /// Establishes right-to-left directionality for this element.
    fn rtl(self) -> Self {
        self.direction(LayoutDirection::RightToLeft)
    }

    /// Establishes left-to-right directionality for this element.
    fn ltr(self) -> Self {
        self.direction(LayoutDirection::LeftToRight)
    }

    /// Sets this element's Unicode bidirectional formatting behavior.
    fn unicode_bidi(mut self, unicode_bidi: UnicodeBidi) -> Self {
        let style = self.style();
        style.unicode_bidi = Some(unicode_bidi);
        style.unicode_bidi_explicit = Some(true);

        self
    }

    /// Blur this element's own content and children, like CSS `filter: blur(<radius>)`.
    ///
    /// This isolates the element's subtree, blurs it as a group, and composites the
    /// result back. To blur the content *behind* the element instead (frosted glass),
    /// use [`Styled::backdrop_blur`].
    ///
    /// *Appends* to the element's filter chain, so it composes with other convenience
    /// setters (`.blur(8.).<other_filter>()`). To replace the whole chain, use
    /// [`Styled::filter`].
    fn blur(mut self, radius: impl Into<Pixels>) -> Self {
        self.style()
            .filter
            .get_or_insert_with(Vec::new)
            .push(Filter::Blur(radius.into()));
        self
    }

    /// Set (replacing any existing) the full list of filters applied to this element's own
    /// content, like CSS `filter`. To add a single filter to the chain instead, use the
    /// convenience setters such as [`Styled::blur`].
    fn filter(mut self, filters: impl Into<Vec<Filter>>) -> Self {
        self.style().filter = Some(filters.into());
        self
    }

    /// Blur the content rendered behind this element — a frosted-glass effect — like CSS
    /// `backdrop-filter: blur(<radius>)`. Typically paired with a translucent [`Styled::bg`]
    /// so the background tints the blurred backdrop.
    ///
    /// *Appends* to the element's backdrop-filter chain. To replace the whole chain, use
    /// [`Styled::backdrop_filter`].
    fn backdrop_blur(mut self, radius: impl Into<Pixels>) -> Self {
        self.style()
            .backdrop_filter
            .get_or_insert_with(Vec::new)
            .push(Filter::Blur(radius.into()));
        self
    }

    /// Set (replacing any existing) the full list of filters applied to the content behind this
    /// element, like CSS `backdrop-filter`. To add a single filter to the chain instead, use the
    /// convenience setters such as [`Styled::backdrop_blur`].
    fn backdrop_filter(mut self, filters: impl Into<Vec<Filter>>) -> Self {
        self.style().backdrop_filter = Some(filters.into());
        self
    }

    /// Sets the display type of the element.
    fn display(mut self, display: Display) -> Self {
        self.style().display = Some(display);
        self
    }

    /// Sets the display type of the element to `block`.
    /// [Docs](https://tailwindcss.com/docs/display)
    fn block(self) -> Self {
        self.display(Display::Block)
    }

    /// Sets the display type of the element to `flex`.
    /// [Docs](https://tailwindcss.com/docs/display)
    fn flex(self) -> Self {
        self.display(Display::Flex)
    }

    /// Sets the display type of the element to `grid`.
    /// [Docs](https://tailwindcss.com/docs/display)
    fn grid(self) -> Self {
        self.display(Display::Grid)
    }

    /// Contributes this element's contents to a block parent's paragraph.
    /// Nested inline elements wrap together without adding whitespace or breaks.
    fn inline(self) -> Self {
        self.display(Display::Inline)
    }

    /// Places this element in a paragraph as one atomic box with flex layout inside.
    fn inline_flex(self) -> Self {
        self.display(Display::InlineFlex)
    }

    /// Sets the vertical alignment of this element when it is placed in an inline layout.
    fn vertical_align(mut self, align: VerticalAlign) -> Self {
        self.style().vertical_align = Some(align);
        self
    }

    /// Aligns this inline element box with the surrounding text baseline.
    fn align_baseline(self) -> Self {
        self.vertical_align(VerticalAlign::Baseline)
    }

    /// Aligns this inline box's midpoint with the parent baseline plus half its font's x-height,
    /// following CSS `vertical-align: middle`.
    /// For flex or grid centering, use `self_center` on the item or `items_center` on its container.
    fn align_middle(self) -> Self {
        self.vertical_align(VerticalAlign::Middle)
    }

    /// Aligns this inline element box with the top of its line.
    fn align_top(self) -> Self {
        self.vertical_align(VerticalAlign::Top)
    }

    /// Aligns this inline element box with the bottom of its line.
    fn align_bottom(self) -> Self {
        self.vertical_align(VerticalAlign::Bottom)
    }

    /// Sets the display type of the element to `none`.
    /// [Docs](https://tailwindcss.com/docs/display)
    fn hidden(self) -> Self {
        self.display(Display::None)
    }

    /// Set the space to be reserved for rendering the scrollbar.
    ///
    /// This will only affect the layout of the element when overflow for this element is set to
    /// `Overflow::Scroll`.
    fn scrollbar_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        self.style().scrollbar_width = Some(width.into());
        self
    }

    /// Sets per-edge fade distances for content clipped by overflow.
    ///
    /// Content is faded to transparent over the given distance as it approaches
    /// the corresponding clipped edge of this element, similar to a CSS
    /// `mask-image: linear-gradient(...)`.
    fn overflow_fade(mut self, fade: impl Into<Edges<AbsoluteLength>>) -> Self {
        let fade = fade.into();
        let overflow_fade = &mut self.style().overflow_fade;
        overflow_fade.top = Some(fade.top);
        overflow_fade.right = Some(fade.right);
        overflow_fade.bottom = Some(fade.bottom);
        overflow_fade.left = Some(fade.left);
        self
    }

    /// Sets equal fade distances on the left and right overflow edges,
    /// clearing any fade on the top and bottom edges.
    fn overflow_fade_x(self, fade: impl Into<AbsoluteLength>) -> Self {
        self.overflow_fade(Edges::horizontal(fade.into()))
    }

    /// Sets equal fade distances on the top and bottom overflow edges,
    /// clearing any fade on the left and right edges.
    fn overflow_fade_y(self, fade: impl Into<AbsoluteLength>) -> Self {
        self.overflow_fade(Edges::vertical(fade.into()))
    }

    /// Sets the whitespace of the element.
    /// [Docs](https://tailwindcss.com/docs/whitespace)
    fn whitespace(mut self, white_space: WhiteSpace) -> Self {
        self.text_style().white_space = Some(white_space);
        self
    }

    /// Sets the whitespace of the element to `normal`.
    /// [Docs](https://tailwindcss.com/docs/whitespace#normal)
    fn whitespace_normal(mut self) -> Self {
        self.text_style().white_space = Some(WhiteSpace::Normal);
        self
    }

    /// Sets the whitespace of the element to `nowrap`.
    /// [Docs](https://tailwindcss.com/docs/whitespace#nowrap)
    fn whitespace_nowrap(mut self) -> Self {
        self.text_style().white_space = Some(WhiteSpace::Nowrap);
        self
    }

    /// Sets the truncate overflowing text with an ellipsis (…) at the end if needed.
    /// [Docs](https://tailwindcss.com/docs/text-overflow#ellipsis)
    fn text_ellipsis(mut self) -> Self {
        self.text_style().text_overflow = Some(TextOverflow::Truncate(ELLIPSIS));
        self
    }

    /// Sets the truncate overflowing text with an ellipsis (…) at the start if needed.
    /// Typically more adequate for file paths where the end is more important than the beginning.
    /// Note: This doesn't exist in Tailwind CSS.
    fn text_ellipsis_start(mut self) -> Self {
        self.text_style().text_overflow = Some(TextOverflow::TruncateStart(ELLIPSIS));
        self
    }

    /// Sets the truncate overflowing text with an ellipsis (…) in the middle if needed.
    /// Preserves the beginning and end of the text. Useful for filenames.
    /// Note: This doesn't exist in Tailwind CSS.
    fn text_ellipsis_middle(mut self) -> Self {
        self.text_style().text_overflow = Some(TextOverflow::TruncateMiddle(ELLIPSIS));
        self
    }

    /// Sets the text overflow behavior of the element.
    fn text_overflow(mut self, overflow: TextOverflow) -> Self {
        self.text_style().text_overflow = Some(overflow);
        self
    }

    /// Set the text alignment of the element.
    fn text_align(mut self, align: TextAlign) -> Self {
        self.text_style().text_align = Some(align);
        self
    }

    /// Aligns text to the start edge for each line's direction.
    fn text_start(self) -> Self {
        self.text_align(TextAlign::Start)
    }

    /// Aligns text to the end edge for each line's direction.
    fn text_end(self) -> Self {
        self.text_align(TextAlign::End)
    }

    /// Sets the text alignment to left
    fn text_left(mut self) -> Self {
        self.text_align(TextAlign::Left)
    }

    /// Sets the text alignment to center
    fn text_center(mut self) -> Self {
        self.text_align(TextAlign::Center)
    }

    /// Sets the text alignment to right
    fn text_right(mut self) -> Self {
        self.text_align(TextAlign::Right)
    }

    /// Sets the letter spacing for text in this element and its children.
    fn letter_spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.text_style().letter_spacing = Some(spacing.into());
        self
    }

    /// Sets the case transformation for text in this element and its children.
    fn text_transform(mut self, transform: TextTransform) -> Self {
        self.text_style().text_transform = Some(transform);
        self
    }

    /// Sets the truncate to prevent text from wrapping and truncate overflowing text with an ellipsis (…) if needed.
    /// [Docs](https://tailwindcss.com/docs/text-overflow#truncate)
    fn truncate(mut self) -> Self {
        self.overflow_hidden().whitespace_nowrap().text_ellipsis()
    }

    /// Sets number of lines to show before truncating the text.
    /// [Docs](https://tailwindcss.com/docs/line-clamp)
    fn line_clamp(mut self, lines: usize) -> Self {
        let mut text_style = self.text_style();
        text_style.line_clamp = Some(lines);
        self.overflow_hidden()
    }

    /// Sets the flex direction of the element.
    /// [Docs](https://tailwindcss.com/docs/flex-direction)
    fn flex_direction(mut self, direction: FlexDirection) -> Self {
        self.style().flex_direction = Some(direction);
        self
    }

    /// Sets the flex direction of the element to `column`.
    /// [Docs](https://tailwindcss.com/docs/flex-direction#column)
    fn flex_col(mut self) -> Self {
        self.style().flex_direction = Some(FlexDirection::Column);
        self
    }

    /// Sets the flex direction of the element to `column-reverse`.
    /// [Docs](https://tailwindcss.com/docs/flex-direction#column-reverse)
    fn flex_col_reverse(mut self) -> Self {
        self.style().flex_direction = Some(FlexDirection::ColumnReverse);
        self
    }

    /// Sets the flex direction of the element to `row`.
    /// [Docs](https://tailwindcss.com/docs/flex-direction#row)
    fn flex_row(mut self) -> Self {
        self.style().flex_direction = Some(FlexDirection::Row);
        self
    }

    /// Sets the flex direction of the element to `row-reverse`.
    /// [Docs](https://tailwindcss.com/docs/flex-direction#row-reverse)
    fn flex_row_reverse(mut self) -> Self {
        self.style().flex_direction = Some(FlexDirection::RowReverse);
        self
    }

    /// Sets the element to allow a flex item to grow and shrink as needed, ignoring its initial size.
    /// [Docs](https://tailwindcss.com/docs/flex#flex-1)
    fn flex_1(mut self) -> Self {
        self.style().flex_grow = Some(1.);
        self.style().flex_shrink = Some(1.);
        self.style().flex_basis = Some(relative(0.).into());
        self
    }

    /// Sets the element to allow a flex item to grow and shrink, taking into account its initial size.
    /// [Docs](https://tailwindcss.com/docs/flex#auto)
    fn flex_auto(mut self) -> Self {
        self.style().flex_grow = Some(1.);
        self.style().flex_shrink = Some(1.);
        self.style().flex_basis = Some(Length::Auto);
        self
    }

    /// Sets the element to allow a flex item to shrink but not grow, taking into account its initial size.
    /// [Docs](https://tailwindcss.com/docs/flex#initial)
    fn flex_initial(mut self) -> Self {
        self.style().flex_grow = Some(0.);
        self.style().flex_shrink = Some(1.);
        self.style().flex_basis = Some(Length::Auto);
        self
    }

    /// Sets the element to prevent a flex item from growing or shrinking.
    /// [Docs](https://tailwindcss.com/docs/flex#none)
    fn flex_none(mut self) -> Self {
        self.style().flex_grow = Some(0.);
        self.style().flex_shrink = Some(0.);
        self.style().flex_basis = Some(Length::Auto);
        self
    }

    /// Sets the initial size of flex items for this element.
    /// [Docs](https://tailwindcss.com/docs/flex-basis)
    fn flex_basis(mut self, basis: impl Into<Length>) -> Self {
        self.style().flex_basis = Some(basis.into());
        self
    }

    /// Sets the flex item's grow factor.
    /// [Docs](https://tailwindcss.com/docs/flex-grow)
    fn flex_grow(mut self, grow: f32) -> Self {
        self.style().flex_grow = Some(grow);
        self
    }

    /// Disables flex item growth (flex-grow: 0).
    /// [Docs](https://tailwindcss.com/docs/flex-grow#dont-grow)
    fn flex_grow_0(mut self) -> Self {
        self.style().flex_grow = Some(0.);
        self
    }

    /// Enables flex item growth (flex-grow: 1).
    /// [Docs](https://tailwindcss.com/docs/flex-grow#grow-1)
    fn flex_grow_1(mut self) -> Self {
        self.style().flex_grow = Some(1.);
        self
    }

    /// Sets the flex item's shrink factor.
    /// [Docs](https://tailwindcss.com/docs/flex-shrink)
    fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style().flex_shrink = Some(shrink);
        self
    }

    /// Disables flex item shrinking (flex-shrink: 0).
    /// [Docs](https://tailwindcss.com/docs/flex-shrink#dont-shrink)
    fn flex_shrink_0(mut self) -> Self {
        self.style().flex_shrink = Some(0.);
        self
    }

    /// Enables flex item shrinking (flex-shrink: 1).
    /// [Docs](https://tailwindcss.com/docs/flex-shrink#shrink-1)
    fn flex_shrink_1(mut self) -> Self {
        self.style().flex_shrink = Some(1.);
        self
    }

    /// Sets the element to allow flex items to wrap.
    /// [Docs](https://tailwindcss.com/docs/flex-wrap#wrap-normally)
    fn flex_wrap(mut self) -> Self {
        self.style().flex_wrap = Some(FlexWrap::Wrap);
        self
    }

    /// Sets the element wrap flex items in the reverse direction.
    /// [Docs](https://tailwindcss.com/docs/flex-wrap#wrap-reversed)
    fn flex_wrap_reverse(mut self) -> Self {
        self.style().flex_wrap = Some(FlexWrap::WrapReverse);
        self
    }

    /// Sets the element to prevent flex items from wrapping, causing inflexible items to overflow the container if necessary.
    /// [Docs](https://tailwindcss.com/docs/flex-wrap#dont-wrap)
    fn flex_nowrap(mut self) -> Self {
        self.style().flex_wrap = Some(FlexWrap::NoWrap);
        self
    }

    /// Sets the element to align flex items to the start of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-items#start)
    fn items_start(mut self) -> Self {
        self.style().align_items = Some(AlignItems::FlexStart);
        self
    }

    /// Sets the element to align flex items to the end of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-items#end)
    fn items_end(mut self) -> Self {
        self.style().align_items = Some(AlignItems::FlexEnd);
        self
    }

    /// Sets the element to align flex items along the center of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-items#center)
    fn items_center(mut self) -> Self {
        self.style().align_items = Some(AlignItems::Center);
        self
    }

    /// Sets the element to align flex items along the baseline of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-items#baseline)
    fn items_baseline(mut self) -> Self {
        self.style().align_items = Some(AlignItems::Baseline);
        self
    }

    /// Sets the element to stretch flex items to fill the available space along the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-items#stretch)
    fn items_stretch(mut self) -> Self {
        self.style().align_items = Some(AlignItems::Stretch);
        self
    }

    /// Sets how this specific element is aligned along the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#start)
    fn self_start(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::Start);
        self
    }

    /// Sets this element to align against the end of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#end)
    fn self_end(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::End);
        self
    }

    /// Sets this element to align against the start of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#start)
    fn self_flex_start(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::FlexStart);
        self
    }

    /// Sets this element to align against the end of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#end)
    fn self_flex_end(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::FlexEnd);
        self
    }

    /// Sets this element to align along the center of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#center)
    fn self_center(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::Center);
        self
    }

    /// Sets this element to align along the baseline of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#baseline)
    fn self_baseline(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::Baseline);
        self
    }

    /// Sets this element to stretch to fill the available space along the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-self#stretch)
    fn self_stretch(mut self) -> Self {
        self.style().align_self = Some(AlignSelf::Stretch);
        self
    }

    /// Sets the element to justify flex items against the start of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#start)
    fn justify_start(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::Start);
        self
    }

    /// Sets the element to justify flex items against the end of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#end)
    fn justify_end(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::End);
        self
    }

    /// Packs items against the flex-relative start of the main axis.
    fn justify_flex_start(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::FlexStart);
        self
    }

    /// Packs items against the flex-relative end of the main axis.
    fn justify_flex_end(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::FlexEnd);
        self
    }

    /// Sets the element to justify flex items along the center of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#center)
    fn justify_center(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::Center);
        self
    }

    /// Sets the element to justify flex items along the container's main axis
    /// such that there is an equal amount of space between each item.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-between)
    fn justify_between(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::SpaceBetween);
        self
    }

    /// Sets the element to justify items along the container's main axis such
    /// that there is an equal amount of space on each side of each item.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-around)
    fn justify_around(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::SpaceAround);
        self
    }

    /// Sets the element to justify items along the container's main axis such
    /// that there is an equal amount of space around each item, but also
    /// accounting for the doubling of space you would normally see between
    /// each item when using justify-around.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-evenly)
    fn justify_evenly(mut self) -> Self {
        self.style().justify_content = Some(JustifyContent::SpaceEvenly);
        self
    }

    /// Sets the element to pack content items in their default position as if no align-content value was set.
    /// [Docs](https://tailwindcss.com/docs/align-content#normal)
    fn content_normal(mut self) -> Self {
        self.style().align_content = None;
        self
    }

    /// Sets the element to pack content items in the center of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-content#center)
    fn content_center(mut self) -> Self {
        self.style().align_content = Some(AlignContent::Center);
        self
    }

    /// Sets the element to pack content items against the start of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-content#start)
    fn content_start(mut self) -> Self {
        self.style().align_content = Some(AlignContent::FlexStart);
        self
    }

    /// Sets the element to pack content items against the end of the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-content#end)
    fn content_end(mut self) -> Self {
        self.style().align_content = Some(AlignContent::FlexEnd);
        self
    }

    /// Sets the element to pack content items along the container's cross axis
    /// such that there is an equal amount of space between each item.
    /// [Docs](https://tailwindcss.com/docs/align-content#space-between)
    fn content_between(mut self) -> Self {
        self.style().align_content = Some(AlignContent::SpaceBetween);
        self
    }

    /// Sets the element to pack content items along the container's cross axis
    /// such that there is an equal amount of space on each side of each item.
    /// [Docs](https://tailwindcss.com/docs/align-content#space-around)
    fn content_around(mut self) -> Self {
        self.style().align_content = Some(AlignContent::SpaceAround);
        self
    }

    /// Sets the element to pack content items along the container's cross axis
    /// such that there is an equal amount of space between each item.
    /// [Docs](https://tailwindcss.com/docs/align-content#space-evenly)
    fn content_evenly(mut self) -> Self {
        self.style().align_content = Some(AlignContent::SpaceEvenly);
        self
    }

    /// Sets the element to allow content items to fill the available space along the container's cross axis.
    /// [Docs](https://tailwindcss.com/docs/align-content#stretch)
    fn content_stretch(mut self) -> Self {
        self.style().align_content = Some(AlignContent::Stretch);
        self
    }

    /// Sets the aspect ratio of the element.
    /// [Docs](https://tailwindcss.com/docs/aspect-ratio)
    fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.style().aspect_ratio = Some(ratio);
        self
    }

    /// Sets the aspect ratio of the element to 1/1 – equal width and height.
    /// [Docs](https://tailwindcss.com/docs/aspect-ratio)
    fn aspect_square(mut self) -> Self {
        self.style().aspect_ratio = Some(1.0);
        self
    }

    /// Sets the background color of the element.
    fn bg<F>(mut self, fill: F) -> Self
    where
        F: Into<Fill>,
        Self: Sized,
    {
        self.style().background = Some(fill.into());
        self
    }

    /// Sets the border style of the element.
    fn border_dashed(mut self) -> Self {
        self.style().border_style = Some(BorderStyle::Dashed);
        self
    }

    /// Sets the length of each border dash as a multiple of the border width.
    fn border_dashed_length(mut self, length_per_border_width: f32) -> Self {
        self.style().border_dashed_length = Some(length_per_border_width.max(0.0));

        self
    }

    /// Sets the gap between border dashes as a multiple of the border width.
    fn border_dashed_gap(mut self, gap_per_border_width: f32) -> Self {
        self.style().border_dashed_gap = Some(gap_per_border_width.max(0.0));

        self
    }

    /// Returns a mutable reference to the text style that has been configured on this element.
    fn text_style(&mut self) -> &mut TextStyleRefinement {
        let style: &mut StyleRefinement = self.style();
        &mut style.text
    }

    /// Sets the text color of this element.
    ///
    /// This value cascades to its child elements.
    fn text_color(mut self, color: impl IntoColor<Hsla>) -> Self {
        self.text_style().color = Some(color.into_color());
        self
    }

    /// Sets the font weight of this element
    ///
    /// This value cascades to its child elements.
    fn font_weight(mut self, weight: FontWeight) -> Self {
        self.text_style().font_weight = Some(weight);
        self
    }

    /// Sets the font width of this element and its children.
    ///
    /// Accepts a percentage or a [`FontWidth`] preset.
    fn font_width(mut self, width: impl Into<FontWidth>) -> Self {
        self.text_style().font_width = Some(width.into());

        self
    }

    /// Sets the background color of this element.
    ///
    /// This value cascades to its child elements.
    fn text_bg(mut self, bg: impl IntoColor<Hsla>) -> Self {
        self.text_style().background_color = Some(bg.into_color());
        self
    }

    /// Sets the text size of this element.
    ///
    /// This value cascades to its child elements.
    fn text_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.text_style().font_size = Some(size.into());
        self
    }

    /// Sets the text size to 'extra small'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_xs(mut self) -> Self {
        self.text_style().font_size = Some(rems(0.75).into());
        self
    }

    /// Sets the text size to 'small'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_sm(mut self) -> Self {
        self.text_style().font_size = Some(rems(0.875).into());
        self
    }

    /// Sets the text size to 'base'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_base(mut self) -> Self {
        self.text_style().font_size = Some(rems(1.0).into());
        self
    }

    /// Sets the text size to 'large'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_lg(mut self) -> Self {
        self.text_style().font_size = Some(rems(1.125).into());
        self
    }

    /// Sets the text size to 'extra large'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_xl(mut self) -> Self {
        self.text_style().font_size = Some(rems(1.25).into());
        self
    }

    /// Sets the text size to 'extra extra large'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_2xl(mut self) -> Self {
        self.text_style().font_size = Some(rems(1.5).into());
        self
    }

    /// Sets the text size to 'extra extra extra large'.
    /// [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
    fn text_3xl(mut self) -> Self {
        self.text_style().font_size = Some(rems(1.875).into());
        self
    }

    /// Sets the font style of the element to italic.
    /// [Docs](https://tailwindcss.com/docs/font-style#italicizing-text)
    fn italic(mut self) -> Self {
        self.text_style().font_style = Some(FontStyle::Italic);
        self
    }

    /// Sets the font style of the element to normal (not italic).
    /// [Docs](https://tailwindcss.com/docs/font-style#displaying-text-normally)
    fn not_italic(mut self) -> Self {
        self.text_style().font_style = Some(FontStyle::Normal);
        self
    }

    /// Sets the text decoration to underline.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-line#underling-text)
    fn underline(mut self) -> Self {
        let style = self.text_style();
        style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            ..Default::default()
        });
        self
    }

    /// Sets the decoration of the text to have a line through it.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-line#adding-a-line-through-text)
    fn line_through(mut self) -> Self {
        let style = self.text_style();
        style.strikethrough = Some(StrikethroughStyle {
            thickness: px(1.),
            ..Default::default()
        });
        self
    }

    /// Removes the text decoration on this element.
    ///
    /// This value cascades to its child elements.
    fn text_decoration_none(mut self) -> Self {
        self.text_style().underline = None;
        self
    }

    /// Sets the color for the underline on this element
    fn text_decoration_color(mut self, color: impl IntoColor<Hsla>) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.color = Some(color.into_color());
        self
    }

    /// Sets the text decoration style to a solid line.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-style)
    fn text_decoration_solid(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.wavy = false;
        self
    }

    /// Sets the text decoration style to a wavy line.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-style)
    fn text_decoration_wavy(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.wavy = true;
        self
    }

    /// Sets the text decoration to be 0px thick.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
    fn text_decoration_0(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.thickness = px(0.);
        self
    }

    /// Sets the text decoration to be 1px thick.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
    fn text_decoration_1(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.thickness = px(1.);
        self
    }

    /// Sets the text decoration to be 2px thick.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
    fn text_decoration_2(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.thickness = px(2.);
        self
    }

    /// Sets the text decoration to be 4px thick.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
    fn text_decoration_4(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.thickness = px(4.);
        self
    }

    /// Sets the text decoration to be 8px thick.
    /// [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
    fn text_decoration_8(mut self) -> Self {
        let style = self.text_style();
        let underline = style.underline.get_or_insert_with(Default::default);
        underline.thickness = px(8.);
        self
    }

    /// Sets the font family of this element and its children.
    fn font_family(mut self, family_name: impl Into<SharedString>) -> Self {
        self.text_style().font_family = Some(family_name.into());
        self
    }

    /// Sets the font features of this element and its children.
    fn font_features(mut self, features: FontFeatures) -> Self {
        self.text_style().font_features = Some(features);
        self
    }

    /// Sets the font of this element and its children.
    fn font(mut self, font: Font) -> Self {
        let Font {
            family,
            features,
            fallbacks,
            weight,
            width,
            style,
        } = font;

        let text_style = self.text_style();
        text_style.font_family = Some(family);
        text_style.font_features = Some(features);
        text_style.font_weight = Some(weight);
        text_style.font_width = Some(width);
        text_style.font_style = Some(style);
        text_style.font_fallbacks = fallbacks;

        self
    }

    /// Sets the line height of this element and its children.
    fn line_height(mut self, line_height: impl Into<DefiniteLength>) -> Self {
        self.text_style().line_height = Some(line_height.into());
        self
    }

    /// Sets the opacity of this element and its children, composited together as one group.
    fn opacity(mut self, opacity: f32) -> Self {
        self.style().opacity = Some(opacity);
        self
    }

    /// Sets the grid columns of this element.
    fn grid_cols(mut self, cols: u16) -> Self {
        self.style().grid_cols = Some(GridTemplate {
            repeat: cols,
            min_size: GridTemplateMinSize::Zero,
        });
        self
    }

    /// Sets the grid columns with min-content minimum sizing.
    /// Unlike grid_cols, it won't shrink to width 0 in AvailableSpace::MinContent constraints.
    fn grid_cols_min_content(mut self, cols: u16) -> Self {
        self.style().grid_cols = Some(GridTemplate {
            repeat: cols,
            min_size: GridTemplateMinSize::MinContent,
        });
        self
    }

    /// Sets the grid columns with max-content maximum sizing for content-based column widths.
    fn grid_cols_max_content(mut self, cols: u16) -> Self {
        self.style().grid_cols = Some(GridTemplate {
            repeat: cols,
            min_size: GridTemplateMinSize::MaxContent,
        });
        self
    }

    /// Sets the grid rows of this element.
    fn grid_rows(mut self, rows: u16) -> Self {
        self.style().grid_rows = Some(GridTemplate {
            repeat: rows,
            min_size: GridTemplateMinSize::Zero,
        });
        self
    }

    /// Sets the grid rows with min-content minimum sizing.
    /// Unlike grid_rows, it won't shrink to height 0 in AvailableSpace::MinContent constraints.
    fn grid_rows_min_content(mut self, rows: u16) -> Self {
        self.style().grid_rows = Some(GridTemplate {
            repeat: rows,
            min_size: GridTemplateMinSize::MinContent,
        });
        self
    }

    /// Sets the grid rows with max-content maximum sizing for content-based row heights.
    fn grid_rows_max_content(mut self, rows: u16) -> Self {
        self.style().grid_rows = Some(GridTemplate {
            repeat: rows,
            min_size: GridTemplateMinSize::MaxContent,
        });
        self
    }

    /// Sets the column start of this element.
    fn col_start(mut self, start: i16) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.column.start = GridPlacement::Line(start);
        self
    }

    /// Sets the column start of this element to auto.
    fn col_start_auto(mut self) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.column.start = GridPlacement::Auto;
        self
    }

    /// Sets the column end of this element.
    fn col_end(mut self, end: i16) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.column.end = GridPlacement::Line(end);
        self
    }

    /// Sets the column end of this element to auto.
    fn col_end_auto(mut self) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.column.end = GridPlacement::Auto;
        self
    }

    /// Sets the column span of this element.
    fn col_span(mut self, span: u16) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.column = GridPlacement::Span(span)..GridPlacement::Span(span);
        self
    }

    /// Sets the row span of this element.
    fn col_span_full(mut self) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.column = GridPlacement::Line(1)..GridPlacement::Line(-1);
        self
    }

    /// Sets the row start of this element.
    fn row_start(mut self, start: i16) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.row.start = GridPlacement::Line(start);
        self
    }

    /// Sets the row start of this element to "auto"
    fn row_start_auto(mut self) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.row.start = GridPlacement::Auto;
        self
    }

    /// Sets the row end of this element.
    fn row_end(mut self, end: i16) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.row.end = GridPlacement::Line(end);
        self
    }

    /// Sets the row end of this element to "auto"
    fn row_end_auto(mut self) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.row.end = GridPlacement::Auto;
        self
    }

    /// Sets the row span of this element.
    fn row_span(mut self, span: u16) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.row = GridPlacement::Span(span)..GridPlacement::Span(span);
        self
    }

    /// Sets the row span of this element.
    fn row_span_full(mut self) -> Self {
        let grid_location = self.style().grid_location_mut();
        grid_location.row = GridPlacement::Line(1)..GridPlacement::Line(-1);
        self
    }

    /// Draws a debug border around this element.
    #[cfg(debug_assertions)]
    fn debug(mut self) -> Self {
        self.style().debug = Some(true);
        self
    }

    /// Draws a debug border on all conforming elements below this element.
    #[cfg(debug_assertions)]
    fn debug_below(mut self) -> Self {
        self.style().debug_below = Some(true);
        self
    }

    /// Sets the amount of smoothing applied to rounded corners.
    /// Use values from `0.0` for circular corners to `1.0` for maximum smoothing.
    fn rounded_smoothing(mut self, amount: f32) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&amount),
            "corner smoothing must be between 0 and 1"
        );
        self.style().corner_smoothing = Some(amount);
        self
    }

    /// Sets corner smoothing to `0.6` for iOS-style corners.
    fn rounded_smoothing_ios(mut self) -> Self {
        self.style().corner_smoothing = Some(0.6);
        self
    }
}
