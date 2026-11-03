use super::{ParentElement, Styled, div, relative, theme};

pub(in crate::app) const MIN_POINTER_TARGET: f32 = 24.0;

/// Center a target on its visual anchor, limiting it to the distance between
/// neighboring handles. The transparent cells meet but never overlap, even
/// when the user shrinks a crop or overlay below the normal target size.
pub(in crate::app) fn frame_handle_hit_region(
    x: f32,
    y: f32,
    horizontal_spacing: f32,
    vertical_spacing: f32,
    handle: gpui::Stateful<gpui::Div>,
) -> gpui::Div {
    div()
        .absolute()
        .left(relative(x - horizontal_spacing / 2.0))
        .top(relative(y - vertical_spacing / 2.0))
        .w(relative(horizontal_spacing))
        .h(relative(vertical_spacing))
        .flex()
        .items_center()
        .justify_center()
        .child(
            handle
                .relative()
                .w(theme::ui_rem(MIN_POINTER_TARGET))
                .h(theme::ui_rem(MIN_POINTER_TARGET))
                .max_w_full()
                .max_h_full(),
        )
}
