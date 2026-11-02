//! Scroll through 10,000 rows with a virtualized list.
//! Each row has the same height, and GPUI requests the ranges needed for the viewport.

#![cfg_attr(target_family = "wasm", no_main)]

use gpui::colors::Colors;
use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
    uniform_list,
};
use gpui_platform::application;

#[path = "../shared/prelude.rs"]
mod example_prelude;
#[path = "../example_support/fonts.rs"]
mod example_support;

struct UniformListExample {}

impl Render for UniformListExample {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .p_4()
            .overflow_hidden()
            .bg(colors.background)
            .text_color(colors.text)
            .child("10,000 rows. Scroll to see more.")
            .child(
                uniform_list("items", 10_000, |range, _window, _cx| {
                    range
                        .map(|index| {
                            div()
                                .id(index)
                                .h(px(24.0))
                                .px_2()
                                .child(format!("Item {index}"))
                        })
                        .collect()
                })
                .h(px(300.0))
                .bg(colors.container),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        if !example_support::load_fonts(cx) {
            return;
        }

        let bounds = Bounds::centered(None, size(px(400.0), px(400.0)), cx);
        cx.open_window(
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |_window, cx| cx.new(|_cx| UniformListExample {}),
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Virtualized lists");
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    env_logger::init();
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
