use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_ce_elements::switch::{BaseSwitch, BaseSwitchThumb};

struct SwitchExample {
    notifications: bool,
    automatic_updates: bool,
}

impl Render for SwitchExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .tab_group()
            .flex()
            .flex_col()
            .gap_6()
            .p_8()
            .bg(rgb(0x111827))
            .text_color(rgb(0xf9fafb))
            .child(div().text_xl().child("Switch"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9ca3af))
                    .child("Click or drag a switch. Tab to focus, then press Space or Enter."),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(switch_row(
                        "Notifications",
                        self.notifications,
                        styled_switch("notifications")
                            .checked(self.notifications)
                            .on_change(cx.listener(|this, checked, _window, cx| {
                                this.notifications = *checked;
                                cx.notify();
                            })),
                    ))
                    .child(switch_row(
                        "Automatic updates",
                        self.automatic_updates,
                        styled_switch("automatic-updates")
                            .checked(self.automatic_updates)
                            .on_change(cx.listener(|this, checked, _window, cx| {
                                this.automatic_updates = *checked;
                                cx.notify();
                            })),
                    ))
                    .child(switch_row(
                        "Disabled, off",
                        false,
                        styled_switch("disabled-off").disabled(true),
                    ))
                    .child(switch_row(
                        "Disabled, on",
                        true,
                        styled_switch("disabled-on").checked(true).disabled(true),
                    )),
            )
    }
}

fn styled_switch(element_id: &'static str) -> BaseSwitch {
    BaseSwitch::new(element_id)
        .flex()
        .flex_none()
        .items_center()
        .w(px(56.))
        .h(px(32.))
        .p(px(2.))
        .rounded_full()
        .border_2()
        .border_color(rgb(0x111827))
        .focus_style(|style| style.border_color(rgb(0x93c5fd)))
        .disabled_style(|style| style.opacity(0.4))
        .with_state(|base, checked, disabled| {
            base.bg(rgb(if checked { 0x2563eb } else { 0x4b5563 }))
                .when(checked, |base| base.justify_end())
                .when(!disabled, |base| base.cursor_pointer())
                .child(
                    BaseSwitchThumb::new("thumb")
                        .size(px(24.))
                        .flex_none()
                        .rounded_full()
                        .bg(rgb(0xffffff)),
                )
        })
}

fn switch_row(label: &'static str, checked: bool, switch: BaseSwitch) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(div().child(label))
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .w(px(24.))
                        .text_sm()
                        .text_color(rgb(0x9ca3af))
                        .child(if checked { "On" } else { "Off" }),
                )
                .child(switch.aria_label(label)),
        )
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(480.), px(360.)), cx);

        cx.open_window(
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |_window, cx| {
                cx.new(|_cx| SwitchExample {
                    notifications: false,
                    automatic_updates: true,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
