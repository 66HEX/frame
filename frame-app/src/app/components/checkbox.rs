use super::{
    ClickEvent, Context, FluentBuilder, FrameRoot, InteractiveElement, MouseButton, ParentElement,
    Styled, Window, apply_accessible_checkbox, apply_accessible_checkbox_with_focus, assets,
    button_mouse_down, color, div, icon_svg, input_highlight_shadows, theme,
};
use gpui::{FocusHandle, StatefulInteractiveElement};
use std::rc::Rc;

pub(in crate::app) const FRAME_CHECKBOX_SIZE: f32 = 16.0;
pub(in crate::app) const FRAME_CHECK_ICON_SIZE: f32 = 14.0;
pub(in crate::app) const FRAME_CHECKBOX_ROW_INDICATOR_OFFSET_Y: f32 = 2.0;
const FRAME_CHECKBOX_MARK_SIZE: f32 = 8.0;
const FRAME_SELECTION_DOT_SIZE: f32 = 12.0;
const FRAME_SELECTION_DOT_MARK_SIZE: f32 = 6.0;

pub(in crate::app) fn frame_checkbox_indicator(
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    palette: &theme::ThemePalette,
) -> gpui::Div {
    let active = checked || indeterminate;
    let mut mark = div()
        .w(theme::ui_rem(FRAME_CHECKBOX_SIZE))
        .h(theme::ui_rem(FRAME_CHECKBOX_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_XS))
        .bg(if active {
            color(palette.border_subtle)
        } else {
            color(palette.transparent)
        });

    if indeterminate {
        mark = mark.child(
            div()
                .w(theme::ui_rem(FRAME_CHECKBOX_MARK_SIZE))
                .h(theme::ui_rem(2.0))
                .rounded(theme::ui_rem(theme::RADIUS_XS))
                .bg(color(palette.text_primary)),
        );
    } else if checked {
        mark = mark.child(icon_svg(
            assets::ICON_CHECK,
            FRAME_CHECK_ICON_SIZE,
            color(palette.text_primary),
        ));
    }

    div()
        .w(theme::ui_rem(FRAME_CHECKBOX_SIZE))
        .h(theme::ui_rem(FRAME_CHECKBOX_SIZE))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_XS))
        .bg(color(palette.canvas))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .shadow(input_highlight_shadows(palette))
        .child(mark)
}

pub(in crate::app) fn frame_selection_dot(
    selected_progress: f32,
    palette: &theme::ThemePalette,
) -> gpui::Div {
    div()
        .w(theme::ui_rem(FRAME_SELECTION_DOT_SIZE))
        .h(theme::ui_rem(FRAME_SELECTION_DOT_SIZE))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(color(palette.canvas))
        .shadow(input_highlight_shadows(palette))
        .child(
            div()
                .w(theme::ui_rem(FRAME_SELECTION_DOT_MARK_SIZE))
                .h(theme::ui_rem(FRAME_SELECTION_DOT_MARK_SIZE))
                .rounded_full()
                .bg(color(palette.control_muted))
                .opacity(selected_progress.clamp(0.0, 1.0)),
        )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The checkbox row keeps label, hint, state, palette, context, and action explicit at call sites."
)]
pub(in crate::app) fn frame_checkbox_row(
    id: impl Into<String>,
    label: impl Into<String>,
    hint: impl Into<String>,
    checked: bool,
    disabled: bool,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row_inner(
        id, label, hint, checked, disabled, None, palette, cx, action,
    )
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
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    frame_checkbox_row_inner(
        id,
        label,
        hint,
        checked,
        disabled,
        Some(focus),
        palette,
        cx,
        action,
    )
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
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
    action: impl Fn(&mut FrameRoot, &ClickEvent, &mut Window, &mut Context<FrameRoot>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let display_label = theme::ui_text_owned(label.clone());
    let hint = hint.into();
    let has_hint = !hint.is_empty();
    let hint = theme::ui_text_owned(hint);
    let enabled = !disabled;
    let action = Rc::new(action);
    let row_action = Rc::clone(&action);
    let indicator_action = Rc::clone(&action);
    let indicator = frame_checkbox_indicator(checked, false, disabled, palette)
        .id(format!("{id}-indicator"))
        .mt(theme::ui_rem(FRAME_CHECKBOX_ROW_INDICATOR_OFFSET_Y))
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            indicator_action(root, event, window, cx);
        }));
    let indicator = if let Some(focus) = focus {
        apply_accessible_checkbox_with_focus(
            indicator, label, enabled, checked, false, focus, palette,
        )
    } else {
        apply_accessible_checkbox(indicator, label, enabled, checked, false, palette)
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
                .when(has_hint, gpui::Styled::gap_1)
                .child(
                    div()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                        .text_color(color(palette.text_muted))
                        .child(display_label),
                )
                .when(has_hint, |this| {
                    this.child(
                        div()
                            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                            .font_weight(theme::TEXT_WEIGHT_REGULAR)
                            .text_color(color(palette.text_muted))
                            .child(hint),
                    )
                }),
        )
}
