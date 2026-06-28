//! Layout Patterns Example
//!
//! This example demonstrates different layout approaches in GPUI:
//!
//! 1. Flexbox - Row and column layouts with alignment
//! 2. Grid - Two-dimensional layouts with spans
//! 3. Common patterns - Sidebar, header/footer, centering

#[path = "../shared/prelude.rs"]
mod example_prelude;

use example_prelude::init_example;
use gpui::colors::Colors;
use gpui::{
    Bounds, Context, Div, Hsla, Render, Rgba, Window, WindowBounds, WindowOptions, div, prelude::*,
    px, size,
};

// Helper: Colored block for visualization

fn block(label: &'static str, color: Hsla, text_color: Rgba) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .bg(color)
        .border_1()
        .border_color(gpui::white().opacity(0.3))
        .rounded_md()
        .text_xs()
        .text_color(text_color)
        .child(label)
}

// Flexbox Examples

fn flexbox_row_example(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("flex().flex_row().gap_2()"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(block("A", gpui::red(), text).size_8())
                .child(block("B", gpui::green(), text).size_8())
                .child(block("C", gpui::blue(), text).size_8()),
        )
}

fn flexbox_column_example(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("flex().flex_col().gap_2()"),
        )
        .child(
            div()
                .h_24()
                .flex()
                .flex_col()
                .gap_2()
                .child(block("A", gpui::red(), text).h_6())
                .child(block("B", gpui::green(), text).h_6())
                .child(block("C", gpui::blue(), text).h_6()),
        )
}

fn flexbox_justify_example(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;
    let surface = colors.container;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("justify_between / justify_center / justify_end"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .p_1()
                        .bg(surface)
                        .rounded_sm()
                        .child(block("Start", gpui::red(), text).px_2().py_1())
                        .child(block("End", gpui::blue(), text).px_2().py_1()),
                )
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .p_1()
                        .bg(surface)
                        .rounded_sm()
                        .child(block("Center", gpui::green(), text).px_2().py_1()),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .p_1()
                        .bg(surface)
                        .rounded_sm()
                        .child(block("End", gpui::yellow(), text).px_2().py_1()),
                ),
        )
}

fn flexbox_grow_example(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("flex_1 (grow) vs flex_none (fixed)"),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(block("fixed", gpui::red(), text).flex_none().w_16().h_8())
                .child(block("flex_1 (grows)", gpui::green(), text).flex_1().h_8())
                .child(block("fixed", gpui::blue(), text).flex_none().w_16().h_8()),
        )
}

// Grid Examples

fn grid_basic_example(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("grid().grid_cols(3).gap_1()"),
        )
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_1()
                .child(block("1", gpui::red(), text).h_8())
                .child(block("2", gpui::green(), text).h_8())
                .child(block("3", gpui::blue(), text).h_8())
                .child(block("4", gpui::yellow(), text).h_8())
                .child(block("5", gpui::red(), text).h_8())
                .child(block("6", gpui::green(), text).h_8()),
        )
}

fn grid_span_example(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("col_span / row_span"),
        )
        .child(
            div()
                .grid()
                .grid_cols(4)
                .grid_rows(3)
                .gap_1()
                .child(
                    block("Header (col_span_full)", gpui::red(), text)
                        .col_span_full()
                        .h_6(),
                )
                .child(
                    block("Side", gpui::green(), text)
                        .col_span(1)
                        .row_span(2)
                        .h_full(),
                )
                .child(
                    block("Content (col_span 3)", gpui::blue(), text)
                        .col_span(3)
                        .row_span(2)
                        .h_full(),
                ),
        )
}

// Common Layout Patterns

fn app_shell_pattern(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.text;
    let surface = colors.container;
    let surface_hover = colors.selected;
    let background = colors.background;
    let border = colors.border;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("App Shell: Header + Sidebar + Content"),
        )
        .child(
            div()
                .h_32()
                .flex()
                .flex_col()
                .border_1()
                .border_color(border)
                .rounded_md()
                .overflow_hidden()
                .child(
                    div()
                        .h_6()
                        .flex()
                        .items_center()
                        .px_2()
                        .bg(surface_hover)
                        .text_xs()
                        .text_color(text)
                        .child("Header"),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .child(
                            div()
                                .w_16()
                                .bg(surface)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .text_color(text_muted)
                                .child("Side"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .bg(background)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .text_color(text_muted)
                                .child("Content"),
                        ),
                ),
        )
}

fn centered_pattern(colors: &Colors) -> impl IntoElement {
    let text_muted = colors.disabled;
    let text = colors.selected_text;
    let surface = colors.container;
    let accent = colors.selected;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(text_muted)
                .child("Centering: items_center + justify_center"),
        )
        .child(
            div()
                .h_20()
                .flex()
                .items_center()
                .justify_center()
                .bg(surface)
                .rounded_md()
                .child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(accent)
                        .rounded_md()
                        .text_xs()
                        .text_color(text)
                        .child("Perfectly Centered"),
                ),
        )
}

fn stack_pattern(colors: &Colors) -> impl IntoElement {
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
                .child("Stack: Overlapping with absolute positioning"),
        )
        .child(
            div()
                .h_20()
                .relative()
                .bg(surface)
                .rounded_md()
                .child(
                    div()
                        .absolute()
                        .top_2()
                        .left_2()
                        .size_10()
                        .bg(gpui::red().opacity(0.7))
                        .rounded_md(),
                )
                .child(
                    div()
                        .absolute()
                        .top_4()
                        .left_4()
                        .size_10()
                        .bg(gpui::green().opacity(0.7))
                        .rounded_md(),
                )
                .child(
                    div()
                        .absolute()
                        .top_6()
                        .left_6()
                        .size_10()
                        .bg(gpui::blue().opacity(0.7))
                        .rounded_md(),
                ),
        )
}

// Main Application View

struct LayoutExample;

impl Render for LayoutExample {
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
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child("Layout Patterns"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.disabled)
                                    .child("Flexbox, Grid, and common layout patterns in GPUI"),
                            ),
                    )
                    .child(section(
                        &colors,
                        "Flexbox: Row",
                        flexbox_row_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Flexbox: Column",
                        flexbox_column_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Flexbox: Justify Content",
                        flexbox_justify_example(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Flexbox: Grow/Shrink",
                        flexbox_grow_example(&colors),
                    ))
                    .child(section(&colors, "Grid: Basic", grid_basic_example(&colors)))
                    .child(section(&colors, "Grid: Spans", grid_span_example(&colors)))
                    .child(section(
                        &colors,
                        "Pattern: App Shell",
                        app_shell_pattern(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Pattern: Centering",
                        centered_pattern(&colors),
                    ))
                    .child(section(&colors, "Pattern: Stack", stack_pattern(&colors))),
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
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(colors.text)
                .child(title),
        )
        .child(content)
}

fn main() {
    gpui_platform::application().run(|cx| {
        let bounds = Bounds::centered(None, size(px(650.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| LayoutExample),
        )
        .expect("Failed to open window");

        init_example(cx, "Layout");
    });
}
