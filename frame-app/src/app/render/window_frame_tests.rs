use super::*;
use gpui::{
    AppContext, Context, Entity, IntoElement, Modifiers, ParentElement, Render, ResizeEdge, Styled,
    TestAppContext, Tiling, div, point, size,
};
use std::{cell::Cell, rc::Rc};

fn client_decorations() -> Decorations {
    Decorations::Client {
        tiling: Tiling::default(),
    }
}

#[test]
fn only_floating_resizable_client_windows_need_resize_regions() {
    assert!(client_resize_region(client_decorations(), true, false, false).is_some());
    for (decorations, resizable, maximized, fullscreen) in [
        (Decorations::Server, true, false, false),
        (client_decorations(), false, false, false),
        (client_decorations(), true, true, false),
        (client_decorations(), true, false, true),
        (
            Decorations::Client {
                tiling: Tiling::tiled(),
            },
            true,
            false,
            false,
        ),
        (
            Decorations::Client {
                tiling: Tiling {
                    left: true,
                    ..Tiling::default()
                },
            },
            true,
            false,
            false,
        ),
    ] {
        assert!(client_resize_region(decorations, resizable, maximized, fullscreen).is_none());
    }
}

#[test]
fn client_resize_handles_use_only_the_inside_edges_and_corners() {
    let region = client_resize_region(client_decorations(), true, false, false).unwrap();
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(1200.0), px(800.0)));
    for (x, y, edge) in [
        (600.0, 2.0, ResizeEdge::Top),
        (1198.0, 400.0, ResizeEdge::Right),
        (600.0, 798.0, ResizeEdge::Bottom),
        (2.0, 400.0, ResizeEdge::Left),
        (12.0, 2.0, ResizeEdge::TopLeft),
        (1188.0, 2.0, ResizeEdge::TopRight),
        (12.0, 798.0, ResizeEdge::BottomLeft),
        (1188.0, 798.0, ResizeEdge::BottomRight),
    ] {
        assert_eq!(region.hit_test(point(px(x), px(y)), bounds), Some(edge));
    }
    for (x, y) in [
        (5.0, 5.0),
        (600.0, 400.0),
        (-1.0, 400.0),
        (1200.0, 400.0),
        (600.0, -1.0),
        (600.0, 800.0),
    ] {
        assert_eq!(region.hit_test(point(px(x), px(y)), bounds), None);
    }
}

struct ResizeTestView {
    owner: Entity<FrameRoot>,
    region: Option<ResizeRegion>,
    presses: Rc<Cell<usize>>,
}

impl Render for ResizeTestView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let presses = self.presses.clone();
        self.owner.update(cx, |_, cx| {
            client_resize_surface(
                div()
                    .id("window-content")
                    .debug_selector(|| "window-content".into())
                    .size_full()
                    .bg(gpui::black())
                    .child(
                        div()
                            .id("window-child")
                            .debug_selector(|| "window-child".into())
                            .size_full()
                            .on_any_mouse_down(move |_, _, _| {
                                presses.set(presses.get() + 1);
                            }),
                    ),
                self.region,
                cx.listener(|root, edge: &ResizeEdge, window, _cx| {
                    begin_client_resize(root, *edge, window);
                }),
            )
        })
    }
}

#[test]
fn outer_surface_fills_the_viewport_and_client_resize_does_not_move_content() {
    for decorations in [Decorations::Server, client_decorations()] {
        let mut app = TestAppContext::single();
        let presses = Rc::new(Cell::new(0));
        let (view, cx) = app.add_window_view(|_, cx| ResizeTestView {
            owner: cx.new(|_| FrameRoot::new()),
            region: client_resize_region(decorations, true, false, false),
            presses: presses.clone(),
        });
        let owner = view.read_with(cx, |view, _| view.owner.clone());
        let viewport = size(px(1200.0), px(800.0));
        cx.update(|window, _| window.activate());
        cx.simulate_resize(viewport);
        cx.simulate_window_hover(true);
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = Bounds::new(point(px(0.0), px(0.0)), viewport);
        assert_eq!(cx.debug_bounds("window-content"), Some(bounds));
        assert_eq!(cx.debug_bounds("window-child"), Some(bounds));
        cx.update(|window, _| {
            assert!(window.client_inset().is_none());
            let quads = window.painted_quads();
            let surface = quads
                .iter()
                .find(|quad| !quad.background.is_transparent())
                .unwrap();
            assert_eq!(surface.bounds, bounds.scale(window.scale_factor()));
            assert_eq!(surface.corner_radii, gpui::Corners::default());
            assert_eq!(surface.border_widths, gpui::Edges::default());
        });
        let edge = point(px(600.0), px(2.0));
        owner.update(cx, |root, _| root.titlebar_drag.should_move = true);
        cx.simulate_mouse_move(edge, None, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            cx.cursor_style(),
            if decorations == Decorations::Server {
                CursorStyle::Arrow
            } else {
                CursorStyle::ResizeUp
            }
        );
        cx.simulate_mouse_down(edge, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(edge, MouseButton::Left, Modifiers::default());
        assert_eq!(
            presses.get(),
            usize::from(decorations == Decorations::Server)
        );
        assert_eq!(
            owner.read_with(cx, |root, _| root.titlebar_drag.should_move),
            decorations == Decorations::Server
        );
        let before = presses.get();
        cx.simulate_mouse_down(edge, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(edge, MouseButton::Right, Modifiers::default());
        cx.simulate_click(bounds.center(), Modifiers::default());
        assert_eq!(presses.get(), before + 2);
        assert_eq!(cx.debug_bounds("window-content"), Some(bounds));
    }
}
