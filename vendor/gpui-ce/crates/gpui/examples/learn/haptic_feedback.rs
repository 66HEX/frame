//! Haptic Feedback Example
//!
//! This example demonstrates haptic feedback in GPUI (macOS only):
//!
//! - Check if haptic feedback is supported
//! - Play the three [`NSHapticFeedbackPattern`] styles
//! - A quantized slider with haptic feedback
//!
//! On macOS, haptics are delivered via `NSHapticFeedbackManager`. On other
//! platforms, the calls are currently no-ops.

#[path = "../shared/prelude.rs"]
mod example_prelude;

use example_prelude::init_example;
use gpui::{
    App, Bounds, ColorExt, Context, DragMoveEvent, FontWeight, HapticFeedbackStyle, Hsla,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Render,
    StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions, colors::Colors, div,
    prelude::*, px, relative, rgb, size,
};
use palette::IntoColor;

const SLIDER_MIN: f32 = 0.0;
const SLIDER_MAX: f32 = 100.0;
const SLIDER_STEP: f32 = 10.0;
const SLIDER_STEP_COUNT: usize = ((SLIDER_MAX - SLIDER_MIN) / SLIDER_STEP) as usize;

#[derive(Clone)]
struct SliderDrag;

impl Render for SliderDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

struct HapticFeedbackExample {
    supported: bool,
    slider_value: f32,
    slider_prev_step: i32,
    slider_bounds: Option<Bounds<Pixels>>,
}

impl HapticFeedbackExample {
    fn new(cx: &mut App) -> Self {
        Self {
            supported: cx.supports_haptic_feedback(),
            slider_value: 0.0,
            slider_prev_step: 0,
            slider_bounds: None,
        }
    }

    fn haptic_button(
        &self,
        id: &'static str,
        label: &'static str,
        style: HapticFeedbackStyle,
        color: impl IntoColor<Hsla>,
        colors: &Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let _ = cx;
        let color = color.into_color();

        div()
            .id(id)
            .flex()
            .flex_col()
            .w_48()
            .items_center()
            .justify_center()
            .px_4()
            .py_3()
            .rounded_md()
            .bg(color)
            .cursor_pointer()
            .hover_style(move |s| s.bg(color.opacity(0.8)))
            .active_style(move |s| s.bg(color.opacity(0.6)))
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.selected_text)
                    .child(label),
            )
            .on_hover(move |hovered, _, cx| {
                if *hovered {
                    cx.play_haptic_feedback(style);
                }
            })
    }

    fn value_to_step(value: f32) -> i32 {
        ((value - SLIDER_MIN) / SLIDER_STEP).round() as i32
    }

    fn snap_to_step(raw: f32) -> f32 {
        let snapped = (raw / SLIDER_STEP).round() * SLIDER_STEP;
        snapped.clamp(SLIDER_MIN, SLIDER_MAX)
    }

    fn value_to_percentage(value: f32) -> f32 {
        ((value - SLIDER_MIN) / (SLIDER_MAX - SLIDER_MIN)).clamp(0.0, 1.0)
    }

    fn update_slider_from_position(
        &mut self,
        position_x: Pixels,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let inner_x = (position_x - bounds.left()).clamp(px(0.), bounds.size.width);
        let percentage = inner_x / bounds.size.width;
        let raw = SLIDER_MIN + (SLIDER_MAX - SLIDER_MIN) * percentage;
        let new_value = Self::snap_to_step(raw);
        let new_step = Self::value_to_step(new_value);

        self.slider_value = new_value;
        self.slider_bounds = Some(bounds);

        if new_step != self.slider_prev_step {
            cx.play_haptic_feedback(HapticFeedbackStyle::LevelChange);
            self.slider_prev_step = new_step;
        }

        cx.notify();
    }
}

impl Render for HapticFeedbackExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);
        let slider_percentage = Self::value_to_percentage(self.slider_value);
        let slider_color = rgb(0x3b82f6);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(colors.background)
            .p_8()
            .gap_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text)
                            .child("Haptic Feedback"),
                    )
                    .child(
                        div().text_sm().text_color(colors.disabled).child(
                            "macOS Force Touch trackpad haptics via NSHapticFeedbackManager",
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(if self.supported {
                        rgb(0x22c55e)
                    } else {
                        rgb(0xf59e0b)
                    }
                    .opacity(0.1))
                    .child({
                        let dot = if self.supported {
                            rgb(0x22c55e)
                        } else {
                            rgb(0xf59e0b)
                        };
                        div().size_2().rounded_full().bg(dot)
                    })
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text)
                            .when_else(
                                self.supported,
                                |this| this.child("Haptics is supported on this machine"),
                                |this| this.child("Haptics is not supported on this machine"),
                            ),
                    ),
            )
            .child(
                div()
                    .child("Hover over each button to trigger haptic feedback")
                    .text_sm()
                    .text_color(colors.text),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(self.haptic_button(
                        "btn-generic",
                        "Generic",
                        HapticFeedbackStyle::Generic,
                        rgb(0x3b82f6),
                        &colors,
                        cx,
                    ))
                    .child(self.haptic_button(
                        "btn-alignment",
                        "Alignment",
                        HapticFeedbackStyle::Alignment,
                        rgb(0x10b981),
                        &colors,
                        cx,
                    ))
                    .child(self.haptic_button(
                        "btn-levelchange",
                        "LevelChange",
                        HapticFeedbackStyle::LevelChange,
                        rgb(0x8b5cf6),
                        &colors,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text)
                            .child("Slider with LevelChange haptic"),
                    )
                    .child(
                        div().flex().items_center().gap_2().child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.selected_text)
                                .child(format!("Value: {:.0}", self.slider_value)),
                        ),
                    )
                    .child(
                        div()
                            .id("slider-container")
                            .w_full()
                            .h_4()
                            .flex()
                            .items_center()
                            .px_2()
                            .cursor_pointer()
                            .on_drag(SliderDrag, |_, _, _, cx| cx.new(|_| SliderDrag))
                            .on_drag_move(cx.listener(
                                |this, e: &DragMoveEvent<SliderDrag>, _, cx| {
                                    this.update_slider_from_position(
                                        e.event.position.x,
                                        e.bounds,
                                        cx,
                                    );
                                },
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                                    if let Some(bounds) = this.slider_bounds {
                                        this.update_slider_from_position(e.position.x, bounds, cx);
                                    }
                                }),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h_1p5()
                                    .rounded_full()
                                    .bg(colors.text.opacity(0.12))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .children((0..(SLIDER_STEP_COUNT + 1)).map(|_| {
                                        div()
                                            .size_1p5()
                                            .rounded_full()
                                            .bg(colors.disabled)
                                            .into_any()
                                    }))
                                    .child(
                                        div()
                                            .absolute()
                                            .h_full()
                                            .left(px(0.))
                                            .right(relative(1.0 - slider_percentage))
                                            .bg(slider_color.opacity(0.7))
                                            .rounded_full(),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(-6.))
                                            .left(relative(slider_percentage))
                                            .ml(px(-8.))
                                            .size_4()
                                            .rounded_full()
                                            .bg(slider_color)
                                            .shadow_md(),
                                    ),
                            ),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(520.), px(520.)), cx);
        cx.open_window(
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |_, cx| cx.new(|cx| HapticFeedbackExample::new(cx)),
        )
        .expect("Failed to open window");

        init_example(cx, "Haptic Feedback");
    });
}
