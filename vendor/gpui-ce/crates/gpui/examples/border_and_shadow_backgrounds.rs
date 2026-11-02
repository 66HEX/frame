#![cfg_attr(target_family = "wasm", no_main)]

#[path = "example_support/fonts.rs"]
mod example_support;

use gpui::{
    App, Background, Bounds, BoxShadow, Context, Div, FontWeight, Render, Window, WindowBounds,
    WindowOptions, div, hsla, linear_color_stop, linear_gradient, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct BackgroundShowcase;

fn ocean_gradient(alpha: f32) -> Background {
    linear_gradient(
        135.,
        linear_color_stop(hsla(0.52, 0.95, 0.62, alpha), 0.),
        linear_color_stop(hsla(0.76, 0.88, 0.64, alpha), 1.),
    )
}

fn sunset_gradient(alpha: f32) -> Background {
    linear_gradient(
        45.,
        linear_color_stop(hsla(0.98, 0.92, 0.64, alpha), 0.),
        linear_color_stop(hsla(0.09, 0.96, 0.65, alpha), 1.),
    )
}

fn panel(title: &'static str, description: &'static str, preview: impl IntoElement) -> Div {
    div()
        .w(px(420.))
        .h(px(250.))
        .p_6()
        .flex()
        .flex_col()
        .gap_4()
        .rounded(px(24.))
        .bg(rgb(0x12182a))
        .border_1()
        .border_color(rgb(0x28324a))
        .child(
            div()
                .h(px(140.))
                .flex()
                .items_center()
                .justify_center()
                .child(preview),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(div().text_sm().text_color(rgb(0x96a3bd)).child(description)),
        )
}

fn preview_card(label: &'static str) -> Div {
    div()
        .w(px(230.))
        .h(px(96.))
        .rounded(px(22.))
        .rounded_smoothing(1.)
        .bg(rgb(0xf8fafc))
        .text_color(rgb(0x172033))
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .child(label)
}

impl Render for BackgroundShowcase {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let gradient_border = preview_card("Gradient border")
            .border_4()
            .border_color(ocean_gradient(1.));

        let gradient_drop_shadow = preview_card("Gradient drop shadow").shadow(vec![
            BoxShadow::new(px(-12.), px(12.), ocean_gradient(0.58))
                .blur_radius(px(22.))
                .spread_radius(px(2.)),
            BoxShadow::new(px(12.), px(12.), sunset_gradient(0.46))
                .blur_radius(px(22.))
                .spread_radius(px(2.)),
        ]);

        let gradient_inset_shadow = preview_card("Gradient inset shadow").shadow(vec![
            BoxShadow::new(px(0.), px(0.), sunset_gradient(0.9))
                .blur_radius(px(12.))
                .spread_radius(px(7.))
                .inset(),
        ]);

        let combined = preview_card("Combined")
            .border_4()
            .border_color(sunset_gradient(1.))
            .shadow(vec![
                BoxShadow::new(px(0.), px(14.), ocean_gradient(0.58))
                    .blur_radius(px(24.))
                    .spread_radius(px(3.)),
                BoxShadow::new(px(0.), px(0.), ocean_gradient(0.7))
                    .blur_radius(px(8.))
                    .spread_radius(px(4.))
                    .inset(),
            ]);

        div()
            .id("border-and-shadow-backgrounds")
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(0x080c18))
            .text_color(rgb(0xf1f5f9))
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Background borders & shadows"),
                    )
                    .child(
                        div().text_color(rgb(0xa7b2c9)).child(
                            "Linear gradients rendered through border and box-shadow paint.",
                        ),
                    ),
            )
            .child(div().flex().flex_wrap().justify_center().gap_6().children([
                panel(
                    "Border background",
                    "A linear gradient follows the full quad coordinate space.",
                    gradient_border,
                ),
                panel(
                    "Drop-shadow backgrounds",
                    "Two translucent gradients overlap outside the card.",
                    gradient_drop_shadow,
                ),
                panel(
                    "Inset-shadow background",
                    "A warm gradient is blurred and clipped inside the card.",
                    gradient_inset_shadow,
                ),
                panel(
                    "Border + drop + inset",
                    "Both background-enabled paths render on one rounded element.",
                    combined,
                ),
            ]))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        if !example_support::load_fonts(cx) {
            return;
        }

        let bounds = Bounds::centered(None, size(px(1000.), px(760.)), cx);
        cx.open_window(
            WindowOptions::new()
                .window_bounds(Some(WindowBounds::Windowed(bounds)))
                .focus(true),
            |_, cx| cx.new(|_| BackgroundShowcase),
        )
        .unwrap();
        cx.activate(true);
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
