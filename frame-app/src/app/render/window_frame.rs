use crate::app::root::FrameRoot;
#[cfg(any(target_os = "linux", test))]
use gpui::{
    App, Bounds, CursorStyle, Decorations, InteractiveElement, MouseButton, Point, ResizeEdge,
    ResizeRegion, px,
};
use gpui::{Context, Div, Stateful, Window};

#[cfg(target_os = "linux")]
pub(super) fn linux_window_frame(
    root: Stateful<Div>,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> Stateful<Div> {
    client_resize_surface(
        root,
        client_resize_region(
            window.window_decorations(),
            window.is_resizable(),
            window.is_maximized(),
            window.is_fullscreen(),
        ),
        cx.listener(|root, edge: &ResizeEdge, window, _cx| {
            begin_client_resize(root, *edge, window);
        }),
    )
}

#[cfg(not(target_os = "linux"))]
pub(super) const fn linux_window_frame(
    root: Stateful<Div>,
    _window: &Window,
    _cx: &Context<FrameRoot>,
) -> Stateful<Div> {
    root
}

#[cfg(any(target_os = "linux", test))]
fn client_resize_region(
    decorations: Decorations,
    resizable: bool,
    maximized: bool,
    fullscreen: bool,
) -> Option<ResizeRegion> {
    let Decorations::Client { tiling } = decorations else {
        return None;
    };
    (resizable && !maximized && !fullscreen && !tiling.is_tiled())
        .then(|| ResizeRegion::new(px(5.0)).corner_size(px(16.0)))
}

#[cfg(any(target_os = "linux", test))]
fn client_resize_surface(
    root: Stateful<Div>,
    region: Option<ResizeRegion>,
    resize: impl Fn(&ResizeEdge, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let Some(region) = region else {
        return root;
    };
    // Keep resize gestures inside the content bounds, with no shadow or invisible margin.
    root.cursor_with(move |position, bounds| {
        region.hit_test(position, bounds).map(CursorStyle::from)
    })
    .capture_any_mouse_down(move |event, window, cx| {
        if event.button == MouseButton::Left
            && let Some(edge) = region.hit_test(
                event.position,
                Bounds::new(Point::default(), window.viewport_size()),
            )
        {
            window.prevent_default();
            cx.stop_propagation();
            resize(&edge, window, cx);
        }
    })
}

#[cfg(any(target_os = "linux", test))]
fn begin_client_resize(root: &mut FrameRoot, edge: ResizeEdge, window: &Window) {
    // A missed release from a titlebar press must not turn the resize into a move.
    root.titlebar_drag.should_move = false;
    window.start_window_resize(edge);
}

#[cfg(test)]
#[path = "window_frame_tests.rs"]
mod tests;
