//! Creating Components Example
//!
//! This example demonstrates three different approaches to creating interactive
//! stateful components in GPUI:
//!
//! 1. `use_state` - Hook-like state scoped to an element's lifetime
//! 2. `RenderOnce` - Stateless component that receives state from parent
//! 3. `Render` - Entity-backed view with persistent internal state

#[path = "../shared/prelude.rs"]
mod example_prelude;

use example_prelude::init_example;
use gpui::colors::Colors;
use gpui::{
    App, Bounds, Context, Entity, IntoElement, Render, RenderOnce, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
};

// ============================================================================
// Approach 1: use_state
// ============================================================================
//
// `use_state` creates element-scoped state that persists across renders.
// It's similar to React's useState hook. The state is automatically tied
// to the element's identity via caller location or a provided key.
//
// Pros:
// - Simple, hook-like API
// - State is scoped to element lifetime
// - No boilerplate for simple state
//
// Cons:
// - Less explicit than Entity-backed state
// - State is tied to call site location

struct UseStateCounter {
    count: i32,
}

fn use_state_counter(colors: &Colors, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let state: Entity<UseStateCounter> =
        window.use_state(cx, |_window, _cx| UseStateCounter { count: 0 });

    let count = state.read(cx).count;

    let error = rgb(0xd32f2f);
    let error_hover = rgb(0xe04545);
    let success = rgb(0x388e3c);
    let success_hover = rgb(0x43a047);

    div()
        .id("use-state-counter")
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded_lg()
        .bg(colors.container)
        .child(
            div()
                .text_sm()
                .text_color(colors.disabled)
                .child("use_state Counter"),
        )
        .child(
            div()
                .text_2xl()
                .text_color(colors.text)
                .child(format!("{}", count)),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .id("use-state-decrement")
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .bg(error)
                        .text_color(colors.selected_text)
                        .cursor_pointer()
                        .hover(move |style| style.bg(error_hover))
                        .child("−")
                        .on_click({
                            let state = state.clone();
                            move |_, _, cx| {
                                state.update(cx, |state, cx| {
                                    state.count -= 1;
                                    cx.notify();
                                });
                            }
                        }),
                )
                .child(
                    div()
                        .id("use-state-increment")
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .bg(success)
                        .text_color(colors.selected_text)
                        .cursor_pointer()
                        .hover(move |style| style.bg(success_hover))
                        .child("+")
                        .on_click(move |_, _, cx| {
                            state.update(cx, |state, cx| {
                                state.count += 1;
                                cx.notify();
                            });
                        }),
                ),
        )
}

// ============================================================================
// Approach 2: RenderOnce
// ============================================================================
//
// `RenderOnce` components are stateless and consumed when rendered.
// They receive all data as props and delegate state management to the parent.
// This is the recommended approach for presentational components.
//
// Pros:
// - Clear data flow (props down, events up)
// - Lightweight (no Entity allocation)
// - Easy to test
// - Highly composable
//
// Cons:
// - Cannot maintain internal state
// - Parent must manage all state

#[derive(IntoElement)]
struct RenderOnceCounter {
    colors: Colors,
    count: i32,
    on_increment: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_decrement: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl RenderOnceCounter {
    fn new(colors: Colors, count: i32) -> Self {
        Self {
            colors,
            count,
            on_increment: None,
            on_decrement: None,
        }
    }

    fn on_increment(mut self, callback: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_increment = Some(Box::new(callback));
        self
    }

    fn on_decrement(mut self, callback: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_decrement = Some(Box::new(callback));
        self
    }
}

impl RenderOnce for RenderOnceCounter {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let colors = self.colors;
        let error = rgb(0xd32f2f);
        let error_hover = rgb(0xe04545);
        let success = rgb(0x388e3c);
        let success_hover = rgb(0x43a047);

        div()
            .id("render-once-counter")
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(colors.container)
            .child(
                div()
                    .text_sm()
                    .text_color(colors.disabled)
                    .child("RenderOnce Counter"),
            )
            .child(
                div()
                    .text_2xl()
                    .text_color(colors.text)
                    .child(format!("{}", self.count)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("render-once-decrement")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(error)
                            .text_color(colors.selected_text)
                            .cursor_pointer()
                            .hover(move |style| style.bg(error_hover))
                            .child("−")
                            .when_some(self.on_decrement, |element, callback| {
                                element.on_click(move |_, window, cx| callback(window, cx))
                            }),
                    )
                    .child(
                        div()
                            .id("render-once-increment")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(success)
                            .text_color(colors.selected_text)
                            .cursor_pointer()
                            .hover(move |style| style.bg(success_hover))
                            .child("+")
                            .when_some(self.on_increment, |element, callback| {
                                element.on_click(move |_, window, cx| callback(window, cx))
                            }),
                    ),
            )
    }
}

// ============================================================================
// Approach 3: Render (Entity-backed)
// ============================================================================
//
// `Render` components are backed by an `Entity<T>` and maintain their own
// internal state. This is the recommended approach for complex components
// that need to manage their own state, subscribe to events, or spawn tasks.
//
// Pros:
// - Full control over internal state
// - Can subscribe to events and observe other entities
// - Can spawn async tasks
// - Has identity (can be passed around as Entity<T>)
//
// Cons:
// - More boilerplate
// - Higher memory overhead
// - More complex lifecycle

struct RenderCounter {
    count: i32,
}

impl RenderCounter {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn increment(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }

    fn decrement(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.count -= 1;
        cx.notify();
    }
}

impl Render for RenderCounter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let error = rgb(0xd32f2f);
        let error_hover = rgb(0xe04545);
        let success = rgb(0x388e3c);
        let success_hover = rgb(0x43a047);

        div()
            .id("render-counter")
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(colors.container)
            .child(
                div()
                    .text_sm()
                    .text_color(colors.disabled)
                    .child("Render Counter"),
            )
            .child(
                div()
                    .text_2xl()
                    .text_color(colors.text)
                    .child(format!("{}", self.count)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("render-decrement")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(error)
                            .text_color(colors.selected_text)
                            .cursor_pointer()
                            .hover(move |style| style.bg(error_hover))
                            .child("−")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.decrement(window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("render-increment")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(success)
                            .text_color(colors.selected_text)
                            .cursor_pointer()
                            .hover(move |style| style.bg(success_hover))
                            .child("+")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.increment(window, cx);
                            })),
                    ),
            )
    }
}

// ============================================================================
// Main Application
// ============================================================================

struct CreatingComponentsExample {
    render_counter: Entity<RenderCounter>,
    render_once_count: i32,
}

impl CreatingComponentsExample {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            render_counter: cx.new(|_| RenderCounter::new()),
            render_once_count: 0,
        }
    }
}

impl Render for CreatingComponentsExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let render_once_count = self.render_once_count;
        let handle = cx.entity().downgrade();

        div()
            .id("main")
            .size_full()
            .flex()
            .flex_col()
            .gap_6()
            .p_8()
            .bg(colors.background)
            .overflow_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(colors.text)
                            .child("Creating Components"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.disabled)
                            .child("Three approaches to stateful components in GPUI"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(use_state_counter(&colors, window, cx))
                    .child(
                        RenderOnceCounter::new(colors.clone(), render_once_count)
                            .on_increment({
                                let handle = handle.clone();
                                move |_window, cx| {
                                    handle
                                        .update(cx, |this, cx| {
                                            this.render_once_count += 1;
                                            cx.notify();
                                        })
                                        .ok();
                                }
                            })
                            .on_decrement(move |_window, cx| {
                                handle
                                    .update(cx, |this, cx| {
                                        this.render_once_count -= 1;
                                        cx.notify();
                                    })
                                    .ok();
                            }),
                    )
                    .child(self.render_counter.clone()),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(700.), px(400.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| CreatingComponentsExample::new(cx)),
        )
        .expect("Failed to open window");

        init_example(cx, "Creating Components");
    });
}
