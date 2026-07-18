use super::{
    App, ButtonVariant, Context, FluentBuilder, FrameRoot, InteractiveElement, IntoElement,
    MouseButton, MouseMoveEvent, ParentElement, PlatformInput, SETTINGS_CONTROL_HEIGHT,
    ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement, Styled, Window,
    animated_button_colors, apply_accessible_select_option,
    apply_accessible_select_option_with_focus, apply_accessible_select_trigger,
    apply_accessible_select_trigger_with_focus, apply_button_motion, assets, button_colors,
    button_highlight_shadows, button_mouse_down, color, div, icon_svg, input_highlight_shadows,
    parse_hex, px, theme,
};
use crate::numeric::usize_to_f32;
use gpui::FocusHandle;

pub(in crate::app) const FRAME_SELECT_MAX_HEIGHT: f32 = 192.0;
pub(in crate::app) const FRAME_SELECT_CONTENT_PADDING: f32 = 4.0;
pub(in crate::app) const FRAME_SELECT_OPTION_HEIGHT: f32 = 28.0;
pub(in crate::app) const FRAME_COLOR_SWATCH_SIZE: f32 = 14.0;

pub(in crate::app) fn frame_select_trigger(
    id: impl Into<String>,
    label: impl Into<String>,
    display: &str,
    enabled: bool,
    expanded: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content(
        id,
        label,
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_color(color(theme::FOREGROUND))
            .child(theme::ui_text(display)),
        enabled,
        expanded,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Select triggers need explicit labels, state, rendering context, and a focus handle."
)]
pub(in crate::app) fn frame_select_trigger_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    display: &str,
    enabled: bool,
    expanded: bool,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content_inner(
        id,
        label,
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_color(color(theme::FOREGROUND))
            .child(theme::ui_text(display)),
        enabled,
        expanded,
        Some(focus),
        window,
        cx,
    )
}

pub(in crate::app) fn frame_select_trigger_content(
    id: impl Into<String>,
    label: impl Into<String>,
    content: impl IntoElement,
    enabled: bool,
    expanded: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content_inner(id, label, content, enabled, expanded, None, window, cx)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Select triggers need optional explicit focus handles while preserving the existing visual builder."
)]
pub(in crate::app) fn frame_select_trigger_content_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    content: impl IntoElement,
    enabled: bool,
    expanded: bool,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content_inner(
        id,
        label,
        content,
        enabled,
        expanded,
        Some(focus),
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared select trigger builder preserves the existing visual contract and optionally wires a focus handle."
)]
fn frame_select_trigger_content_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    content: impl IntoElement,
    enabled: bool,
    expanded: bool,
    focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let colors = button_colors(ButtonVariant::Secondary, false, enabled);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;

    let trigger = div()
        .id(id.clone())
        .group(id)
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .min_w_0()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(background)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .child(content)
        .child(
            div()
                .flex_shrink_0()
                .child(icon_svg(assets::ICON_UNFOLD_MORE, 12.0, foreground)),
        );

    let trigger = apply_button_motion(trigger, motion, enabled).on_mouse_down(
        MouseButton::Left,
        move |_, _window, cx| {
            cx.stop_propagation();
        },
    );

    if let Some(focus) = focus {
        apply_accessible_select_trigger_with_focus(trigger, label, enabled, expanded, focus)
    } else {
        apply_accessible_select_trigger(trigger, label, enabled, expanded)
    }
}

pub(in crate::app) fn frame_select_popover(
    id: &'static str,
    top: f32,
    progress: f32,
    list: impl IntoElement,
) -> gpui::Stateful<gpui::Div> {
    div()
        .absolute()
        .id(id)
        .top(px(top))
        .left_0()
        .right_0()
        .max_h(px(FRAME_SELECT_MAX_HEIGHT))
        .overflow_hidden()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::DROPDOWN))
        .opacity(progress)
        .shadow(button_highlight_shadows())
        .occlude()
        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
            cx.stop_propagation();
        })
        .child(list)
}

pub(in crate::app) fn frame_select_options_list(
    id: &'static str,
    scroll_handle: &ScrollHandle,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .role(gpui::Role::ListBox)
        .max_h(px(FRAME_SELECT_MAX_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .p(px(FRAME_SELECT_CONTENT_PADDING))
        .on_scroll_wheel(refresh_select_hover_after_scroll)
}

pub(in crate::app) fn frame_select_content_height(option_count: usize) -> f32 {
    usize_to_f32(option_count).mul_add(
        FRAME_SELECT_OPTION_HEIGHT,
        FRAME_SELECT_CONTENT_PADDING * 2.0,
    )
}

pub(in crate::app) fn frame_select_option(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    frame_select_option_inner(id, label, selected, enabled, None)
}

pub(in crate::app) fn frame_select_option_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    focus: &FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    frame_select_option_inner(id, label, selected, enabled, Some(focus))
}

fn frame_select_option_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let display_label = theme::ui_text_owned(label.clone());
    let text_color = if selected {
        theme::FOREGROUND
    } else {
        theme::FRAME_GRAY_600
    };

    let option = div()
        .id(id.into())
        .h(px(FRAME_SELECT_OPTION_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .rounded(px(theme::RADIUS_XS))
        .px(px(12.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(text_color))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| {
            this.hover(|style| {
                style
                    .bg(color(theme::FRAME_GRAY_100))
                    .text_color(color(theme::FOREGROUND))
                    .cursor_pointer()
            })
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            button_mouse_down(enabled, window, cx);
        })
        .child(div().min_w_0().truncate().child(display_label))
        .when(selected, |this| {
            this.child(icon_svg(assets::ICON_CHECK, 12.0, color(theme::FOREGROUND)))
        });

    if let Some(focus) = focus {
        apply_accessible_select_option_with_focus(option, label, enabled, selected, focus)
    } else {
        apply_accessible_select_option(option, label, enabled, selected)
    }
}

pub(in crate::app) fn frame_color_select_value(value: &str) -> gpui::Div {
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_2()
        .child(frame_color_swatch(value))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .w_full()
                .truncate()
                .text_color(color(theme::FOREGROUND))
                .child(value.to_uppercase()),
        )
}

pub(in crate::app) fn frame_color_swatch(value: &str) -> gpui::Div {
    div()
        .w(px(FRAME_COLOR_SWATCH_SIZE))
        .h(px(FRAME_COLOR_SWATCH_SIZE))
        .flex_shrink_0()
        .rounded(px(theme::RADIUS_XS))
        .bg(parse_hex(value))
        .shadow(input_highlight_shadows())
}

fn refresh_select_hover_after_scroll(
    _event: &ScrollWheelEvent,
    window: &mut Window,
    _cx: &mut App,
) {
    window.refresh();
    window.on_next_frame(|window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: window.mouse_position(),
                pressed_button: None,
                modifiers: window.modifiers(),
            }),
            cx,
        );
    });
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::float_cmp,
        reason = "Component tests compare exact deterministic layout constants."
    )]

    use super::*;

    #[test]
    fn frame_select_content_height_includes_vertical_padding() {
        let expected = 3.0_f32.mul_add(
            FRAME_SELECT_OPTION_HEIGHT,
            FRAME_SELECT_CONTENT_PADDING * 2.0,
        );
        assert!((frame_select_content_height(3) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_color_swatch_uses_compact_visual_size() {
        assert_eq!(FRAME_COLOR_SWATCH_SIZE, 14.0);
    }
}
