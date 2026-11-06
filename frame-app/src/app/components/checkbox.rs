use crate::app::accessibility::{apply_accessible_checkbox, apply_accessible_checkbox_with_focus};
use crate::app::primitives::{button_mouse_down, color, icon_svg};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, MouseButton, StatefulInteractiveElement,
    Window, div,
};
use std::rc::Rc;

pub(in crate::app) const FRAME_CHECKBOX_SIZE: f32 = 16.0;
pub(in crate::app) const FRAME_CHECK_ICON_SIZE: f32 = 14.0;
pub(in crate::app) const FRAME_CHECKBOX_ROW_INDICATOR_OFFSET_Y: f32 = 2.0;

pub(in crate::app) fn frame_checkbox_indicator(
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    palette: &theme::ThemePalette,
) -> gpui::Div {
    let mut mark = div()
        .w(theme::ui_rem(FRAME_CHECKBOX_SIZE))
        .h(theme::ui_rem(FRAME_CHECKBOX_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_XS));

    if checked || indeterminate {
        mark = mark.child(icon_svg(
            assets::ICON_CHECK,
            FRAME_CHECK_ICON_SIZE,
            color(if checked {
                palette.primary_foreground
            } else {
                palette.text_primary
            }),
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
        .border_1()
        .border_color(color(palette.transparent))
        .bg(if checked {
            theme::shadows::surface_fill(palette.primary, palette)
        } else {
            color(palette.input_background).into()
        })
        .shadow(if checked {
            theme::shadows::ShadowLevel::Xxs.shadows()
        } else {
            theme::shadows::recessed(palette)
        })
        .opacity(if disabled { 0.5 } else { 1.0 })
        .child(mark)
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
    let indicator = frame_checkbox_indicator(checked, false, false, palette)
        .id(format!("{id}-indicator"))
        .mt(theme::ui_rem(FRAME_CHECKBOX_ROW_INDICATOR_OFFSET_Y))
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if enabled {
                indicator_action(root, event, window, cx);
            }
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
        .min_w_0()
        .flex_1()
        .flex()
        .items_start()
        .gap_2()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(enabled, gpui::Styled::cursor_default)
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if enabled {
                row_action(root, event, window, cx);
            }
        }))
        .child(indicator)
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .when(has_hint, gpui::Styled::gap_1)
                .child(
                    div()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                        .text_color(color(palette.text_primary))
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
