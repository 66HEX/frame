//! Transition Example
//!
//! This example demonstrates declarative style transitions in GPUI.

#[path = "../shared/prelude.rs"]
mod example_prelude;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, ElementId, Lerp, MotionDurationExt, Rgba, Window,
    WindowBounds, WindowOptions, actions, div, ease_in_out, millis, prelude::*, px, rgb, size,
    spring,
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        const HOVER_STRENGTH: f32 = 0.2;
        const ACTIVE_STRENGTH: f32 = 0.3;

        let base_color: Rgba = rgb(0x663399);
        let hover_color = base_color.lerp(&rgb(0x000), HOVER_STRENGTH);
        let active_color = base_color.lerp(&rgb(0x000), ACTIVE_STRENGTH);

        div()
            .id(self.id)
            .cursor_pointer()
            .rounded(px(99999.))
            .pl(px(14.))
            .pr(px(14.))
            .pt(px(10.))
            .pb(px(10.))
            .bg(base_color)
            .text_color(rgb(0x110F15))
            .children(self.children)
            .transitions(|transitions| {
                transitions
                    .bg(millis(200).with_easing(ease_in_out))
                    .p(spring(600.0, 22.0, 1.0))
            })
            .hover_style(|refinement| refinement.bg(hover_color).p(px(50.)))
            .active_style(|refinement| refinement.bg(active_color))
    }
}

struct StyleTransitionsExample;

impl Render for StyleTransitionsExample {
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
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |_, cx| cx.new(|_| StyleTransitionsExample),
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Transition");
    });
}
