use super::*;
use crate::app::components::{
    frame_color_picker_panel, frame_select_options_list, frame_select_popover,
};
use gpui::prelude::*;
use gpui::{Context, ScrollHandle, TestAppContext, VisualTestContext, deferred, div};

struct PopoverTestView {
    color_picker: bool,
    scroll: ScrollHandle,
}

impl gpui::Render for PopoverTestView {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(crate::appearance::BASE_REM_PX));
        let palette = crate::theme::palette(crate::appearance::ColorTheme::Dark);
        let anchor = PopoverAnchor::default();
        let measured_anchor = anchor.clone();
        let content = if self.color_picker {
            frame_color_picker_panel(
                1.0,
                div().h(px(96.0)),
                div().h(px(24.0)),
                div().h(px(28.0)).debug_selector(|| "last-content".into()),
                palette,
            )
            .id("panel")
            .debug_selector(|| "panel".into())
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
        } else {
            let list = frame_select_options_list("list", &self.scroll, false).children((0..4).map(
                |index| {
                    div().h(px(28.0)).when(index == 3, |item| {
                        item.debug_selector(|| "last-content".into())
                    })
                },
            ));
            frame_select_popover("panel", 1.0, list, palette)
        };
        div()
            .size_full()
            .child(
                div()
                    .id("trigger")
                    .absolute()
                    .top(px(100.0))
                    .left(px(220.0))
                    .w(px(180.0))
                    .h(px(28.0))
                    .on_prepaint(move |info, _, _| measured_anchor.measure(info.bounds)),
            )
            .child(deferred(frame_floating_popover(
                content, anchor, 1.0, 10.0, window,
            )))
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[test]
fn constrained_popovers_shift_inside_the_right_edge_and_scroll_to_the_last_control() {
    for color_picker in [false, true] {
        let mut app = TestAppContext::single();
        let (_, cx) = app.add_window_view(|_, _| PopoverTestView {
            color_picker,
            scroll: ScrollHandle::new(),
        });
        cx.simulate_resize(size(px(360.0), px(240.0)));
        draw(cx);
        let panel = cx.debug_bounds("panel").unwrap();
        assert!(panel.right() <= px(350.0), "{panel:?}");
        assert!(
            panel.top() >= px(10.0) && panel.bottom() <= px(230.0),
            "{panel:?}"
        );
        assert!(
            panel.size.height < px(120.0),
            "must constrain content: {panel:?}"
        );
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: panel.center(),
            delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(-1000.0))),
            ..Default::default()
        });
        draw(cx);
        let last = cx.debug_bounds("last-content").unwrap();
        assert!(
            last.top() >= panel.top() && last.bottom() <= panel.bottom(),
            "last control must remain reachable: {last:?}, {panel:?}"
        );
    }
}
