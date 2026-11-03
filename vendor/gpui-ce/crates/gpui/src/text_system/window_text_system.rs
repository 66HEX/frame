use crate::{
    InlineLayout, InlineLayoutRequest, LineLayout, LineLayoutCache, LineLayoutIndex, Pixels,
    Result, ShapedLine, ShapedText, SharedString, TextLayoutOptions, TextRun, TextSystem,
};
use derive_more::Deref;
use std::{ops::Range, sync::Arc};

/// The GPUI text layout subsystem.
#[derive(Deref)]
pub struct WindowTextSystem {
    line_layout_cache: LineLayoutCache,
    #[deref]
    text_system: Arc<TextSystem>,
}

impl WindowTextSystem {
    /// Create a new WindowTextSystem with the given TextSystem.
    pub fn new(text_system: Arc<TextSystem>) -> Self {
        Self {
            line_layout_cache: LineLayoutCache::new(text_system.platform_text_system.clone()),
            text_system,
        }
    }

    pub(crate) fn layout_index(&self) -> LineLayoutIndex {
        self.line_layout_cache.layout_index()
    }

    pub(crate) fn reuse_layouts(&self, index: Range<LineLayoutIndex>) {
        self.line_layout_cache.reuse_layouts(index)
    }

    pub(crate) fn truncate_layouts(&self, index: LineLayoutIndex) {
        self.line_layout_cache.truncate_layouts(index)
    }

    /// Shape the given line, at the given font_size, for painting to the screen.
    /// Subsets of the line can be styled independently with the `runs` parameter.
    ///
    /// Note that this method can only shape a single line of text. It will panic
    /// if the text contains newlines. If you need to shape multiple lines of text,
    /// use [`Self::shape_text`] instead.
    pub fn shape_line(
        &self,
        text: SharedString,
        font_size: Pixels,
        runs: &[TextRun],
    ) -> ShapedLine {
        debug_assert!(
            text.find('\n').is_none(),
            "text argument should not contain newlines"
        );

        let layout = self.layout_line(&text, font_size, runs);

        ShapedLine { layout, text }
    }

    /// Shape a multi line string of text, at the given font_size, for painting to the screen.
    /// Subsets of the text can be styled independently with the `runs` parameter,
    /// where each run gives the number of UTF-8 bytes that it styles. Runs must cover the
    /// complete string and end on UTF-8 character boundaries.
    ///
    /// If `wrap_width` is provided, the line breaks will be adjusted to fit within the given width.
    ///
    /// The backend receives the complete string. Hard breaks, trailing empty lines, bidi
    /// paragraphs, wrapping, and `line_clamp` therefore share one layout model.
    pub fn shape_text<S: AsRef<str> + Into<SharedString>>(
        &self,
        text: S,
        font_size: Pixels,
        runs: &[TextRun],
        wrap_width: Option<Pixels>,
        line_clamp: Option<usize>,
    ) -> Result<ShapedText> {
        self.shape_text_with_options(
            text,
            font_size,
            runs,
            TextLayoutOptions {
                wrap_width,
                line_clamp,
                ..Default::default()
            },
        )
    }

    /// Shapes a complete text document with explicit direction and alignment options.
    #[doc(hidden)]
    pub fn shape_text_with_options<S: AsRef<str> + Into<SharedString>>(
        &self,
        text: S,
        font_size: Pixels,
        runs: &[TextRun],
        options: TextLayoutOptions,
    ) -> Result<ShapedText> {
        let text = text.into();
        let layout = self
            .line_layout_cache
            .layout_text_with_options(&text, font_size, runs, options);

        Ok(ShapedText { layout, text })
    }

    /// Layout text and atomic element boxes in one inline formatting context.
    pub fn layout_inline(&self, request: InlineLayoutRequest<'_>) -> Arc<InlineLayout> {
        self.line_layout_cache.layout_inline(request)
    }

    /// Layout the given line of text, at the given font_size.
    /// Subsets of the line can be styled independently with the `runs` parameter.
    /// Generally, you should prefer to use [`Self::shape_line`] instead, which
    /// can be painted directly.
    pub fn layout_line(&self, text: &str, font_size: Pixels, runs: &[TextRun]) -> Arc<LineLayout> {
        self.line_layout_cache
            .layout_line(&SharedString::new(text), font_size, runs)
    }

    pub(crate) fn finish_frame(&self) {
        self.line_layout_cache.finish_frame()
    }
}
