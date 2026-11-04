use crate::app::input::runtime::{FrameTextInputKind, clamp_text_input_scroll_x};
use crate::app::input::text::clamp_text_offset;
use crate::app::input::text_line_closest_index_for_x;
use crate::app::root::FrameRoot;
use gpui::{
    Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ShapedLine, Window, px,
};

impl FrameRoot {
    pub(in crate::app) fn text_input_index_for_mouse_position(
        &mut self,
        kind: FrameTextInputKind,
        position: Point<Pixels>,
    ) -> usize {
        let text = self.text_input_value(kind);
        if text.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line_width)) = ({
            let runtime = self.text_input_runtime(kind);
            (
                runtime.last_bounds,
                runtime.last_layout.as_ref().map(ShapedLine::width),
            )
        }) else {
            return text.len();
        };
        let viewport_width = bounds.size.width;
        let mut local_x = position.x - bounds.left();
        let ui_scale = self.appearance.ui_scale.factor();

        {
            let runtime = self.text_input_runtime_mut(kind);
            let mut scroll_x =
                clamp_text_input_scroll_x(runtime.scroll_x, line_width, viewport_width);
            if position.x < bounds.left() {
                scroll_x = clamp_text_input_scroll_x(
                    scroll_x - text_input_drag_scroll_amount(bounds.left() - position.x, ui_scale),
                    line_width,
                    viewport_width,
                );
                local_x = Pixels::ZERO;
            } else if position.x > bounds.right() {
                scroll_x = clamp_text_input_scroll_x(
                    scroll_x + text_input_drag_scroll_amount(position.x - bounds.right(), ui_scale),
                    line_width,
                    viewport_width,
                );
                local_x = viewport_width;
            }
            runtime.scroll_x = scroll_x;
        }

        let runtime = self.text_input_runtime(kind);
        let Some(line) = runtime.last_layout.as_ref() else {
            return text.len();
        };
        clamp_text_offset(
            &text,
            text_line_closest_index_for_x(line, local_x + runtime.scroll_x),
        )
    }

    pub(in crate::app) fn text_input_mouse_down(
        &mut self,
        kind: FrameTextInputKind,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_disabled(kind) {
            return;
        }
        if let Some(focus) = self.text_input_focus_handle(kind) {
            focus.focus(window, cx);
        }
        self.activate_text_input(kind, cx);
        self.text_input_runtime_mut(kind).is_selecting = true;
        let offset = self.text_input_index_for_mouse_position(kind, event.position);
        if event.modifiers.shift {
            self.select_text_input_to(kind, offset, cx);
        } else {
            self.move_text_input_to(kind, offset, cx);
        }
    }

    pub(in crate::app) fn text_input_mouse_move(
        &mut self,
        kind: FrameTextInputKind,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_runtime(kind).is_selecting {
            let offset = self.text_input_index_for_mouse_position(kind, event.position);
            self.select_text_input_to(kind, offset, cx);
        }
    }

    pub(in crate::app) const fn text_input_mouse_up(
        &mut self,
        kind: FrameTextInputKind,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.text_input_runtime_mut(kind).is_selecting = false;
    }
}

pub(super) fn text_input_drag_scroll_amount(distance: Pixels, ui_scale: f32) -> Pixels {
    distance.clamp(px(4.0 * ui_scale), px(40.0 * ui_scale))
}
