//! Text system example.

#[path = "shared/prelude.rs"]
mod example_prelude;

use std::borrow::Cow;

use example_prelude::init_example;
use gpui::{
    App, Bounds, Context, Div, FontFallbacks, FontFeatures, FontStyle, FontWeight, FontWidth,
    HighlightStyle, Render, StrikethroughStyle, StyledText, TextTransform, UnderlineStyle, Window,
    WindowBounds, WindowOptions, div, font, hsla, prelude::*, px, relative, rgb, size,
};

const BACKGROUND: u32 = 0x0d1117;
const SURFACE: u32 = 0x161b22;
const SAMPLE_SURFACE: u32 = 0x21262d;
const BORDER: u32 = 0x30363d;
const TEXT: u32 = 0xe6edf3;
const MUTED: u32 = 0x8b949e;
const ACCENT: u32 = 0x58a6ff;

struct TextSystemExample;

fn register_fonts(cx: &App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(*gpui_fonts::IBM_PLEX),
            Cow::Borrowed(*gpui_fonts::IBM_PLEX_ITALIC),
            Cow::Borrowed(*gpui_fonts::IBM_PLEX_SEMIBOLD),
            Cow::Borrowed(*gpui_fonts::IBM_PLEX_SEMIBOLD_ITALIC),
            Cow::Borrowed(*gpui_fonts::LILEX),
            Cow::Borrowed(*gpui_fonts::LILEX_BOLD),
            Cow::Borrowed(*gpui_fonts::NOTO_SANS),
            Cow::Borrowed(*gpui_fonts::NOTO_ARABIC),
            Cow::Borrowed(*gpui_fonts::NOTO_HEBREW),
            Cow::Borrowed(*gpui_fonts::NOTO_COLOR_EMOJI),
        ])
        .expect("failed to register the text system example fonts");
}

fn section(
    title: &'static str,
    description: &'static str,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_3()
        .p_5()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(div().text_sm().text_color(rgb(MUTED)).child(description)),
        )
        .child(content)
}

fn sample(label: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .min_w(px(240.))
        .max_w_full()
        .flex_1()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded_md()
        .bg(rgb(SAMPLE_SURFACE))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child(label),
        )
        .child(content)
}

fn sample_row() -> Div {
    div().flex().flex_wrap().gap_3()
}

fn paragraph_container() -> Div {
    div().block().w_full().min_w_0()
}

fn font_faces() -> impl IntoElement {
    sample_row()
        .child(sample(
            "IBM Plex Sans",
            div()
                .font_family("IBM Plex Sans")
                .text_lg()
                .flex()
                .flex_col()
                .gap_2()
                .child("Regular: Hamburgefonts 0123456789")
                .child(div().italic().child("Italic: Hamburgefonts 0123456789"))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Semibold: Hamburgefonts 0123456789"),
                )
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .italic()
                        .child("Semibold italic: Hamburgefonts 0123456789"),
                ),
        ))
        .child(sample(
            "Lilex",
            div()
                .font_family("Lilex")
                .text_lg()
                .flex()
                .flex_col()
                .gap_2()
                .child("fn main() { println!(\"hello\"); }")
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .child("let answer = Some(42);"),
                ),
        ))
}

fn variable_font_widths() -> impl IntoElement {
    sample_row().children(
        [
            ("Extra-condensed (62.5%)", FontWidth::EXTRA_CONDENSED),
            ("Condensed (75%)", FontWidth::CONDENSED),
            ("Normal (100%)", FontWidth::NORMAL),
        ]
        .map(|(label, width)| {
            sample(
                label,
                div()
                    .font_family("Noto Sans")
                    .font_width(width)
                    .text_2xl()
                    .child("hello gpui-ce"),
            )
        }),
    )
}

fn styled_runs() -> impl IntoElement {
    const TEXT_RUNS: &str = "Styled runs share one line of text.";

    let highlights = [
        (
            0..6,
            HighlightStyle {
                color: Some(hsla(0.58, 0.9, 0.7, 1.)),
                font_weight: Some(FontWeight::BOLD),
                ..Default::default()
            },
        ),
        (
            7..11,
            HighlightStyle {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        ),
        (
            12..17,
            HighlightStyle {
                background_color: Some(hsla(0.13, 0.7, 0.35, 0.45)),
                ..Default::default()
            },
        ),
        (
            18..21,
            HighlightStyle {
                underline: Some(UnderlineStyle {
                    thickness: px(2.),
                    color: Some(hsla(0.36, 0.65, 0.55, 1.)),
                    wavy: false,
                }),
                ..Default::default()
            },
        ),
        (
            22..26,
            HighlightStyle {
                strikethrough: Some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(hsla(0.0, 0.75, 0.65, 1.)),
                }),
                ..Default::default()
            },
        ),
    ];

    div()
        .rounded_md()
        .bg(rgb(SAMPLE_SURFACE))
        .p_4()
        .font_family("IBM Plex Sans")
        .text_2xl()
        .child(StyledText::new(TEXT_RUNS).with_highlights(highlights))
}

fn open_type_and_spacing() -> impl IntoElement {
    let ligature_text = "!=  ==  =>  ->  >=  <=";

    sample_row()
        .child(sample(
            "Default font features",
            div().font_family("Lilex").text_2xl().child(ligature_text),
        ))
        .child(sample(
            "Ligatures disabled",
            div()
                .font_family("Lilex")
                .font_features(FontFeatures::disable_ligatures())
                .text_2xl()
                .child(ligature_text),
        ))
        .child(sample(
            "Letter spacing and case",
            div()
                .font_family("IBM Plex Sans")
                .text_transform(TextTransform::Uppercase)
                .letter_spacing(px(2.5))
                .text_lg()
                .child("spaced heading"),
        ))
}

fn multilingual_text() -> impl IntoElement {
    let mut multilingual_font = font("IBM Plex Sans");
    multilingual_font.fallbacks = Some(FontFallbacks::from_fonts(vec![
        "Noto Sans Arabic".into(),
        "Noto Sans Hebrew".into(),
        "Noto Color Emoji".into(),
    ]));

    let mut emoji_samples = div().flex().flex_col().gap_2();
    for font_size in [16.0, 24.0, 32.0] {
        emoji_samples = emoji_samples.child(
            div()
                .text_size(px(font_size))
                .child("Color emoji: 😀 🎉 🚀 💡 🔥 ✨"),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_md()
        .bg(rgb(SAMPLE_SURFACE))
        .p_4()
        .font(multilingual_font)
        .text_2xl()
        .line_height(relative(1.6))
        .child("Latin and combining marks: Café · Cafe\u{301} · naïve")
        .child("Arabic: مرحباً بالعالم")
        .child("Hebrew: שלום עולם")
        .child("Mixed direction: GPUI يكتب النص 42 مرة")
        .child(emoji_samples)
}

fn paragraph_layout() -> impl IntoElement {
    const PARAGRAPH: &str = "A paragraph can wrap at word boundaries while keeping styled text, punctuation, and spacing together. Resize the window and this sample will follow the available width.";
    const PATH: &str = "/workspace/projects/gpui/examples/text_system/really_long_filename.rs";

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            paragraph_container()
                .rounded_md()
                .bg(rgb(SAMPLE_SURFACE))
                .p_4()
                .font_family("IBM Plex Sans")
                .text_lg()
                .line_height(relative(1.55))
                .child(PARAGRAPH),
        )
        .child(
            sample_row()
                .child(sample(
                    "Line height 1.0",
                    div()
                        .font_family("IBM Plex Sans")
                        .line_height(relative(1.0))
                        .child("Tight lines\nkeep rows close\ntogether."),
                ))
                .child(sample(
                    "Line height 1.8",
                    div()
                        .font_family("IBM Plex Sans")
                        .line_height(relative(1.8))
                        .child("Loose lines\nleave more room\nbetween rows."),
                ))
                .child(sample(
                    "Centered",
                    paragraph_container()
                        .font_family("IBM Plex Sans")
                        .text_center()
                        .child("Every visual line is centered inside the available width."),
                )),
        )
        .child(
            sample_row()
                .child(sample(
                    "End ellipsis",
                    paragraph_container()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(PATH),
                ))
                .child(sample(
                    "Middle ellipsis",
                    paragraph_container()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis_middle()
                        .child(PATH),
                ))
                .child(sample(
                    "Two-line clamp",
                    paragraph_container()
                        .text_ellipsis()
                        .line_clamp(2)
                        .child(PARAGRAPH),
                )),
        )
}

fn inline_badge(label: &'static str) -> impl IntoElement {
    div()
        .inline_flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(rgb(0x1f6f78))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
        .align_middle()
        .child(div().size_2().rounded_full().bg(rgb(0xffffff)))
        .child(label)
}

fn inline_layout() -> impl IntoElement {
    div()
        .block()
        .w_full()
        .rounded_md()
        .bg(rgb(SAMPLE_SURFACE))
        .p_5()
        .font_family("IBM Plex Sans")
        .text_size(px(20.))
        .line_height(relative(1.65))
        .child("A paragraph can contain ")
        .child(inline_badge("inline elements"))
        .child(
            div()
                .inline()
                .text_color(rgb(0xf2cc60))
                .child(" alongside nested spans with ")
                .child(
                    div()
                        .inline()
                        .font_weight(FontWeight::BOLD)
                        .child("their own styles"),
                )
                .child(". Everything remains in the same flow, so this "),
        )
        .child(inline_badge("badge"))
        .child(" moves naturally as the window becomes narrower.")
}

impl Render for TextSystemExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("text-system-content")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .overflow_y_scroll()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .max_w(px(960.))
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .gap_5()
                    .p_8()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_3xl()
                                        .font_weight(FontWeight::BOLD)
                                        .child("GPUI text system"),
                                )
                                .child(
                                    div()
                                        .text_color(rgb(MUTED))
                                        .child("Font selection, rich text, multilingual text, wrapping, truncation, and inline layout. Resize the window to see the examples reflow."),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(ACCENT))
                                        .child("The sample fonts are bundled with this example."),
                                ),
                        )
                        .child(section(
                            "Font families and faces",
                            "Select a family, weight, and style through inherited element styles.",
                            font_faces(),
                        ))
                        .child(section(
                            "Variable font width",
                            "Noto Sans at different widths, with the same text, weight, and font size.",
                            variable_font_widths(),
                        ))
                        .child(section(
                            "Styled runs",
                            "A single text value can contain independent paint and font styles.",
                            styled_runs(),
                        ))
                        .child(section(
                            "OpenType features and spacing",
                            "Control font features, case transformation, and letter spacing.",
                            open_type_and_spacing(),
                        ))
                        .child(section(
                            "Scripts and fallback fonts",
                            "Mix combining marks, right-to-left scripts, and emoji in the same view.",
                            multilingual_text(),
                        ))
                        .child(section(
                            "Paragraph layout",
                            "Wrap, align, space, clamp, and truncate text within its container.",
                            paragraph_layout(),
                        ))
                        .child(section(
                            "Inline layout",
                            "Place styled spans and element boxes inside a wrapping paragraph.",
                            inline_layout(),
                        )),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        register_fonts(cx);

        let bounds = Bounds::centered(None, size(px(1000.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| TextSystemExample),
        )
        .expect("failed to open the text system example window");

        init_example(cx, "Text system");
    });
}
