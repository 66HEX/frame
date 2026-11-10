use super::*;
use crate::{Render, TestAppContext, VisualTestContext};

struct FocusScrollView {
    outer: ScrollHandle,
    inner: ScrollHandle,
    sibling: ScrollHandle,
    first: FocusHandle,
    last: FocusHandle,
}

impl Render for FocusScrollView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().children([
            div()
                .id("outer")
                .w(px(120.))
                .h(px(100.))
                .overflow_y_scroll()
                .track_scroll(&self.outer)
                .child(div().h(px(120.)))
                .child(
                    div()
                        .id("inner")
                        .h(px(80.))
                        .w_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.inner)
                        .child(
                            div()
                                .id("first")
                                .track_focus(&self.first)
                                .h(px(20.))
                                .w_full(),
                        )
                        .child(div().h(px(400.)))
                        .child(
                            div()
                                .id("last")
                                .debug_selector(|| "last".into())
                                .track_focus(&self.last)
                                .h(px(20.))
                                .w_full(),
                        ),
                )
                .child(div().h(px(120.))),
            div()
                .id("sibling")
                .w(px(120.))
                .h(px(100.))
                .overflow_y_scroll()
                .track_scroll(&self.sibling)
                .child(div().h(px(600.))),
        ])
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
}

#[gpui::test]
fn keyboard_focus_reveals_nested_controls_without_scrolling_siblings_or_fighting_manual_scroll(
    app: &mut TestAppContext,
) {
    let (view, cx) = app.add_window_view(|_, cx| FocusScrollView {
        outer: ScrollHandle::new(),
        inner: ScrollHandle::new(),
        sibling: ScrollHandle::new(),
        first: cx.focus_handle().tab_stop(true),
        last: cx.focus_handle().tab_stop(true),
    });
    draw(cx);
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| view.read(cx).last.clone().focus(window, cx));
    draw(cx);
    view.read_with(cx, |view, _| {
        assert!(view.inner.offset().y < px(0.));
        assert!(view.outer.offset().y < px(0.));
        assert_eq!(view.sibling.offset(), Point::default());
    });
    let target = cx.debug_bounds("last").unwrap();
    assert!(target.top() >= px(0.) && target.bottom() <= px(100.));
    // A user may scroll away while leaving keyboard focus on the same control.
    view.read_with(cx, |view, _| {
        view.outer.set_offset(Point::default());
        view.inner.set_offset(Point::default());
    });
    draw(cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.outer.offset(), Point::default());
        assert_eq!(view.inner.offset(), Point::default());
    });
    // A new focus move should reveal again.
    cx.update(|window, cx| view.read(cx).first.clone().focus(window, cx));
    draw(cx);
    cx.update(|window, cx| view.read(cx).last.clone().focus(window, cx));
    draw(cx);
    let target = cx.debug_bounds("last").unwrap();
    assert!(target.top() >= px(0.) && target.bottom() <= px(100.));
    // Pointer-initiated focus never changes the scroll position.
    cx.simulate_click(point(px(250.), px(10.)), Default::default());
    view.read_with(cx, |view, _| {
        view.outer.set_offset(Point::default());
        view.inner.set_offset(Point::default());
    });
    cx.update(|window, cx| view.read(cx).first.clone().focus(window, cx));
    draw(cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.outer.offset(), Point::default());
        assert_eq!(view.inner.offset(), Point::default());
    });
}
