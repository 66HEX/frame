//! Transition Example
//!
//! This example uses a keyed transition to bounce a button upward.
//! Each click resets the transition before starting the same bounce again.

#[path = "../shared/prelude.rs"]
mod example_prelude;

use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, ElementId, Pixels, Point, Window, WindowBounds,
    WindowOptions, actions, bounce, div, ease_in_out, point, prelude::*, px, rgb, size,
};
use smallvec::SmallVec;

actions!(app, [Quit]);

const BUTTON_WIDTH: f32 = 120.;
const BUTTON_HEIGHT: f32 = 44.;
const BOUNCE_HEIGHT: f32 = 120.;
const BOUNCE_DURATION: Duration = Duration::from_millis(1200);

fn centered_position(window: &Window) -> Point<Pixels> {
    let viewport = window.viewport_size();

    point(
        px(((f32::from(viewport.width) - BUTTON_WIDTH) / 2.).max(0.)),
        px(((f32::from(viewport.height) - BUTTON_HEIGHT) / 2.).max(0.)),
    )
}

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
        let bounce_transition = window
            .use_keyed_transition(
                (self.id.clone(), "bounce"),
                cx,
                BOUNCE_DURATION,
                |_window, _cx| 0.,
            )
            .with_easing(bounce(ease_in_out));
        let progress = *bounce_transition.evaluate(window, cx);
        let resting_position = centered_position(window);

        div()
            .id(self.id)
            .absolute()
            .left(resting_position.x)
            .top(resting_position.y - px(BOUNCE_HEIGHT * progress))
            .w(px(BUTTON_WIDTH))
            .h(px(BUTTON_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .rounded(px(100.))
            .bg(rgb(0x663399))
            .text_color(rgb(0xffffff))
            .children(self.children)
            .on_click(move |_, _window, cx| {
                bounce_transition.reset(cx);
                bounce_transition.update(cx, |progress, cx| {
                    *progress = 1.;
                    cx.notify();
                });
            })
    }
}

struct TransitionExample;

impl Render for TransitionExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x110F15))
            .child(Button::new("btn").child("Bounce!"))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(650.)), cx);
        cx.open_window(
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |_, cx| cx.new(|_| TransitionExample),
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Transition");
    });
}
