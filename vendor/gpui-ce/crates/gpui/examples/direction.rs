use gpui::{
    App, Bounds, Context, LayoutDirection, Window, WindowBounds, WindowOptions, div, prelude::*,
    px, rgb, size,
};
use gpui_ce_elements::editable_text::{
    actions::{DEFAULT_INPUT_CONTEXT, default_bindings},
    text_input,
};

struct DirectionExample {
    direction: LayoutDirection,
}

impl DirectionExample {
    fn direction_button(
        &self,
        id: &'static str,
        label: &'static str,
        direction: LayoutDirection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.direction == direction;

        div()
            .id(id)
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .bg(if selected {
                rgb(0x2563eb)
            } else {
                rgb(0x334155)
            })
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.direction = direction;
                cx.notify();
            }))
    }
}

impl Render for DirectionExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(rgb(0x0f172a))
            .text_color(rgb(0xf8fafc))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.direction_button(
                        "ltr",
                        "Left to right",
                        LayoutDirection::LeftToRight,
                        cx,
                    ))
                    .child(self.direction_button(
                        "rtl",
                        "Right to left",
                        LayoutDirection::RightToLeft,
                        cx,
                    ))
                    .child(self.direction_button("auto", "Auto", LayoutDirection::Auto, cx)),
            )
            .child(
                div()
                    .direction(self.direction)
                    .text_start()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0x475569))
                    .child("مرحبا GPUI · Direction-aware text · 12345")
                    .child(
                        div()
                            .p_2()
                            .bg(rgb(0x1e293b))
                            .child("This row inherits the selected direction."),
                    )
                    .child(
                        div()
                            .ltr()
                            .text_start()
                            .p_2()
                            .bg(rgb(0x1e293b))
                            .child("Nested LTR override: English אבג العربية"),
                    )
                    .child(
                        div()
                            .rtl()
                            .text_start()
                            .p_2()
                            .bg(rgb(0x1e293b))
                            .child("تجاوز RTL متداخل: English 123"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_start()
                            .gap_2()
                            .child(div().p_2().bg(rgb(0x7c3aed)).child("First"))
                            .child(div().p_2().bg(rgb(0x0f766e)).child("Second"))
                            .child(div().p_2().bg(rgb(0xbe123c)).child("Third")),
                    )
                    .child(
                        text_input("direction-input")
                            .direction(self.direction)
                            .text_start()
                            .placeholder("Type text")
                            .border_1()
                            .border_color(rgb(0x94a3b8))
                            .rounded_md()
                            .p_2()
                            .w_full()
                            .whitespace_nowrap(),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys(default_bindings().as_keybindings(Some(DEFAULT_INPUT_CONTEXT)));

        let bounds = Bounds::centered(None, size(px(760.0), px(620.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| DirectionExample {
                    direction: LayoutDirection::Auto,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
