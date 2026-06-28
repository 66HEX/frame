//! Text Example
//!
//! This example demonstrates text capabilities in GPUI:
//!
//! 1. Text Styling - Font sizes, weights, and colors
//! 2. Text Alignment - Left, center, right alignment
//! 3. Text Decoration - Underline, strikethrough
//! 4. Text Overflow - Ellipsis, truncation, line clamping
//! 5. Styled Text - Inline style variations with highlights
//! 6. Character Grid - Unicode and emoji support

#[path = "../shared/prelude.rs"]
mod example_prelude;

use example_prelude::init_example;
use gpui::{
    App, Bounds, Context, FontStyle, FontWeight, Hsla, Render, StyledText, TextOverflow, Window,
    WindowBounds, WindowOptions, colors::Colors, div, prelude::*, px, relative, rgb, size,
};

// Text Styling Examples

fn text_sizes_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Font sizes: text_xs, text_sm, text_base, text_lg, text_xl"),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_baseline()
                .gap_3()
                .child(div().text_xs().text_color(text).child("Extra Small"))
                .child(div().text_sm().text_color(text).child("Small"))
                .child(div().text_base().text_color(text).child("Base"))
                .child(div().text_lg().text_color(text).child("Large"))
                .child(div().text_xl().text_color(text).child("Extra Large")),
        )
}

fn text_weights_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Font weights: THIN through BLACK"),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_3()
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::THIN)
                        .child("Thin"),
                )
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::LIGHT)
                        .child("Light"),
                )
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::NORMAL)
                        .child("Normal"),
                )
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::MEDIUM)
                        .child("Medium"),
                )
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Semibold"),
                )
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::BOLD)
                        .child("Bold"),
                )
                .child(
                    div()
                        .text_color(text)
                        .font_weight(FontWeight::BLACK)
                        .child("Black"),
                ),
        )
}

// Text Alignment Examples

fn text_alignment_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;
    let surface = colors.container;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Alignment: default (left), text_center, text_right"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .p_2()
                        .bg(surface)
                        .rounded_sm()
                        .text_color(text)
                        .child("Left aligned (default)"),
                )
                .child(
                    div()
                        .p_2()
                        .bg(surface)
                        .rounded_sm()
                        .text_center()
                        .text_color(text)
                        .child("Center aligned"),
                )
                .child(
                    div()
                        .p_2()
                        .bg(surface)
                        .rounded_sm()
                        .text_right()
                        .text_color(text)
                        .child("Right aligned"),
                ),
        )
}

// Text Decoration Examples

fn text_decoration_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;
    let accent = colors.selected;
    let error = rgb(0xd32f2f);

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Decorations: underline, strikethrough, italic"),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(
                    div()
                        .text_color(text)
                        .text_decoration_1()
                        .text_decoration_color(accent)
                        .child("Underlined text"),
                )
                .child(
                    div()
                        .text_color(text)
                        .line_through()
                        .text_decoration_color(error)
                        .child("Strikethrough text"),
                )
                .child(div().text_color(text).italic().child("Italic text")),
        )
}

// Text Overflow Examples

fn text_overflow_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;
    let surface = colors.container;
    let border = colors.border;

    let long_text = "The quick brown fox jumps over the lazy dog. This is a long sentence that will overflow its container.";

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Overflow handling: ellipsis, truncate, line_clamp"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_muted)
                                .child("text_ellipsis (single line):"),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .rounded_sm()
                                .text_color(text)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(long_text),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_muted)
                                .child("line_clamp(2):"),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .rounded_sm()
                                .text_color(text)
                                .overflow_hidden()
                                .text_ellipsis()
                                .line_clamp(2)
                                .child(long_text),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_muted)
                                .child("truncate (hard cut):"),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .rounded_sm()
                                .text_color(text)
                                .overflow_hidden()
                                .text_overflow(TextOverflow::Truncate("".into()))
                                .child(long_text),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_muted)
                                .child("whitespace_nowrap:"),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .rounded_sm()
                                .text_color(text)
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(long_text),
                        ),
                ),
        )
}

// Styled Text Examples

fn styled_text_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("StyledText with inline highlights"),
        )
        .child(div().text_lg().text_color(text).child(
            StyledText::new("Bold Italic Normal Semibold").with_highlights([
                (0..4, FontWeight::BOLD.into()),
                (5..11, FontStyle::Italic.into()),
                (19..27, FontWeight::SEMIBOLD.into()),
            ]),
        ))
}

// Character Grid Example

fn character_grid_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;
    let surface = colors.container;
    let border = colors.border;

    let characters = [
        // Latin
        "A", "B", "C", "D", "E", "a", "b", "c", "d", "e", // Numbers
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", // Greek
        "α", "β", "γ", "δ", "ε", "θ", "λ", "π", "σ", "ω", // Cyrillic
        "Д", "Ж", "И", "Л", "Ф", "Ц", "Ш", "Щ", "Ы", "Я", // CJK
        "你", "好", "世", "界", "日", "本", "語", "中", "文", "字", // Symbols
        "→", "←", "↑", "↓", "•", "★", "♠", "♥", "♦", "♣", // Emoji
        "😀", "🎉", "🚀", "💡", "🔥", "✨", "🎨", "📚", "🎵", "❤️",
    ];

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Unicode and emoji support"),
        )
        .child(
            div()
                .p_2()
                .bg(surface)
                .border_1()
                .border_color(border)
                .rounded_md()
                .child(
                    div()
                        .grid()
                        .grid_cols(10)
                        .gap_1()
                        .children(characters.iter().map(|c| {
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_8()
                                .text_lg()
                                .text_color(text)
                                .line_height(relative(1.0))
                                .child(*c)
                        })),
                ),
        )
}

// Line Height Example

fn line_height_example(colors: &Colors) -> impl IntoElement {
    let text = colors.text;
    let text_muted = colors.disabled;
    let surface = colors.container;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Line height: relative(1.0), relative(1.5), relative(2.0)"),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .p_2()
                        .bg(surface)
                        .rounded_sm()
                        .text_color(text)
                        .text_sm()
                        .line_height(relative(1.0))
                        .child("Tight\nline\nheight"),
                )
                .child(
                    div()
                        .flex_1()
                        .p_2()
                        .bg(surface)
                        .rounded_sm()
                        .text_color(text)
                        .text_sm()
                        .line_height(relative(1.5))
                        .child("Normal\nline\nheight"),
                )
                .child(
                    div()
                        .flex_1()
                        .p_2()
                        .bg(surface)
                        .rounded_sm()
                        .text_color(text)
                        .text_sm()
                        .line_height(relative(2.0))
                        .child("Loose\nline\nheight"),
                ),
        )
}

// Main Application View

struct TextExample;

impl Render for TextExample {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

        div()
            .id("main")
            .size_full()
            .p_4()
            .bg(colors.background)
            .overflow_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .max_w(px(600.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child("Text & Typography"),
                            )
                            .child(
                                div().text_sm().text_color(colors.disabled).child(
                                    "Font styling, alignment, overflow, and unicode support",
                                ),
                            ),
                    )
                    .child(section(&colors, "Font Sizes", text_sizes_example(&colors)))
                    .child(section(
                        &colors,
                        "Font Weights",
                        text_weights_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Text Alignment",
                        text_alignment_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Text Decoration",
                        text_decoration_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Line Height",
                        line_height_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Styled Text",
                        styled_text_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Text Overflow",
                        text_overflow_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Character Grid",
                        character_grid_example(&colors),
                    )),
            )
    }
}

fn section(colors: &Colors, title: &'static str, content: impl IntoElement) -> impl IntoElement {
    let surface: Hsla = colors.container.into();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .bg(surface.opacity(0.5))
        .rounded_lg()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.text)
                .child(title),
        )
        .child(content)
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(650.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| TextExample),
        )
        .expect("Failed to open window");

        init_example(cx, "Text");
    });
}
