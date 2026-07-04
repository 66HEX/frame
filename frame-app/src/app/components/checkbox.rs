use super::{
    ClickEvent, Context, FluentBuilder, FrameRoot, InteractiveElement, MouseButton, ParentElement,
    Styled, Window, apply_accessible_checkbox, apply_accessible_checkbox_with_focus, assets,
    button_mouse_down, color, div, icon_svg, input_highlight_shadows, px, theme,
};
use gpui::{FocusHandle, StatefulInteractiveElement};
use std::rc::Rc;

pub(in crate::app) const FRAME_CHECKBOX_SIZE: f32 = 14.0;
pub(in crate::app) const FRAME_CHECK_ICON_SIZE: f32 = 12.0;
pub(in crate::app) const FRAME_CHECKBOX_ROW_INDICATOR_OFFSET_Y: f32 = 3.0;
const FRAME_CHECKBOX_MARK_SIZE: f32 = 8.0;
const FRAME_SELECTION_DOT_SIZE: f32 = 12.0;
const FRAME_SELECTION_DOT_MARK_SIZE: f32 = 6.0;

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
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row_inner(id, label, hint, checked, disabled, None, cx, action)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Checkbox rows need explicit focus plus the shared activation handler."
)]
pub(in crate::app) fn frame_checkbox_row_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    hint: impl Into<String>,
    checked: bool,
    disabled: bool,
    focus: &FocusHandle,
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row_inner(id, label, hint, checked, disabled, Some(focus), cx, action)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared checkbox row builder preserves the visual row contract and separates focus from activation."
)]
fn frame_checkbox_row_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    hint: impl Into<String>,
    checked: bool,
    disabled: bool,
    focus: Option<&FocusHandle>,
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let display_label = theme::ui_text_owned(label.clone());
    let hint = theme::ui_text_owned(hint.into());
    let enabled = !disabled;
    let action = Rc::new(action);
    let row_action = Rc::clone(&action);
    let indicator_action = Rc::clone(&action);
    let indicator = frame_checkbox_indicator(checked, false, disabled)
        .id(format!("{id}-indicator"))
        .mt(px(FRAME_CHECKBOX_ROW_INDICATOR_OFFSET_Y))
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            indicator_action(root, event, window, cx);
        }));
    let indicator = if let Some(focus) = focus {
        apply_accessible_checkbox_with_focus(indicator, label, enabled, checked, false, focus)
    } else {
        apply_accessible_checkbox(indicator, label, enabled, checked, false)
    };

    div()
        .id(id)
        .flex()
        .items_start()
        .gap_2()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(enabled, gpui::Styled::cursor_pointer)
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            row_action(root, event, window, cx);
        }))
        .child(indicator)
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
                        .child(display_label),
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
