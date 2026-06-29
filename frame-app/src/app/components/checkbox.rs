use super::*;

pub(in crate::app) const FRAME_CHECKBOX_SIZE: f32 = 14.0;
pub(in crate::app) const FRAME_CHECK_ICON_SIZE: f32 = 12.0;
const FRAME_CHECKBOX_MARK_SIZE: f32 = 8.0;
const FRAME_SELECTION_DOT_SIZE: f32 = 12.0;
const FRAME_SELECTION_DOT_MARK_SIZE: f32 = 6.0;

pub(in crate::app) fn frame_checkbox_hit_area(
    checked: bool,
    indeterminate: bool,
    enabled: bool,
    height: f32,
) -> gpui::Div {
    div()
        .w(px(theme::MIN_HIT_AREA))
        .h(px(height))
        .flex()
        .items_center()
        .justify_start()
        .child(frame_checkbox_indicator(checked, indeterminate, !enabled))
}

pub(in crate::app) fn frame_checkbox_indicator(
    checked: bool,
    indeterminate: bool,
    disabled: bool,
) -> gpui::Div {
    let active = checked || indeterminate;
    let mut mark = div()
        .w(px(FRAME_CHECKBOX_SIZE))
        .h(px(FRAME_CHECKBOX_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .bg(if active {
            color(theme::FRAME_GRAY_600)
        } else {
            color(theme::TRANSPARENT)
        });

    if indeterminate {
        mark = mark.child(
            div()
                .w(px(FRAME_CHECKBOX_MARK_SIZE))
                .h(px(2.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(color(theme::FOREGROUND)),
        );
    } else if checked {
        mark = mark.child(icon_svg(
            assets::ICON_CHECK,
            FRAME_CHECK_ICON_SIZE,
            color(theme::FOREGROUND),
        ));
    }

    div()
        .w(px(FRAME_CHECKBOX_SIZE))
        .h(px(FRAME_CHECKBOX_SIZE))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .bg(color(theme::BACKGROUND))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .shadow(input_highlight_shadows())
        .child(mark)
}

pub(in crate::app) fn frame_selection_dot(is_selected: bool) -> gpui::Div {
    div()
        .w(px(FRAME_SELECTION_DOT_SIZE))
        .h(px(FRAME_SELECTION_DOT_SIZE))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(color(theme::BACKGROUND))
        .shadow(input_highlight_shadows())
        .child(
            div()
                .w(px(FRAME_SELECTION_DOT_MARK_SIZE))
                .h(px(FRAME_SELECTION_DOT_MARK_SIZE))
                .rounded_full()
                .bg(color(theme::FRAME_GRAY_600))
                .opacity(if is_selected { 1.0 } else { 0.0 }),
        )
}

pub(in crate::app) fn frame_checkbox_row(
    id: impl Into<String>,
    label: impl Into<String>,
    hint: impl Into<String>,
    checked: bool,
    disabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let label = theme::ui_text_owned(label.into());
    let hint = theme::ui_text_owned(hint.into());
    let enabled = !disabled;

    div()
        .id(id.into())
        .flex()
        .items_start()
        .gap_2()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(enabled, |this| this.cursor_pointer())
        .when(!enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(frame_checkbox_indicator(checked, false, disabled))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(theme::TEXT_LABEL_SIZE))
                        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(theme::TEXT_LABEL_SIZE))
                        .font_weight(theme::TEXT_WEIGHT_REGULAR)
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(hint),
                ),
        )
}
