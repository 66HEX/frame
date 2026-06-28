//! Interactive Elements Example
//!
//! This example demonstrates interactive patterns in GPUI:
//!
//! 1. Click events - single click, double click, click count
//! 2. Hover states - hover styling and on_hover callbacks
//! 3. Mouse events - mouse down, mouse up, mouse move
//! 4. Drag and drop - draggable elements and drop targets

#[path = "../shared/prelude.rs"]
mod example_prelude;

use example_prelude::init_example;
use gpui::colors::Colors;
use gpui::{
    App, Bounds, ClickEvent, Context, Entity, Half, Hsla, IntoElement, MouseButton, MouseMoveEvent,
    Pixels, Point, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};

// ============================================================================
// Click Events Demo
// ============================================================================
//
// Demonstrates different click interactions:
// - Single click
// - Double click (click_count == 2)
// - Click count tracking

struct ClickDemo {
    click_count: usize,
    last_click_type: String,
}

impl ClickDemo {
    fn new() -> Self {
        Self {
            click_count: 0,
            last_click_type: "None".to_string(),
        }
    }
}

impl Render for ClickDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .bg(colors.container)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child("Click Events"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.disabled)
                    .child("Single click, double click, or triple click the button"),
            )
            .child(
                div()
                    .id("click-target")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(colors.selected)
                    .text_color(colors.selected_text)
                    .text_sm()
                    .cursor_pointer()
                    .hover(|style| style.bg(colors.selected))
                    .active(|style| style.bg(colors.selected))
                    .child("Click Me!")
                    // on_click receives a ClickEvent with click_count() method
                    .on_click(cx.listener(|this, event: &ClickEvent, _window, cx| {
                        this.click_count += 1;
                        this.last_click_type = match event.click_count() {
                            1 => "Single Click".to_string(),
                            2 => "Double Click".to_string(),
                            3 => "Triple Click".to_string(),
                            n => format!("{n}x Click"),
                        };
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .mt_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.disabled)
                            .child(format!("Total clicks: {}", self.click_count)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.disabled)
                            .child(format!("Last: {}", self.last_click_type)),
                    ),
            )
    }
}

// ============================================================================
// Hover Demo
// ============================================================================
//
// Demonstrates hover interactions:
// - hover() style modifier for CSS-like hover states
// - on_hover() callback for programmatic hover detection

struct HoverDemo {
    is_hovered: bool,
    hover_count: usize,
}

impl HoverDemo {
    fn new() -> Self {
        Self {
            is_hovered: false,
            hover_count: 0,
        }
    }
}

impl Render for HoverDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let is_hovered = self.is_hovered;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .bg(colors.container)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child("Hover Events"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.disabled)
                    .child("Move your mouse in and out of the target"),
            )
            .child(
                div()
                    .id("hover-target")
                    .px_4()
                    .py_3()
                    .rounded_md()
                    .border_2()
                    .border_color(if is_hovered {
                        colors.selected
                    } else {
                        colors.border
                    })
                    .bg(if is_hovered {
                        colors.selected
                    } else {
                        colors.selected
                    })
                    .text_color(if is_hovered {
                        colors.selected_text
                    } else {
                        colors.text
                    })
                    .text_sm()
                    .cursor_pointer()
                    .child(if is_hovered {
                        "Mouse Inside!"
                    } else {
                        "Hover Over Me"
                    })
                    // on_hover callback receives a bool: true when mouse enters, false when it leaves
                    .on_hover(cx.listener(|this, &hovered, _window, cx| {
                        this.is_hovered = hovered;
                        if hovered {
                            this.hover_count += 1;
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.disabled)
                    .mt_2()
                    .child(format!("Times hovered: {}", self.hover_count)),
            )
    }
}

// ============================================================================
// Mouse Events Demo
// ============================================================================
//
// Demonstrates low-level mouse events:
// - on_mouse_down - fires when mouse button is pressed
// - on_mouse_up - fires when mouse button is released
// - on_mouse_move - fires when mouse moves over element

struct MouseEventsDemo {
    mouse_position: Option<Point<Pixels>>,
    is_pressed: bool,
    event_log: Vec<String>,
}

impl MouseEventsDemo {
    fn new() -> Self {
        Self {
            mouse_position: None,
            is_pressed: false,
            event_log: Vec::new(),
        }
    }

    fn log_event(&mut self, event: &str) {
        self.event_log.push(event.to_string());
        if self.event_log.len() > 5 {
            self.event_log.remove(0);
        }
    }
}

impl Render for MouseEventsDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let is_pressed = self.is_pressed;
        let position_text = self
            .mouse_position
            .map(|p| format!("({:.0}, {:.0})", f32::from(p.x), f32::from(p.y)))
            .unwrap_or_else(|| "—".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .bg(colors.container)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child("Mouse Events"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.disabled)
                    .child("Move and click within the target area"),
            )
            .child(
                div()
                    .id("mouse-events-target")
                    .h_20()
                    .rounded_md()
                    .border_2()
                    .border_color(if is_pressed {
                        colors.selected
                    } else {
                        colors.border
                    })
                    .bg(if is_pressed {
                        colors.selected
                    } else {
                        colors.selected
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(colors.text)
                    .child(format!("Position: {}", position_text))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.is_pressed = true;
                            this.log_event("Mouse Down");
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.is_pressed = false;
                            this.log_event("Mouse Up");
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                        this.mouse_position = Some(event.position);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .mt_2()
                    .text_xs()
                    .text_color(colors.disabled)
                    .children(
                        self.event_log
                            .iter()
                            .map(|e| div().child(format!("• {}", e))),
                    ),
            )
    }
}

// ============================================================================
// Drag and Drop Demo
// ============================================================================
//
// Demonstrates drag and drop:
// - on_drag - makes an element draggable, provides drag data
// - on_drop - makes an element a drop target, receives drag data

#[derive(Clone, Copy)]
struct DragData {
    index: usize,
    color: Hsla,
    position: Point<Pixels>,
}

impl DragData {
    fn new(index: usize, color: Hsla) -> Self {
        Self {
            index,
            color,
            position: Point::default(),
        }
    }

    fn with_position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

// Render trait for DragData allows it to be rendered as drag feedback
impl Render for DragData {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let size = gpui::size(px(80.), px(40.));

        // Position the drag preview at the cursor
        div()
            .pl(self.position.x - size.width.half())
            .pt(self.position.y - size.height.half())
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .w(size.width)
                    .h(size.height)
                    .bg(self.color.opacity(0.8))
                    .text_color(gpui::white())
                    .text_xs()
                    .rounded_md()
                    .shadow_lg()
                    .child(format!("Item {}", self.index + 1)),
            )
    }
}

struct DragDropDemo {
    dropped_item: Option<DragData>,
}

impl DragDropDemo {
    fn new() -> Self {
        Self { dropped_item: None }
    }
}

impl Render for DragDropDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let item_colors = [rgb(0xd32f2f), rgb(0x388e3c), rgb(0xf9a825)];

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .bg(colors.container)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child("Drag and Drop"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.disabled)
                    .child("Drag items to the drop zone below"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children(item_colors.into_iter().enumerate().map(|(index, color)| {
                        let drag_data = DragData::new(index, color.into());

                        div()
                            .id(("drag-item", index))
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .border_2()
                            .border_color(color)
                            .text_color(color)
                            .text_xs()
                            .cursor_grab()
                            .hover(move |style| {
                                let c: Hsla = color.into();
                                style.bg(c.opacity(0.1))
                            })
                            .child(format!("Item {}", index + 1))
                            // on_drag takes: drag data, and a closure that creates the drag preview
                            .on_drag(drag_data, |data: &DragData, position, _, cx| {
                                cx.new(|_| data.with_position(position))
                            })
                    })),
            )
            .child(
                div()
                    .id("drop-target")
                    .mt_2()
                    .h_16()
                    .rounded_md()
                    .border_2()
                    .border_dashed()
                    .border_color(
                        self.dropped_item
                            .map(|d| d.color)
                            .unwrap_or_else(|| colors.border.into()),
                    )
                    .when_some(self.dropped_item, |el, data| el.bg(data.color.opacity(0.2)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(colors.disabled)
                    // on_drop receives the drag data when an item is dropped
                    .on_drop(cx.listener(|this, data: &DragData, _window, cx| {
                        this.dropped_item = Some(*data);
                        cx.notify();
                    }))
                    .child(
                        self.dropped_item
                            .map(|d| format!("Dropped: Item {}", d.index + 1))
                            .unwrap_or_else(|| "Drop Zone".to_string()),
                    ),
            )
    }
}

// ============================================================================
// Main Application
// ============================================================================

struct InteractiveElementsExample {
    click_demo: Entity<ClickDemo>,
    hover_demo: Entity<HoverDemo>,
    mouse_events_demo: Entity<MouseEventsDemo>,
    drag_drop_demo: Entity<DragDropDemo>,
}

impl InteractiveElementsExample {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            click_demo: cx.new(|_| ClickDemo::new()),
            hover_demo: cx.new(|_| HoverDemo::new()),
            mouse_events_demo: cx.new(|_| MouseEventsDemo::new()),
            drag_drop_demo: cx.new(|_| DragDropDemo::new()),
        }
    }
}

impl Render for InteractiveElementsExample {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

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
                    .max_w(px(800.))
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
                                    .child("Interactive Elements"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.disabled)
                                    .child("Click, hover, mouse events, and drag-and-drop in GPUI"),
                            ),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_4()
                            .child(self.click_demo.clone())
                            .child(self.hover_demo.clone())
                            .child(self.mouse_events_demo.clone())
                            .child(self.drag_drop_demo.clone()),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(700.), px(650.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| InteractiveElementsExample::new(cx)),
        )
        .expect("Failed to open window");

        init_example(cx, "Interactive Elements");
    });
}
