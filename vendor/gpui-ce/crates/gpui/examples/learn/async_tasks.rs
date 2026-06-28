//! Async Tasks Example
//!
//! This example demonstrates different async patterns in GPUI:
//!
//! 1. `cx.spawn` - Foreground tasks for UI updates
//! 2. `cx.background_spawn` - Background tasks for heavy computation
//! 3. Task management - Storing, canceling, and detaching tasks
//! 4. Progress updates - Communicating from background to UI

#[path = "../shared/prelude.rs"]
mod example_prelude;

use std::time::Duration;

use gpui::colors::Colors;
use gpui::{
    App, Bounds, Context, Entity, Render, Task, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size,
};

// Example 1: Simple Foreground Task
//
// `cx.spawn` runs an async closure on the foreground thread.
// Use it when you need to perform async work that updates UI.

struct ForegroundTaskDemo {
    message: String,
    is_loading: bool,
}

impl ForegroundTaskDemo {
    fn new() -> Self {
        Self {
            message: "Click to start a foreground task".into(),
            is_loading: false,
        }
    }

    fn start_task(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;
        self.message = "Loading...".into();
        cx.notify();

        cx.spawn(async move |this, cx| {
            cx.background_spawn(async {
                std::thread::sleep(Duration::from_secs(1));
            })
            .await;

            this.update(cx, |this, cx| {
                this.message = "Task completed!".into();
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

// Example 2: Background Task with Progress
//
// `cx.background_spawn` runs work off the UI thread.
// Use it for heavy computation that shouldn't block the UI.

struct BackgroundTaskDemo {
    progress: u32,
    result: Option<u64>,
    is_computing: bool,
}

impl BackgroundTaskDemo {
    fn new() -> Self {
        Self {
            progress: 0,
            result: None,
            is_computing: false,
        }
    }

    fn start_computation(&mut self, cx: &mut Context<Self>) {
        self.is_computing = true;
        self.progress = 0;
        self.result = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            for i in 0..100 {
                let computation = cx.background_spawn(async move {
                    std::thread::sleep(Duration::from_millis(10));
                    (i + 1) as u64
                });

                let partial_result = computation.await;

                this.update(cx, |this, cx| {
                    this.progress = i as u32 + 1;
                    this.result = Some(partial_result);
                    cx.notify();
                })
                .ok();
            }

            this.update(cx, |this, cx| {
                this.is_computing = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

// Example 3: Cancellable Task
//
// Tasks can be cancelled by dropping them.
// Store a task in a field to keep it running.

struct CancellableTaskDemo {
    counter: u32,
    counting_task: Option<Task<()>>,
}

impl CancellableTaskDemo {
    fn new() -> Self {
        Self {
            counter: 0,
            counting_task: None,
        }
    }

    fn is_running(&self) -> bool {
        self.counting_task.is_some()
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.counting_task.is_some() {
            self.counting_task = None;
            cx.notify();
        } else {
            self.counting_task = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_spawn(async {
                        std::thread::sleep(Duration::from_millis(100));
                    })
                    .await;

                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.counter += 1;
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);

                    if !should_continue {
                        break;
                    }
                }
            }));
            cx.notify();
        }
    }
}

// Example 4: Task with Return Value
//
// Tasks can return values that you can await.

struct ReturnValueDemo {
    numbers: Vec<i32>,
    sum: Option<i32>,
    is_calculating: bool,
}

impl ReturnValueDemo {
    fn new() -> Self {
        Self {
            numbers: vec![1, 2, 3, 4, 5],
            sum: None,
            is_calculating: false,
        }
    }

    fn calculate_sum(&mut self, cx: &mut Context<Self>) {
        self.is_calculating = true;
        cx.notify();

        let numbers = self.numbers.clone();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    std::thread::sleep(Duration::from_millis(500));
                    numbers.iter().sum::<i32>()
                })
                .await;

            this.update(cx, |this, cx| {
                this.sum = Some(result);
                this.is_calculating = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn randomize(&mut self, cx: &mut Context<Self>) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let hasher = RandomState::new().build_hasher().finish();
        self.numbers = (0..5)
            .map(|i| ((hasher >> (i * 8)) & 0xFF) as i32 % 100)
            .collect();
        self.sum = None;
        cx.notify();
    }
}

// Main Application

struct AsyncTasksExample {
    foreground_demo: Entity<ForegroundTaskDemo>,
    background_demo: Entity<BackgroundTaskDemo>,
    cancellable_demo: Entity<CancellableTaskDemo>,
    return_demo: Entity<ReturnValueDemo>,
}

impl AsyncTasksExample {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            foreground_demo: cx.new(|_| ForegroundTaskDemo::new()),
            background_demo: cx.new(|_| BackgroundTaskDemo::new()),
            cancellable_demo: cx.new(|_| CancellableTaskDemo::new()),
            return_demo: cx.new(|_| ReturnValueDemo::new()),
        }
    }
}

impl Render for AsyncTasksExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let foreground = self.foreground_demo.read(cx);
        let background = self.background_demo.read(cx);
        let cancellable = self.cancellable_demo.read(cx);
        let return_demo = self.return_demo.read(cx);

        div()
            .id("main")
            .size_full()
            .p_6()
            .bg(colors.background)
            .overflow_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .max_w(px(500.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child("Async Tasks"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.disabled)
                                    .child("Spawning, background work, and task management"),
                            ),
                    )
                    .child(demo_section(
                        &colors,
                        "1. Foreground Task (cx.spawn)",
                        "Runs async work on the UI thread. Good for sequential async operations.",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text)
                                    .child(foreground.message.clone()),
                            )
                            .child(
                                button(&colors, "foreground-btn", "Start Task", foreground.is_loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.foreground_demo.update(cx, |demo, cx| {
                                            demo.start_task(cx);
                                        });
                                    })),
                            ),
                    ))
                    .child(demo_section(
                        &colors,
                        "2. Background Task (cx.background_spawn)",
                        "Runs heavy computation off the UI thread with progress updates.",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(progress_bar(&colors, background.progress))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text)
                                    .child(format!(
                                        "Progress: {}% | Result: {}",
                                        background.progress,
                                        background
                                            .result
                                            .map(|r| r.to_string())
                                            .unwrap_or_else(|| "-".into())
                                    )),
                            )
                            .child(
                                button(&colors, "background-btn", "Compute", background.is_computing)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.background_demo.update(cx, |demo, cx| {
                                            demo.start_computation(cx);
                                        });
                                    })),
                            ),
                    ))
                    .child(demo_section(
                        &colors,
                        "3. Cancellable Task",
                        "Store Task in a field to keep it running. Drop to cancel.",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child(format!("{}", cancellable.counter)),
                            )
                            .child({
                                let is_running = cancellable.is_running();
                                let (bg, bg_hover) = if is_running {
                                    (rgb(0xd32f2f), rgb(0xe04545))
                                } else {
                                    (rgb(0x388e3c), rgb(0x43a047))
                                };
                                div()
                                    .id("cancel-btn")
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .text_sm()
                                    .text_color(colors.text)
                                    .cursor_pointer()
                                    .bg(bg)
                                    .hover(move |style| style.bg(bg_hover))
                                    .child(if is_running { "Stop" } else { "Start Counter" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancellable_demo.update(cx, |demo, cx| {
                                            demo.toggle(cx);
                                        });
                                    }))
                            }),
                    ))
                    .child(demo_section(
                        &colors,
                        "4. Task with Return Value",
                        "Tasks can return values that can be awaited or used in chained operations.",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.disabled)
                                    .child(format!("Numbers: {:?}", return_demo.numbers)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text)
                                    .child(format!(
                                        "Sum: {}",
                                        if return_demo.is_calculating {
                                            "Calculating...".into()
                                        } else {
                                            return_demo
                                                .sum
                                                .map(|s| s.to_string())
                                                .unwrap_or_else(|| "Not calculated".into())
                                        }
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        button(
                                            &colors,
                                            "sum-btn",
                                            "Calculate Sum",
                                            return_demo.is_calculating,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.return_demo.update(cx, |demo, cx| {
                                                demo.calculate_sum(cx);
                                            });
                                        })),
                                    )
                                    .child(
                                        secondary_button(&colors, "random-btn", "Randomize")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.return_demo.update(cx, |demo, cx| {
                                                    demo.randomize(cx);
                                                });
                                            })),
                                    ),
                            ),
                    ))

            )
    }
}

// Helper Components

fn demo_section(
    colors: &Colors,
    title: &'static str,
    description: &'static str,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_lg()
        .bg(colors.container)
        .border_1()
        .border_color(colors.border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(colors.text)
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(colors.disabled)
                        .child(description),
                ),
        )
        .child(content)
}

fn button(
    colors: &Colors,
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    disabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let disabled_bg = colors.selected;
    let bg = colors.selected;
    let bg_hover = colors.selected;
    let bg_active = colors.selected;
    let text = colors.selected_text;

    div()
        .id(id)
        .px_3()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .text_color(text)
        .when(disabled, |el| {
            el.bg(disabled_bg).cursor_not_allowed().opacity(0.6)
        })
        .when(!disabled, |el| {
            el.bg(bg)
                .cursor_pointer()
                .hover(move |style| style.bg(bg_hover))
                .active(move |style| style.bg(bg_active))
        })
        .child(label)
}

fn secondary_button(
    colors: &Colors,
    id: impl Into<gpui::ElementId>,
    label: &'static str,
) -> gpui::Stateful<gpui::Div> {
    let bg = colors.selected;
    let bg_hover = colors.border;
    let text = colors.text;

    div()
        .id(id)
        .px_3()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .text_color(text)
        .bg(bg)
        .cursor_pointer()
        .hover(move |style| style.bg(bg_hover))
        .child(label)
}

fn progress_bar(colors: &Colors, progress: u32) -> impl IntoElement {
    let clamped = progress.min(100);
    let bar_bg = colors.selected;
    let bar_fill = rgb(0x388e3c);

    div()
        .h_2()
        .w_full()
        .rounded_full()
        .bg(bar_bg)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .rounded_full()
                .bg(bar_fill)
                .w(gpui::relative(clamped as f32 / 100.0)),
        )
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(550.), px(850.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| AsyncTasksExample::new(cx)),
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Async Tasks");
    });
}
