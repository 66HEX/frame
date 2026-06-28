//! Transition Example
//!
//! This example demonstrates transition capabilities in GPUI via `use_keyed_transition`.

#[path = "../shared/prelude.rs"]
mod example_prelude;

use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, ElementId, Lerp, Rgba, Window, WindowBounds,
    WindowOptions, actions, div, ease_in_out, prelude::*, px, rgb, size,
};
use smallvec::SmallVec;

actions!(app, [Quit]);

#[derive(IntoElement)]
struct Button {
    id: ElementId,
    children: SmallVec<[AnyElement; 2]>,
}

impl Button {
    fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: SmallVec::new(),
        }
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        const HOVER_STRENGTH: f32 = 0.3;
        let base_color: Rgba = rgb(0x663399);

        let hover_transition = window
            .use_keyed_transition(
                (self.id.clone(), "hover"),
                cx,
                Duration::from_millis(300),
                |_window, _cx| 0.,
            )
            .with_easing(ease_in_out);

        let bg_color = base_color.lerp(
            &rgb(0x000),
            *hover_transition.evaluate(window, cx) * HOVER_STRENGTH,
        );

        div()
            .id(self.id)
            .cursor_pointer()
            .rounded(px(100.))
            .pl(px(14.))
            .pr(px(14.))
            .pt(px(10.))
            .pb(px(10.))
            .bg(bg_color)
            .text_color(rgb(0x110F15))
            .children(self.children)
            .on_hover(move |hover, _window, cx| {
                hover_transition.update(cx, |this, cx| {
                    *this = *hover as u8 as f32;
                    cx.notify();
                });
            })
    }
}

struct TransitionExample;

impl Render for TransitionExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .absolute()
            .bg(rgb(0x110F15))
            .gap(px(20.))
            .p(px(100.))
            .child(Button::new("btn").child("Click me!"))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(650.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| TransitionExample),
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Transition");
    });
}
