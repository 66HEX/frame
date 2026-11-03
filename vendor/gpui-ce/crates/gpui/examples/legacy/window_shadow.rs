#![cfg_attr(target_family = "wasm", no_main)]

#[path = "../example_support/fonts.rs"]
mod example_support;

use gpui::{
    App, Bounds, Context, Decorations, MouseButton, ResizeEdge, ResizeRegion, Window, WindowBounds,
    WindowDecorations, WindowOptions, black, div, green, hsla, point, prelude::*, px, rgb, size,
    transparent_black, white,
};
use gpui_platform::application;

struct WindowShadow {
    hovered_edge: Option<ResizeEdge>,
}

impl Render for WindowShadow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let decorations = window.window_decorations();
        let rounding = px(10.0);
        let shadow_size = px(10.0);
        let border_size = px(1.0);
        let grey = rgb(0x808080);
        window.set_client_inset(shadow_size);
        let is_resizable = window.is_resizable();
        let hovered_edge = self.hovered_edge;
        let resize_region = match decorations {
            Decorations::Server => ResizeRegion::new(shadow_size).corner_size(shadow_size),
            Decorations::Client { tiling } => ResizeRegion::new(shadow_size)
                .corner_size(shadow_size)
                .without_tiled_edges(tiling),
        };
        let edge_hint = match hovered_edge {
            Some(edge) => format!("Resize region: {edge:?}"),
            None => "Resize region: none".to_string(),
        };

        div()
            .id("window-backdrop")
            .bg(transparent_black())
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling, .. } => div
                    .bg(gpui::transparent_black())
                    .cursor_with(move |position, bounds| {
                        if is_resizable {
                            resize_region
                                .hit_test(position, bounds)
                                .map(gpui::CursorStyle::from)
                        } else {
                            None
                        }
                    })
                    .on_hover_region(
                        move |position, bounds| {
                            if is_resizable {
                                resize_region.hit_test(position, bounds)
                            } else {
                                None
                            }
                        },
                        cx.listener(|this, edge: &Option<ResizeEdge>, _window, cx| {
                            this.hovered_edge = *edge;
                            cx.notify();
                        }),
                    )
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(rounding)
                    })
                    .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                    .when(!tiling.top, |div| div.pt(shadow_size))
                    .when(!tiling.bottom, |div| div.pb(shadow_size))
                    .when(!tiling.left, |div| div.pl(shadow_size))
                    .when(!tiling.right, |div| div.pr(shadow_size))
                    .on_mouse_down(MouseButton::Left, move |e, window, _cx| {
                        let bounds = Bounds::new(point(px(0.0), px(0.0)), window.viewport_size());
                        match resize_region.hit_test(e.position, bounds) {
                            Some(edge) if window.is_resizable() => window.start_window_resize(edge),
                            None => window.start_window_move(),
                            Some(_) => window.start_window_move(),
                        };
                    }),
            })
            .size_full()
            .child(
                div()
                    .map(|div| match decorations {
                        Decorations::Server => div,
                        Decorations::Client { tiling } => div
                            .border_color(grey)
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(rounding)
                            })
                            .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                            .when(!tiling.top, |div| div.border_t(border_size))
                            .when(!tiling.bottom, |div| div.border_b(border_size))
                            .when(!tiling.left, |div| div.border_l(border_size))
                            .when(!tiling.right, |div| div.border_r(border_size))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(vec![
                                    gpui::BoxShadow::new(px(0.), px(0.), hsla(0., 0., 0., 0.4))
                                        .blur_radius(shadow_size / 2.),
                                ])
                            }),
                    })
                    .on_mouse_move(|_e, _, cx| {
                        cx.stop_propagation();
                    })
                    .bg(gpui::rgb(0xCCCCFF))
                    .size_full()
                    .flex()
                    .flex_col()
                    .justify_around()
                    .child(div().text_sm().text_color(black()).child(edge_hint))
                    .child(
                        div().w_full().flex().flex_row().justify_around().child(
                            div()
                                .flex()
                                .bg(white())
                                .size(px(300.0))
                                .justify_center()
                                .items_center()
                                .shadow_lg()
                                .border_1()
                                .border_color(rgb(0x0000ff))
                                .text_xl()
                                .text_color(rgb(0xffffff))
                                .child(
                                    div()
                                        .id("hello")
                                        .w(px(200.0))
                                        .h(px(100.0))
                                        .bg(green())
                                        .shadow(vec![
                                            gpui::BoxShadow::new(
                                                px(0.),
                                                px(0.),
                                                hsla(0., 0., 0., 1.),
                                            )
                                            .blur_radius(px(20.0)),
                                        ])
                                        .map(|div| match decorations {
                                            Decorations::Server => div,
                                            Decorations::Client { .. } => div
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    |_e, window, _| {
                                                        window.start_window_move();
                                                    },
                                                )
                                                .on_click(|e, window, _| {
                                                    if e.is_right_click() {
                                                        window.show_window_menu(e.position());
                                                    }
                                                })
                                                .text_color(black())
                                                .child("this is the custom titlebar"),
                                        }),
                                ),
                        ),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        if !example_support::load_fonts(cx) {
            return;
        }
        let bounds = Bounds::centered(None, size(px(600.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions::new()
                .window_bounds(Some(WindowBounds::Windowed(bounds)))
                .window_decorations(Some(WindowDecorations::Client)),
            |window, cx| {
                cx.new(|cx| {
                    cx.observe_window_appearance(window, |_, window, _| {
                        window.refresh();
                    })
                    .detach();
                    WindowShadow { hovered_edge: None }
                })
            },
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    env_logger::init();
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
