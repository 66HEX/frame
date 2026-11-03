use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_ce_elements::editable_text::{
    actions::{DEFAULT_INPUT_CONTEXT, default_bindings},
    text_area, text_input,
};

struct Example;
impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x505050))
            .flex()
            .flex_col()
            .p_2()
            .gap_2()
            .items_start()
            .justify_start()
            .child(
                text_input("input-field")
                    .caret_blink_interval_500ms()
                    .placeholder("some placeholder text")
                    .border_1()
                    .rounded_lg()
                    .border_color(gpui::white()) // has a border
                    .p_2() // padding between the text and border
                    .min_w_10()
                    .max_w_128()
                    .min_h_auto()
                    .max_h_auto()
                    .whitespace_nowrap(),
            )
            .child(
                text_area("text-area")
                    .placeholder("empty text")
                    .border_1()
                    .rounded_lg()
                    .border_color(gpui::white()) // has a border
                    .p_2() // padding between the text and border
                    .w_full()
                    .min_h_24()
                    .max_h_128()
                    .whitespace_normal() // default
                    .overflow_y_scroll(),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys(default_bindings().as_keybindings(Some(DEFAULT_INPUT_CONTEXT)));

        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |_, cx| cx.new(|_| Example),
        )
        .unwrap();
        cx.activate(true);
    });
}
