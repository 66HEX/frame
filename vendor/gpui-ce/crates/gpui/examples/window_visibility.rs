//! Run with `cargo run -p gpui-ce --example window_visibility`.
//! Pass `-- --smoke` for an automatic hide/show cycle followed by exit.

use gpui::{
    App, Bounds, Context, TitlebarOptions, Window, WindowBounds, WindowOptions, div, prelude::*,
    px, rgb, size,
};
use gpui_platform::application;
use std::time::Duration;

struct VisibilityExample {
    restores: usize,
}

impl Render for VisibilityExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .bg(rgb(0x18181b))
            .text_color(rgb(0xf4f4f5))
            .child(div().text_2xl().child("Window visibility test"))
            .child("Hide this window without closing it. It returns after three seconds.")
            .child(format!("Successful restores: {}", self.restores))
            .child(
                div()
                    .id("hide")
                    .p_4()
                    .bg(rgb(0x2563eb))
                    .rounded_md()
                    .cursor_pointer()
                    .child("Hide for 3 seconds")
                    .on_click(cx.listener(|_, _, window, cx| {
                        let handle = window.window_handle();
                        window.set_visible(false);
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_secs(3)).await;
                            this.update(cx, |this, cx| {
                                this.restores += 1;
                                cx.notify();
                            })
                            .ok();
                            handle
                                .update(cx, |_, window, _| window.set_visible(true))
                                .ok();
                        })
                        .detach();
                    })),
            )
    }
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(520.), px(280.)), cx);
        let handle = cx
            .open_window(
                WindowOptions::new()
                    .show(false)
                    .focus(false)
                    .app_id(Some("dev.gpui.visibility-test".into()))
                    .titlebar(Some(TitlebarOptions {
                        title: Some("GPUI visibility test".into()),
                        ..Default::default()
                    }))
                    .window_bounds(Some(WindowBounds::Windowed(bounds))),
                |_, cx| cx.new(|_| VisibilityExample { restores: 0 }),
            )
            .unwrap();
        eprintln!("VISIBILITY hidden initially");
        cx.spawn(async move |cx| {
            let states: &[bool] = if smoke {
                &[true, false, true, false, true]
            } else {
                &[true]
            };
            for &visible in states {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                handle
                    .update(cx, |this, window, cx| {
                        // Repeated calls should be harmless.
                        window.set_visible(visible);
                        window.set_visible(visible);
                        if visible {
                            this.restores += 1;
                            cx.notify();
                        }
                    })
                    .unwrap();
                eprintln!("VISIBILITY {}", if visible { "shown" } else { "hidden" });
            }
            if smoke {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                cx.update(|cx| cx.quit());
            }
        })
        .detach();
    });
}
