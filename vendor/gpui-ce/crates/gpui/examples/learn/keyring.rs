//! Keyring Example
//!
//! This example demonstrates the platform credentials API:
//!
//! 1. `cx.write_credentials` - store a username/password in the system keyring
//! 2. `cx.read_credentials` - read them back
//! 3. `cx.delete_credentials` - remove them
//!
//! On Linux/FreeBSD, every stored item is tagged with a keyring *label*. It
//! defaults to `"gpui-ce"`, but consumers can override it with
//! `cx.set_keyring_label(..)` so the items show up under their own app's name.

#[path = "../shared/prelude.rs"]
mod example_prelude;

use gpui::colors::Colors;
use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};

/// The URL the credentials are keyed by. The keyring label (Linux/FreeBSD) is a
/// separate, app-wide identifier set via `cx.set_keyring_label`.
const CREDENTIAL_URL: &str = "https://example.com/keyring-demo";

struct KeyringExample {
    status: String,
}

impl KeyringExample {
    fn new() -> Self {
        Self {
            status: "Use the buttons to store, load, and delete credentials.".into(),
        }
    }

    fn set_status(&mut self, status: impl Into<String>, cx: &mut Context<Self>) {
        self.status = status.into();
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        self.set_status("Saving...", cx);
        cx.spawn(async move |this, cx| {
            let task =
                cx.update(|cx| cx.write_credentials(CREDENTIAL_URL, "ada@example.com", b"hunter2"));
            let result = task.await;
            this.update(cx, |this, cx| match result {
                Ok(()) => this.set_status("Saved credentials for ada@example.com.", cx),
                Err(err) => this.set_status(format!("Failed to save: {err}"), cx),
            })
        })
        .detach();
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        self.set_status("Loading...", cx);
        cx.spawn(async move |this, cx| {
            let task = cx.update(|cx| cx.read_credentials(CREDENTIAL_URL));
            let result = task.await;
            this.update(cx, |this, cx| match result {
                Ok(Some((username, password))) => this.set_status(
                    format!("Loaded {username} (password is {} bytes).", password.len()),
                    cx,
                ),
                Ok(None) => this.set_status("No credentials stored yet.", cx),
                Err(err) => this.set_status(format!("Failed to load: {err}"), cx),
            })
        })
        .detach();
    }

    fn delete(&mut self, cx: &mut Context<Self>) {
        self.set_status("Deleting...", cx);
        cx.spawn(async move |this, cx| {
            let task = cx.update(|cx| cx.delete_credentials(CREDENTIAL_URL));
            let result = task.await;
            this.update(cx, |this, cx| match result {
                Ok(()) => this.set_status("Deleted stored credentials.", cx),
                Err(err) => this.set_status(format!("Failed to delete: {err}"), cx),
            })
        })
        .detach();
    }
}

impl Render for KeyringExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

        div().size_full().p_6().bg(colors.background).child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .max_w(px(460.))
                .child(
                    div()
                        .text_xl()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(colors.text)
                        .child("Keyring Credentials"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.disabled)
                        .child(format!("Stored under: {CREDENTIAL_URL}")),
                )
                .child(
                    div()
                        .p_4()
                        .rounded_lg()
                        .bg(colors.container)
                        .border_1()
                        .border_color(colors.border)
                        .text_sm()
                        .text_color(colors.text)
                        .child(self.status.clone()),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button(&colors, "save", "Save")
                                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                        )
                        .child(
                            button(&colors, "load", "Load")
                                .on_click(cx.listener(|this, _, _, cx| this.load(cx))),
                        )
                        .child(
                            button(&colors, "delete", "Delete")
                                .on_click(cx.listener(|this, _, _, cx| this.delete(cx))),
                        ),
                ),
        )
    }
}

fn button(
    colors: &Colors,
    id: impl Into<gpui::ElementId>,
    label: &'static str,
) -> gpui::Stateful<gpui::Div> {
    let bg_hover = colors.border;
    div()
        .id(id)
        .px_3()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .text_color(colors.selected_text)
        .bg(colors.selected)
        .cursor_pointer()
        .hover(move |style| style.bg(bg_hover))
        .child(label)
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.set_keyring_label("gpui-ce-keyring-example");

        let bounds = Bounds::centered(None, size(px(500.), px(360.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| KeyringExample::new()),
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Keyring");
    });
}
